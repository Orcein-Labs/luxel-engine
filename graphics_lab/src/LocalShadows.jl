"""One 2D atlas of omnidirectional point-light depth: four lights × six cube faces.

Lava has no cube textures. Each face reuses the existing shadow camera path.
Atlas layout is light rows × face columns, face order +X −X +Y −Y +Z −Z.
"""
const POINT_SHADOW_FACE_COUNT = 6
const POINT_SHADOW_LIGHT_CAPACITY = 4
const POINT_SHADOW_NEAR_M = 0.05f0
const POINT_SHADOW_MAP_SIZES_PX = (256, 512)
const POINT_SHADOW_BINDING = UInt32(19)
const POINT_SHADOW_DEPTH_BIAS = 0.002f0

struct PointShadowResources
    cache_key::String
    texture::Lava.LavaTexture2D{NTuple{4,Float32}}
    sampler::Lava.LavaSampler
    atlas_framebuffer::Union{Nothing,Lava.LavaFramebuffer}
    face_framebuffer::Union{Nothing,Lava.LavaFramebuffer}
    blit_pipeline::Any
    map_size::Int
    enabled::Bool
end

"""Major-axis cube face for a vector from the light toward the receiver."""
@inline function _point_shadow_face_index(dx::Float32, dy::Float32, dz::Float32)::Int32
    ax = abs(dx)
    ay = abs(dy)
    az = abs(dz)
    if ax >= ay && ax >= az
        return dx >= 0.0f0 ? Int32(0) : Int32(1)
    elseif ay >= az
        return dy >= 0.0f0 ? Int32(2) : Int32(3)
    end
    return dz >= 0.0f0 ? Int32(4) : Int32(5)
end

"""Orthonormal (forward, right, up) for one cube face.

Bases are explicit so the shader and CPU agree without calling the throwing
basis helper. `right = forward × up_hint`, then `up = right × forward`.
"""
@inline function _point_shadow_face_basis(face::Int32)::NTuple{3,Vec4f}
    if face == Int32(0)
        return (Vec4f(1.0f0, 0.0f0, 0.0f0, 0.0f0), Vec4f(0.0f0, 0.0f0, -1.0f0, 0.0f0), Vec4f(0.0f0, -1.0f0, 0.0f0, 0.0f0))
    elseif face == Int32(1)
        return (Vec4f(-1.0f0, 0.0f0, 0.0f0, 0.0f0), Vec4f(0.0f0, 0.0f0, 1.0f0, 0.0f0), Vec4f(0.0f0, -1.0f0, 0.0f0, 0.0f0))
    elseif face == Int32(2)
        return (Vec4f(0.0f0, 1.0f0, 0.0f0, 0.0f0), Vec4f(1.0f0, 0.0f0, 0.0f0, 0.0f0), Vec4f(0.0f0, 0.0f0, 1.0f0, 0.0f0))
    elseif face == Int32(3)
        return (Vec4f(0.0f0, -1.0f0, 0.0f0, 0.0f0), Vec4f(1.0f0, 0.0f0, 0.0f0, 0.0f0), Vec4f(0.0f0, 0.0f0, -1.0f0, 0.0f0))
    elseif face == Int32(4)
        return (Vec4f(0.0f0, 0.0f0, 1.0f0, 0.0f0), Vec4f(1.0f0, 0.0f0, 0.0f0, 0.0f0), Vec4f(0.0f0, -1.0f0, 0.0f0, 0.0f0))
    end
    return (Vec4f(0.0f0, 0.0f0, -1.0f0, 0.0f0), Vec4f(-1.0f0, 0.0f0, 0.0f0, 0.0f0), Vec4f(0.0f0, -1.0f0, 0.0f0, 0.0f0))
end

function _point_shadow_face_frame(position_xyz_m::NTuple{3,Float32}, range_m::Float32, face::Integer)::CameraFrame
    forward, right, up = _point_shadow_face_basis(Int32(face))
    far_plane = max(Float32(range_m), POINT_SHADOW_NEAR_M + 0.05f0)
    return CameraFrame(
        Vec4f(position_xyz_m[1], position_xyz_m[2], position_xyz_m[3], 0.0f0),
        right,
        up,
        forward,
        Vec4f(1.0f0, 1.0f0, POINT_SHADOW_NEAR_M, far_plane),
        1.0f0,
        1.0f0,
        1.0f0,
    )
end

@inline function _point_shadow_atlas_uv(
    light_index::Int32,
    face::Int32,
    ndc_x::Float32,
    ndc_y::Float32,
)::Vec2f
    face_u = ndc_x * 0.5f0 + 0.5f0
    face_v = ndc_y * 0.5f0 + 0.5f0
    return Vec2f(
        (Float32(face) + clamp(face_u, 0.0f0, 1.0f0)) / Float32(POINT_SHADOW_FACE_COUNT),
        (Float32(light_index) + clamp(face_v, 0.0f0, 1.0f0)) / Float32(POINT_SHADOW_LIGHT_CAPACITY),
    )
