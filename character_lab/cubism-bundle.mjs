import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

export const MOC3_MAGIC = 'MOC3';
export const CYAN_BUNDLE_ID = 'cyan_source';
export const CYAN_MODEL3_RELATIVE = 'assets/avatars/cyan/live2d/cyan_source.model3.json';
export const CYAN_CMO3_RELATIVE = 'assets/avatars/cyan/live2d/cyan_source.cmo3';

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = join(here, '..');

function fail(message) {
  throw new Error(message);
}

export function sha256Bytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

export const CMO3_MAGIC = 'CAFF';

export const REQUIRED_CONTROL_IDS = Object.freeze({
  blinkLeft: 'ParamEyeLOpen',
  blinkRight: 'ParamEyeROpen',
  gazeX: 'ParamEyeBallX',
  gazeY: 'ParamEyeBallY',
  mouthOpen: 'ParamMouthOpenY',
  mouthForm: 'ParamMouthForm',
  headX: 'ParamAngleX',
  headY: 'ParamAngleY',
  headZ: 'ParamAngleZ',
});

export function assertMoc3(bytes, label = 'moc3') {
  if (!bytes || bytes.length < 8) fail(`${label} is too small to be a Cubism moc3`);
  const head = Buffer.from(bytes.subarray(0, 8));
  if (head[0] === 0x7b) fail(`${label} is JSON, not a Cubism moc3`);
  if (head[0] === 0x89 && head[1] === 0x50) fail(`${label} is PNG, not a Cubism moc3`);
  if (head.subarray(0, 4).toString('ascii') !== MOC3_MAGIC) fail(`${label} missing MOC3 magic`);
  return { magic: MOC3_MAGIC, versionByte: head[4], byteLength: bytes.length };
}

export function assertCmo3(bytes, label = 'cmo3') {
  if (!bytes || bytes.length < 8) fail(`${label} is too small to be a Cubism cmo3`);
  const magic = Buffer.from(bytes.subarray(0, 4)).toString('ascii');
  if (magic !== CMO3_MAGIC) fail(`${label} missing CAFF magic (got ${JSON.stringify(magic)})`);
  return { magic: CMO3_MAGIC, byteLength: bytes.length };
}

export function displayParameterIds(display) {
  return (display?.Parameters || []).map((row) => row.Id);
}

export function selectBlinkIds(parameterIds) {
  const ids = [];
  if (parameterIds.has(REQUIRED_CONTROL_IDS.blinkLeft)) ids.push(REQUIRED_CONTROL_IDS.blinkLeft);
  if (parameterIds.has(REQUIRED_CONTROL_IDS.blinkRight)) ids.push(REQUIRED_CONTROL_IDS.blinkRight);
  if (ids.length === 0) fail('DisplayInfo has no blink parameters');
  return ids;
}

export function selectLipSyncIds(parameterIds) {
  for (const id of [REQUIRED_CONTROL_IDS.mouthOpen, 'ParamA', 'ParamMouthOpen']) {
    if (parameterIds.has(id)) return [id];
  }
  fail('DisplayInfo has no lip-sync parameter');
}

function linearCurve(id, keys) {
  const segments = [keys[0][0], keys[0][1]];
  for (let i = 1; i < keys.length; i += 1) {
    segments.push(0, keys[i][0], keys[i][1]);
  }
  return { Target: 'Parameter', Id: id, Segments: segments };
}

