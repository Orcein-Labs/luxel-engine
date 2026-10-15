"""A pair of light-space fits for one camera frame and one directional light."""
struct ShadowCascadeFrames
    near::CameraFrame
    far::CameraFrame
    blend_start_m::Float32
    blend_end_m::Float32
end

"""Fit stable, texel-snapped orthographic frames for the requested split.

The near frame reaches to split + half the blend width. The far frame starts
at split - half the blend width and always reaches `far_distance_m`, even when
the current camera's far plane is shorter. Each extent is derived from the
slice shape, so translating a fixed camera does not change its map scale.
"""
function _shadow_cascade_frames(
    camera::CameraFrame,
    light_direction::Vec4f,
    policy::LuxelGraphics.ShadowCascadesPolicy,
)::ShadowCascadeFrames
    raw_direction = light_direction
    light_forward, light_right, light_up = _basis_vectors(
        raw_direction,
        _shadow_up_hint(raw_direction),
        "cascade directional light basis",
    )
    split = Float32(policy.near_distance_m)
    half_blend = 0.5f0 * Float32(policy.blend_width_m)
    blend_start = split - half_blend
    blend_end = split + half_blend
    near_start = min(camera.projection[3], blend_start)
    far_end = Float32(policy.far_distance_m)
    near_start < blend_end || throw(AdapterError(
        "unsupported_render_policy",
        "render_policy.shadow_cascades near fit does not reach its overlap band",
    ))
    far_end > blend_start || throw(AdapterError(
        "unsupported_render_policy",
        "render_policy.shadow_cascades far fit does not reach its overlap band",
    ))
    near_frame = _view_fitted_shadow_slice(
        camera, light_forward, light_right, light_up, near_start, blend_end,
        Int(policy.near_map_size_px),
    )
    far_frame = _view_fitted_shadow_slice(
        camera, light_forward, light_right, light_up, blend_start, far_end,
        Int(policy.far_map_size_px),
    )
    return ShadowCascadeFrames(near_frame, far_frame, blend_start, blend_end)
end

function _view_fitted_shadow_slice(
    camera::CameraFrame,
    light_direction::Vec4f,
    right::Vec4f,
    up::Vec4f,
    start_depth::Float32,
    end_depth::Float32,
    map_size::Int,
)::CameraFrame
    0.0f0 < start_depth < end_depth || throw(AdapterError(
        "unsupported_render_policy",
        "shadow cascade slice must have positive ordered depth bounds",
    ))
    map_size > 2 || throw(AdapterError(
        "unsupported_render_policy",
        "shadow cascade map size must be positive",
    ))
    corners = Vec4f[]
    for depth in (start_depth, end_depth)
        half_width = camera.mode < 0.5f0 ? camera.projection[1] : camera.projection[1] * depth
        half_height = camera.mode < 0.5f0 ? camera.projection[2] : camera.projection[2] * depth
        for sx in (-1.0f0, 1.0f0), sy in (-1.0f0, 1.0f0)
            push!(corners, Vec4f(
                camera.position[1] + camera.forward[1] * depth + camera.right[1] * sx * half_width + camera.up[1] * sy * half_height,
                camera.position[2] + camera.forward[2] * depth + camera.right[2] * sx * half_width + camera.up[2] * sy * half_height,
                camera.position[3] + camera.forward[3] * depth + camera.right[3] * sx * half_width + camera.up[3] * sy * half_height,
                0.0f0,
            ))
        end
    end
    center = Vec4f(
        sum(corner[1] for corner in corners) / 8.0f0,
        sum(corner[2] for corner in corners) / 8.0f0,
        sum(corner[3] for corner in corners) / 8.0f0,
        0.0f0,
    )
    depth_radius = 1.0f0
    half_extent = 1.0f0
    for corner in corners
        offset = Vec4f(corner[1] - center[1], corner[2] - center[2], corner[3] - center[3], 0.0f0)
        depth_radius = max(depth_radius, sqrt(_dot_vector(offset, offset)))
        half_extent = max(half_extent, abs(_dot_vector(offset, right)), abs(_dot_vector(offset, up)))
    end
    # A tight square in light-space is more texel-efficient than the historical
    # 3D sphere diameter. Reserve one texel on each edge for center snapping.
    base_span = 2.0f0 * half_extent
    span = base_span * Float32(map_size) / Float32(map_size - 2)
    texel_world = span / Float32(map_size)
    center_right = _dot_vector(center, right)
    center_up = _dot_vector(center, up)
    snapped_right = round(center_right / texel_world) * texel_world
    snapped_up = round(center_up / texel_world) * texel_world
    right_shift = snapped_right - center_right
    up_shift = snapped_up - center_up
    snapped_center = Vec4f(
        center[1] + right[1] * right_shift + up[1] * up_shift,
        center[2] + right[2] * right_shift + up[2] * up_shift,
        center[3] + right[3] * right_shift + up[3] * up_shift,
        0.0f0,
    )
    distance = max(2.0f0 * depth_radius, 32.0f0)
    position = Vec4f(
        snapped_center[1] - light_direction[1] * distance,
        snapped_center[2] - light_direction[2] * distance,
        snapped_center[3] - light_direction[3] * distance,
        0.0f0,
    )
    far_plane = 2.0f0 * distance + depth_radius
    return CameraFrame(
        position,
        right,
        up,
        light_direction,
        Vec4f(0.5f0 * span, 0.5f0 * span, 0.1f0, far_plane),
        0.0f0,
        1.0f0,
        1.0f0,
    )
