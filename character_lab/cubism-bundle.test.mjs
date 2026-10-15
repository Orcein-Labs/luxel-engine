import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  CMO3_MAGIC,
  CYAN_BUNDLE_ID,
  MOC3_MAGIC,
  REPO_ROOT,
  REQUIRED_CONTROL_IDS,
  assertCmo3,
  assertMoc3,
  bindRuntimeControls,
  cyanAuthoredLive2dPaths,
  inspectCubismBundle,
  integrityReport,
  selectLipSyncIds,
} from './cubism-bundle.mjs';

// The pinned Open-LLM-VTuber consumer checkout lives outside this repo; point
// CYAN_FRONTEND_ROOT at it to run the consumer-side checks.
const CONSUMER_ROOT = process.env.CYAN_FRONTEND_ROOT;
const consumerSkip = CONSUMER_ROOT ? false : 'set CYAN_FRONTEND_ROOT to the Cyan avatar frontend checkout';
const consumerModel3 = CONSUMER_ROOT
  ? join(CONSUMER_ROOT, 'vendor/Open-LLM-VTuber/live2d-models/mao_pro/runtime/mao_pro.model3.json')
  : '';

test('renamed JSON, native puppet JSON, and PNG bytes are not moc3', () => {
  const native = readFileSync(join(REPO_ROOT, 'assets/avatars/native/cyan_vtuber.json'));
  assert.throws(() => assertMoc3(native, 'cyan_vtuber.json'), /JSON/);
  assert.throws(() => assertMoc3(Buffer.from('{"Version":3,"FileReferences":{"Moc":"x.moc3"}}'), 'fake'), /JSON/);
  assert.throws(() => assertMoc3(Buffer.from([0x89, 0x50, 0x4e, 0x47, 13, 10, 26, 10, 1, 2, 3, 4]), 'png'), /PNG/);
  assert.throws(() => assertMoc3(Buffer.from('cmo3'), 'short'));
});

test('LipSync binds the DisplayInfo mouth id rather than assuming ParamMouthOpenY', () => {
  assert.deepEqual(selectLipSyncIds(new Set(['ParamA', 'ParamEyeLOpen'])), ['ParamA']);
  assert.deepEqual(selectLipSyncIds(new Set(['ParamMouthOpenY'])), ['ParamMouthOpenY']);
  assert.throws(() => selectLipSyncIds(new Set(['ParamAngleX'])), /lip-sync/);
});

test('cyan Cubism 5 Editor export is a real moc3/cmo3 bundle with blink, gaze, mouth, and head ids', () => {
  const authored = cyanAuthoredLive2dPaths();
  assert.equal(existsSync(authored.model3), true);
  assert.equal(existsSync(authored.cmo3), true);
  const moc3 = assertMoc3(readFileSync(join(REPO_ROOT, 'assets/avatars/cyan/live2d/cyan_source.moc3')), 'cyan_source.moc3');
  assert.equal(moc3.magic, MOC3_MAGIC);
  assert.equal(
    moc3.versionByte,
    5,
    'pinned Open-LLM-VTuber Cubism Core 5.0.0 latestMoc is 5; version 6 fails CubismMoc.create',
  );
  const cmo3 = assertCmo3(readFileSync(authored.cmo3), 'cyan_source.cmo3');
  assert.equal(cmo3.magic, CMO3_MAGIC);
  const bound = bindRuntimeControls(authored.model3);
  for (const id of Object.values(REQUIRED_CONTROL_IDS)) {
    assert.ok(bound.parameterIds.includes(id), `missing ${id}`);
  }
  assert.deepEqual(bound.eyeBlinkIds, ['ParamEyeLOpen', 'ParamEyeROpen']);
  assert.deepEqual(bound.lipSyncIds, ['ParamMouthOpenY']);
  const report = inspectCubismBundle(authored.model3);
  assert.ok(report.resolved.some((row) => row.path.endsWith('.moc3')));
  assert.ok(report.textures.length >= 1);
  assert.ok(report.idleMotionGroups.includes('Idle'));
  assert.deepEqual(report.eyeBlinkIds, bound.eyeBlinkIds);
  assert.deepEqual(report.lipSyncIds, bound.lipSyncIds);
  const settings = JSON.parse(readFileSync(authored.model3, 'utf8'));
  const display = JSON.parse(
    readFileSync(join(REPO_ROOT, 'assets/avatars/cyan/live2d', settings.FileReferences.DisplayInfo), 'utf8'),
  );
  const ids = new Set(display.Parameters.map((row) => row.Id));
  for (const exp of settings.FileReferences.Expressions) {
    const body = JSON.parse(readFileSync(join(REPO_ROOT, 'assets/avatars/cyan/live2d', exp.File), 'utf8'));
    for (const parameter of body.Parameters) {
      assert.ok(ids.has(parameter.Id), `${exp.File} uses unknown ${parameter.Id}`);
    }
  }
  const integrity = integrityReport({ repoRoot: REPO_ROOT });
  assert.equal(integrity.blocking, null);
  assert.ok(integrity.fakeMoc3Rejected.every((row) => row.ok));
  assert.equal(CYAN_BUNDLE_ID, 'cyan_source');
  assert.ok(integrity.cyanBundle);
  if (CONSUMER_ROOT) {
    const consumerCyan = join(
      CONSUMER_ROOT,
      'vendor/Open-LLM-VTuber/live2d-models/cyan_source/runtime/cyan_source.moc3',
    );
    assert.equal(assertMoc3(readFileSync(consumerCyan), 'consumer cyan_source.moc3').versionByte, 5);
  }
});

test('integrity inspector accepts the pinned mao_pro Cubism 5 sample and binds LipSync to declared IDs', { skip: consumerSkip }, () => {
  assert.equal(existsSync(consumerModel3), true);
  const report = inspectCubismBundle(consumerModel3);
  assert.ok(report.resolved.some((row) => row.path.endsWith('.moc3')));
  assert.ok(report.textures.length >= 1);
  assert.ok(report.idleMotionGroups.includes('Idle'));
  assert.deepEqual(report.eyeBlinkIds, ['ParamEyeLOpen', 'ParamEyeROpen']);
  assert.deepEqual(report.lipSyncIds, ['ParamA']);
});
