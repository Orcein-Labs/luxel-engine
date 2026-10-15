using Base64
using Test
using LuxelGraphics
if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end
using .LavaAdapter

function _llgpu_packet(lights, suffix)
    camera = LuxelGraphics.CameraPacket("local-light-camera",
        LuxelGraphics.OrthographicProjection(12f0), (0f0, 12f0, 0f0),
        (0f0, -1f0, 0f0), (0f0, 0f0, -1f0), 0.1f0, 80f0, UInt32(64), UInt32(64))
    terrain = LuxelGraphics.TerrainPacket("local-light-terrain", 12f0, 12f0, 3,
        "local-light-material", zeros(Float32, 9), zeros(Float32, 9), zeros(UInt8, 9))
    material = LuxelGraphics.MaterialPacket("local-light-material",
        (0.45f0, 0.38f0, 0.25f0, 1f0), 0.15f0, 0.48f0, 0.1f0, 0.25f0,
        :opaque, String[], nothing, nothing, nothing, nothing, 1f0, 1f0,
        (0f0, 0f0, 0f0))
    environment = LuxelGraphics.EnvironmentPacket((0f0, 0f0, 0f0),
        (0f0, 0f0, 0f0), (0f0, 0f0, 0f0), (0f0, 0f0, 0f0), 0f0, 1f0)
    return LuxelGraphics.GraphicsScenePacket("packet-" * suffix, "content-" * suffix,
        "local-light-" * suffix, "world", "world-sha", "spatial-sha", UInt64(0),
        camera, terrain, [material], LuxelGraphics.TexturePacket[], LuxelGraphics.MeshPacket[],
        LuxelGraphics.InstancePacket[], lights, environment, LuxelGraphics.OverlayValue[],
        "capture-" * suffix, UInt32(64), UInt32(64), false, LuxelGraphics.RenderPolicy())
end

function _llgpu_replace(value::T; fields...) where T
    T((get(fields, name, getfield(value, name)) for name in fieldnames(T))...)
end

function _llgpu_layers(packet; wet=false, cascades=false, suffix="layers")
    layers = LuxelGraphics.TerrainLayersPacket("ll-layers", "ll-layers-sha",
        [LuxelGraphics.TerrainLayerPacket("base", "local-light-material", UInt32(1000), nothing)],
        "ll-macro")
    zone = wet ? LuxelGraphics.TerrainWetZone((0f0, 0f0), (5f0, 5f0), 1f0,
        0.65f0, 0.3f0, 0.4f0,
        LuxelGraphics.StandingWater((0.018f0, 0.022f0, 0.03f0), 0.08f0, 0f0, 0.4f0, 0f0)) : nothing
    terrain = LuxelGraphics.TerrainPacket("ll-layer-terrain", 12f0, 12f0, 3,
        "local-light-material", zeros(Float32, 9), zeros(Float32, 9), zeros(UInt8, 9), layers, zone)
    macro_texture = LuxelGraphics.TexturePacket("ll-macro", "ll-source", "ll-sha",
        UInt32(1), UInt32(1), UInt32(1), :data, UInt8[128, 128, 128, 255], LuxelGraphics.TextureMipPacket[])
    updated = _llgpu_replace(packet; packet_sha256="packet-" * suffix, content_sha256="content-" * suffix,
        terrain=terrain, textures=[macro_texture])
    policy = _llgpu_replace(updated.render_policy;
        terrain_surface=LuxelGraphics.TerrainSurfacePolicy(1000, true, 0, 40))
    cascades || return _llgpu_replace(updated; render_policy=policy)
    policy = _llgpu_replace(policy;
        shadow_cascades=LuxelGraphics.ShadowCascadesPolicy(10, 30, 4, 512, 512))
    return _llgpu_replace(updated; render_policy=policy)
end

