using Base64
using JSON3
using SHA
using Test
using LuxelGraphics

const SHADOW_TEST_ROOT = normpath(joinpath(@__DIR__, "..", ".."))
const SHADOW_ADAPTER_PATH = abspath(get(ENV, "LUXEL_SHADOW_ADAPTER", joinpath(@__DIR__, "..", "src", "LavaAdapter.jl")))
if !isdefined(@__MODULE__, :LavaAdapter)
    include(SHADOW_ADAPTER_PATH)
end
using .LavaAdapter

const SHADOW_ARTIFACTS = get(ENV, "LUXEL_SHADOW_OUTPUT",
    joinpath(SHADOW_TEST_ROOT, "artifacts", "shadow-consistency-20261012", "corrected"))
mkpath(SHADOW_ARTIFACTS)

function shadow_replace(value::T; fields...) where T
    T((get(fields, name, getfield(value, name)) for name in fieldnames(T))...)
end

function shadow_box_mesh()
    # One closed 1.6 x 4.0 x 1.6 m prism. The top is high enough to cast a
    # measurable, several-pixel shadow while remaining inside the sparse view.
    positions = NTuple{3,Float32}[
        (-0.8f0, 0.0f0, -0.8f0), (0.8f0, 0.0f0, -0.8f0),
        (0.8f0, 4.0f0, -0.8f0), (-0.8f0, 4.0f0, -0.8f0),
        (-0.8f0, 0.0f0, 0.8f0), (0.8f0, 0.0f0, 0.8f0),
        (0.8f0, 4.0f0, 0.8f0), (-0.8f0, 4.0f0, 0.8f0),
    ]
    indices = UInt32[
        3, 7, 6, 3, 6, 2,  # top
        0, 1, 5, 0, 5, 4,  # bottom
        0, 3, 2, 0, 2, 1,  # -Z
        4, 5, 6, 4, 6, 7,  # +Z
        1, 2, 6, 1, 6, 5,  # +X
        0, 4, 7, 0, 7, 3,  # -X
    ]
    normals = fill((0.0f0, 1.0f0, 0.0f0), length(positions))
    uv = fill((0.0f0, 0.0f0), length(positions))
    return LuxelGraphics.MeshPacket("shadow-caster", positions, normals, uv, indices, "shadow-material")
end

function shadow_camera(x::Float32)
    LuxelGraphics.CameraPacket(
        "shadow-view-$(round(Int, x))",
        LuxelGraphics.OrthographicProjection(32.0f0),
        (x, 28.0f0, 0.0f0),
        (0.0f0, -1.0f0, 0.0f0),
        (0.0f0, 0.0f0, -1.0f0),
        0.1f0, 80.0f0, UInt32(256), UInt32(256),
    )
end

function shadow_packet(camera, direction; content="shadow-consistency", instances=true)
    material = LuxelGraphics.MaterialPacket(
        "shadow-material", (0.48f0, 0.43f0, 0.34f0, 1.0f0),
        0.0f0, 1.0f0, 0.0f0, 0.5f0, :opaque, String[],
        nothing, nothing, nothing, nothing, 1.0f0, 1.0f0, (0.0f0, 0.0f0, 0.0f0),
    )
    terrain = LuxelGraphics.TerrainPacket(
        "flat-shadow-receiver", 120.0f0, 64.0f0, 2, "shadow-material",
        zeros(Float32, 4), zeros(Float32, 4), zeros(UInt8, 4),
    )
    caster = LuxelGraphics.InstancePacket[
        LuxelGraphics.InstancePacket("caster-west", "shadow-caster", "shadow-material",
            LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((0.0f0, 0.0f0, 0.0f0),
                (0.0f0, 0.0f0, 0.0f0, 1.0f0), (1.0f0, 1.0f0, 1.0f0))),
        LuxelGraphics.InstancePacket("caster-east", "shadow-caster", "shadow-material",
            LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((42.0f0, 0.0f0, 0.0f0),
                (0.0f0, 0.0f0, 0.0f0, 1.0f0), (1.0f0, 1.0f0, 1.0f0))),
    ]
    lights = [LuxelGraphics.LightPacket("sun",
        LuxelGraphics.DirectionalLightPacket(direction), (1.0f0, 0.96f0, 0.88f0), 3.0f0)]
    environment = LuxelGraphics.EnvironmentPacket(
        (0.12f0, 0.15f0, 0.20f0), (0.22f0, 0.24f0, 0.26f0),
        (0.12f0, 0.12f0, 0.12f0), (0.18f0, 0.19f0, 0.20f0), 0.0f0, 1.0f0,
    )
    policy = LuxelGraphics.RenderPolicy(
        LuxelGraphics.GradePolicy(), LuxelGraphics.BloomPolicy(), LuxelGraphics.VignettePolicy(),
        LuxelGraphics.DitherPolicy(), LuxelGraphics.TerrainSurfacePolicy(), LuxelGraphics.SamplerPolicy(),
        LuxelGraphics.ShadowPolicy(), LuxelGraphics.MeshSurfacePolicy(),
        LuxelGraphics.ShadowFitPolicy(36, 1024), nothing,
    )
    return LuxelGraphics.GraphicsScenePacket(
        "packet-shadow-consistency", content, "shadow-consistency", "shadow-control-world",
        "world-hash", "spatial-hash", UInt64(77), camera,
        terrain, [material], LuxelGraphics.TexturePacket[], [shadow_box_mesh()],
        instances ? caster : LuxelGraphics.InstancePacket[], lights, environment,
        LuxelGraphics.OverlayValue[], "shadow-consistency", UInt32(256), UInt32(256), false, policy,
    )
