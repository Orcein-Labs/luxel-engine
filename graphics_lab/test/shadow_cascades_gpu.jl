using Base64
using JSON3
using SHA
using Test
using LuxelGraphics

const CASCADE_GPU_ROOT = normpath(joinpath(@__DIR__, "..", ".."))
const CASCADE_GPU_OUTPUT = get(ENV, "LUXEL_CASCADE_OUTPUT",
    joinpath(CASCADE_GPU_ROOT, "artifacts", "shadow-cascades-20261012"))
mkpath(CASCADE_GPU_OUTPUT)

if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end
using .LavaAdapter

function cascade_gpu_replace(value::T; fields...) where T
    T((get(fields, name, getfield(value, name)) for name in fieldnames(T))...)
end

function cascade_gpu_camera(depth::Float32)
    LuxelGraphics.CameraPacket(
        "cascade-depth-$(round(Int, depth))",
        LuxelGraphics.OrthographicProjection(32.0f0),
        (0.0f0, depth, 0.0f0),
        (0.0f0, -1.0f0, 0.0f0),
        (0.0f0, 0.0f0, -1.0f0),
        0.1f0, 400.0f0, UInt32(256), UInt32(256),
    )
end

function cascade_gpu_mesh()
    positions = NTuple{3,Float32}[
        (-0.8f0, 0.0f0, -0.8f0), (0.8f0, 0.0f0, -0.8f0),
        (0.8f0, 4.0f0, -0.8f0), (-0.8f0, 4.0f0, -0.8f0),
        (-0.8f0, 0.0f0, 0.8f0), (0.8f0, 0.0f0, 0.8f0),
        (0.8f0, 4.0f0, 0.8f0), (-0.8f0, 4.0f0, 0.8f0),
    ]
    indices = UInt32[3, 7, 6, 3, 6, 2, 0, 1, 5, 0, 5, 4,
                     0, 3, 2, 0, 2, 1, 4, 5, 6, 4, 6, 7,
                     1, 2, 6, 1, 6, 5, 0, 4, 7, 0, 7, 3]
    normals = fill((0.0f0, 1.0f0, 0.0f0), length(positions))
    uvs = fill((0.0f0, 0.0f0), length(positions))
    return LuxelGraphics.MeshPacket("cascade-box", positions, normals, uvs, indices, "cascade-material")
end

function cascade_gpu_packet(camera, content; instances=true, cascades=false)
    material = LuxelGraphics.MaterialPacket(
        "cascade-material", (0.48f0, 0.43f0, 0.34f0, 1.0f0),
        0.0f0, 1.0f0, 0.0f0, 0.5f0, :opaque, String[],
        nothing, nothing, nothing, nothing, 1.0f0, 1.0f0, (0.0f0, 0.0f0, 0.0f0),
    )
    terrain = LuxelGraphics.TerrainPacket(
        "cascade-receiver", 120.0f0, 64.0f0, 2, "cascade-material",
        zeros(Float32, 4), zeros(Float32, 4), zeros(UInt8, 4),
    )
    caster_instances = LuxelGraphics.InstancePacket[
        LuxelGraphics.InstancePacket("cascade-caster", "cascade-box", "cascade-material",
            LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((0.0f0, 0.0f0, 0.0f0),
                (0.0f0, 0.0f0, 0.0f0, 1.0f0), (1.0f0, 1.0f0, 1.0f0))),
        LuxelGraphics.InstancePacket("offscreen-caster", "cascade-box", "cascade-material",
            LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((42.0f0, 0.0f0, 0.0f0),
                (0.0f0, 0.0f0, 0.0f0, 1.0f0), (1.0f0, 1.0f0, 1.0f0))),
    ]
    direction = (0.55f0, -0.70f0, 0.45f0)
    lights = [LuxelGraphics.LightPacket("cascade-sun",
        LuxelGraphics.DirectionalLightPacket(direction), (1.0f0, 0.96f0, 0.88f0), 3.0f0)]
    environment = LuxelGraphics.EnvironmentPacket(
        (0.12f0, 0.15f0, 0.20f0), (0.22f0, 0.24f0, 0.26f0),
        (0.12f0, 0.12f0, 0.12f0), (0.18f0, 0.19f0, 0.20f0), 0.0f0, 1.0f0,
    )
    base = LuxelGraphics.GraphicsScenePacket(
        "cascade-gpu", content, "cascade-control-world", "shadow-cascade-control",
        "world-hash", "spatial-hash", UInt64(910), camera, terrain, [material],
        LuxelGraphics.TexturePacket[], [cascade_gpu_mesh()], instances ? caster_instances : LuxelGraphics.InstancePacket[],
        lights, environment, LuxelGraphics.OverlayValue[], "cascade-control", UInt32(256), UInt32(256), false,
        LuxelGraphics.RenderPolicy(),
    )
    cascades || return base
    policy = cascade_gpu_replace(base.render_policy;
        shadow_fit=nothing,
        shadow_cascades=LuxelGraphics.ShadowCascadesPolicy(30, 250, 6, 512, 1024))
    return cascade_gpu_replace(base; render_policy=policy)
end