function _llgpu_mesh(packet; alpha=:opaque, coverage=false, cascades=false, suffix="mesh")
    mesh = LuxelGraphics.MeshPacket("ll-mesh",
        NTuple{3,Float32}[(-1f0, 1f0, -1f0), (1f0, 1f0, -1f0), (1f0, 1f0, 1f0), (-1f0, 1f0, 1f0)],
        fill((0f0, 1f0, 0f0), 4), NTuple{2,Float32}[(0f0, 0f0), (1f0, 0f0), (1f0, 1f0), (0f0, 1f0)],
        UInt32[0, 1, 2, 0, 2, 3], "local-light-material")
    base = only(packet.materials)
    material = LuxelGraphics.MaterialPacket(base.material_id, base.base_color_rgba, base.metallic,
        base.roughness, base.clearcoat, base.clearcoat_roughness, alpha, base.texture_ids,
        base.normal_texture_id, base.roughness_texture_id, base.occlusion_texture_id,
        base.emissive_texture_id, base.normal_scale, base.occlusion_strength, base.emissive_factor_rgb,
        alpha === :mask ? 0.5f0 : nothing, false, false)
    instance = LuxelGraphics.InstancePacket("ll-instance", "ll-mesh", material.material_id,
        LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((0f0, 0f0, 0f0),
        (0f0, 0f0, 0f0, 1f0), (1f0, 1f0, 1f0)))
    updated = _llgpu_replace(packet; packet_sha256="packet-" * suffix, content_sha256="content-" * suffix,
        materials=[material], meshes=[mesh], instances=[instance])
    if coverage
        policy = _llgpu_replace(updated.render_policy; foliage_coverage=LuxelGraphics.FoliageCoveragePolicy(4))
        updated = _llgpu_replace(updated; render_policy=policy)
    end
    if cascades
        policy = _llgpu_replace(updated.render_policy;
            shadow_cascades=LuxelGraphics.ShadowCascadesPolicy(10, 30, 4, 512, 512))
        updated = _llgpu_replace(updated; render_policy=policy)
    end
    return updated
end

function _llgpu_luminance_centroid_x(result)
    bytes = base64decode(result.capture_base64)
    weighted = 0.0
    total = 0.0
    for y in 0:63, x in 0:63
        offset = 4 * (y * 64 + x) + 1
        luma = 0.2126 * bytes[offset] + 0.7152 * bytes[offset + 1] + 0.0722 * bytes[offset + 2]
        weighted += (x + 0.5) * luma
        total += luma
    end
    return weighted / max(total, 1.0)
end

function _llgpu_rgb_energy(result)
    pixels = base64decode(result.capture_base64)
    return sum(Int(pixels[index]) for index in eachindex(pixels) if index % 4 != 0)
end

function _llgpu_zero_points(packet, suffix)
    lights = LuxelGraphics.LightPacket[
        LuxelGraphics.LightPacket(light.light_id, light.kind, light.color_rgb,
            light.kind isa LuxelGraphics.PointLightPacket ? 0f0 : light.intensity)
        for light in packet.lights
    ]
    return _llgpu_replace(packet; packet_sha256="packet-" * suffix,
        content_sha256="content-" * suffix, lights=lights)
end

function _llgpu_ibl(packet, suffix)
    environment = LuxelGraphics.TexturePacket(LavaAdapter.IBL_ENVIRONMENT_TEXTURE_ID,
        "ll-ibl", "ll-ibl-environment", UInt32(130), UInt32(130), UInt32(4),
        :linear, fill(UInt8(128), 130 * 130 * 4), LuxelGraphics.TextureMipPacket[])
    lut = LuxelGraphics.TexturePacket(LavaAdapter.IBL_BRDF_LUT_TEXTURE_ID,
        "ll-ibl", "ll-ibl-lut", UInt32(1), UInt32(1), UInt32(4),
        :data, UInt8[255, 0, 0, 255], LuxelGraphics.TextureMipPacket[])
    policy = _llgpu_replace(packet.render_policy; ibl=LuxelGraphics.IblPolicy(true))
    return _llgpu_replace(packet; packet_sha256="packet-" * suffix,
        content_sha256="content-" * suffix, textures=[environment, lut], render_policy=policy)
end

function _llgpu_light(id, color; position=(0f0, 2.6f0, 0f0), range=5.5f0, intensity=18f0)
    return LuxelGraphics.LightPacket(id,
        LuxelGraphics.PointLightPacket(position, range), color, intensity)
end

