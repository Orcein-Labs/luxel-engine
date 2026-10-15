using Test, LuxelGraphics
using GeometryBasics: Vec4f
if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end

@testset "GGX projected-area normalization" begin
    # Independent Float64 quadrature, concentrated near the narrow polished
    # peak: mu=cos(theta)=1-t^2, dmu=-2t dt. A valid NDF integrates D*mu to 1
    # over the hemisphere, including its azimuthal factor 2pi.
    for roughness in (0.045f0, 0.1f0, 0.25f0, 0.5f0, 1f0)
        count = 65_536
        integral = 0.0
        for index in 1:count
            t = (index - 0.5) / count
            mu = 1.0 - t * t
            distribution = LavaAdapter._ggx_distribution(Float32(mu), roughness)
            integral += Float64(distribution) * mu * 2.0 * t
        end
        @test 2pi * integral / count ≈ 1.0 rtol=0.002
        @test LavaAdapter._ggx_distribution(1f0, roughness) ≈
              1.0 / (pi * Float64(roughness)^4) rtol=1e-5
    end
    @test LavaAdapter._ggx_distribution(1f0, 0f0) == LavaAdapter._ggx_distribution(1f0, 0.045f0)
    @test LavaAdapter._ggx_distribution(1f0, 2f0) == LavaAdapter._ggx_distribution(1f0, 1f0)
end

# No environment, emission, texture or post-process contribution: isolate
# direct lighting at N = L = V, where the reference reduces analytically.
function direct_specular_peak(roughness::Float32; coat::Bool=false)
    zero = Vec4f(0f0, 0f0, 0f0, 0f0)
    normal = Vec4f(0f0, 1f0, 0f0, 0f0)
    return LavaAdapter._material_response(
        coat ? Vec4f(0f0, 0f0, 0f0, 1f0) : Vec4f(0.8f0, 0.8f0, 0.8f0, 1f0),
        normal, Vec4f(0f0, -1f0, 0f0, 0f0), Vec4f(1f0, 1f0, 1f0, 1f0),
        1f0, zero, zero, zero, 1f0, roughness, coat ? 1f0 : 0f0, roughness,
        1f0, normal, 1f0, 1f0, 1f0, zero, zero, zero,
    )[1]
end

@testset "direct GGX material response" begin
    for roughness in (0.045f0, 0.1f0, 0.25f0, 0.5f0, 1f0)
        # Isotropic Trowbridge-Reitz at N dot H = 1 is 1/(pi * alpha^2).
        # Luxel maps perceptual roughness to alpha = roughness^2.
        distribution = 1.0 / (pi * Float64(roughness)^4)
        denominator = 4.0 + 0.0001 # existing BRDF grazing guard, unchanged
        @test direct_specular_peak(roughness) ≈ 0.8 * distribution / denominator rtol=1e-4
        @test direct_specular_peak(roughness; coat=true) ≈ 0.04 * distribution / denominator rtol=1e-4
        @test isfinite(direct_specular_peak(roughness))
    end
    @test direct_specular_peak(0.1f0) > direct_specular_peak(0.25f0)
    @test direct_specular_peak(0.25f0) > direct_specular_peak(0.5f0)
end
