"""Resources shared by the depth prepass and contact-occlusion composite."""
struct ContactOcclusionResources
    cache_key::String
    packet_sha256::String
    capture_id::String
    camera_frame::CameraFrame
    source_framebuffer::Lava.LavaFramebuffer
    depth_framebuffer::Lava.LavaFramebuffer
    depth_texture::Lava.LavaTexture2D{NTuple{4,Float32}}
    ao_framebuffer::Lava.LavaFramebuffer
    ao_texture::Lava.LavaTexture2D{NTuple{4,Float32}}
    color_sampler::Lava.LavaSampler
    depth_sampler::Lava.LavaSampler
    bindings::Lava.TextureBindings
end

"""Invert this renderer's [0,1] depth mapping into positive camera metres."""
@inline function _contact_depth_meters(
    depth01::Float32,
    projection::Vec4f,
    mode::Float32,
)::Float32
    near = projection[3]
    far = projection[4]
    depth = clamp(depth01, 0.0f0, 1.0f0)
    if mode < 0.5f0
        return near + depth * (far - near)
    end
    denominator = far - depth * (far - near)
    return denominator <= 1.0f-7 ? far : clamp(far * near / denominator, near, far)
end

@inline function _contact_view_position(
    uv::Vec2f,
    depth01::Float32,
    projection::Vec4f,
    mode::Float32,
)::Vec4f
    depth = _contact_depth_meters(depth01, projection, mode)
    ndc_x = 2.0f0 * uv[1] - 1.0f0
    ndc_y = 2.0f0 * uv[2] - 1.0f0
    if mode < 0.5f0
        return Vec4f(ndc_x * projection[1], -ndc_y * projection[2], depth, 0.0f0)
    end
    return Vec4f(
        ndc_x * projection[1] * depth,
        -ndc_y * projection[2] * depth,
        depth,
        0.0f0,
    )
end

@inline function _contact_occlusion_sample(
    normal::Vec4f,
    delta::Vec4f,
    radius_m::Float32,
    bias_m::Float32,
)::Float32
    distance_squared = _dot_vector(delta, delta)
    radius_squared = radius_m * radius_m
    (distance_squared <= 1.0f-8 || distance_squared >= radius_squared) && return 0.0f0
    distance = sqrt(distance_squared)
    height = _dot_vector(normal, delta) - bias_m
    height <= 0.0f0 && return 0.0f0
    return clamp(height / radius_m, 0.0f0, 1.0f0) * (1.0f0 - distance / radius_m)
end