@testset "GPU point-light single-material smoke and cache stability" begin
    direction = LuxelGraphics.LightPacket("sun",
        LuxelGraphics.DirectionalLightPacket((0f0, -1f0, 0f0)),
        (0f0, 0f0, 0f0), 0f0)
    baseline_packet = _llgpu_packet([direction], "off")
    warm_packet = _llgpu_packet([direction, _llgpu_light("warm", (1f0, 0.38f0, 0.1f0))], "warm")
    state = LavaAdapter.backend()
    baseline = LavaAdapter.render_scene(baseline_packet, state)
    warm = LavaAdapter.render_scene(warm_packet, state)
    warm_again = LavaAdapter.render_scene(warm_packet, state)
    @test baseline.capture_sha256 != warm.capture_sha256
    @test warm.capture_sha256 == warm_again.capture_sha256
    @test state.point_light_resources !== nothing
    @test state.point_pipeline_compiled[1]
    @test warm.telemetry.upload_bytes >= 9 * 16
    pixels = base64decode(warm.capture_base64)
    @test length(pixels) == 64 * 64 * 4

    zero = _llgpu_packet([direction, _llgpu_light("zero", (1f0, 1f0, 1f0); intensity=0f0)], "zero")
    outside = _llgpu_packet([direction, _llgpu_light("outside", (1f0, 1f0, 1f0);
        position=(20f0, 2.6f0, 0f0), range=1f0)], "outside")
    @test LavaAdapter.render_scene(zero, state).capture_sha256 == baseline.capture_sha256
    @test LavaAdapter.render_scene(outside, state).capture_sha256 == baseline.capture_sha256

    cool = _llgpu_packet([direction, _llgpu_light("cool", (0.12f0, 0.48f0, 1f0))], "cool")
    warm_bytes = base64decode(warm.capture_base64)
    cool_bytes = base64decode(LavaAdapter.render_scene(cool, state).capture_base64)
    @test sum(warm_bytes[1:4:end]) > sum(cool_bytes[1:4:end])
    @test sum(cool_bytes[3:4:end]) > sum(warm_bytes[3:4:end])
    left = _llgpu_packet([direction, _llgpu_light("moved", (1f0, 1f0, 1f0); position=(-2f0, 2.6f0, 0f0))], "left")
    right = _llgpu_packet([direction, _llgpu_light("moved", (1f0, 1f0, 1f0); position=(2f0, 2.6f0, 0f0))], "right")
    @test _llgpu_luminance_centroid_x(LavaAdapter.render_scene(left, state)) <
          _llgpu_luminance_centroid_x(LavaAdapter.render_scene(right, state))

    # Compile each opted-in surface specialization and exercise the wet
    # reflection/main pair. These are deliberately small controls; the native
    # gallery owns the full visual comparison set.
    variants = [
        _llgpu_layers(warm_packet; suffix="layered"),
        _llgpu_layers(warm_packet; cascades=true, suffix="layered-cascade"),
        _llgpu_layers(warm_packet; wet=true, suffix="wet"),
        _llgpu_layers(warm_packet; wet=true, cascades=true, suffix="wet-cascade"),
        _llgpu_mesh(warm_packet; suffix="mesh"),
        _llgpu_mesh(warm_packet; cascades=true, suffix="mesh-cascade"),
        _llgpu_mesh(warm_packet; alpha=:mask, suffix="cutout"),
        _llgpu_mesh(warm_packet; alpha=:mask, cascades=true, suffix="cutout-cascade"),
        _llgpu_mesh(warm_packet; alpha=:mask, coverage=true, suffix="coverage"),
        _llgpu_mesh(warm_packet; alpha=:mask, coverage=true, cascades=true, suffix="coverage-cascade"),
    ]
    for variant in variants
        image = LavaAdapter.render_scene(variant, state)
        zero_variant = _llgpu_zero_points(variant, variant.capture_id * "-zero")
        no_point_image = LavaAdapter.render_scene(zero_variant, state)
        @test length(base64decode(image.capture_base64)) == 64 * 64 * 4
        @test image.capture_sha256 != no_point_image.capture_sha256
        @test _llgpu_rgb_energy(image) > _llgpu_rgb_energy(no_point_image)
    end
    @test state.point_pipeline_compiled == fill(true, 12)

    ibl_absent_points = _llgpu_ibl(_llgpu_packet([direction], "ibl-control"), "ibl-control")
    ibl_active_points = _llgpu_ibl(_llgpu_packet([direction,
        _llgpu_light("ibl-warm", (1f0, 0.38f0, 0.1f0))], "ibl-point"), "ibl-point")
    ibl_zero_points = _llgpu_zero_points(ibl_active_points, "ibl-point-zero")
    ibl_reference = LavaAdapter.render_scene(ibl_absent_points, state)
    ibl_active = LavaAdapter.render_scene(ibl_active_points, state)
    ibl_zero = LavaAdapter.render_scene(ibl_zero_points, state)
    @test ibl_active.capture_sha256 != ibl_reference.capture_sha256
    @test _llgpu_rgb_energy(ibl_active) > _llgpu_rgb_energy(ibl_reference)
    @test ibl_zero.capture_sha256 == ibl_reference.capture_sha256
end
