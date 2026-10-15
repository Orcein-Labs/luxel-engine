import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compileProject, defaultValues, evaluateCharacter, sampleClip } from './runtime.mjs';

const sources = ['cyan', 'hacker_bird'].map(id => JSON.parse(readFileSync(new URL(`../assets/avatars/native/${id}.json`, import.meta.url))));
const clone = value => structuredClone(value);
const near = (actual, expected) => actual.forEach((v, i) => assert.ok(Math.abs(v - expected[i]) < 1e-9, `${actual} != ${expected}`));

test('analytical parent pivot, child translation and control-point deformation', () => {
  const p = clone(sources[0]);
  p.parameters = [{ id: 'angle', label: 'Angle', min: -1, max: 1, default: 0 }, { id: 'shape', label: 'Shape', min: -1, max: 1, default: 0 }];
  p.nodes = [{ id: 'root', parent: null, pivot: [0, 0], channels: [{ parameter: 'angle', property: 'rotate', gain: 90 }] },
    { id: 'child', parent: 'root', pivot: [10, 0], channels: [{ parameter: 'angle', property: 'x', gain: 5 }] }];
  p.parts = [{ id: 'triangle', label: 'Triangle', node: 'child', pivot: [10, 0], order: 0, visible: true,
    fill: '#ffffff', stroke: 'none', strokeWidth: 0, channels: [], mask: null,
    path: [['M', 10, 0], ['L', 12, 0], ['L', 10, 2], ['Z']],
    deforms: [{ parameter: 'shape', negative: [0, 0, -2, 0, 0, 0], positive: [0, 0, 2, 0, 0, 0] }] }];
  p.bindings = {}; p.clips = [];
  const c = compileProject(p), pose = evaluateCharacter(c, { angle: 1, shape: 0.5 });
  near(pose.parts[0].path[0].slice(1), [0, 15]);
  near(pose.parts[0].path[1].slice(1), [0, 18]);
  near(pose.parts[0].path[2].slice(1), [-2, 15]);
  near(pose.pivots.get('child'), [0, 15]);
});

test('real authored sources roundtrip, preserve neutral geometry and finite combined endpoints', () => {
  for (const source of sources) {
    const c = compileProject(source);
    assert.deepEqual(compileProject(JSON.stringify(c.project)).project, source);
    const neutral = evaluateCharacter(c);
    for (const part of neutral.parts) assert.deepEqual(part.path, c.parts.get(part.id).path);
    for (const side of ['min', 'max']) {
      const pose = evaluateCharacter(c, Object.fromEntries([...c.parameters].map(([id, p]) => [id, p[side]])));
      assert.ok(pose.parts.every(p => p.bounds.every(Number.isFinite)));
    }
    const pose = evaluateCharacter(c, { blinkL: 1, blinkR: 0, speech: 1, gazeX: 1 });
    const box = id => pose.parts.find(p => p.id === id).bounds;
    assert.ok(box('eye_left')[3] - box('eye_left')[1] < 3);
    assert.ok(box('eye_right')[3] - box('eye_right')[1] > 30);
    assert.notDeepEqual(pose.parts, neutral.parts);
    assert.deepEqual(evaluateCharacter(c).values, defaultValues(c));
  }
});

test('clip time is stateless, linear and independent of evaluation order', () => {
  const p = clone(sources[0]); p.clips = [{ id: 'oracle', duration: 2, loop: false, tracks: [{ parameter: 'head', keys: [[0, -1], [2, 1]] }] }];
  const c = compileProject(p);
  assert.equal(sampleClip(c, 'oracle', 0.5).head, -0.5);
  assert.equal(sampleClip(c, 'oracle', 100).head, 1);
  sampleClip(c, 'oracle', 1.75);
  assert.deepEqual(evaluateCharacter(c, sampleClip(c, 'oracle', 0.5)), evaluateCharacter(c, { head: -0.5 }));
  const loop = compileProject(sources[1]);
  near(Object.values(sampleClip(loop, 'demo', 7.15)), Object.values(sampleClip(loop, 'demo', 1.15)));
});

