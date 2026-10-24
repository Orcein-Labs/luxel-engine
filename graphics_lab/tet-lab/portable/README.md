# portable/ — TetCage kernels and drivers on any GPU vendor

Everything in `gpu/` was written against CUDA.jl and measured on an RTX 5060.
This folder runs the same work on AMD GPUs (ROCm, through AMDGPU.jl), on NVIDIA
and on the CPU, from one source. The `gpu/` scripts stay untouched as the
recorded evidence; these files are held to it.

## What is here

| File | Role |
|---|---|
| `TetKernels.jl` | KernelAbstractions ports of all 11 device kernels in `gpu/`, arithmetic verbatim |
| `KernelCases.jl` | Deterministic inputs for every kernel, shaped like the drivers' launches |
| `PortableBackend.jl` | The only vendor-specific code: arrays, launch, sync, event timing, pinning, graphs. Loads only the selected vendor package |
| `parity.jl` | NVIDIA only. Each port must be bit-identical to its `@cuda` original, parsed from `gpu/` |
| `compile_amd.jl` | No GPU needed. Builds every kernel to AMD code objects for gfx942, gfx90a and gfx1100 |
| `smoke_rocm.jl` | First run on the AMD box: gates R-0 to R-3, then rough timings. `cuda` argument = local dry run |
| `spiral_*_portable.jl` | Generated copies of the five `gpu/` drivers. Do not edit; rerun `port_driver.py` |
| `port_driver.py` | Generates the portable drivers from `gpu/`; refuses if CUDA-specific code survives |
| `compare_rows.py` | Checks a backend's driver CSVs against the recorded `gpu/` results |

Every script takes the backend as its first argument (`cuda`, `rocm`, `cpu`) and
writes its CSV next to the original in `../results/` with a `-portable-<backend>`
suffix. It never overwrites a recorded result.

## Day one on the AMD Developer Cloud

Run from the root of a TetCage clone (the public mirror of this lab). Each
step has a pass condition; stop at the first failure and keep its output.

1. **Confirm the GPU.** `rocminfo | grep -m1 gfx` should print `gfx942` (MI300X).
   Another ISA still works if `compile_amd.jl` lists it; otherwise add it to
   `TARGETS` there and rerun step 4 locally first.
2. **Install Julia 1.12.** `curl -fsSL https://install.julialang.org | sh -s -- -y`,
   then `juliaup add 1.12 && juliaup default 1.12`.
3. **Get the code.** `git clone https://github.com/latentcollapse/TetCage && cd TetCage`.
4. **Instantiate.** `julia --project=portable -e 'using Pkg; Pkg.instantiate(); Pkg.precompile()'`.
   CUDA.jl is in the environment for the NVIDIA paths. On the AMD box it is
   installed but never loaded; its large runtime artifacts are lazy and do not download.
5. **Smoke test, about a minute.** `julia --project=portable portable/smoke_rocm.jl`
   must end `SMOKE: ALL GATES PASS` (exit 0). R-3 proves HIP graph capture of a
   KernelAbstractions launch works; spirals H, H2 and I depend on it.
6. **Spiral E, the correctness gates on real cages.**
   `julia --project=portable portable/spiral_e_gpu_portable.jl rocm` must end
   `SPIRAL E GATES: ALL PASS`. Run it twice and `cmp` the two CSVs (E-G5).
7. **Performance spirals.** For each of `spiral_f_perf`, `spiral_h_fusion`,
   `spiral_h2_wind_ladder`, `spiral_i_comparators`:
   `julia --project=portable portable/<name>_portable.jl rocm`.
   Exit 0 means every gate held. **Exit 2 means a performance hypothesis
   registered on the RTX 5060 was falsified on this hardware**: correctness
   passed and the data is complete. That is a result, not a crash; the CSV
   names the hypothesis. Any other non-zero exit is a correctness failure.
8. **Check rows.** `python3 portable/compare_rows.py rocm`. Sizes and upload
   bytes must match; correctness columns are reported (last-digit differences
   from AMD's math library are expected); timings are reported as AMD/RTX 5060 ratios.
9. **Bring the results home.** Copy `results/*-portable-rocm.csv` and
   `results/portable-smoke-gfx942.csv` off the VM before it is released, then
   into this lab's `results/`.

## Reading the timings

The portable drivers launch through KernelAbstractions, which adds per-launch
host work that raw `@cuda` does not do. On the RTX 5060 that costs up to about 1.4× on
directly launched microsecond kernels and nothing on graph-replayed paths
(see the evidence doc). So compare AMD numbers with
`results/*-portable-cuda.csv`, not with the original `gpu/` CSVs.

## Regenerating

```sh
python3 portable/port_driver.py                         # after changing a gpu/ driver
julia --project=portable portable/parity.jl             # NVIDIA: ports still bit-identical
julia --project=portable portable/compile_amd.jl        # AMD builds still clean
julia --project=portable portable/smoke_rocm.jl cuda    # smoke logic dry run
python3 portable/compare_rows.py cuda                   # after rerunning the drivers on cuda
```

## Not covered

- The Lava/Vulkan render path in `vk/`. MI300-class accelerators are
  compute-only, and whether they expose a usable Vulkan device is unverified.
  The compute path here does not depend on it.
- Throughput tuning for AMD (wavefront 64, workgroup size). The drivers keep
  the RTX 5060's 256-thread groups so the first run measures the same program.
