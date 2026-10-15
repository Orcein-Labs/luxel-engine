import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  ASSEMBLY_SCHEMA,
  FEMALE_V2_SHA256,
  REQUIRED_SURFACES,
  assembleRest,
  collarCoveragePresent,
  coverageLedger,
  displacedEyeFragment,
  duplicateUnderlay,
  fidelityReport,
  isolationReport,
  leakedBackground,
  loadCyanAssembly,
  omittedCollar,
  ownershipReport,
  sha256Bytes,
} from './cyan-source.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, '..');
const sourcePath = join(repoRoot, 'assets/avatars/cyan/source/cyan-source-art-female-v2.png');
const recipePath = join(repoRoot, 'assets/avatars/cyan/layered/recipe.json');

const assembly = loadCyanAssembly(recipePath);

test('female-v2 freeze is the rest-assembly source of record', () => {
  const bytes = readFileSync(sourcePath);
  assert.equal(sha256Bytes(bytes), FEMALE_V2_SHA256);
  assert.equal(assembly.recipe.schema, ASSEMBLY_SCHEMA);
  assert.equal(assembly.recipe.source_digest, FEMALE_V2_SHA256);
  assert.equal(sha256Bytes(assembly.sourceBytes), FEMALE_V2_SHA256);
  assert.equal(assembly.recipe.provenance.flattened_underlay, false);
  assert.equal(assembly.recipe.provenance.reference_preserved, true);
  assert.ok(!assembly.layers.some((layer) => layer.id.includes('underlay') && layer.kind === 'visible'));
});

test('coverage ledger lists lids, sclera, pupils, lips, mouth interior, face-behind-bangs, neck-behind-jaw/collar, and hair overlaps as present', () => {
  const ledger = coverageLedger(assembly);
  assert.deepEqual(ledger.map((row) => row.surface), [...REQUIRED_SURFACES]);
  for (const row of ledger) {
    assert.equal(row.status, 'present', `${row.surface} must be present`);
    assert.ok(row.pixels > 0, `${row.surface} must have authored pixels`);
    assert.ok(row.layer_ids.length > 0, `${row.surface} must name layer ids`);
    for (const id of row.layer_ids) {
      const layer = assembly.layers.find((item) => item.id === id);
      assert.ok(layer, `missing coverage layer ${id}`);
      assert.ok((layer.surfaces || []).includes(row.surface));
    }
  }
  const concealed = new Set(['lids', 'mouth_interior', 'face_behind_bangs', 'neck_behind_jaw_collar']);
  for (const surface of concealed) {
    const row = ledger.find((item) => item.surface === surface);
    assert.ok(row.layer_ids.every((id) => assembly.layers.find((layer) => layer.id === id).kind === 'concealed'));
  }
});

test('visible rest pixels have one owner, stay inside the silhouette, and match female-v2 interior', () => {
  const ownership = ownershipReport(assembly);
  const isolation = isolationReport(assembly);
  const fidelity = fidelityReport(assembly, { boundaryPx: assembly.recipe.boundary_policy_px });
  assert.equal(ownership.collisions, 0);
  assert.equal(ownership.exclusive, true);
  assert.equal(ownership.unowned, 0);
  assert.equal(ownership.outside, 0);
  assert.ok(ownership.owned > 0);
  assert.equal(ownership.owned, ownership.silhouette);
  assert.equal(isolation.leaked, 0);
  assert.equal(isolation.isolated, true);
  assert.equal(fidelity.changed, 0);
  assert.equal(fidelity.passed, true);
  assert.ok(fidelity.interior > 0);
  const rest = assembleRest(assembly);
  const concealedOwners = new Set(
    assembly.layers.flatMap((layer, index) => (layer.kind === 'concealed' ? [index] : [])),
  );
  for (let i = 0; i < rest.owner.length; i++) {
    if (rest.owner[i] >= 0) assert.equal(concealedOwners.has(rest.owner[i]), false);
  }
});

test('displaced eye fragment, omitted collar, duplicate underlay, and leaked background fail their checks', () => {
  const eye = displacedEyeFragment(assembly);
  assert.equal(ownershipReport(eye).exclusive, false, 'displaced iris must collide or leave a hole');

  const collar = omittedCollar(assembly);
  assert.equal(collarCoveragePresent(collar), false);
  assert.ok(ownershipReport(collar).unowned > 0, 'omitting the collar must leave unowned silhouette pixels');
  assert.equal(collarCoveragePresent(assembly), true);

  const underlay = duplicateUnderlay(assembly);
  const underlayOwnership = ownershipReport(underlay);
  const underlayIsolation = isolationReport(underlay);
  assert.equal(underlayOwnership.exclusive, false, 'flattened underlay must collide with exclusive visible layers');
  assert.equal(underlayIsolation.isolated, false, 'flattened source underlay must leak painted backdrop');

  const leaked = leakedBackground(assembly);
  assert.equal(isolationReport(leaked).isolated, false, 'magenta canvas leak must fail isolation');
  assert.equal(isolationReport(assembly).isolated, true);
});

test('Cubism import candidate is a Photoshop 8BPS with one layer per part, not a renamed JSON', () => {
  const relative = assembly.recipe.provenance.cubism_import;
  const psdPath = join(repoRoot, relative);
  assert.equal(existsSync(psdPath), true);
  const bytes = readFileSync(psdPath);
  assert.equal(bytes.subarray(0, 4).toString('ascii'), '8BPS');
  assert.notEqual(bytes.subarray(0, 1).toString('ascii'), '{');
  const layerCount = psdLayerCount(bytes);
  assert.equal(Math.abs(layerCount), assembly.recipe.part_stack.length);
  assert.equal(psdImageResourceSignature(bytes), '8BIM', 'Cubism requires at least one 8BIM image resource');
});

function psdLayerCount(bytes) {
  let offset = 26;
  offset += 4 + bytes.readUInt32BE(offset);
  offset += 4 + bytes.readUInt32BE(offset);
  const sectionLength = bytes.readUInt32BE(offset);
  offset += 4;
  if (sectionLength === 0) throw new Error('PSD has no layer section');
  offset += 4;
  return bytes.readInt16BE(offset);
}

function psdImageResourceSignature(bytes) {
  let offset = 26;
  offset += 4 + bytes.readUInt32BE(offset);
  const resourceLength = bytes.readUInt32BE(offset);
  offset += 4;
  if (resourceLength < 4) throw new Error('PSD image resources missing 8BIM block');
  return bytes.subarray(offset, offset + 4).toString('ascii');
}
