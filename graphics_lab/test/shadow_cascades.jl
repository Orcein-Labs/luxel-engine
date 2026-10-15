using Test
using LuxelGraphics

const CASCADE_ADAPTER_PATH = joinpath(@__DIR__, "..", "src", "LavaAdapter.jl")
if !isdefined(@__MODULE__, :LavaAdapter)
    include(CASCADE_ADAPTER_PATH)
end
using .LavaAdapter
using GeometryBasics: Vec4f

function cascade_test_camera(x::Float32=0.0f0)
    camera = LuxelGraphics.CameraPacket(
        "cascade-camera",
        LuxelGraphics.OrthographicProjection(32.0f0),
        (x, 12.0f0, 0.0f0),
        (0.0f0, 0.0f0, -1.0f0),
        (0.0f0, 1.0f0, 0.0f0),
        0.1f0, 400.0f0, UInt32(1280), UInt32(720),
    )
    return LavaAdapter._camera_frame(camera)
end

function campaign2_perspective_camera()
    camera = LuxelGraphics.CameraPacket(
        "campaign2-medium",
        LuxelGraphics.PerspectiveProjection(50.0f0),
        (14.0f0, 8.0f0, 18.0f0),
        (-15.5f0, -5.8f0, -17.0f0),
        (0.0f0, 1.0f0, 0.0f0),
        0.1f0, 400.0f0, UInt32(960), UInt32(640),
    )
    return LavaAdapter._camera_frame(camera)
end

function cascade_slice_corners(frame, first_depth, last_depth)
    corners = Vec4f[]
    for depth in (Float32(first_depth), Float32(last_depth))
        half_width = frame.mode < 0.5f0 ? frame.projection[1] : frame.projection[1] * depth
        half_height = frame.mode < 0.5f0 ? frame.projection[2] : frame.projection[2] * depth
        for sx in (-1.0f0, 1.0f0), sy in (-1.0f0, 1.0f0)
            push!(corners, Vec4f(
                frame.position[1] + frame.forward[1] * depth + frame.right[1] * sx * half_width + frame.up[1] * sy * half_height,
                frame.position[2] + frame.forward[2] * depth + frame.right[2] * sx * half_width + frame.up[2] * sy * half_height,
                frame.position[3] + frame.forward[3] * depth + frame.right[3] * sx * half_width + frame.up[3] * sy * half_height,
                0.0f0,
            ))
        end
    end
    return corners
end

@testset "cascade geometry, stable scale, and fragment overlap interpolation" begin
    policy = LuxelGraphics.ShadowCascadesPolicy()
    camera = cascade_test_camera()
    light = LavaAdapter._normalize_vector(Vec4f(0.42f0, -0.81f0, 0.40f0, 0.0f0))
    frames = LavaAdapter._shadow_cascade_frames(camera, light, policy)

    @test frames.blend_start_m == 27.0f0
    @test frames.blend_end_m == 33.0f0
    @test Int(policy.near_map_size_px) / (2 * frames.near.projection[1]) >= 20.0f0

    for (fit, first_depth, last_depth) in (
        (frames.near, min(camera.projection[3], frames.blend_start_m), frames.blend_end_m),
        (frames.far, frames.blend_start_m, Float32(policy.far_distance_m)),
    )
        for corner in cascade_slice_corners(camera, first_depth, last_depth)
            ndc = LavaAdapter._project_world(
                corner, fit.position, fit.right, fit.up, fit.forward, fit.projection, fit.mode,
            )
            @test -1.0f0 <= ndc[1] <= 1.0f0
            @test -1.0f0 <= ndc[2] <= 1.0f0
            @test 0.0f0 <= ndc[3] <= 1.0f0
        end
    end

    moved = cascade_test_camera(0.25f0)
    moved_frames = LavaAdapter._shadow_cascade_frames(moved, light, policy)
    @test frames.near.projection[1] == moved_frames.near.projection[1]
    @test frames.far.projection[1] == moved_frames.far.projection[1]
    @test isapprox(LavaAdapter._dot_vector(frames.near.position, frames.near.right) /
                   (2 * frames.near.projection[1] / policy.near_map_size_px),
                   round(LavaAdapter._dot_vector(frames.near.position, frames.near.right) /
                         (2 * frames.near.projection[1] / policy.near_map_size_px)); atol=1.0f-4)

    campaign_camera = campaign2_perspective_camera()
    campaign_light = LavaAdapter._normalize_vector(Vec4f(0.38f0, -1.0f0, -0.32f0, 0.0f0))
    campaign_frames = LavaAdapter._shadow_cascade_frames(campaign_camera, campaign_light, policy)
    @test Int(policy.near_map_size_px) / (2 * campaign_frames.near.projection[1]) >= 20.0f0
    for (fit, first_depth, last_depth) in (
        (campaign_frames.near, min(campaign_camera.projection[3], campaign_frames.blend_start_m), campaign_frames.blend_end_m),
        (campaign_frames.far, campaign_frames.blend_start_m, Float32(policy.far_distance_m)),
    )
        for corner in cascade_slice_corners(campaign_camera, first_depth, last_depth)
            ndc = LavaAdapter._project_world(
                corner, fit.position, fit.right, fit.up, fit.forward, fit.projection, fit.mode,
            )
            @test -1.0f0 <= ndc[1] <= 1.0f0
            @test -1.0f0 <= ndc[2] <= 1.0f0
            @test 0.0f0 <= ndc[3] <= 1.0f0
        end
    end

    # One coarse perspective triangle spans from 5 m to 90 m. At screen
    # barycentrics (2/17, 15/34, 15/34), perspective-correct interpolation of the
    # per-vertex linear view depth reconstructs 30 m. Smoothstep must therefore
    # return 0.5. Interpolating pre-smoothed vertex weights returns 0.6.
    vertex_depths = (5.0f0, 90.0f0, 90.0f0)
    screen_weights = (2.0f0 / 17.0f0, 15.0f0 / 34.0f0, 15.0f0 / 34.0f0)
    fragment_depth = inv(sum(screen_weights[i] / vertex_depths[i] for i in 1:3))
    normalized = (fragment_depth - 27.0f0) / 6.0f0
    correct_blend = LavaAdapter._cascade_overlap_blend(normalized)
    wrong_vertex_blend = sum(screen_weights[i] *
        LavaAdapter._cascade_overlap_blend((vertex_depths[i] - 27.0f0) / 6.0f0) for i in 1:3)
    @test fragment_depth ≈ 30.0f0 atol=1.0f-5
    @test correct_blend ≈ 0.5f0 atol=1.0f-5
    @test wrong_vertex_blend ≈ 15.0f0 / 17.0f0 atol=1.0f-5
    @test abs(correct_blend - wrong_vertex_blend) > 0.38f0
end
