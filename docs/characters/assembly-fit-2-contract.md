# ASSEMBLY-FIT-2 repair contract

2026-10-09. Owner Codex, Luxel character authoring/rendering. BUILD a bounded
repair from Matt's 09:25 screenshots: fitted Cyan loses iris visibility in the
two-pane desk, while fitted Ornith's paws paint over the opaque laptop lid.

Evidence: mountCharacter uses a character-only SVG definition prefix in both
panes; fitted iris coordinates therefore reference an original-pane mask.
The bird's paws have order 95, above laptop order 90, despite wrist contact
constraints passing. Isolated SVG captures concealed the shared-document bug.

Permitted files: renderer.mjs, author_assembly.mjs, existing browser-smoke.mjs,
rebuilt generated character_lab bundles/guides, README and repair receipt;
evidence under artifacts/assembly-fit-2-20261009. Do not alter atlas PNGs,
built-in native sources, runtime/receipt schemas, GPU or unrelated work.

Assign each mounted renderer independent definition IDs, preserving every
internal reference. Set fitted paws behind the opaque lid; keep complete head
seating into the hoodie. Retain inspectable guides, source export and existing
controls. Add real-browser checks for unique document IDs and rasterized lid
occlusion, inspect the full two-pane screenshot as well as isolated captures,
and run existing assembly/runtime/browser checks. A deliberate reversal of
each fix must fail its check. Preserve preceding receipts in a new evidence
directory. Visual quality remains rejected for unfinished art/deformation.

Non-goals: missing arm generation, a new art style, automatic anatomy, mesh
deformation or professional avatar acceptance. The remaining question is how
to author connected sleeves/hands for the reference gesture without inventing
contacts that the painted art does not depict.
