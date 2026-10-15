# TETCAGE — AMD Portability: one kernel source for NVIDIA, AMD and CPU

Prepared 2026-10-05 ahead of AMD Developer Cloud access. Every TetCage
measurement so far ran on an RTX 5060 through CUDA.jl, which is the first gap an
AMD reader sees. This pass makes the whole compute study run on AMD GPUs
(ROCm, through AMDGPU.jl) without changing what it computes, and proves as much
of that as is possible before touching AMD hardware.

Code and runbook: [`portable/`](portable/README.md). The `gpu/` drivers and
their recorded results are untouched; everything below is held to them.

## Approach

- **Kernels.** All 11 device kernels in `gpu/` (`deform_corners!`,
  `reconstruct!`, `pair_dev!`, `fused_deform_reconstruct!`,
  `fused_batched_wind!`, `reconstruct_batched!`, `rot_direct!`, `lbs!`,
  `morph_lerp!`, `fused_batched_shared!`, `probe_div!`) are ported to
  KernelAbstractions in `portable/TetKernels.jl`, keeping argument order,
  expression order, f32 literals and 1-based index math verbatim. All are 1-D
  elementwise: no shared memory, atomics or warp shuffles, so nothing
  vendor-specific was needed.
- **Drivers.** `portable/port_driver.py` generates a portable copy of each of the
  five spiral drivers (E, F, H, H2, I) by mechanical substitution through one
  shim, `portable/PortableBackend.jl`: device arrays, unsynchronized launch,
  sync, event timing (`CUDA.@elapsed` → `AMDGPU.@elapsed`), host pinning
  (`CUDA.pin` → `hipHostRegister`) and graphs (CUDA graphs → HIP graphs). The
  generator refuses to write a file in which CUDA-specific code survives.
- **Performance hypotheses** (F-G2 frame budget, HF-G3 fusion headroom, HF-G4′
  decomposition sanity) were registered on the RTX 5060. In the portable
  copies they are recorded rather than asserted: a falsified hypothesis on new
  hardware is a result, so the run completes, names it in the CSV and exits 2.
  Correctness gates (provenance, identity, D-contract, snapping, fragments)
  remain hard asserts.

## Evidence (all on this machine, no AMD GPU present)

### P — ports are faithful (`results/portable-parity.csv`)

Each original `@cuda` kernel is parsed straight out of its `gpu/` file and run
next to its port on the RTX 5060 (CUDA.jl 6.3.1, runtime 13.3).

- **P-1, bit-identity: 15/15 cases, 0 bits differ.** This covers every kernel,
  with deform and fused-deform in all three families.
- **P-2, port on CPU vs original on GPU: worst 3.05e-3 px** at d = 5, against
  E-G2's 0.02 px device-vs-host tolerance.
- **probe3 defect found.** `gpu/probe3.jl` writes `threadIdx.x` (no call
  parentheses); CUDA.jl rejects it as invalid IR (`call to jl_f_getfield`), so
  that probe never ran as written. Parity compares `probe_div!` against the
  one-token fix and says so in the CSV.

### A — every kernel builds for AMD offline (`results/portable-amd-compile.csv`)

The same pipeline a live ROCm launch runs (AMDGPU.jl codegen, ROCm 7.2.4 device
libraries, LLVM AMDGPU backend, `ld.lld`) produced a real HSA code object for
every case on three targets, with no unresolved calls:

| Target | Hardware | Cases | Max VGPR | Max SGPR | Scratch | Code objects |
|---|---|---|---|---|---|---|
| gfx942 | MI300X / MI300A / MI325X | 15/15 | 56 | 106 | 0 B | 15.6–51.6 KB |
| gfx90a | MI210 / MI250X | 15/15 | 56 | 106 | 0 B | 15.7–52.7 KB |
| gfx1100 | RX 7900 XTX / W7900 | 15/15 | 62 | 62 | 0 B | 14.7–56.0 KB |

One note: `reconstruct_batched!` reaches the SGPR ceiling on the two CDNA
targets (16 spills on gfx942, 34 on gfx90a). With zero scratch these spill
into VGPR lanes, which is cheap. Narrowing the integer counts to Int32 does not
change it; the pressure comes from eight device-array arguments plus the
KernelAbstractions launch context.

### D — the portable drivers reproduce the study (`compare_rows.py cuda`)

All five portable drivers, run on CUDA, pass every gate, and every
deterministic column matches the recorded results exactly:

| Spiral | Rows | Deterministic columns | Gates |
|---|---|---|---|
| E (hardware confirmation) | 12 | all rows byte-identical; E-G5 byte-identical across two processes | all pass |
| F (frame budget) | 14 | identical | all pass, F-G2 held |
| H (fusion + graphs) | 14 | identical | all pass, HF-G3/HF-G4′ held |
| H2 (wind ladder) | 28 | identical | HF2-G0/G1 pass |
| I (comparators) | 140 | identical | all pass |

On the CPU backend the same drivers pass every correctness gate. As expected,
H's launch-floor hypotheses are falsified there (9 and 12 checks in two runs;
the count is timing noise, not code), because a CPU has no
launch floor. That run is the test that falsified-hypothesis reporting works.

### Launch overhead of the portable path (CUDA, portable / recorded medians)

Range over three CUDA runs of the portable drivers:

| Spiral | Direct launches | Graph-replayed |
|---|---|---|
| F | 1.20–1.42× | — |
| H | 1.14–1.42× | 0.92–0.98× |
| H2 | 0.99–1.16× | — |
| I | 1.03–1.06× | — |

KernelAbstractions adds host-side launch work. It is visible on single
microsecond kernels and absent once launches are graph-replayed. The recordings
also differ in CUDA.jl version (6.4 vs 6.3.1) and date, so these ratios are
indicative. **AMD timings should be compared against the
`*-portable-cuda.csv` files, not the original `gpu/` results.**

### S — the day-one smoke test is proven on NVIDIA (`results/portable-smoke-cuda-dryrun.csv`)

`smoke_rocm.jl cuda` runs the AMD smoke test's gates on the RTX 5060: R-1
(≤ 0.02 px vs CPU), R-2 (determinism) and R-3 (graph capture + replay
bit-identical to a direct launch) pass on all 15 cases. Outputs also match
`parity.jl`'s hashes from a separate process. In `rocm` mode on this machine
the test stops cleanly at R-0 (`hipErrorNoDevice`), exit 1.

## What remains for the hardware

- **Execution on AMD**: R-0 to R-3, then spirals E to I with `rocm`
  (runbook in `portable/README.md`). Expected: E-G2..G5 pass with last-digit
  differences from AMD's libm. Performance hypotheses may or may not hold, and
  either outcome is data.
- **HIP graph capture of KernelAbstractions launches** is the main unproven
  mechanism. The launch path with a static workgroup size makes no device
  queries and the drivers warm up before capturing, so it should be
  capture-safe. R-3 checks this in the first minute.
- **Not in scope**: the Lava/Vulkan render path (`vk/`). MI300-class parts are
  compute accelerators and a usable Vulkan device on them is unverified.
  Tuning for wavefront 64 and workgroup size is also out of scope; the first
  run keeps the RTX 5060's 256-thread groups so it measures the same program.