"""Normal-aware, radius-limited occlusion from eight reconstructed neighbours.

The caller supplies an estimated center normal and clockwise neighbor positions
in camera space. Flat coplanar neighbors have zero normal separation; samples
beyond the metric radius contribute nothing.
"""
@inline function _contact_ao_factor(
    normal::Vec4f,
    center::Vec4f,
    neighbors::NTuple{8,Vec4f},
    radius_m::Float32,
    bias_m::Float32,
)::Float32
    sum =
        _contact_occlusion_sample(normal, Vec4f(neighbors[1][1] - center[1], neighbors[1][2] - center[2], neighbors[1][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[2][1] - center[1], neighbors[2][2] - center[2], neighbors[2][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[3][1] - center[1], neighbors[3][2] - center[2], neighbors[3][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[4][1] - center[1], neighbors[4][2] - center[2], neighbors[4][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[5][1] - center[1], neighbors[5][2] - center[2], neighbors[5][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[6][1] - center[1], neighbors[6][2] - center[2], neighbors[6][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[7][1] - center[1], neighbors[7][2] - center[2], neighbors[7][3] - center[3], 0.0f0), radius_m, bias_m) +
        _contact_occlusion_sample(normal, Vec4f(neighbors[8][1] - center[1], neighbors[8][2] - center[2], neighbors[8][3] - center[3], 0.0f0), radius_m, bias_m)
    return clamp(sum * 0.25f0, 0.0f0, 0.75f0)
end

@inline function _contact_depth_sample(u::Float32, v::Float32)::Float32
    return Lava.sample_texture_2d(UInt32(1), u, v, UInt32(0))
end

@inline function _contact_sample_position(
    uv::Vec2f,
    center::Vec4f,
    projection::Vec4f,
    mode::Float32,
)::Vec4f
    if uv[1] < 0.0f0 || uv[1] > 1.0f0 || uv[2] < 0.0f0 || uv[2] > 1.0f0
        return center
    end
    return _contact_view_position(uv, _contact_depth_sample(uv[1], uv[2]), projection, mode)
end

@inline function _contact_ao_fragment()
    uv = Lava.gfx_input(Vec2f, 0)
    texel_size = Lava.gfx_input(Vec2f, 1)
    projection = Lava.gfx_input(Vec4f, 2)
    parameters = Lava.gfx_input(Vec4f, 3)
    color = Vec4f(
        Lava.sample_texture_2d(UInt32(0), uv[1], uv[2], UInt32(0)),
        Lava.sample_texture_2d(UInt32(0), uv[1], uv[2], UInt32(1)),
        Lava.sample_texture_2d(UInt32(0), uv[1], uv[2], UInt32(2)),
        Lava.sample_texture_2d(UInt32(0), uv[1], uv[2], UInt32(3)),
    )
    raw_center = _contact_depth_sample(uv[1], uv[2])
    raw_center >= 0.999999f0 && (Lava.gfx_output(0, color); return nothing)

    mode = parameters[4]
    center = _contact_view_position(uv, raw_center, projection, mode)
    radius_m = clamp(parameters[1], 0.01f0, 2.0f0)
    strength = parameters[2]
    bias_m = parameters[3]
    # Use one-pixel derivatives for the receiver normal. Full-radius points
    # cross silhouette discontinuities and corrupt the normal right at contact.
    immediate_right = _contact_sample_position(Vec2f(uv[1] + texel_size[1], uv[2]), center, projection, mode)
    immediate_left = _contact_sample_position(Vec2f(uv[1] - texel_size[1], uv[2]), center, projection, mode)
    immediate_up = _contact_sample_position(Vec2f(uv[1], uv[2] + texel_size[2]), center, projection, mode)
    immediate_down = _contact_sample_position(Vec2f(uv[1], uv[2] - texel_size[2]), center, projection, mode)
    delta_right = Vec4f(immediate_right[1] - center[1], immediate_right[2] - center[2], immediate_right[3] - center[3], 0.0f0)
    delta_left = Vec4f(immediate_left[1] - center[1], immediate_left[2] - center[2], immediate_left[3] - center[3], 0.0f0)
    delta_up = Vec4f(immediate_up[1] - center[1], immediate_up[2] - center[2], immediate_up[3] - center[3], 0.0f0)
    delta_down = Vec4f(immediate_down[1] - center[1], immediate_down[2] - center[2], immediate_down[3] - center[3], 0.0f0)
    tangent_x = _dot_vector(delta_right, delta_right) < _dot_vector(delta_left, delta_left) ? delta_right : Vec4f(-delta_left[1], -delta_left[2], -delta_left[3], 0.0f0)
    tangent_y = _dot_vector(delta_up, delta_up) < _dot_vector(delta_down, delta_down) ? delta_up : Vec4f(-delta_down[1], -delta_down[2], -delta_down[3], 0.0f0)
    normal = _normalize_vector(_cross_vector(tangent_x, tangent_y))
    to_camera = Vec4f(-center[1], -center[2], -center[3], 0.0f0)
    _dot_vector(normal, to_camera) < 0.0f0 && (normal = Vec4f(-normal[1], -normal[2], -normal[3], 0.0f0))

    # Eight fixed samples at 65% of the bounded metric radius. UVs outside the
    # image return the center point, avoiding clamped-border false occluders.
    sample_radius = radius_m * 0.65f0
    scale_x = mode < 0.5f0 ? sample_radius / (2.0f0 * projection[1]) : sample_radius / (2.0f0 * projection[1] * center[3])
    scale_y = mode < 0.5f0 ? sample_radius / (2.0f0 * projection[2]) : sample_radius / (2.0f0 * projection[2] * center[3])
    dx = scale_x * 0.70710678f0
    dy = scale_y * 0.70710678f0
    neighbors = (
        _contact_sample_position(Vec2f(uv[1] + scale_x, uv[2]), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1] + dx, uv[2] + dy), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1], uv[2] + scale_y), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1] - dx, uv[2] + dy), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1] - scale_x, uv[2]), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1] - dx, uv[2] - dy), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1], uv[2] - scale_y), center, projection, mode),
        _contact_sample_position(Vec2f(uv[1] + dx, uv[2] - dy), center, projection, mode),
    )
    occlusion = _contact_ao_factor(normal, center, neighbors, radius_m, bias_m)
    visibility = 1.0f0 - strength * occlusion
    Lava.gfx_output(0, Vec4f(color[1] * visibility, color[2] * visibility, color[3] * visibility, color[4]))
    return nothing
end

function _contact_ao_vertex(
    texel_size::Vec2f,
    projection::Vec4f,
    parameters::Vec4f,
)
    vertex_id = Lava.vertex_index() - Int32(1)
    x = Float32(Int32(vertex_id & Int32(1)) * 4 - 1)
    y = Float32(Int32((vertex_id >> Int32(1)) & Int32(1)) * 4 - 1)
    Lava.set_position!(Vec4f(x, y, 0.5f0, 1.0f0))
    Lava.gfx_output(0, Vec2f((x + 1.0f0) * 0.5f0, (y + 1.0f0) * 0.5f0))
    Lava.gfx_output(1, texel_size)
    Lava.gfx_output(2, projection)
    Lava.gfx_output(3, parameters)
    return nothing
