using Base64
using Test
using LuxelGraphics
if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end
using .LavaAdapter

function _lsgpu_replace(value::T; fields...) where T
    T((get(fields, name, getfield(value, name)) for name in fieldnames(T))...)
end

function _lsgpu_box()
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
    return LuxelGraphics.MeshPacket("ls-box", positions, normals, uvs, indices, "ls-material")
end

function _lsgpu_packet(content; caster=true, policy=true, light_x=0f0, caster_x=0f0)
    camera = LuxelGraphics.CameraPacket("ls-camera",
        LuxelGraphics.OrthographicProjection(12f0), (0f0, 12f0, 0f0),
        (0f0, -1f0, 0f0), (0f0, 0f0, -1f0), 0.1f0, 80f0, UInt32(64), UInt32(64))
    material = LuxelGraphics.MaterialPacket("ls-material",
        (0.45f0, 0.38f0, 0.25f0, 1f0), 0.15f0, 0.48f0, 0.1f0, 0.25f0,
        :opaque, String[], nothing, nothing, nothing, nothing, 1f0, 1f0,
        (0f0, 0f0, 0f0))
    terrain = LuxelGraphics.TerrainPacket("ls-terrain", 12f0, 12f0, 3,
        "ls-material", zeros(Float32, 9), zeros(Float32, 9), zeros(UInt8, 9))
    lights = LuxelGraphics.LightPacket[
        LuxelGraphics.LightPacket("sun", LuxelGraphics.DirectionalLightPacket((0f0, -1f0, 0f0)),
            (0f0, 0f0, 0f0), 0f0),
        LuxelGraphics.LightPacket("lamp", LuxelGraphics.PointLightPacket((light_x, 3f0, 0f0), 8f0),
            (1f0, 1f0, 1f0), 18f0),
    ]
    instances = caster ? LuxelGraphics.InstancePacket[
        LuxelGraphics.InstancePacket("ls-caster", "ls-box", "ls-material",
            LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((caster_x, 0f0, 0f0),
            (0f0, 0f0, 0f0, 1f0), (1f0, 1f0, 1f0))),
    ] : LuxelGraphics.InstancePacket[]
    environment = LuxelGraphics.EnvironmentPacket((0f0, 0f0, 0f0),
        (0f0, 0f0, 0f0), (0f0, 0f0, 0f0), (0f0, 0f0, 0f0), 0f0, 1f0)
    render_policy = policy ?
        _lsgpu_replace(LuxelGraphics.RenderPolicy();
            point_shadows=LuxelGraphics.PointShadowsPolicy(256)) :
        LuxelGraphics.RenderPolicy()
    return LuxelGraphics.GraphicsScenePacket("packet-" * content, "content-" * content,
        "ls-" * content, "world", "world-sha", "spatial-sha", UInt64(0),
        camera, terrain, [material], LuxelGraphics.TexturePacket[], [_lsgpu_box()],
        instances, lights, environment, LuxelGraphics.OverlayValue[],
        "capture-" * content, UInt32(64), UInt32(64), false, render_policy)
end

function _lsgpu_ring_luma(result)
    bytes = base64decode(result.capture_base64)
    total = 0.0
    count = 0
    for y in 0:63, x in 0:63
        dx = x + 0.5 - 32.0
        dy = y + 0.5 - 32.0
        r = sqrt(dx * dx + dy * dy)
        (8.0 <= r <= 16.0) || continue
        offset = 4 * (y * 64 + x) + 1
        total += 0.2126 * bytes[offset] + 0.7152 * bytes[offset + 1] + 0.0722 * bytes[offset + 2]
        count += 1
    end
    return total / max(count, 1)
end

function _lsgpu_dark_centroid_x(result)
    bytes = base64decode(result.capture_base64)
    weighted = 0.0
    total = 0.0
    for y in 16:47, x in 0:63
        offset = 4 * (y * 64 + x) + 1
        luma = 0.2126 * bytes[offset] + 0.7152 * bytes[offset + 1] + 0.0722 * bytes[offset + 2]
        darkness = max(40.0 - luma, 0.0)
        weighted += (x + 0.5) * darkness
        total += darkness
    end
    return weighted / max(total, 1.0)
end

@testset "GPU point-shadow occluder, identity, and movement" begin
    state = LavaAdapter.backend()
    unshadowed = LavaAdapter.render_scene(_lsgpu_packet("unshadowed"; policy=false), state)
    unshadowed_again = LavaAdapter.render_scene(_lsgpu_packet("unshadowed"; policy=false), state)
    @test unshadowed.capture_sha256 == unshadowed_again.capture_sha256

    no_points = _lsgpu_replace(_lsgpu_packet("unshadowed"; policy=false);
        packet_sha256="packet-no-points", content_sha256="content-no-points",
        lights=[LuxelGraphics.LightPacket("sun",
            LuxelGraphics.DirectionalLightPacket((0f0, -1f0, 0f0)), (0f0, 0f0, 0f0), 0f0)])
    zero_points = _lsgpu_replace(_lsgpu_packet("unshadowed"; policy=false);
        packet_sha256="packet-zero-points", content_sha256="content-zero-points",
        lights=[
            LuxelGraphics.LightPacket("sun", LuxelGraphics.DirectionalLightPacket((0f0, -1f0, 0f0)),
                (0f0, 0f0, 0f0), 0f0),
            LuxelGraphics.LightPacket("lamp", LuxelGraphics.PointLightPacket((0f0, 3f0, 0f0), 8f0),
                (1f0, 1f0, 1f0), 0f0),
        ])
    @test LavaAdapter.render_scene(no_points, state).capture_sha256 ==
          LavaAdapter.render_scene(zero_points, state).capture_sha256

    shadowed = LavaAdapter.render_scene(_lsgpu_packet("shadowed"), state)
    receiver = LavaAdapter.render_scene(_lsgpu_packet("receiver"; caster=false), state)
    unshadowed_caster = unshadowed
    @test _lsgpu_ring_luma(shadowed) < _lsgpu_ring_luma(receiver)
    @test _lsgpu_ring_luma(shadowed) < _lsgpu_ring_luma(unshadowed_caster)
    @test shadowed.capture_sha256 != unshadowed_caster.capture_sha256

    moved_caster = LavaAdapter.render_scene(_lsgpu_packet("moved-caster"; caster_x=4f0), state)
    @test _lsgpu_ring_luma(moved_caster) > _lsgpu_ring_luma(shadowed)

    left = LavaAdapter.render_scene(_lsgpu_packet("light-left"; light_x=-2.5f0), state)
    right = LavaAdapter.render_scene(_lsgpu_packet("light-right"; light_x=2.5f0), state)
    # Central caster: the umbra is thrown away from the light.
    @test _lsgpu_dark_centroid_x(left) > _lsgpu_dark_centroid_x(right)

    disabled = LavaAdapter.render_scene(_lsgpu_packet("disabled"; policy=false), state)
    @test _lsgpu_ring_luma(disabled) > _lsgpu_ring_luma(shadowed)
    @test state.dummy_point_shadows !== nothing
    @test state.point_shadow_resources !== nothing
    @test state.point_shadow_resources.enabled
end
