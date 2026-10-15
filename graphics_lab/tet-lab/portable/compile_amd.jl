# compile_amd.jl — offline AMD build of every TetKernels kernel, no AMD GPU needed.
#
# For each kernel case and each target ISA this runs the same pipeline a live
# ROCBackend launch would (KernelAbstractions -> AMDGPU.jl codegen -> ROCm
# device libs -> LLVM AMDGPU backend -> ld.lld), producing the HSA code object
# the GPU would load. It then reports register use, spills and scratch, and
# fails a case if the final LLVM module still declares any non-intrinsic
# function (an unresolved device-library or CUDA-only call).
#
#   julia --project=portable portable/compile_amd.jl            # all targets
#   julia --project=portable portable/compile_amd.jl gfx942     # one target
#
# Writes results/portable-amd-compile.csv (a filtered run: ...-compile-<targets>.csv).
using AMDGPU, KernelAbstractions, GPUCompiler, LLVM, Printf
const KA = KernelAbstractions

include("TetKernels.jl"); using .TetKernels
include("KernelCases.jl"); using .KernelCases

const ROOT = dirname(@__DIR__)

# name => (HIP arch string, wavefront 64?)
const TARGETS = [
    "gfx942"  => ("gfx942:sramecc+:xnack-", true),   # MI300X / MI300A / MI325X (CDNA3)
    "gfx90a"  => ("gfx90a:sramecc+:xnack-", true),   # MI210 / MI250X (CDNA2)
    "gfx1100" => ("gfx1100", false),                 # RX 7900 XTX / W7900 (RDNA3)
]

# mirrors AMDGPU.Compiler._compiler_config, minus the device query
function config_for(arch::String, wave64::Bool)
    dev_isa, features = AMDGPU.Compiler.parse_llvm_features(arch)
    features = (isempty(features) ? "" : features * ",") *
               (wave64 ? "-wavefrontsize32,+wavefrontsize64" : "+wavefrontsize32,-wavefrontsize64")
    CompilerConfig(GCNCompilerTarget(; dev_isa, features),
                   AMDGPU.Compiler.HIPCompilerParams(wave64, true);
                   kernel=true, name=nothing, always_inline=true)
end

device_type(a::AbstractVector{T}) where {T} = AMDGPU.Device.ROCDeviceArray{T,1,AMDGPU.Device.AS.Global}
device_type(x) = typeof(x)

function job_for(c::Case, config)
    kern = c.kernel(ROCBackend(), 256)
    ndrange, _, iterspace, _ = KA.launch_config(kern, (c.ndrange,), nothing)
    ctx = KA.mkcontext(kern, ndrange, iterspace)
    tt = Tuple{typeof(ctx), map(device_type, c.args)...}
    CompilerJob(methodinstance(typeof(kern.f), tt), config)
end

metric(asm, key) = (m = match(Regex("\\Q$key\\E:\\s*(\\d+)"), asm); m === nothing ? -1 : parse(Int, m[1]))

function unresolved_calls(job)
    JuliaContext() do _
        mod, _ = GPUCompiler.compile(:llvm, job)
        [LLVM.name(f) for f in LLVM.functions(mod)
         if LLVM.isdeclaration(f) && !startswith(LLVM.name(f), "llvm.")]
    end
end

function main(selected)
    targets = isempty(selected) ? TARGETS : filter(t -> first(t) in selected, TARGETS)
    rows = String["target,case,code_object_bytes,vgpr,sgpr,vgpr_spill,sgpr_spill,scratch_bytes,unresolved,status"]
    failures = 0
    for (tname, (arch, wave64)) in targets
        config = config_for(arch, wave64)
        println("== $tname ($arch, wave$(wave64 ? 64 : 32))")
        for c in cases()
            job = job_for(c, config)
            status = "ok"; bytes = -1; asm = ""; unres = String[]
            try
                bytes = length(AMDGPU.Compiler.hipcompile(job).obj)
                asm = JuliaContext() do _; GPUCompiler.compile(:asm, job)[1]; end
                unres = unresolved_calls(job)
                isempty(unres) || (status = "unresolved")
            catch e
                status = "error: " * first(split(sprint(showerror, e), '\n'))
            end
            vg, sg = metric(asm, ".vgpr_count"), metric(asm, ".sgpr_count")
            vs, ss = metric(asm, ".vgpr_spill_count"), metric(asm, ".sgpr_spill_count")
            scratch = metric(asm, ".private_segment_fixed_size")
            # VGPR spills or any scratch hit memory: fail. SGPR-only spills with
            # zero scratch land in VGPR lanes (v_writelane), cheap: note only.
            if status == "ok" && (vs > 0 || scratch > 0)
                status = "spills to scratch"
            elseif status == "ok" && ss > 0
                status = "ok (sgpr spill to vgpr lanes)"
            end
            startswith(status, "ok") || (failures += 1)
            @printf("  %-34s %6d B  vgpr %3d  sgpr %3d  spill %d/%d  scratch %d  %s\n",
                    c.name, bytes, vg, sg, vs, ss, scratch, status)
            push!(rows, join((tname, c.name, bytes, vg, sg, vs, ss, scratch,
                              join(unres, ";"), replace(status, ',' => ';')), ","))
        end
    end
    # a filtered run gets its own file, so it cannot overwrite the full evidence CSV
    suffix = isempty(selected) ? "" : "-" * join(selected, "-")
    out = joinpath(ROOT, "results", "portable-amd-compile$suffix.csv")
    write(out, join(rows, "\n") * "\n")
    println("\nwrote $out")
    println(failures == 0 ? "ALL CASES BUILT CLEAN" : "$failures case(s) not clean")
    failures == 0 || exit(1)
end

if abspath(PROGRAM_FILE) == @__FILE__
    main(ARGS)
end