function cascade_gpu_save_ppm(name, result)
    bytes = base64decode(result.capture_base64)
    @assert length(bytes) == 256 * 256 * 4
    path = joinpath(CASCADE_GPU_OUTPUT, "$(name).ppm")
    open(path, "w") do io
        write(io, "P6\n256 256\n255\n")
        for offset in 1:4:length(bytes)
            write(io, bytes[offset], bytes[offset + 1], bytes[offset + 2])
        end
    end
    return (name=name, ppm="$(name).ppm", render_capture_sha256=result.capture_sha256,
        ppm_sha256="sha256:" * bytes2hex(sha256(read(path))),
        camera_center_xz=[0.0, 0.0], caster_base_xz=[0.0, 0.0],
        direction_xyz=[0.55, -0.70, 0.45], caster_height_m=4.0, pixels_per_meter=8.0)
end

@testset "GPU two-map cascade controls, blend, refit, and legacy identity" begin
    state = LavaAdapter.backend()
    initial_camera = cascade_gpu_camera(28.0f0)
    far_camera = cascade_gpu_camera(105.0f0)
    baseline = cascade_gpu_packet(initial_camera, "legacy-before-cascade")
    baseline_result = LavaAdapter.render_scene(baseline, state)
    rows = NamedTuple[]

    for (name, depth) in (("near", 28.0f0), ("blend-low", 27.0f0),
                          ("blend-center", 30.0f0), ("blend-high", 33.0f0),
                          ("far", 105.0f0))
        camera = cascade_gpu_camera(depth)
        bare = cascade_gpu_packet(camera, "cascade-$(name)-receiver"; instances=false, cascades=true)
        caster = cascade_gpu_packet(camera, "cascade-$(name)-caster"; cascades=true)
        receiver_result = LavaAdapter.render_scene(bare, state)
        caster_result = LavaAdapter.render_scene(caster, state)
        push!(rows, cascade_gpu_save_ppm("$(name)-receiver", receiver_result))
        push!(rows, cascade_gpu_save_ppm("$(name)-caster", caster_result))
        @test state.shadow_resources.far_frame !== nothing
        @test state.shadow_resources.far_texture !== nothing
        @test state.shadow_resources.far_sampler !== nothing
        @test state.cascade_pipeline_compiled[1]
    end

    # Reuse one immutable packet identity across camera A -> B. This mirrors the
    # presentation override path, where the camera changes while scene content
    # keeps its content hash.
    caster_a = cascade_gpu_packet(initial_camera, "cascade-camera-refit"; cascades=true)
    LavaAdapter.render_scene(caster_a, state)
    refit_a = state.shadow_resources
    same_content_far = cascade_gpu_replace(caster_a; camera=far_camera, capture_id="far-camera-override")
    warm_far = LavaAdapter.render_scene(same_content_far, state)
    refit_b = state.shadow_resources
    @test refit_b !== refit_a
    @test refit_b.light_frame !== refit_a.light_frame
    @test refit_b.framebuffer === refit_a.framebuffer
    @test refit_b.texture === refit_a.texture
    @test refit_b.sampler === refit_a.sampler
    @test refit_b.far_framebuffer === refit_a.far_framebuffer
    @test refit_b.far_texture === refit_a.far_texture
    @test refit_b.far_sampler === refit_a.far_sampler
    same_far_again = LavaAdapter.render_scene(same_content_far, state)
    @test same_far_again.capture_sha256 == warm_far.capture_sha256
    @test same_far_again.telemetry.upload_bytes == 0
    @test state.shadow_resources === refit_b

    state.shadow_resources = nothing
    state.mesh_resources = nothing
    state.reflection_mesh_resources = nothing
    state.overlay_resources = nothing
    empty!(state.material_texture_resources)
    cold_far = LavaAdapter.render_scene(same_content_far, state)
    push!(rows, cascade_gpu_save_ppm("far-camera-warm", warm_far))
    push!(rows, cascade_gpu_save_ppm("far-camera-cold", cold_far))
    @test warm_far.capture_sha256 == cold_far.capture_sha256

    # Re-run the no-cascade packet after all cascade pipelines/resources have
    # been exercised. Its historical single-map path remains byte-identical.
    legacy_after = cascade_gpu_packet(initial_camera, "legacy-after-cascade")
    legacy_result = LavaAdapter.render_scene(legacy_after, state)
    @test legacy_result.capture_sha256 == baseline_result.capture_sha256

    manifest = (schema="luxel.shadow-cascade-gpu-control/v1", width_px=256, height_px=256, cases=rows)
    manifest_path = joinpath(CASCADE_GPU_OUTPUT, "measurement-input.json")
    open(manifest_path, "w") do io
        JSON3.write(io, manifest; indent=2)
    end
    helper = joinpath(CASCADE_GPU_ROOT, "tools", "shadow_consistency_measure.py")
    measurement_path = joinpath(CASCADE_GPU_OUTPUT, "measurement.json")
    process = run(ignorestatus(`python3 $helper $manifest_path --output $measurement_path --strict`))
    @test process.exitcode == 0
    measured = JSON3.read(read(measurement_path, String))
    for name in ("near", "blend-low", "blend-center", "blend-high", "far")
        row = only(filter(entry -> String(entry.name) == "$(name)-caster", measured.cases))
        @test row.shadow_pixels >= 8
        @test row.direction_cosine >= 0.85
        @test 1.0 <= row.displacement_m <= 8.0
    end
end
