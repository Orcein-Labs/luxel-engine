#!/usr/bin/env bash
# The CPU-only gate: every check that runs without Julia, a GPU or the
# Codeweald reference content. CI runs this; run it locally to reproduce CI.
#
# The excluded targets all build worlds or frames through the Julia worker.
# Measured 2026-10-17 by running the full suites with LUXEL_JULIA=/bin/false
# and no reference content: every failure was julia_worker_failure (or the GPU
# test lock poisoned by one). They stay in the local GPU gate.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$root/world_core"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

julia_packages=(
    luxel-certification-authority
    luxel-native-graphics-contract
    luxel-project-ledger
    luxel-reference-runtime
)
cargo test --workspace $(printf -- '--exclude %s ' "${julia_packages[@]}")
cargo test --doc $(printf -- '-p %s ' "${julia_packages[@]}")
cargo test -p luxel-reference-runtime --lib --bins
cargo test -p luxel-certification-authority --lib --bins --test rigging_authority
cargo test -p luxel-project-ledger --bins --test scene
cargo test -p luxel-project-ledger --lib -- \
    --skip tests::native_ \
    --skip tests::traversal_identity_must_be_bound_to_the_project_objective

# native-graphics-contract: every integration target except the ones that
# start a graphics worker.
graphics_worker_targets=" depth_capture feature_axes live native_graphics presented_session resolution_axis session window "
graphics_targets=()
for target in "$root"/world_core/crates/native_graphics_contract/tests/*.rs; do
    name="$(basename "$target" .rs)"
    [[ "$graphics_worker_targets" == *" $name "* ]] || graphics_targets+=(--test "$name")
done
cargo test -p luxel-native-graphics-contract --lib --bins "${graphics_targets[@]}"

# The Python suite shells out to the debug CLIs.
cargo build --workspace --bins

cd "$root/tests"
julia_modules=" test_erosion test_erosion_julia_worker test_luxel_control_plane test_luxel_engine_neutral test_luxel_native_mvp test_semantic_kernel test_semantic_kernel_goal003 test_semantic_kernel_persistence test_semantic_kernel_worker "
modules=()
for file in test_*.py; do
    name="${file%.py}"
    [[ "$julia_modules" == *" $name "* ]] || modules+=("$name")
done
# test_luxel_control_plane: the playtest-and-capture test builds a world
# through Julia; the other two are CPU-only.
PYTHONPATH="$root" python3 -m unittest "${modules[@]}" \
    test_luxel_control_plane.ControlPlaneCliTest.test_profile_create_open_and_current_pointer_are_typed \
    test_luxel_control_plane.ControlPlaneCliTest.test_forged_candidate_identity_is_rejected_before_publication
