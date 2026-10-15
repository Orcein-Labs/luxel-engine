# Static extraction at the render import boundary

2026-10-09. Decision closed. Implementation is a separate contract after
CHAR-CPU-1; the current request, receipt and package schemas remain unchanged.

## Decision

Reject unsupported deformation by default at the render import boundary, with
named findings. Permit deliberately discarding deformation only when the
caller supplies explicit static-extraction intent. A geometry preparation
receipt establishes geometry/material/texture preparation only. It never
establishes animation evaluation, native animated rendering, or character
quality.

The current `condition_render_asset` extracts primitive POSITION buffers and
render attributes into `RenderMesh`, without joint/weight, inverse-bind,
animation or morph streams. The real CLI probe in
[`cpu-capability-audit-20261009.md`](cpu-capability-audit-20261009.md) demonstrated
that static, skinned/animated and weighted-morph inputs could all return Ready
with undeformed base geometry. Preserve that historical meaning and evidence;
do not retroactively label those receipts as animation-ready or change their
schemas during CHAR-CPU-1.

## Next implementation requirements

The new versioned import request must distinguish normal static-geometry import
from explicit extraction of undeformed base geometry from a deformation-bearing
source. Missing intent must never authorize discarding deformation. Unknown
intent is rejected. Do not use a defaulted boolean that silently opts old
requests into extraction.

The source analysis must detect skins/skin attributes, morph targets and weights
(including nonzero default weights), and animation channels. Attribute-only
checks miss node-level skin bindings; a clip-only check misses a rig without
clips. The current conditioner visits all mesh primitives, so checking only
the active scene would leave inactive emitted meshes unguarded. Its detection
scope must match the geometry actually extracted.

Use distinct findings for unsupported skinning, unsupported morph deformation
and unsupported animation. Findings identify the source node/mesh/primitive or
channel. Malformed containers and references remain errors, not permission to
extract partially validated geometry.

When extraction is explicitly requested, the result must identify its output
as static base geometry and record every discarded capability. Base accessor
geometry is not the bind pose, default morph pose, a sampled animation frame,
or a baked scene instance. Pose baking needs a separate named selection/time
contract and is not included in this decision. Emit neither a silent warning
nor an animation-ready flag on a geometry-only result.

Version request and receipt semantics explicitly. A package schema change is
required if extraction/discard metadata becomes part of package identity.
Existing validators must not accept new semantic claims under old schema IDs.
Migrate the actual CLI, native kit/backdrop/calibration callers and independent
receipt revalidation together; a wrapper that leaves production imports on
the old unguarded path does not close the boundary.

## Acceptance before implementation promotion

Use real GLB bytes through the production CLI and validator: plain static,
skinned without clips, animated skin, morph with nonzero default weight,
animation-only TRS, inactive deformation-bearing emitted mesh, malformed skin,
and unknown/missing intent. Default import must reject each unsupported case
with its named finding. Explicit extraction must retain analytic base positions
and disclose what was discarded. Its output must remain unusable as evidence
of animation readiness.

Pin old static receipts as historical controls; test explicit old/new schema
handling rather than rewriting expected digests in place. Temporarily disable
deformation detection and require the default-import negative control to fail.

Before source edits, close the exact file/caller ownership manifest and new
version identifiers as the next bounded implementation recipe. No import
schema or conditioning behavior was changed by this decision document.
