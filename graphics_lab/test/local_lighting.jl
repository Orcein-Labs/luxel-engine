using Test
using GeometryBasics: Vec4f
using LuxelGraphics
if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end

const LA = LavaAdapter
const LL_ZERO4 = Vec4f(0f0, 0f0, 0f0, 0f0)
const LL_UP4 = Vec4f(0f0, 1f0, 0f0, 0f0)

function local_light(id, position=(0f0, 1f0, 0f0); range=5f0, color=(1f0, 0.5f0, 0.25f0), intensity=2f0)
    return LuxelGraphics.LightPacket(id,
        LuxelGraphics.PointLightPacket(position, range), color, intensity)
end

function sun_light()
    return LuxelGraphics.LightPacket("sun",
        LuxelGraphics.DirectionalLightPacket((0.2f0, -1f0, 0.1f0)),
        (1f0, 1f0, 1f0), 1f0)
end

@testset "bounded point-light adapter contract" begin
    lights = LuxelGraphics.LightPacket[sun_light(), local_light("warm")]
    @test LA._lighting_from_lights(lights) isa LA.DirectionalLighting
    table = LA._point_light_table(lights)
    @test size(table) == (1, 9)
    @test table[1, 1] == (1f0, 0f0, 0f0, 0f0)
    @test table[1, 2] == (0f0, 1f0, 0f0, 5f0)
    @test table[1, 6] == (1f0, 0.5f0, 0.25f0, 2f0)
    @test table[1, 5] == (0f0, 0f0, 0f0, 1f0) # padded records cannot contribute

    four = LuxelGraphics.LightPacket[sun_light()]
    append!(four, [local_light("p$i") for i in 1:4])
    @test LA._point_light_table(four)[1, 1][1] == 4f0
    @test_throws LA.AdapterError LA._lighting_from_lights([sun_light(); [local_light("p$i") for i in 1:5]])
    @test_throws LA.AdapterError LA._lighting_from_lights([local_light("only-point")])
    @test_throws LA.AdapterError LA._lighting_from_lights([sun_light(), sun_light()])
    @test_throws LA.AdapterError LA._lighting_from_lights([sun_light(), local_light("bad-range"; range=0f0)])
    @test_throws LA.AdapterError LA._lighting_from_lights([sun_light(), local_light("too-far", (2f6, 0f0, 0f0))])
    @test_throws LA.AdapterError LA._lighting_from_lights([sun_light(), local_light("too-bright"; intensity=2f6)])
    @test_throws LA.AdapterError LA._lighting_from_lights(vcat([sun_light(), local_light("duplicate")], [local_light("duplicate")]))
end

@testset "local point attenuation and direct BRDF" begin
    attenuation(distance, range=5f0, intensity=2f0) =
        LA._point_light_attenuation(distance * distance, range, intensity)
    @test attenuation(1f0) ≈ 2f0 * (1f0 - (1f0 / 5f0)^4)^2
    @test attenuation(5f0) == 0f0
    @test attenuation(5.01f0) == 0f0
    @test isfinite(attenuation(0f0))
    @test attenuation(0f0) == 200f0 # inverse-square floor is 0.01 m²

    position = Vec4f(0f0, 1f0, 0f0, 5f0)
    color = Vec4f(1f0, 0.5f0, 0.25f0, 2f0)
    base = Vec4f(0.7f0, 0.4f0, 0.2f0, 1f0)
    center = LA._point_sample_direct(base, LL_UP4, LL_UP4, LL_ZERO4, position, color,
        0f0, 0.5f0, 1f0, 0.2f0, 0.3f0)
    @test all(isfinite, center)
    @test center[1] > center[2] > center[3] > 0f0
    @test center[4] == 0f0

    doubled = LA._point_light_direct(base, LL_UP4, LL_UP4, LL_UP4, Vec4f(1f0, 1f0, 1f0, 1f0),
        2f0, 0f0, 0.5f0, 1f0, 0f0, 0.5f0)
    unit = LA._point_light_direct(base, LL_UP4, LL_UP4, LL_UP4, Vec4f(1f0, 1f0, 1f0, 1f0),
        1f0, 0f0, 0.5f0, 1f0, 0f0, 0.5f0)
    @test doubled[1:3] ≈ 2f0 .* unit[1:3]

    metal_red = LA._point_light_direct(Vec4f(1f0, 0f0, 0f0, 1f0), LL_UP4, LL_UP4, LL_UP4,
        Vec4f(1f0, 1f0, 1f0, 1f0), 1f0, 1f0, 0.4f0, 1f0, 0f0, 0.5f0)
    metal_blue = LA._point_light_direct(Vec4f(0f0, 0f0, 1f0, 1f0), LL_UP4, LL_UP4, LL_UP4,
        Vec4f(1f0, 1f0, 1f0, 1f0), 1f0, 1f0, 0.4f0, 1f0, 0f0, 0.5f0)
    @test metal_red[1] > metal_red[3]
    @test metal_blue[3] > metal_blue[1]
    coated = LA._point_light_direct(base, LL_UP4, LL_UP4, LL_UP4, Vec4f(1f0, 1f0, 1f0, 1f0),
        1f0, 0f0, 0.5f0, 1f0, 1f0, 0.2f0)
    uncoated = LA._point_light_direct(base, LL_UP4, LL_UP4, LL_UP4, Vec4f(1f0, 1f0, 1f0, 1f0),
        1f0, 0f0, 0.5f0, 1f0, 0f0, 0.2f0)
    @test coated[1] > uncoated[1]

    # Point lights must use the existing BRDF exactly. Zero environment,
    # full occlusion and zero emission isolate its directional direct lobe.
    for light_vector in (LL_UP4, Vec4f(0.3f0, 0.9f0, 0.1f0, 0f0)),
        view_vector in (LL_UP4, Vec4f(-0.2f0, 0.95f0, 0.1f0, 0f0)),
        metallic in (0f0, 0.6f0, 1f0), roughness in (0.08f0, 0.35f0, 0.8f0),
        coat in (0f0, 0.7f0)
        light_unit = LA._normalize_vector(light_vector)
        view_unit = LA._normalize_vector(view_vector)
        actual = LA._point_light_direct(base, LL_UP4, view_unit, light_unit,
            Vec4f(0.8f0, 0.6f0, 0.4f0, 1f0), 1.7f0, metallic,
            roughness, 0.72f0, coat, 0.22f0)
        reference = LA._material_response(base, LL_UP4,
            Vec4f(-light_unit[1], -light_unit[2], -light_unit[3], 0f0),
            Vec4f(0.8f0, 0.6f0, 0.4f0, 1f0), 1.7f0,
            LL_ZERO4, LL_ZERO4, LL_ZERO4, metallic, roughness, coat, 0.22f0,
            1f0, view_unit, 0.72f0, 0f0, 1f0, LL_ZERO4, LL_ZERO4, LL_ZERO4, 0f0, LA.NoIbl())
        @test actual[1:3] ≈ reference[1:3] atol=1f-6 rtol=1f-6
    end
end
