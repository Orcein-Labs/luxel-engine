# Local point-light shadows

Status: landed October 12, 2026 with CPU, GPU and native evidence. Bounded
lighting milestone after unshadowed local points. This is not a universal
shadow rewrite, a cubemap API, or campaign completion. Fill/environment
balance remains the next controlled native arm.

## Why this shape

Lava exposes `LavaTexture2D` only. Directional shadows already render
perspective or orthographic depth into a 2D colour target and sample it with
four-tap PCF. Point packets already occupy descriptor slot 18 (19 sampled
image/sampler slots). Cascades occupy slot 17. Combined point-plus-cascade
packets therefore already sit at 19 slots.

Omnidirectional occlusion therefore uses one extra 2D atlas at slot 19: four
lights by six axis-aligned cube faces. Face cameras reuse the existing shadow
vertex/fragment path. Absent `render_policy.point_shadows` keeps the
unshadowed point shaders' results: the atlas binding is a 1×1 far-depth
stand-in and the point table stores map size 0 so the shader returns full
visibility without cube math.

## Policy

`render_policy.point_shadows` is optional and omitted from canonical JSON when
absent. Present requires `map_size_px` in `{256, 512}`. Darkness and PCF radius
come from the existing `render_policy.shadow` channel. Near plane is 5 cm;
far plane is the light's range. Local calibration warm/cool rigs opt in at
512 px. GPU controls use 256 px.

Packets without point lights ignore the axis. Zero-intensity and out-of-range
points remain dark. Directional-only packets keep historical pipelines and
pixels.

## Acceptance

1. Absent policy: point-lit GPU captures match the unshadowed path exactly,
   including zero-point versus absent-point identity.
2. A caster between a point and a receiver darkens the receiver; removing the
   caster, moving the caster off the ray, or disabling the policy restores
   energy.
3. Moving the light or the caster moves the umbra. A reversed-light control
   does not count as a pass.
4. Native warm/cool mood captures with the policy enabled show grounded
   contact on bark, wet stone and metal against the preserved unshadowed
   gallery. Off/warm/cool files from the unshadowed pass stay untouched.
5. Descriptor limit is 20 sampled-image/sampler slots on the point path.
   Peak extra residency is one face map plus one atlas; faces composite
   through a scratch framebuffer.

## Out of scope

Area lights, cubemap hardware, volumetric scattering, cube-face seam
filtering beyond four-tap PCF, interactive frame budgets, and key/fill
retuning of the preserved unshadowed rigs. Fill/environment balance is the
next controlled native arm after this slice lands.

## Evidence

CPU face/atlas/projection tests, GPU occluder controls, and native calibration
captures live under `artifacts/local-shadows-20261012/`. The unshadowed local
gallery remains the pre-shadow control.