test('invalid graphs, controls, vector markup and ranges fail before source mutation', () => {
  const bad = change => { const p = clone(sources[0]); change(p); assert.throws(() => compileProject(p)); };
  bad(p => { p.nodes[0].parent = 'head'; });
  bad(p => { p.parts[0].mask = p.parts[1].id; p.parts[1].mask = p.parts[0].id; });
  bad(p => { p.parts[0].path = '<svg onload="alert(1)">'; });
  bad(p => { p.parts[0].fill = 'url(https://example.invalid)'; });
  bad(p => { p.parts[0].path[0] = ['M', 1]; });
  bad(p => { p.parts[0].channels = [{ parameter: 'missing', property: 'rotate', gain: 1 }]; });
  bad(p => { p.parts[0].channels = [{ parameter: 'head', property: 'scaleX', gain: 1 }]; });
  bad(p => { p.parts[0].deforms = [{ parameter: 'head', positive: [], negative: [] }]; });
  bad(p => { p.parts[0].mask = 'missing'; });
  bad(p => { p.parts[1].id = p.parts[0].id; });
  bad(p => { p.schema = 'luxel.asset-runtime-receipt/v1'; });
  bad(p => { p.parts[0].script = 'alert(1)'; });
  bad(p => { p.clips[0].tracks[0].keys = [[0, 0], [0, 1]]; });
  bad(p => { p.bindings.speech = 'missing'; });
  const c = compileProject(sources[0]), before = JSON.stringify(c.project);
  for (const controls of [{ missing: 0 }, { head: 2 }, { head: Infinity }, { speech: NaN }]) assert.throws(() => evaluateCharacter(c, controls));
  assert.throws(() => sampleClip(c, 'demo', -1));
  assert.throws(() => compileProject('x'.repeat(1000001)));
  assert.equal(JSON.stringify(c.project), before);
});

test('texture-backed source, independent eyes, bilateral demo and strict raster boundaries', () => {
  for (const id of ['cyan_vtuber','ornith_chibi']) {
    const source = JSON.parse(readFileSync(new URL(`../assets/avatars/native/${id}.json`, import.meta.url)));
    const c = compileProject(source), open = evaluateCharacter(c), closed = evaluateCharacter(c, {blinkL:1,blinkR:0});
    assert.equal(open.parts.find(p => p.id === 'eye_left').opacity, 1);
    assert.equal(open.parts.find(p => p.id === 'closed_left').opacity, 0);
    assert.equal(closed.parts.find(p => p.id === 'eye_left').opacity, 0);
    assert.equal(closed.parts.find(p => p.id === 'closed_left').opacity, 1);
    assert.equal(closed.parts.find(p => p.id === 'eye_right').opacity, 1);
    const blink = sampleClip(c,'demo',1.28); assert.equal(blink.blinkL,1); assert.equal(blink.blinkR,1);
    assert.deepEqual(compileProject(JSON.stringify(c.project)).project, source);
    for (const property of ['min','max']) assert.ok(evaluateCharacter(c,Object.fromEntries([...c.parameters].map(([key,p])=>[key,p[property]]))).parts.every(p=>p.bounds.every(Number.isFinite)));
    const bad = change => { const p=clone(source); change(p); assert.throws(()=>compileProject(p)); };
    bad(p=>{p.textures[0].data='https://example.invalid/image.png';});
    bad(p=>{p.textures[0].size[0]++;});
    bad(p=>{p.parts[0].raster.source[2]=9999;});
    bad(p=>{p.parts[0].raster.texture='missing';});
    bad(p=>{p.parts[0].path[0][1]++;});
    bad(p=>{p.parts[0].deforms=[{parameter:'head',negative:[],positive:[]}];});
    bad(p=>{p.schema='luxel.layered-character/v1';});
    bad(p=>{p.parts[0].maskSampling='unknown';});
    const pixels=clone(source);pixels.parts[0].maskSampling='pixel';assert.equal(compileProject(JSON.stringify(pixels)).parts.get(pixels.parts[0].id).maskSampling,'pixel');
    const curve=clone(source); curve.clips=[{id:'oracle',duration:2,loop:false,tracks:[{parameter:'head',keys:[[0,0],[2,1]],interpolation:'smooth'}]}];
    assert.equal(sampleClip(compileProject(curve),'oracle',.5).head,.15625);
  }
});
