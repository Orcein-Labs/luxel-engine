# PortableBackend — the only place the portable drivers touch a vendor API.
#
# The backend is chosen by the first command-line argument: cuda (default),
# rocm or cpu. Every function here maps one CUDA.jl call the gpu/ drivers made
# onto its AMDGPU.jl (or plain-Julia) equivalent with the same semantics:
#
#   gpu/ original                     portable
#   CuArray(x)                        todev(x)
#   CUDA.zeros(T, n)                  devzeros(T, n)
#   @cuda threads=.. blocks=.. k!(a…) launch(k!, n, a…)        (no sync, like @cuda)
#   CUDA.synchronize()                sync!()
#   CUDA.@elapsed f!()                gpu_elapsed(f!)          (GPU events: CUDA / HIP)
#   CUDA.pin(x)                       pin_host(x)              (cuMemHostRegister / hipHostRegister)
#   capture / instantiate / launch    graph_capture / graph_instantiate / graph_launch
#
# On cpu() there are no events, pinning or graphs: elapsed is wall time and a
# "graph" is the captured closure replayed. That mode exists to exercise the
# drivers' logic, not to produce timings.
module PortableBackend

using KernelAbstractions

export BACKEND_NAME, BK, todev, devzeros, launch, sync!, gpu_elapsed, pin_host,
       graph_capture, graph_instantiate, graph_launch, device_notes, device_isa,
       soft_gate, soft_gate_notes, report_soft_gates

const BACKEND_NAME = get(ARGS, 1, "cuda")
BACKEND_NAME in ("cuda", "rocm", "cpu") || error("backend must be cuda, rocm or cpu, got $(BACKEND_NAME)")

const WORKGROUP = 256   # the gpu/ drivers' thread-block size

# Load only the selected vendor package: on an AMD-only VM, CUDA.jl is never
# touched (and vice versa). Backend functions are @eval'd after the `using`,
# because macros such as CUDA.@elapsed expand when a function is defined.
if BACKEND_NAME == "cuda"
    @eval using CUDA
    @eval begin
        const BK = CUDA.CUDABackend()
        todev(a::AbstractArray) = CUDA.CuArray(a)
        gpu_elapsed(f) = CUDA.@elapsed f()
        pin_host(a::Array) = (CUDA.pin(a); a)
        graph_capture(f) = CUDA.capture(f)
        graph_instantiate(g) = CUDA.instantiate(g)
        graph_launch(e) = CUDA.launch(e)
        _vendor_notes() = ["device: $(CUDA.name(CUDA.device())) cap $(CUDA.capability(CUDA.device()))",
                           "cuda runtime: $(CUDA.runtime_version())"]
        device_isa() = "sm_" * replace(string(CUDA.capability(CUDA.device())), "." => "")[1:end-1]
    end
elseif BACKEND_NAME == "rocm"
    @eval using AMDGPU
    @eval begin
        const BK = AMDGPU.ROCBackend()
        todev(a::AbstractArray) = AMDGPU.ROCArray(a)
        gpu_elapsed(f) = AMDGPU.@elapsed f()
        pin_host(a::Array) = (AMDGPU.Mem.register(Ptr{Cvoid}(pointer(a)), sizeof(a)); a)
        graph_capture(f) = AMDGPU.HIP.capture(f)
        graph_instantiate(g) = AMDGPU.HIP.instantiate(g)
        graph_launch(e) = AMDGPU.HIP.launch(e)
        _vendor_notes() = (dev = AMDGPU.device();
            ["device: $(AMDGPU.HIP.name(dev)) arch $(AMDGPU.HIP.gcn_arch(dev)) wavefront $(AMDGPU.HIP.wavefrontsize(dev))",
             "amdgpu.jl: $(pkgversion(AMDGPU))"])
        device_isa() = String(first(split(AMDGPU.HIP.gcn_arch(AMDGPU.device()), ':')))
    end
else
    const BK = CPU()
    todev(a::AbstractArray) = copy(a)
    gpu_elapsed(f) = (t0 = time_ns(); f(); (time_ns() - t0) / 1e9)
    pin_host(a::Array) = a
    graph_capture(f) = f
    graph_instantiate(g) = g
    graph_launch(e) = (e(); nothing)
    _vendor_notes() = ["device: KernelAbstractions CPU() ($(Threads.nthreads()) threads)"]
    device_isa() = "cpu"
end

devzeros(::Type{T}, n) where {T} = KernelAbstractions.zeros(BK, T, n)

# like @cuda: enqueue and return, no synchronization
launch(k, n, args...) = (k(BK, WORKGROUP)(args...; ndrange=n); nothing)
sync!() = KernelAbstractions.synchronize(BK)

device_notes() = [_vendor_notes();
    "backend: $(BACKEND_NAME) via KernelAbstractions $(pkgversion(KernelAbstractions)), workgroup $(WORKGROUP)"]

# Performance hypotheses (frame budget, fusion headroom, timing decomposition)
# were registered on an RTX 5060. On other hardware a falsified hypothesis is a
# result, not a crash: record it, keep measuring, and exit 2 at the end.
# Correctness gates (provenance, identity) stay hard @asserts in the drivers.
const SOFT_FAILS = String[]

function soft_gate(cond::Bool, msg::AbstractString)
    cond && return true
    push!(SOFT_FAILS, msg)
    println("FALSIFIED (perf hypothesis, run continues): ", msg)
    false
end

soft_gate_notes() = isempty(SOFT_FAILS) ? ["perf hypotheses: all held on this backend"] :
    ["perf hypothesis FALSIFIED on this backend: " * m for m in SOFT_FAILS]

function report_soft_gates()
    if isempty(SOFT_FAILS)
        println("PERF HYPOTHESES: all held on backend $(BACKEND_NAME)")
    else
        println("PERF HYPOTHESES: $(length(SOFT_FAILS)) FALSIFIED on backend $(BACKEND_NAME) (correctness gates passed; data is complete). The gate summary line above predates this check.")
        exit(2)
    end
end

end # module