end

function shadow_write_ppm(path, capture_base64)
    bytes = base64decode(capture_base64)
    @assert length(bytes) == 256 * 256 * 4
    open(path, "w") do io
        write(io, "P6\n256 256\n255\n")
        for offset in 1:4:length(bytes)
            write(io, bytes[offset], bytes[offset + 1], bytes[offset + 2])
        end
    end
    return "sha256:" * bytes2hex(sha256(read(path)))
end

function shadow_save_capture!(rows, name, result, camera_x, caster_x, direction)
    ppm = "$(name).ppm"
    digest = shadow_write_ppm(joinpath(SHADOW_ARTIFACTS, ppm), result.capture_base64)
    push!(rows, (name=name, ppm=ppm, ppm_sha256=digest,
        render_capture_sha256=result.capture_sha256, camera_x=camera_x,
        caster_x=caster_x, direction_xyz=collect(direction), pixels_per_meter=8.0))
    return result
end

@testset "sparse caster shadow direction and camera-fitted continuity" begin
    state = LavaAdapter.backend()
    rows = NamedTuple[]
    directions = (
        (name="sun-east-north", vector=(0.55f0, -0.70f0, 0.45f0)),
        (name="sun-west-south", vector=(-0.55f0, -0.70f0, -0.45f0)),
    )
    view_a = shadow_camera(0.0f0)
    view_b = shadow_camera(42.0f0)
    packet_for_b = nothing
    lighting_for_b = nothing

    for (index, sun) in enumerate(directions)
        bare = shadow_packet(view_a, sun.vector; content="direction-$(index)-receiver", instances=false)
        caster = shadow_packet(view_a, sun.vector; content="direction-$(index)-caster")
        bare_result = LavaAdapter.render_scene(bare, state)
        caster_result = LavaAdapter.render_scene(caster, state)
        shadow_save_capture!(rows, "$(sun.name)-receiver", bare_result, 0.0, 0.0, sun.vector)
        shadow_save_capture!(rows, "$(sun.name)-caster", caster_result, 0.0, 0.0, sun.vector)
        if index == 1
            # Same immutable content identity, changed camera: this is the
            # presentation override behavior the recent continuity slice uses.
            packet_for_b = shadow_replace(caster; camera=view_b, capture_id="camera-b")
            lighting_for_b = LavaAdapter._lighting(caster)
            warm_b = LavaAdapter.render_scene(packet_for_b, state)
            warm_b_shadow = state.shadow_resources
            shadow_save_capture!(rows, "camera-b-warm-after-a", warm_b, 42.0, 42.0, sun.vector)
            repeated_b = LavaAdapter.render_scene(packet_for_b, state)
            @test repeated_b.capture_sha256 == warm_b.capture_sha256
            @test repeated_b.telemetry.upload_bytes == 0
            @test state.shadow_resources === warm_b_shadow
            # Force cold view resources while preserving the same packet data.
            state.mesh_resources = nothing
            state.reflection_mesh_resources = nothing
            empty!(state.mesh_geometry_cache)
            state.overlay_resources = nothing
            state.shadow_resources = nothing
            # The material descriptor embeds the shadow texture handle. Rebuild
            # it as well so the cold render samples its new shadow allocation.
            empty!(state.material_texture_resources)
            cold_b = LavaAdapter.render_scene(packet_for_b, state)
            shadow_save_capture!(rows, "camera-b-cold", cold_b, 42.0, 42.0, sun.vector)
            @test warm_b.capture_sha256 == cold_b.capture_sha256

            frame_b = LavaAdapter._camera_frame(view_b)
            sun_forward, sun_right, sun_up = LavaAdapter._basis_vectors(
                lighting_for_b.direction,
                LavaAdapter._shadow_up_hint(lighting_for_b.direction), "shadow test sun")
            fitted_b = LavaAdapter._view_fitted_shadow_frame(packet_for_b.camera, sun_forward,
                sun_right, sun_up,
                Float32(packet_for_b.render_policy.shadow_fit.view_distance_m))
            if applicable(LavaAdapter._shadow_resources!, state, packet_for_b, lighting_for_b, frame_b)
                shadow_resource_b = LavaAdapter._shadow_resources!(state, packet_for_b, lighting_for_b, frame_b)
                @test shadow_resource_b.light_frame == fitted_b
                @test shadow_resource_b.light_frame != LavaAdapter._shadow_frame(caster, lighting_for_b)
                state.shadow_resources = nothing
                shadow_resource_a = LavaAdapter._shadow_resources!(state, caster, lighting_for_b,
                    LavaAdapter._camera_frame(caster.camera))
                refit_b = LavaAdapter._shadow_resources!(state, packet_for_b, lighting_for_b, frame_b)
                @test refit_b !== shadow_resource_a
                @test refit_b.framebuffer === shadow_resource_a.framebuffer
                @test refit_b.texture === shadow_resource_a.texture
                @test refit_b.sampler === shadow_resource_a.sampler
                @test refit_b.light_frame == fitted_b
                @test LavaAdapter._shadow_resources!(state, packet_for_b, lighting_for_b, frame_b) === refit_b

                legacy_a = shadow_replace(caster; content_sha256="shadow-consistency-whole-world",
                    render_policy=LuxelGraphics.RenderPolicy())
                legacy_b = shadow_replace(legacy_a; camera=view_b)
                state.shadow_resources = nothing
                legacy_resource_a = LavaAdapter._shadow_resources!(state, legacy_a, lighting_for_b,
                    LavaAdapter._camera_frame(legacy_a.camera))
                legacy_resource_b = LavaAdapter._shadow_resources!(state, legacy_b, lighting_for_b, frame_b)
                @test legacy_resource_b === legacy_resource_a
            else
                # The frozen source snapshot used for the regression baseline
                # predates the view-frame overload; warm/cold equality above is
                # the intended baseline failure and remains the primary signal.
                @info "adapter has no view-frame shadow resource entry point (baseline)"
            end

            # The actual presented override must update the shared shadow fit.
            # Prior direct resource checks intentionally replaced the resource
            # object, so rebuild descriptors before starting this entry path.
            empty!(state.material_texture_resources)
            LavaAdapter.open_window_session(state, 256, 256; vsync=false)
            try
                @test LavaAdapter.render_window_frames!(state, caster, nothing, 1).frames_presented == 1
                @test LavaAdapter.render_window_frames!(state, caster, view_b, 1).frames_presented == 1
                @test state.shadow_resources.light_frame == fitted_b
            finally
                @test LavaAdapter.close_window_session!(state).closed
            end
            camera_b_after_window = LavaAdapter.render_scene(packet_for_b, state)
            @test camera_b_after_window.capture_sha256 == cold_b.capture_sha256
        end
    end

    manifest = (schema="luxel.shadow-consistency-control/v1", width_px=256, height_px=256,
        cases=[(name=row.name, ppm=row.ppm, ppm_sha256=row.ppm_sha256,
            render_capture_sha256=row.render_capture_sha256, camera_center_xz=[row.camera_x, 0.0],
            caster_base_xz=[row.caster_x, 0.0], direction_xyz=row.direction_xyz,
            caster_height_m=4.0, pixels_per_meter=row.pixels_per_meter) for row in rows])
    manifest_path = joinpath(SHADOW_ARTIFACTS, "measurement-input.json")
    open(manifest_path, "w") do io
        JSON3.write(io, manifest; indent=2)
    end
    helper = joinpath(SHADOW_TEST_ROOT, "tools", "shadow_consistency_measure.py")
    measurement_path = joinpath(SHADOW_ARTIFACTS, "measurement.json")
    command = `python3 $helper $manifest_path --output $measurement_path --strict`
    measurement_process = run(ignorestatus(command))
    @test measurement_process.exitcode == 0
    measurements = JSON3.read(read(joinpath(SHADOW_ARTIFACTS, "measurement.json"), String))
    by_name = Dict(String(row.name) => row for row in measurements.cases)
    for sun in directions
        measured = by_name["$(sun.name)-caster"]
        @test measured.shadow_pixels >= 8
        @test measured.direction_cosine >= 0.85
        @test measured.displacement_m >= 1.0
        @test measured.displacement_m <= 8.0
        @test measured.unmasked_geometric_reference_error_m <= 2.0
        @test measured.opposite_direction_rejected
        @info "sparse shadow direction" sun=sun.name pixels=measured.shadow_pixels displacement_m=measured.displacement_m cosine=measured.direction_cosine
    end
    @test measurements.warm_cold_camera_b_equal
    @test by_name["camera-b-warm-after-a"].render_capture_sha256 == by_name["camera-b-cold"].render_capture_sha256

    negative_output = joinpath(SHADOW_ARTIFACTS, "measurement-negative-control.json")
    negative_command = `python3 $helper $manifest_path --negative-control --output $negative_output`
    negative_process = run(ignorestatus(negative_command))
    @test negative_process.exitcode == 2
    negative_measurement = JSON3.read(read(negative_output, String))
    @test negative_measurement.negative_control_passed
    @test all(row -> !row.accepted,
        filter(row -> endswith(String(row.name), "-caster"), negative_measurement.cases))
end