end

@inline function _point_shadow_blit_dest(light_index::Int, face::Int)::Vec4f
    # Vulkan maps positive NDC Y to lower framebuffer rows (higher v). Light 0
    # occupies atlas v in [0, 1/4], so its cell is the more-negative NDC Y.
    x0 = 2.0f0 * Float32(face) / Float32(POINT_SHADOW_FACE_COUNT) - 1.0f0
    x1 = 2.0f0 * Float32(face + 1) / Float32(POINT_SHADOW_FACE_COUNT) - 1.0f0
    y0 = -1.0f0 + 2.0f0 * Float32(light_index) / Float32(POINT_SHADOW_LIGHT_CAPACITY)
    y1 = -1.0f0 + 2.0f0 * Float32(light_index + 1) / Float32(POINT_SHADOW_LIGHT_CAPACITY)
    return Vec4f(x0, y0, x1, y1)
end

"""PCF visibility for one point light using the cube atlas at slot 19.

`map_size_px == 0` is the unshadowed stand-in: return fully visible without
sampling. Darkness/filter follow the directional shadow uniform.
"""
@inline function _point_shadow_visibility(
    world_position::Vec4f,
    light_position::Vec4f,
    light_index::Int32,
    map_size_px::Float32,
    darkness::Float32,
    filter_texel::Float32,
)::Float32
    map_size_px < 1.0f0 && return 1.0f0
    dx = world_position[1] - light_position[1]
    dy = world_position[2] - light_position[2]
    dz = world_position[3] - light_position[3]
    face = _point_shadow_face_index(dx, dy, dz)
    forward, right, up = _point_shadow_face_basis(face)
    range_m = light_position[4]
    light_space = _project_world(
        world_position,
        Vec4f(light_position[1], light_position[2], light_position[3], 0.0f0),
        right,
        up,
        forward,
        Vec4f(1.0f0, 1.0f0, POINT_SHADOW_NEAR_M, max(range_m, POINT_SHADOW_NEAR_M + 0.05f0)),
        1.0f0,
    )
    inside =
        -1.0f0 <= light_space[1] <= 1.0f0 &&
            -1.0f0 <= light_space[2] <= 1.0f0 &&
            0.0f0 <= light_space[3] <= 1.0f0
    inside || return 1.0f0
    uv = _point_shadow_atlas_uv(light_index, face, light_space[1], light_space[2])
    texel_u = filter_texel / (map_size_px * Float32(POINT_SHADOW_FACE_COUNT))
    texel_v = filter_texel / (map_size_px * Float32(POINT_SHADOW_LIGHT_CAPACITY))
    depth = light_space[3] - POINT_SHADOW_DEPTH_BIAS
    visible = 0.0f0
    visible += depth <= Lava.sample_texture_2d(POINT_SHADOW_BINDING, uv[1] - texel_u, uv[2] - texel_v, UInt32(0)) ? 1.0f0 : 0.0f0
    visible += depth <= Lava.sample_texture_2d(POINT_SHADOW_BINDING, uv[1] + texel_u, uv[2] - texel_v, UInt32(0)) ? 1.0f0 : 0.0f0
    visible += depth <= Lava.sample_texture_2d(POINT_SHADOW_BINDING, uv[1] - texel_u, uv[2] + texel_v, UInt32(0)) ? 1.0f0 : 0.0f0
    visible += depth <= Lava.sample_texture_2d(POINT_SHADOW_BINDING, uv[1] + texel_u, uv[2] + texel_v, UInt32(0)) ? 1.0f0 : 0.0f0
    pcf = visible * 0.25f0
    return (1.0f0 - darkness) + darkness * pcf
end

function _point_shadow_blit_vertex(dest::Vec4f)
    vertex_id = Lava.vertex_index() - Int32(1)
    corner = rem(vertex_id, Int32(6))
    u = corner == Int32(1) || corner == Int32(2) || corner == Int32(4) ? 1.0f0 : 0.0f0
    v = corner == Int32(2) || corner == Int32(4) || corner == Int32(5) ? 1.0f0 : 0.0f0
    x = dest[1] + (dest[3] - dest[1]) * u
    y = dest[2] + (dest[4] - dest[2]) * v
    Lava.set_position!(Vec4f(x, y, 0.5f0, 1.0f0))
    Lava.gfx_output(0, Vec2f(u, v))
    return nothing
end

function _point_shadow_blit_fragment()
    uv = Lava.gfx_input(Vec2f, 0)
    depth = Lava.sample_texture_2d(UInt32(0), uv[1], uv[2], UInt32(0))
    Lava.gfx_output(0, Vec4f(depth, depth, depth, 1.0f0))
    return nothing
end
