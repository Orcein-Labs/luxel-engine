using JSON3
using Test
using GeometryBasics: Vec4f
using LuxelGraphics
if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end

const LS = LavaAdapter

@testset "point-shadow face selection and orthonormal bases" begin
    @test LS._point_shadow_face_index(1f0, 0.1f0, 0.1f0) == Int32(0)
    @test LS._point_shadow_face_index(-1f0, 0.1f0, 0.1f0) == Int32(1)
    @test LS._point_shadow_face_index(0.1f0, 1f0, 0.1f0) == Int32(2)
    @test LS._point_shadow_face_index(0.1f0, -1f0, 0.1f0) == Int32(3)
    @test LS._point_shadow_face_index(0.1f0, 0.1f0, 1f0) == Int32(4)
    @test LS._point_shadow_face_index(0.1f0, 0.1f0, -1f0) == Int32(5)
    for face in Int32(0):Int32(5)
        forward, right, up = LS._point_shadow_face_basis(face)
        @test isapprox(LS._dot_vector(forward, forward), 1f0; atol=1f-6)
        @test isapprox(LS._dot_vector(right, right), 1f0; atol=1f-6)
        @test isapprox(LS._dot_vector(up, up), 1f0; atol=1f-6)
        @test abs(LS._dot_vector(forward, right)) < 1f-6
        @test abs(LS._dot_vector(forward, up)) < 1f-6
        @test abs(LS._dot_vector(right, up)) < 1f-6
        triple = LS._dot_vector(forward, LS._cross_vector(right, up))
        @test isapprox(triple, -1f0; atol=1f-5)
    end
end

@testset "point-shadow atlas layout and 90-degree face cameras" begin
    uv = LS._point_shadow_atlas_uv(Int32(0), Int32(0), -1f0, -1f0)
    @test uv[1] ≈ 0f0
    @test uv[2] ≈ 0f0
    uv = LS._point_shadow_atlas_uv(Int32(3), Int32(5), 1f0, 1f0)
    @test uv[1] ≈ 1f0
    @test uv[2] ≈ 1f0
    dest = LS._point_shadow_blit_dest(0, 0)
    @test dest[1] ≈ -1f0
    @test dest[2] ≈ -1f0
    @test dest[3] ≈ -1f0 + 2f0 / 6f0
    @test dest[4] ≈ -1f0 + 2f0 / 4f0
    frame = LS._point_shadow_face_frame((0f0, 2f0, 0f0), 5.5f0, 0)
    @test frame.mode == 1f0
    @test frame.projection[1] == 1f0
    @test frame.projection[2] == 1f0
    @test frame.projection[3] == LS.POINT_SHADOW_NEAR_M
    @test frame.projection[4] == 5.5f0
end

@testset "point-shadow policy parse and table packing" begin
    policy_json(overrides) = JSON3.read(JSON3.write(Dict{String,Any}("render_policy" => overrides)))
    parsed = LuxelGraphics._parse_render_policy(policy_json(Dict("point_shadows" => Dict("map_size_px" => 256))))
    @test parsed.point_shadows == LuxelGraphics.PointShadowsPolicy(256)
    parsed512 = LuxelGraphics._parse_render_policy(policy_json(Dict("point_shadows" => Dict("map_size_px" => 512))))
    @test parsed512.point_shadows == LuxelGraphics.PointShadowsPolicy(512)
    @test LuxelGraphics._parse_render_policy(policy_json(Dict{String,Any}())).point_shadows === nothing
    for bad in (128, 384, 1024, 256.5)
        @test_throws LuxelGraphics.ProtocolError LuxelGraphics._parse_render_policy(
            policy_json(Dict("point_shadows" => Dict("map_size_px" => bad))),
        )
    end
    @test_throws LuxelGraphics.ProtocolError LuxelGraphics._parse_render_policy(
        policy_json(Dict("point_shadows" => Dict("map_size_px" => 256, "foo" => 1))),
    )
    sun = LuxelGraphics.LightPacket("sun",
        LuxelGraphics.DirectionalLightPacket((0.2f0, -1f0, 0.1f0)), (1f0, 1f0, 1f0), 1f0)
    point = LuxelGraphics.LightPacket("warm",
        LuxelGraphics.PointLightPacket((0f0, 1f0, 0f0), 5f0), (1f0, 0.5f0, 0.25f0), 2f0)
    @test LS._point_light_table([sun, point])[1, 1] == (1f0, 0f0, 0f0, 0f0)
    packed = LS._point_light_table([sun, point], 256f0, 0.75f0, 1f0)
    @test packed[1, 1] == (1f0, 0.75f0, 256f0, 1f0)
    grade = LuxelGraphics.GradePolicy()
    bloom = LuxelGraphics.BloomPolicy()
    vignette = LuxelGraphics.VignettePolicy()
    dither = LuxelGraphics.DitherPolicy()
    terrain = LuxelGraphics.TerrainSurfacePolicy()
    sampler = LuxelGraphics.SamplerPolicy()
    shadow = LuxelGraphics.ShadowPolicy()
    mesh = LuxelGraphics.MeshSurfacePolicy()
    sixteen = LuxelGraphics.RenderPolicy(
        grade, bloom, vignette, dither, terrain, sampler, shadow, mesh,
        nothing, nothing, nothing, nothing, nothing, nothing, nothing, nothing,
    )
    @test sixteen.point_shadows === nothing
    @test sixteen.contact_ao === nothing
end
