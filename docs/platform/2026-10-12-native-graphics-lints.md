# Native graphics strict lint cleanup — 2026-10-12

This bounded cleanup clears the pre-existing strict Clippy findings that blocked the native graphics crate and its asset-contract dependency. Changes preserve behavior: modulo checks use `is_multiple_of`, indexed three-channel updates iterate over the same three elements, membership uses the equivalent `contains`, checked division still maps a zero divisor to zero, nested checks use equivalent let-chain conditions, and default derives reproduce the previous zero/false defaults. Test-only clone and error extraction cleanups preserve their assertions. The benchmark test module moved after its helper so no production code moved.

## Verification

Using the isolated cache `CARGO_TARGET_DIR=/tmp/luxel-r2-working-target`:

```sh
cargo clippy --manifest-path world_core/crates/native_graphics_contract/Cargo.toml --locked --all-targets -- -D warnings
cargo clippy --manifest-path world_core/crates/asset_contract/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path world_core/crates/native_graphics_contract/Cargo.toml --locked --lib --bin luxel-native-graphics-contract --test render_policy --test converge3 --test converge4 --test ibl --test resolution_axis
cargo test --manifest-path world_core/crates/asset_contract/Cargo.toml --locked --lib
```

Both strict Clippy commands pass with no lint flags or source suppressions. The native focused test command passes 74 tests and ignores 8 kit-dependent tests; the asset-contract library suite passes 9 tests. Together these runs cover the R-2 resolution axis, supervisor authority rejection, render policy, IBL, the edited convergence tests, library unit tests, binary unit tests, and alpha coverage. The ignored v6 replay packet needed by one native library test was linked read-only from the primary checkout into this worktree's ignored `artifacts/` path; the primary asset was not changed. No GPU test was run.

Raw baseline and final logs are in the ignored directory `artifacts/crispness-20261012/`: `clippy-native-baseline.log`, `clippy-native-test-residual-baseline.log`, `clippy-asset-baseline.log`, `cargo-fmt-package-baseline.log`, `clippy-native-fixed.log`, `clippy-native-full-fixed.log`, `clippy-asset-fixed.log`, `tests-focused-fixed.log`, and `tests-asset-fixed.log`. The baseline Clippy ledger contained 15 findings in the native crate plus one in `asset_contract/src/render.rs`; the all-target run exposed four more test-only findings after the first blockers were removed. All 20 findings are fixed.

The package-wide formatter check remains a known baseline failure. `cargo fmt --manifest-path world_core/crates/native_graphics_contract/Cargo.toml --package luxel-native-graphics-contract -- --check --config skip_children=true` exits 1 with 2,849 diff lines, including untouched integration tests such as `tests/alpha_mask.rs`, `tests/asset_projection.rs`, and `tests/calibration.rs`. No package-wide formatting was applied; changed lint sites were kept local.

## Files changed

- `world_core/crates/asset_contract/src/render.rs`
- `world_core/crates/native_graphics_contract/src/deformation.rs`
- `world_core/crates/native_graphics_contract/src/ibl.rs`
- `world_core/crates/native_graphics_contract/src/input_session.rs`
- `world_core/crates/native_graphics_contract/src/lib.rs` (one checked-division expression; also contains the R-2 implementation)
- `world_core/crates/native_graphics_contract/src/main.rs` (test module moved after its helper; also contains the R-2 CLI implementation)
- `world_core/crates/native_graphics_contract/src/material_maps.rs`
- `world_core/crates/native_graphics_contract/src/render_policy.rs`
- `world_core/crates/native_graphics_contract/tests/converge3.rs`
- `world_core/crates/native_graphics_contract/tests/converge4.rs`
- `world_core/crates/native_graphics_contract/tests/ibl.rs`
- `world_core/crates/native_graphics_contract/tests/render_policy.rs`
- `world_core/crates/native_graphics_contract/tests/support/synthetic_backdrop.rs`

No commit was created. The worktree fixture symlink is local and ignored.

## Commit gate completed

At the user's request to commit the verified checkpoint, the native-graphics
and asset-contract packages were formatted with `cargo fmt --package` using
their manifests. Both package formatter checks now pass. Strict all-target
Clippy was rerun without suppressions and passes for both crates; the focused
native suite still passes 74 tests (8 kit-dependent ignored), and the asset
unit suite passes 9. Raw receipts: `artifacts/converge-next-20261012/`.
This supersedes the earlier formatter-debt disposition while retaining its
baseline log. Formatting touches additional existing files but introduces no
new renderer policy or functional feature.
