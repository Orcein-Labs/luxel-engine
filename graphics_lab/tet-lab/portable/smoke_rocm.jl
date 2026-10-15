# smoke_rocm.jl — first run on real AMD hardware (AMD Developer Cloud, day one).
#
# Runs every TetKernels case on the GPU and judges it against:
#   R-0  a usable ROCm device exists, and its ISA is one compile_amd.jl built
#        clean offline (gfx942 / gfx90a / gfx1100)
#   R-1  P-2's gate, on AMD: |GPU − CPU| scaled to screen space <= 0.02 px
#        (E-G2's device-vs-host f32 tolerance)
#   R-2  determinism: two launches of the same case are bit-identical
#   R-3  graph replay: the launch captured into a device graph and replayed into
#        zeroed outputs is bit-identical to the direct launch (spirals H, H2
#        and I time graph-launched paths; this proves capture works first)
#   info cross-vendor bit-equality: the output hash vs the CUDA hash parity.jl
#        recorded on the RTX 5060 (results/portable-parity.csv). Kernels with
#        sin/cos/tanh are expected to differ (vendor libm); pure-arithmetic
#        kernels may or may not match (fma contraction).
# Then times each case (median of 50 launches) as a first, rough number.
#
#   julia --project=portable portable/smoke_rocm.jl          # on the AMD box
#   julia --project=portable portable/smoke_rocm.jl cuda     # local dry run of the
#                                                            # same gates on NVIDIA
#
# Writes results/portable-smoke-<isa>.csv. Exits non-zero on any R failure.
isempty(ARGS) && push!(ARGS, "rocm")
include("PortableBackend.jl"); using .PortableBackend
using KernelAbstractions, Printf, SHA, Statistics

include("TetKernels.jl"); using .TetKernels
include("KernelCases.jl"); using .KernelCases

const ROOT = dirname(@__DIR__)
const PX_PER_UNIT = 500.0 / 5.0
const E_G2_PX = 0.02
const BUILT_CLEAN = ("gfx942", "gfx90a", "gfx1100")

output_sha(outs) = bytes2hex(sha256(reduce(vcat, [reinterpret(UInt8, o) for o in outs])))

function run_case(backend, adapt, c)
    dargs = to_device(adapt, deepcopy(c.args))
    c.kernel(backend, 256)(dargs...; ndrange=c.ndrange)
    KernelAbstractions.synchronize(backend)
    [Array(a) for a in dargs[1:c.nout]], dargs
end

# capture one warmed launch into a device graph, zero the outputs, replay
function graph_replay(c)
    dargs = to_device(todev, deepcopy(c.args))
    launch(c.kernel, c.ndrange, dargs...); sync!()        # warm: compile outside capture
    exec = graph_instantiate(graph_capture(() -> launch(c.kernel, c.ndrange, dargs...)))
    foreach(a -> fill!(a, 0f0), dargs[1:c.nout]); sync!()
    graph_launch(exec); sync!()
    [Array(a) for a in dargs[1:c.nout]]
end

function time_case(c, dargs; reps=50)
    launch(c.kernel, c.ndrange, dargs...); sync!()        # warm
    median(map(_ -> (t0 = time_ns(); launch(c.kernel, c.ndrange, dargs...); sync!(); (time_ns() - t0) / 1e3), 1:reps))
end

function cuda_hashes()
    f = joinpath(ROOT, "results", "portable-parity.csv")
    isfile(f) || return Dict{String,String}()
    lines = readlines(f)
    col = findfirst(==("cuda_output_sha256"), split(lines[1], ','))
    col === nothing && return Dict{String,String}()
    Dict(split(l, ',')[1] => split(l, ',')[col] for l in lines[2:end])
end

# R-0, or the dry-run banner; returns (isa tag, r0 pass?)
function check_device()
    foreach(println, device_notes())
    if BACKEND_NAME == "cuda"
        println("DRY RUN: same gates on the CUDA backend; R-0 not applicable")
        return "cuda-dryrun", true
    end
    BACKEND_NAME == "rocm" || error("smoke test runs on rocm (or cuda for a dry run)")
    isa = device_isa()
    r0 = isa in BUILT_CLEAN
    println("R-0 ", r0 ? "PASS" : "FAIL", ": $isa ",
            r0 ? "was built clean offline" : "was NOT in the offline build set $(BUILT_CLEAN)")
    isa, r0
end

function main()
    tag, r0 = try
        check_device()
    catch e
        # HIP raises hipErrorNoDevice instead of returning an empty device list
        println("R-0 FAIL: no usable $(BACKEND_NAME) device: ", first(split(sprint(showerror, e), '\n')))
        exit(1)
    end
    cuda = cuda_hashes()
    pf(x) = x ? "PASS" : "FAIL"
    rows = ["case,r1_max_px,r1,r2,r3,output_sha256,matches_cuda_bits,median_us"]
    fails = r0 ? 0 : 1
    for c in cases()
        cpu, _ = run_case(CPU(), identity, c)
        a, dargs = run_case(BK, todev, c)
        b, _ = run_case(BK, todev, c)
        px = maximum(maximum(abs.(Float64.(x) .- Float64.(y))) for (x, y) in zip(a, cpu)) * PX_PER_UNIT
        h = output_sha(a)
        r1 = px <= E_G2_PX
        r2 = h == output_sha(b)
        r3 = try
            output_sha(graph_replay(c)) == h
        catch e
            println("    R-3 graph capture threw: ", first(split(sprint(showerror, e), '\n')))
            false
        end
        xv = haskey(cuda, c.name) ? string(cuda[c.name] == h) : "no-cuda-ref"
        us = time_case(c, dargs)
        fails += !r1 + !r2 + !r3
        @printf("  %-34s R-1 %s (%.2e px)  R-2 %s  R-3 %s  cuda-bits %-11s  %8.1f us\n",
                c.name, pf(r1), px, pf(r2), pf(r3), xv, us)
        push!(rows, join((c.name, px, pf(r1), pf(r2), pf(r3), h, xv, us), ","))
    end
    out = joinpath(ROOT, "results", "portable-smoke-$tag.csv")
    write(out, join(rows, "\n") * "\n")
    println("\nwrote $out")
    println(fails == 0 ? "SMOKE: ALL GATES PASS" : "SMOKE: $fails gate failure(s)")
    fails == 0 || exit(1)
end

if abspath(PROGRAM_FILE) == @__FILE__
    main()
end