function countMotionMeta(curves) {
  let totalSegmentCount = 0;
  let totalPointCount = 0;
  for (const curve of curves) {
    const segs = curve.Segments;
    let i = 2;
    let points = 1;
    let segments = 0;
    while (i < segs.length) {
      const type = segs[i];
      segments += 1;
      if (type === 1) {
        i += 7;
        points += 1;
      } else {
        i += 3;
        points += 1;
      }
    }
    totalSegmentCount += segments;
    totalPointCount += points;
  }
  return { totalSegmentCount, totalPointCount };
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, '\t')}\n`);
}

function expressionFile(name, parameters) {
  return {
    Type: 'Live2D Expression',
    FadeInTime: 0.5,
    FadeOutTime: 0.5,
    Name: name,
    Parameters: parameters.filter((row) => row && row.Id),
  };
}

export function bindRuntimeControls(model3Path) {
  const settings = JSON.parse(readFileSync(model3Path, 'utf8'));
  const bundleDir = dirname(model3Path);
  const refs = settings.FileReferences || {};
  if (!refs.DisplayInfo) fail(`${model3Path} has no DisplayInfo`);
  const display = JSON.parse(readFileSync(join(bundleDir, refs.DisplayInfo), 'utf8'));
  const parameterIds = new Set(displayParameterIds(display));
  for (const [surface, id] of Object.entries(REQUIRED_CONTROL_IDS)) {
    if (!parameterIds.has(id)) fail(`DisplayInfo missing ${surface} parameter ${id}`);
  }
  const eyeBlinkIds = selectBlinkIds(parameterIds);
  const lipSyncIds = selectLipSyncIds(parameterIds);
  const present = (id) => parameterIds.has(id);

  const expressionsDir = join(bundleDir, 'expressions');
  const motionsDir = join(bundleDir, 'motions');
  mkdirSync(expressionsDir, { recursive: true });
  mkdirSync(motionsDir, { recursive: true });

  const expressions = [
    {
      name: 'exp_neutral',
      file: 'expressions/exp_neutral.exp3.json',
      parameters: [
        present('ParamEyeLOpen') && { Id: 'ParamEyeLOpen', Value: 1, Blend: 'Multiply' },
        present('ParamEyeROpen') && { Id: 'ParamEyeROpen', Value: 1, Blend: 'Multiply' },
      ],
    },
    {
      name: 'exp_sad',
      file: 'expressions/exp_sad.exp3.json',
      parameters: [
        present('ParamBrowLY') && { Id: 'ParamBrowLY', Value: -0.6, Blend: 'Add' },
        present('ParamBrowRY') && { Id: 'ParamBrowRY', Value: -0.6, Blend: 'Add' },
        present('ParamMouthForm') && { Id: 'ParamMouthForm', Value: -0.5, Blend: 'Add' },
        present('ParamEyeLOpen') && { Id: 'ParamEyeLOpen', Value: 0.85, Blend: 'Multiply' },
        present('ParamEyeROpen') && { Id: 'ParamEyeROpen', Value: 0.85, Blend: 'Multiply' },
      ],
    },
    {
      name: 'exp_anger',
      file: 'expressions/exp_anger.exp3.json',
      parameters: [
        present('ParamBrowLAngle') && { Id: 'ParamBrowLAngle', Value: 0.7, Blend: 'Add' },
        present('ParamBrowRAngle') && { Id: 'ParamBrowRAngle', Value: -0.7, Blend: 'Add' },
        present('ParamBrowLForm') && { Id: 'ParamBrowLForm', Value: -0.4, Blend: 'Add' },
        present('ParamBrowRForm') && { Id: 'ParamBrowRForm', Value: -0.4, Blend: 'Add' },
        present('ParamMouthForm') && { Id: 'ParamMouthForm', Value: -0.3, Blend: 'Add' },
      ],
    },
    {
      name: 'exp_joy',
      file: 'expressions/exp_joy.exp3.json',
      parameters: [
        present('ParamEyeLSmile') && { Id: 'ParamEyeLSmile', Value: 0.8, Blend: 'Add' },
        present('ParamEyeRSmile') && { Id: 'ParamEyeRSmile', Value: 0.8, Blend: 'Add' },
        present('ParamMouthForm') && { Id: 'ParamMouthForm', Value: 0.7, Blend: 'Add' },
        present('ParamCheek') && { Id: 'ParamCheek', Value: 0.5, Blend: 'Add' },
      ],
    },
  ];

  for (const row of expressions) {
    writeJson(join(bundleDir, row.file), expressionFile(row.name, row.parameters));
  }

  const idleCurves = [
    linearCurve('ParamAngleX', [[0, 0], [1.5, 4], [3, 0], [4.5, -4], [6, 0]]),
    linearCurve('ParamAngleY', [[0, 0], [2, 2], [4, -1], [6, 0]]),
    linearCurve('ParamBreath', [[0, 0], [1.5, 0.6], [3, 0], [4.5, 0.6], [6, 0]]),
    linearCurve('ParamHairFront', [[0, 0], [2, 0.3], [4, -0.2], [6, 0]]),
  ].filter((curve) => parameterIds.has(curve.Id));
  const idleMeta = countMotionMeta(idleCurves);
  writeJson(join(motionsDir, 'idle.motion3.json'), {
    Version: 3,
    Meta: {
      Duration: 6.0,
      Fps: 30.0,
      Loop: true,
      AreBeziersRestricted: true,
      CurveCount: idleCurves.length,
      TotalSegmentCount: idleMeta.totalSegmentCount,
      TotalPointCount: idleMeta.totalPointCount,
      UserDataCount: 0,
      TotalUserDataSize: 0,
    },
    Curves: idleCurves,
  });

  settings.Groups = [
    { Target: 'Parameter', Name: 'EyeBlink', Ids: eyeBlinkIds },
    { Target: 'Parameter', Name: 'LipSync', Ids: lipSyncIds },
  ];
  refs.Expressions = expressions.map((row) => ({ Name: row.name, File: row.file }));
  refs.Motions = { Idle: [{ File: 'motions/idle.motion3.json' }] };
  settings.FileReferences = refs;
  writeJson(model3Path, settings);

  return {
    eyeBlinkIds,
    lipSyncIds,
    parameterIds: [...parameterIds],
    expressions: expressions.map((row) => row.name),
    idleMotion: 'motions/idle.motion3.json',
  };
}

export function collectFiles(root) {
  const out = [];
  const walk = (dir) => {
    if (!existsSync(dir)) return;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else out.push(path);
    }
  };
  walk(root);
  return out;
}

export function cyanAuthoredLive2dPaths(repoRoot = REPO_ROOT) {
  const cyanRoot = join(repoRoot, 'assets/avatars/cyan');
  const hits = collectFiles(cyanRoot).filter((path) => {
    const ext = extname(path).toLowerCase();
    return ext === '.moc3' || ext === '.cmo3' || path.endsWith('.model3.json');
  });
  return {
    cyanRoot,
    model3: join(repoRoot, CYAN_MODEL3_RELATIVE),
    cmo3: join(repoRoot, CYAN_CMO3_RELATIVE),
    hits,
  };
}

function collectReferenceFiles(refs) {
  const files = [];
  if (refs.Moc) files.push(refs.Moc);
  for (const texture of refs.Textures || []) files.push(texture);
  for (const key of ['Physics', 'Pose', 'DisplayInfo', 'UserData']) {
    if (refs[key]) files.push(refs[key]);
  }
  for (const item of refs.Expressions || []) if (item.File) files.push(item.File);
  for (const items of Object.values(refs.Motions || {})) {
    for (const item of items) if (item.File) files.push(item.File);
  }
  return files;
}

export function inspectCubismBundle(model3Path) {
  const settingsBytes = readFileSync(model3Path);
  const settings = JSON.parse(settingsBytes.toString('utf8'));
  const bundleDir = dirname(model3Path);
  const refs = settings.FileReferences || {};
  if (!refs.Moc) fail(`${model3Path} has no FileReferences.Moc`);
  if (!Array.isArray(refs.Textures) || refs.Textures.length === 0) fail(`${model3Path} has no textures`);
  const resolved = [];
  for (const rel of collectReferenceFiles(refs)) {
    const path = join(bundleDir, rel);
    if (!existsSync(path)) fail(`missing bundle file ${rel}`);
    const bytes = readFileSync(path);
    if (bytes.length === 0) fail(`empty bundle file ${rel}`);
    if (path.endsWith('.moc3')) assertMoc3(bytes, rel);
    if (path.endsWith('.json')) JSON.parse(bytes.toString('utf8'));
    resolved.push({ path: rel, byteLength: bytes.length, sha256: sha256Bytes(bytes) });
  }
  const display = refs.DisplayInfo ? JSON.parse(readFileSync(join(bundleDir, refs.DisplayInfo), 'utf8')) : null;
  const parameterIds = new Set((display?.Parameters || []).map((row) => row.Id));
  const groups = settings.Groups || [];
  const byName = new Map(groups.map((group) => [group.Name, group]));
  const blink = byName.get('EyeBlink');
  const lip = byName.get('LipSync');
  if (!blink || !Array.isArray(blink.Ids) || blink.Ids.length === 0) fail('EyeBlink group missing real parameter IDs');
  if (!lip || !Array.isArray(lip.Ids) || lip.Ids.length === 0) fail('LipSync group missing real parameter IDs');
  if (parameterIds.size) {
    for (const id of blink.Ids) if (!parameterIds.has(id)) fail(`EyeBlink id ${id} is not in DisplayInfo parameters`);
    for (const id of lip.Ids) if (!parameterIds.has(id)) fail(`LipSync id ${id} is not in DisplayInfo parameters`);
  }
  return {
    model3: relative(bundleDir, model3Path) || model3Path,
    version: settings.Version,
    moc: refs.Moc,
    textures: refs.Textures,
    idleMotionGroups: Object.keys(refs.Motions || {}),
    groups: groups.map((group) => ({ name: group.Name, ids: group.Ids })),
    eyeBlinkIds: blink.Ids,
    lipSyncIds: lip.Ids,
    resolved,
    model3Sha256: sha256Bytes(settingsBytes),
  };
}

export function integrityReport({ repoRoot = REPO_ROOT, consumerBundle = null, editorProbe = null } = {}) {
  const authored = cyanAuthoredLive2dPaths(repoRoot);
  const fakeMoc3Rejected = [];
  for (const sample of [
    Buffer.from('{"schema":"luxel.layered-character/v2"}'),
    Buffer.from('{"Version":3,"FileReferences":{"Moc":"nope.moc3"}}'),
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 13, 10, 26, 10]),
  ]) {
    try {
      assertMoc3(sample, 'control');
      fakeMoc3Rejected.push({ ok: false, error: 'accepted a non-moc3' });
    } catch (error) {
      fakeMoc3Rejected.push({ ok: true, error: error.message });
    }
  }
  let cyanBundle = null;
  let cyanError = null;
  if (existsSync(authored.model3)) {
    try {
      cyanBundle = inspectCubismBundle(authored.model3);
    } catch (error) {
      cyanError = error.message;
    }
  }
  let control = null;
  if (consumerBundle && existsSync(consumerBundle)) {
    control = inspectCubismBundle(consumerBundle);
  }
  const present = Boolean(cyanBundle);
  return {
    schema: 'luxel.cubism-bundle-integrity/v1',
    blocking: present ? null : 'unverifiable',
    reason: present
      ? 'cyan Cubism 5 bundle resolved'
      : 'Cubism Editor is absent; no cyan .cmo3/.moc3 export exists. JSON was not renamed to moc3.',
    authored,
    fakeMoc3Rejected,
    cyanBundle,
    cyanError,
    controlSample: control ? { path: consumerBundle, ...control } : null,
    editorProbe,
  };
}
