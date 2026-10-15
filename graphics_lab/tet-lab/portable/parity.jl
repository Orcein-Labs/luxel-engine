# parity.jl — prove the KernelAbstractions ports are faithful to the gpu/ originals.
#
# For every kernel case, on the local NVIDIA GPU:
#   orig   the ORIGINAL @cuda kernel, parsed straight out of its gpu/*.jl file
#          (function definitions only; the drivers' top-level code never runs)
#   ka_cu  the TetKernels port on CUDABackend()
#   ka_cpu the TetKernels port on CPU()
#
# Gates:
#   P-1  ka_cu is BIT-IDENTICAL to orig (same device, same compiler, same math
#        library): any difference means the port changed the arithmetic.
#   P-2  ka_cpu vs orig is reported in ULPs (host libm vs CUDA libdevice for
#        sin/cos/tanh, plus fma contraction). Bounded by the E-G2 device-vs-host
#        tolerance: max |Δ| scaled to screen space <= 0.02 px at d = 5.
#
#   julia --project=portable portable/parity.jl
#
# Writes results/portable-parity.csv. This file is also the reference the
# ROCm smoke test (smoke_rocm.jl) is judged against.
using CUDA, KernelAbstractions, Printf, SHA

include("TetKernels.jl"); using .TetKernels
include("KernelCases.jl"); using .KernelCases

const ROOT = dirname(@__DIR__)
const PX_PER_UNIT = 500.0 / 5.0          # F_PX / D_CAM, spiral E's screen scale
const E_G2_PX = 0.02

# Load only the device-kernel function definitions from an original gpu/ file.
# Known defect in an original, corrected only to have something to compare to:
# probe3.jl reads `threadIdx.x` (no call parentheses), which CUDA.jl rejects
# as invalid IR. The fixed copy changes that one token and nothing else.
const ONE_TOKEN_FIXES = Dict("probe3.jl" => ("threadIdx.x" => "threadIdx().x"))

function load_originals(file; fixed=false)
    m = Module(Symbol(replace(file, ".jl" => "") * (fixed ? "_fixed" : "")))
    Core.eval(m, :(using CUDA))
    Core.eval(m, :(const F_PX = 500.0f0; const D_CAM = 5.0f0))
    src = read(joinpath(ROOT, "gpu", file), String)
    fixed && (src = replace(src, ONE_TOKEN_FIXES[file]))
    ast = Meta.parseall(src)
    for ex in ast.args
        ex isa Expr && ex.head === :function || continue
        occursin("blockIdx", string(ex)) && Core.eval(m, ex)
    end
    m
end

# hash of the raw output bytes, so another vendor's run can test bit-equality
output_sha(outs) = bytes2hex(sha256(reduce(vcat, [reinterpret(UInt8, o) for o in outs])))

ulps(a::Float32, b::Float32) = begin
    ia = reinterpret(Int32, a); ib = reinterpret(Int32, b)
    ia = ia < 0 ? typemin(Int32) - ia : ia
    ib = ib < 0 ? typemin(Int32) - ib : ib
    abs(Int64(ia) - Int64(ib))
end

function compare(ref, got)
    maxulp = 0; maxabs = 0.0; nbits = 0
    for (r, g) in zip(ref, got), k in eachindex(r)
        a, b = r[k], g[k]
        nbits += reinterpret(UInt32, a) != reinterpret(UInt32, b)
        max(abs(a), abs(b)) >= 1f-3 && (maxulp = max(maxulp, ulps(a, b)))
        maxabs = max(maxabs, abs(Float64(a) - Float64(b)))
    end
    (; maxulp, maxabs, nbits)
end

function run_orig(m, c)
    dargs = to_device(CuArray, deepcopy(c.args))
    # invokelatest: the originals are eval'd at runtime, after main() was compiled
    Base.invokelatest(launch_orig, getfield(m, c.orig), dargs, c.ndrange)
    [Array(a) for a in dargs[1:c.nout]]
end

function launch_orig(f, dargs, n)
    thr = 256
    @cuda threads=thr blocks=cld(n, thr) f(dargs...)
    CUDA.synchronize()
end

function run_ka(backend, adapt, c)
    dargs = to_device(adapt, deepcopy(c.args))
    c.kernel(backend, 256)(dargs...; ndrange=c.ndrange)
    KernelAbstractions.synchronize(backend)
    [Array(a) for a in dargs[1:c.nout]]
end

function main(originals)
    println("device: $(CUDA.name(CUDA.device()))  CUDA.jl $(pkgversion(CUDA))  runtime $(CUDA.runtime_version())")
    rows = ["case,source,orig_status,p1_bits_differing,p1,cpu_max_ulp,cpu_max_abs,cpu_max_px,p2,cuda_output_sha256"]
    fails = 0
    for c in cases()
        m = originals[c.source]
        orig_status = "ok"; ref = nothing
        try
            ref = run_orig(m, c)
        catch e
            orig_status = "orig failed: " * first(split(sprint(showerror, e), '\n'))
            if haskey(originals, c.source * "#fixed")
                ref = run_orig(originals[c.source * "#fixed"], c)
                tok = ONE_TOKEN_FIXES[c.source]
                why = occursin("jl_f_getfield", sprint(showerror, e)) ? "InvalidIRError: call to jl_f_getfield" : "error"
                orig_status = "orig as written does not compile ($why); compared against the one-token fix $(tok.first) -> $(tok.second)"
            end
        end
        ka_cu = run_ka(CUDABackend(), CuArray, c)
        ka_cpu = run_ka(CPU(), identity, c)
        if ref === nothing
            # no original to hold the port to: report CPU vs CUDA port instead
            p1 = "FAIL"; d1 = (; nbits=-1)
            d2 = compare(ka_cu, ka_cpu)
        else
            d1 = compare(ref, ka_cu)
            p1 = d1.nbits == 0 ? "PASS" : "FAIL"
            d2 = compare(ref, ka_cpu)
        end
        px = d2.maxabs * PX_PER_UNIT
        p2 = px <= E_G2_PX ? "PASS" : "FAIL"
        fails += (p1 == "FAIL") + (p2 == "FAIL")
        @printf("  %-34s P-1 %-4s (%d bits differ)  P-2 %s  cpu: %d ulp, %.2e abs, %.2e px  %s\n",
                c.name, p1, d1.nbits, p2, d2.maxulp, d2.maxabs, px, orig_status == "ok" ? "" : orig_status)
        push!(rows, join((c.name, c.source, replace(orig_status, ',' => ';'), d1.nbits, p1,
                          d2.maxulp, d2.maxabs, px, p2, output_sha(ka_cu)), ","))
    end
    out = joinpath(ROOT, "results", "portable-parity.csv")
    write(out, join(rows, "\n") * "\n")
    println("\nwrote $out")
    println(fails == 0 ? "PARITY: ALL GATES PASS" : "PARITY: $fails gate failure(s)")
    fails == 0 || exit(1)
end

if abspath(PROGRAM_FILE) == @__FILE__
    # loaded at top level so main() runs in a world that can see them (Julia 1.12)
    const ORIGINALS = Dict{String,Module}(f => load_originals(f) for f in unique(c.source for c in cases()))
    for f in keys(ONE_TOKEN_FIXES)
        ORIGINALS[f * "#fixed"] = load_originals(f; fixed=true)
    end
    main(ORIGINALS)
end
