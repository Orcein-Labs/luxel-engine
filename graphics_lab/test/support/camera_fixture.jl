using Base64, SHA, Test, LuxelGraphics
if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "..", "src", "LavaAdapter.jl"))
end
using .LavaAdapter

# Preserve packet identity while supplying the camera used by the preview path.
function camera_test_replace(value::T; fields...) where T
    T((get(fields, name, getfield(value, name)) for name in fieldnames(T))...)
end

function camera_test_packet()
    mesh = LuxelGraphics.MeshPacket("rotated-bounds-mesh",
        [(-2f0, 1f0, -2f0), (2f0, 1f0, -2f0), (0f0, 1f0, 2f0)],
        fill((0f0, 1f0, 0f0), 3), [(0f0, 0f0), (1f0, 0f0), (0.5f0, 1f0)],
        UInt32[0, 2, 1], "bounds-material")
    return LuxelGraphics.GraphicsScenePacket(
        "packet-sha",
        "content-sha",
        "bounds-packet",
        "world",
        "world-sha",
        "spatial-sha",
        UInt64(1),
        LuxelGraphics.CameraPacket(
            "camera",
            LuxelGraphics.OrthographicProjection(16.0f0),
            (0.0f0, 8.0f0, 0.0f0),
            (0.0f0, -1.0f0, 0.0f0),
            (0.0f0, 0.0f0, -1.0f0),
            0.1f0,
            100.0f0,
            UInt32(64),
            UInt32(64),
        ),
        LuxelGraphics.TerrainPacket(
            "terrain",
            16.0f0,
            16.0f0,
            2,
            "bounds-material",
            Float32[0.0f0, 0.0f0, 0.0f0, 0.0f0],
            Float32[0.0f0, 0.0f0, 0.0f0, 0.0f0],
            UInt8[0, 0, 0, 0],
        ),
        [LuxelGraphics.MaterialPacket(
            "bounds-material",
            (0.5f0, 0.5f0, 0.5f0, 1.0f0),
            0.0f0,
            1.0f0,
            0.0f0,
            0.5f0,
            :opaque,
            String[],
            nothing,
            nothing,
            nothing,
            nothing,
            1.0f0,
            1.0f0,
            (0.0f0, 0.0f0, 0.0f0),
        )],
        LuxelGraphics.TexturePacket[],
        [mesh],
        LuxelGraphics.InstancePacket[],
        [LuxelGraphics.LightPacket(
            "sun",
            LuxelGraphics.DirectionalLightPacket((0.0f0, -1.0f0, 0.0f0)),
            (1.0f0, 1.0f0, 1.0f0),
            1.0f0,
        )],
        LuxelGraphics.EnvironmentPacket(
            (0.1f0, 0.1f0, 0.1f0),
            (0.2f0, 0.2f0, 0.2f0),
            (0.05f0, 0.05f0, 0.05f0),
            (0.1f0, 0.1f0, 0.1f0),
            0.0f0,
            1.0f0,
        ),
        LuxelGraphics.OverlayValue[],
        "capture",
        UInt32(64),
        UInt32(64),
        false,
        LuxelGraphics.RenderPolicy(),
    )
end

function camera_test_scene()
    packet = camera_test_packet()
    instances = [LuxelGraphics.InstancePacket("instance-$x", "rotated-bounds-mesh", "bounds-material",
        LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((x, 0f0, 0f0),
            (0f0, 0f0, 0f0, 1f0), (1f0, 1f0, 1f0))) for x in (0f0, 40f0)]
    overlays = LuxelGraphics.OverlayValue[
        LuxelGraphics.PointOverlay("marker-$x", :objective, (x, 2f0, x == 0f0 ? 3f0 : -3f0), 0.5f0, (1f0, 0f0, 0f0, 1f0))
        for x in (0f0, 40f0)]
    return camera_test_replace(packet; instances, overlays)
end

function camera_test_clear_view!(state)
    state.mesh_resources = nothing
    state.reflection_mesh_resources = nothing
    empty!(state.mesh_geometry_cache)
    state.overlay_resources = nothing
end

