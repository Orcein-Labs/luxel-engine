# KernelCases — deterministic inputs for every TetKernels kernel, shaped like
# the spiral drivers' real launches (cage corners in 4-slot tets, barycentric
# weights summing to 1, batched instances, 3-bone palettes).
#
# Each case names its kernel, its original gpu/ source, the host arguments in
# the original argument order, the ndrange, and how many leading arguments are
# outputs. Shared by parity.jl, compile_amd.jl and smoke_rocm.jl.
module KernelCases

using Random
using ..TetKernels

export Case, cases, output_args, to_device

struct Case
    name::String
    source::String        # gpu/ file holding the CUDA original
    orig::Symbol          # function name in that file
    kernel::Any
    args::Tuple
    ndrange::Int
    nout::Int
end

output_args(c::Case) = c.args[1:c.nout]

# move host arrays to a backend's array type (`adapt` = e.g. CuArray, ROCArray)
to_device(adapt, args::Tuple) = map(a -> a isa AbstractArray ? adapt(a) : a, args)

function barycentric(rng, n)
    w = Vector{Float32}(undef, 4n)
    for i in 1:n
        r = rand(rng, Float32, 4) .+ 0.05f0
        r ./= sum(r)
        w[4i-3:4i] .= r
    end
    w
end

coords(rng, n, scale) = (2f0 .* rand(rng, Float32, n) .- 1f0) .* scale

function cases(; nT=1100, nV=4096, G=8, seed=20261005)
    rng = Xoshiro(seed)
    s = 1.5f0; A = 0.2f0; phi = 0.7f0
    nC = 4nT                                   # 4 corner slots per tet
    cx, cy, cz = coords(rng, nC, s), abs.(coords(rng, nC, s)), coords(rng, nC, s)
    idx = Int32.(rand(rng, 1:nC, 4nV))
    w = barycentric(rng, nV)
    out3(n) = (zeros(Float32, n), zeros(Float32, n), zeros(Float32, n))
    ntot = G * nV
    cs = Case[]

    for fam in Int32.(1:3)
        push!(cs, Case("deform_corners fam=$fam", "spiral_e_gpu.jl", :deform_corners!,
            deform_corners!, (out3(nC)..., cx, cy, cz, nC, s, fam, A, phi), nC, 3))
    end
    push!(cs, Case("reconstruct", "spiral_e_gpu.jl", :reconstruct!,
        reconstruct!, (out3(nV)..., cx, cy, cz, idx, w, nV), nV, 3))
    np = 2048
    pa = Int32.(rand(rng, 1:nC, np)); pb = Int32.(rand(rng, 1:nC, np))
    push!(cs, Case("pair_dev", "spiral_e_gpu.jl", :pair_dev!,
        pair_dev!, (zeros(Float32, np), cx, cy, cz, pa, pb, np), np, 1))
    for fam in Int32.(1:3)
        push!(cs, Case("fused_deform_reconstruct fam=$fam", "spiral_h_fusion.jl", :fused_deform_reconstruct!,
            fused_deform_reconstruct!, (out3(nV)..., cx, cy, cz, idx, w, nV, s, fam, A, phi), nV, 3))
    end
    push!(cs, Case("fused_batched_wind", "spiral_h2_wind_ladder.jl", :fused_batched_wind!,
        fused_batched_wind!, (out3(ntot)..., cx, cy, cz, idx, w, nV, ntot, s, A, phi), ntot, 3))

    # spiral I: per-instance deformed corner blocks of 4nT corners each
    idxT = Int32.(rand(rng, 1:4nT, 4nV))
    ox, oy, oz = coords(rng, G * 4nT, s), coords(rng, G * 4nT, s), coords(rng, G * 4nT, s)
    push!(cs, Case("reconstruct_batched", "spiral_i_comparators.jl", :reconstruct_batched!,
        reconstruct_batched!, (out3(ntot)..., ox, oy, oz, idxT, w, nV, nT, ntot), ntot, 3))
    vx, vy, vz = coords(rng, ntot, s), coords(rng, ntot, s), coords(rng, ntot, s)
    c, sn = cos(0.3f0), sin(0.3f0)
    push!(cs, Case("rot_direct", "spiral_i_comparators.jl", :rot_direct!,
        rot_direct!, (out3(ntot)..., vx, vy, vz, ntot, c, sn), ntot, 3))
    bidx = Int32.(rand(rng, 1:3, 4nV))
    θ = Float32.(range(0, 1.2; length=3G))
    push!(cs, Case("lbs", "spiral_i_comparators.jl", :lbs!,
        lbs!, (out3(ntot)..., vx[1:nV], vy[1:nV], vz[1:nV], bidx, w, cos.(θ), sin.(θ), nV, ntot), ntot, 3))
    m = coords(rng, 6ntot, s)
    push!(cs, Case("morph_lerp", "spiral_i_comparators.jl", :morph_lerp!,
        morph_lerp!, (out3(ntot)..., m, ntot, 0.37f0), ntot, 3))
    push!(cs, Case("fused_batched_shared", "spiral_i_comparators.jl", :fused_batched_shared!,
        fused_batched_shared!, (out3(ntot)..., cx, cy, cz, idx, w, nV, ntot, c, sn), ntot, 3))
    push!(cs, Case("probe_div", "probe3.jl", :probe_div!,
        probe_div!, (zeros(Float32, ntot), cx, idx, nV, ntot), ntot, 1))
    cs
end

end # module