end

function _contact_depth_fragment()
    depth = Lava.frag_coord(3)
    Lava.gfx_output(0, Vec4f(depth, depth, depth, 1.0f0))
    return nothing
end

function _contact_depth_cutout_fragment()
    uv = Lava.gfx_input(Vec2f, 1)
    coverage = Lava.gfx_input(Vec4f, 2)
    weight = clamp(coverage[2], 0.0f0, 1.0f0)
    alpha = coverage[1] * (1.0f0 - weight + weight * Lava.sample_texture_2d(UInt32(0), uv[1], uv[2], UInt32(3)))
    if alpha < coverage[3]
        Lava.discard()
    end
    depth = Lava.frag_coord(3)
    Lava.gfx_output(0, Vec4f(depth, depth, depth, 1.0f0))
    return nothing
end

function _contact_depth_bytes(depth_m::AbstractMatrix{Float32})::Vector{UInt8}
    width, height = size(depth_m)
    bytes = Vector{UInt8}(undef, 4 * width * height)
    offset = 1
    # Lava readback matrices are width × height. Serialize rows top-to-bottom,
    # then columns left-to-right, explicitly little-endian Float32.
    for y in 1:height, x in 1:width
        bits = reinterpret(UInt32, depth_m[x, y])
        bytes[offset] = UInt8(bits & 0xff)
        bytes[offset + 1] = UInt8((bits >> 8) & 0xff)
        bytes[offset + 2] = UInt8((bits >> 16) & 0xff)
        bytes[offset + 3] = UInt8((bits >> 24) & 0xff)
        offset += 4
    end
    return bytes
end

function _contact_downsample_nearest(
    depth_m::AbstractMatrix{Float32},
    output_width::Int,
    output_height::Int,
)::Matrix{Float32}
    size(depth_m) == (2 * output_width, 2 * output_height) || throw(AdapterError(
        "depth_unavailable", "contact AO depth is not exactly 2x the capture dimensions",
    ))
    output = Matrix{Float32}(undef, output_width, output_height)
    for y in 1:output_height, x in 1:output_width
        x0, y0 = 2 * x - 1, 2 * y - 1
        output[x, y] = min(
            depth_m[x0, y0], depth_m[x0 + 1, y0],
            depth_m[x0, y0 + 1], depth_m[x0 + 1, y0 + 1],
        )
    end
    return output
end

"""Read the most recent enabled AO depth prepass without rendering again.

The returned bytes are little-endian Float32 positive camera-space metres in
row-major top-left order at capture resolution. Each output pixel is the
minimum positive depth of its source 2x2 block, retaining the nearest visible
surface. Identity fields bind the diagnostic to the capture.
"""
function capture_scene_depth(packet::LuxelGraphics.GraphicsScenePacket, state)
    # A presented frame may still be writing the depth target this reads back.
    _drain_window_frame!(state)
    resources = state.contact_ao_resources
    resources === nothing && throw(AdapterError("depth_unavailable", "contact AO depth was not captured"))
    resources.packet_sha256 == packet.packet_sha256 || throw(AdapterError(
        "depth_unavailable", "cached contact AO depth belongs to another packet",
    ))
    resources.capture_id == packet.capture_id || throw(AdapterError(
        "depth_unavailable", "cached contact AO depth belongs to another capture",
    ))
    camera = _camera_frame(packet.camera)
    resources.camera_frame == camera || throw(AdapterError(
        "depth_unavailable", "cached contact AO depth used a presented camera override",
    ))
    depth01 = Lava.readback_framebuffer(resources.depth_framebuffer)
    width, height = size(depth01)
    camera = resources.camera_frame
    projection = camera.projection
    depth_m = Matrix{Float32}(undef, width, height)
    for y in 1:height, x in 1:width
        depth_m[x, y] = _contact_depth_meters(depth01[x, y][1], projection, camera.mode)
    end
    capture_width, capture_height = Int(packet.width_px), Int(packet.height_px)
    depth_capture = _contact_downsample_nearest(depth_m, capture_width, capture_height)
    bytes = _contact_depth_bytes(depth_capture)
    return (
        schema="luxel.view-depth/v1",
        width_px=capture_width,
        height_px=capture_height,
        packet_sha256=resources.packet_sha256,
        capture_id=resources.capture_id,
        depth_sha256="sha256:" * bytes2hex(SHA.sha256(bytes)),
        depth_base64=base64encode(bytes),
    )
end