end

@inline function _cascade_surface_spaces(
    world_position::Vec4f,
    camera::CameraFrame,
    frames::ShadowCascadeFrames,
)::Tuple{Vec4f,Vec4f,Float32}
    near = _project_world(
        world_position,
        frames.near.position,
        frames.near.right,
        frames.near.up,
        frames.near.forward,
        frames.near.projection,
        frames.near.mode,
    )
    far = _project_world(
        world_position,
        frames.far.position,
        frames.far.right,
        frames.far.up,
        frames.far.forward,
        frames.far.projection,
        frames.far.mode,
    )
    near_bias = SHADOW_BIAS_M / (frames.near.projection[4] - frames.near.projection[3])
    far_bias = SHADOW_BIAS_M / (frames.far.projection[4] - frames.far.projection[3])
    near_biased = Vec4f(near[1], near[2], near[3] - near_bias, near[4])
    far_biased = Vec4f(far[1], far[2], far[3] - far_bias, far[4])
    relative = Vec4f(
        world_position[1] - camera.position[1],
        world_position[2] - camera.position[2],
        world_position[3] - camera.position[3],
        0.0f0,
    )
    view_depth = _dot_vector(camera.forward, relative)
    blend = _ramp(frames.blend_start_m, frames.blend_end_m, view_depth)
    return near_biased, far_biased, blend
end

@inline function _surface_shadow_varyings(
    world_position::Vec4f,
    camera_position::Vec4f,
    camera_forward::Vec4f,
    near_position::Vec4f,
    near_right::Vec4f,
    near_up::Vec4f,
    near_forward::Vec4f,
    near_projection::Vec4f,
    near_mode::Float32,
    far_position::Vec4f,
    far_right::Vec4f,
    far_up::Vec4f,
    far_forward::Vec4f,
    far_projection::Vec4f,
    far_mode::Float32,
    cascade_parameters::Vec4f,
    far_bias::Float32,
    selection_position::Vec4f,
    selection_forward::Vec4f,
    sky_parameters::Vec4f,
    surface_options::Vec4f,
)
    near = _project_world(
        world_position, near_position, near_right, near_up, near_forward, near_projection, near_mode,
    )
    if cascade_parameters[1] > 0.5f0
        far = _project_world(
            world_position, far_position, far_right, far_up, far_forward, far_projection, far_mode,
        )
        relative = Vec4f(
            world_position[1] - selection_position[1],
            world_position[2] - selection_position[2],
            world_position[3] - selection_position[3],
            0.0f0,
        )
        view_depth = _dot_vector(selection_forward, relative)
        # Carry linear, unclamped depth; the fragment computes smoothstep after
        # perspective-correct interpolation so a large triangle still crosses
        # exactly the authored 27-33 m overlap band.
        blend = (view_depth - cascade_parameters[2]) /
                (cascade_parameters[3] - cascade_parameters[2])
        near_bias = cascade_parameters[4]
        return (
            Vec4f(near[1], near[2], near[3] - near_bias, blend),
            Vec4f(sky_parameters[1], sky_parameters[2], far[1], far[2]),
            Vec4f(surface_options[1], surface_options[2], surface_options[3], far[3] - far_bias),
        )
    end
    # The legacy fragment entry points ignore light-space w. Keep their shader
    # paths free of the binding-17 lookup while reserving a negative sentinel
    # for the cascade-specialized fragments.
    return (
        Vec4f(near[1], near[2], near[3], -1.0f0),
        sky_parameters,
        surface_options,
    )
end
