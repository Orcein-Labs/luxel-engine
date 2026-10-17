#!/usr/bin/env bash
# The local GPU gate: the Julia/Lava GPU suites under the Khronos validation
# layer with sync validation on. A suite failure or any validation message
# fails the gate. CI cannot run this (no GPU); run it before merging GPU work.
#
# LUXEL_VVL_LAYER_PATH  directory holding VkLayer_khronos_validation.json
#                       (passed to the loader as VK_ADD_LAYER_PATH). Required
#                       unless LUXEL_GPU_GATE_NO_VALIDATION=1.
# LUXEL_JULIA           julia binary (default: julia on PATH).
# LUXEL_GPU_TIMEOUT     per-suite timeout in seconds (default: 3600).
#
# Each suite takes the shared GPU lock, so the gate waits behind other GPU jobs
# instead of racing them. Logs land in .local/gpu-gate/.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
julia="${LUXEL_JULIA:-julia}"
timeout_s="${LUXEL_GPU_TIMEOUT:-3600}"
logs="$root/.local/gpu-gate"
mkdir -p "$logs"

suites=(
    lava_adapter
    present_blit_gpu
    camera_continuity
    local_lighting_gpu
    local_shadows_gpu
    shadow_cascades_gpu
)

validation=1
if [[ "${LUXEL_GPU_GATE_NO_VALIDATION:-}" == 1 ]]; then
    validation=0
    echo "gpu-gate: VALIDATION LAYER OFF (LUXEL_GPU_GATE_NO_VALIDATION=1); this run does not count as a clean gate" >&2
elif [[ -z "${LUXEL_VVL_LAYER_PATH:-}" || ! -f "$LUXEL_VVL_LAYER_PATH/VkLayer_khronos_validation.json" ]]; then
    echo "gpu-gate: set LUXEL_VVL_LAYER_PATH to the directory holding VkLayer_khronos_validation.json" >&2
    exit 2
fi

failed=()
cd "$root/graphics_lab"
for suite in "${suites[@]}"; do
    vvl_log="$logs/vvl-$suite.txt"
    rm -f "$vvl_log"
    layer_env=()
    if (( validation )); then
        layer_env=(
            VK_ADD_LAYER_PATH="$LUXEL_VVL_LAYER_PATH"
            VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
            VK_KHRONOS_VALIDATION_VALIDATE_SYNC=true
            VK_KHRONOS_VALIDATION_REPORT_FLAGS=error,warn
            VK_KHRONOS_VALIDATION_DEBUG_ACTION=VK_DBG_LAYER_ACTION_LOG_MSG
            VK_KHRONOS_VALIDATION_LOG_FILENAME="$vvl_log"
        )
    fi
    status=0
    env "${layer_env[@]}" flock -x /tmp/gesso-gpu.lock \
        timeout "$timeout_s" "$julia" --project=. "test/$suite.jl" \
        > "$logs/$suite.log" 2>&1 || status=$?
    messages=0
    [[ -s "$vvl_log" ]] && messages="$(wc -l < "$vvl_log")"
    echo "$suite: exit $status, validation lines $messages"
    if (( status != 0 || messages != 0 )); then
        failed+=("$suite")
    fi
done

if (( ${#failed[@]} )); then
    echo "gpu-gate: FAILED: ${failed[*]} (logs in $logs)" >&2
    exit 1
fi
echo "gpu-gate: all ${#suites[@]} suites passed$( (( validation )) && echo ' with an empty validation log')"
