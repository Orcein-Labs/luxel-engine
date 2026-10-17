# Vendored dependencies

## Lava

`vendor/Lava` is SimonDanisch/Lava.jl at commit
`11c7e31bdf62408d22bf379e9e59510f69d2103e` (git tree
`795df8f1fb847d261f4da4d944448c4d96683862`), `src/`, `Project.toml`,
`LICENSE` and `README.md` only, with these patches applied in order:

| Patch | What it adds | Why |
|---|---|---|
| `lava-0001-fragment-discard.patch` | `Lava.discard()`: a fragment-stage block terminator emitted as SPIR-V `OpKill` | Alpha-mask materials (CONVERGE-2 N-6). Upstream has no discard. |
| `lava-0002-multisample.patch` | Multisampled offscreen targets (`LavaFramebuffer(samples=)`, drawn into an MSAA image and resolved by average into the readable one), `GraphicsPipeline(alpha_to_coverage=)`, and `alphaToOne` enabled at device creation when supported | Alpha-to-coverage foliage (CONVERGE-3 L-2a). Upstream renders single-sample only. Single-sample pipelines, framebuffers and cache keys are exactly upstream's. |
| `lava-0003-draw-resolve.patch` | `draw!(...; resolve=false)` on an offscreen target skips the multisample resolve for that draw | One resolve per scene pass instead of one per draw (985600e). The default `resolve=true` is upstream's behaviour. |
| `lava-0004-fold-uniform-function-ptr-phis.patch` | `fold_uniform_function_ptr_phis!`, run in `run_llvm_passes!`: a pointer phi whose incoming values all name one Function-storage address (or are undef) becomes that address | StructurizeCFG left `phi ptr` chains over a partially written sret box, which emit as a Function-pointer `OpPhi` (VUID-VkShaderModuleCreateInfo-pCode-08737) plus an illegal pointer `OpBitcast`. 14 terrain and point-light fragment shaders were invalid SPIR-V (audit L-15). |
| `lava-0005-recorded-upload.patch` | `upload_recorded!(dst, data)`: records a host→device copy into the active batch through a staging buffer of its own, plus a transfer→all-commands barrier, and never submits or waits | Per-instance uploads in `_mesh_resources!` used `upload!`, whose `flush!` host-waits on all GPU work in flight, inside every panning frame (audit L-02). |
| `lava-0006-type-stable-arg-map.patch` | `map_args(f, args)`: a `@generated`, type-stable tuple map, used for draw, compute launch and ray-tracing argument conversion | Base `map` over a tuple of more than 31 elements boxes every element. The 47-argument mesh draw allocated about 7 MiB a frame (audit L-05). |

Patches 0001–0003 and 0005 only add code. Patch 0006 changes how host-side
arguments are converted, not what they convert to, and no generated code. A shader that does not call
`discard()` compiles to the same SPIR-V as upstream, a single-sample target
creates the same pipelines and images as upstream, `draw!` without `resolve=`
resolves as upstream does, and `upload!` is untouched. Patch 0004 does change
the SPIR-V of the shaders it fixes (and only those: a module with no
Function-pointer phi is untouched); their pixels are unchanged: all 18 parity
frames from `tools/verify_identity.py` render byte-identical to a render of the
commit before the patch. (Compare
against such a render; the frozen references fail by known drift, audit L-07.)
`LAVA_REVISION` stays the base commit.

`tools/verify_identity.py` does not run spirv-val. The validator that ships in
`SPIRV_Tools_jll` (v2025.4) accepts the L-15 shaders; a current
`spirv-val --target-env vulkan1.3 --scalar-block-layout` rejects them.

To verify: check out upstream at the commit above, copy `src/` over a clean
directory, apply the patches with `patch -p1`, and `diff -r` against
`vendor/Lava/src`.
