import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { createContext, runInContext } from 'node:vm';

// The Virgil frontend lives outside this repo; point CYAN_FRONTEND_ROOT at the
// Cyan avatar frontend checkout to run these checks.
const CONSUMER_ROOT = process.env.CYAN_FRONTEND_ROOT;
const skip = CONSUMER_ROOT ? false : 'set CYAN_FRONTEND_ROOT to the Cyan avatar frontend checkout';
const FRONTEND = join(CONSUMER_ROOT || '.', 'vendor/Open-LLM-VTuber/frontend');
const INDEX = join(FRONTEND, 'index.html');
const GUARD = join(FRONTEND, 'virgil_live2d_guard.js');
const BUNDLE = join(FRONTEND, 'assets/main-nu7uwxNJ.js');
const CHANGE_SCENE_GUARD = 'changeScene(o){if(!ModelDir[o])return;';
const UNGUARDED_CHANGE_SCENE = 'changeScene(o){this._sceneIndex=o;const s=ModelDir[o]';

function loadGuard(fetchImpl) {
  const source = readFileSync(GUARD, 'utf8');
  const window = {};
  const nativeCalls = [];
  const fetch =
    fetchImpl ||
    (async (input) => {
      nativeCalls.push(typeof input === 'string' ? input : input.url);
      return { ok: true, url: String(input) };
    });
  window.fetch = fetch;
  const context = createContext({
    window,
    globalThis: window,
    console,
    Date,
    Error,
    Promise,
    String,
    Boolean,
    Number,
    Math,
  });
  runInContext(source, context, { filename: 'virgil_live2d_guard.js' });
  return { window, nativeCalls, api: window.VirgilLive2DGuard };
}

test('Virgil index loads the Live2D race guard before the Cubism bundle', { skip }, () => {
  assert.equal(existsSync(INDEX), true);
  assert.equal(existsSync(GUARD), true);
  assert.equal(existsSync(BUNDLE), true);
  const html = readFileSync(INDEX, 'utf8');
  const guardPos = html.indexOf('virgil_live2d_guard.js');
  const bundlePos = html.indexOf('assets/main-nu7uwxNJ.js');
  assert.ok(guardPos > 0, 'index.html must include virgil_live2d_guard.js');
  assert.ok(bundlePos > guardPos, 'guard script must precede the Cubism bundle');
});

test('changeScene no-ops when ModelDir is empty instead of loading undefined.model3.json', { skip }, () => {
  const bundle = readFileSync(BUNDLE, 'utf8');
  assert.equal(bundle.includes(CHANGE_SCENE_GUARD), true);
  assert.equal(bundle.includes(UNGUARDED_CHANGE_SCENE), false);
  assert.equal(bundle.includes('undefined.model3.json'), false);
  const start = bundle.indexOf('changeScene(o){');
  const snippet = bundle.slice(start, start + 80);
  assert.match(snippet, /if\(!ModelDir\[o\]\)return;/);
});

test('shipped guard blocks only the empty-scene model3 URL and forwards real cyan_source', { skip }, async () => {
  const { window, nativeCalls, api } = loadGuard();
  assert.ok(api);
  assert.equal(api.isBlockedLive2dModelUrl('/undefined/undefined.model3.json'), true);
  assert.equal(
    api.isBlockedLive2dModelUrl('http://127.0.0.1:12393/undefined/undefined.model3.json'),
    true,
  );
  assert.equal(
    api.isBlockedLive2dModelUrl(
      'http://127.0.0.1:12393/live2d-models/cyan_source/runtime/cyan_source.model3.json',
    ),
    false,
  );
  assert.equal(api.isBlockedLive2dModelUrl('/live2d-models/mao_pro/runtime/mao_pro.model3.json'), false);

  await assert.rejects(
    () => window.fetch('/undefined/undefined.model3.json'),
    /blocked undefined\.model3\.json race/,
  );
  assert.equal(api.blockedFetches.length, 1);
  assert.equal(api.blockedFetches[0].url, '/undefined/undefined.model3.json');
  assert.deepEqual(nativeCalls, []);

  const ok = await window.fetch(
    '/live2d-models/cyan_source/runtime/cyan_source.model3.json',
  );
  assert.equal(ok.ok, true);
  assert.deepEqual(nativeCalls, ['/live2d-models/cyan_source/runtime/cyan_source.model3.json']);
  assert.equal(api.blockedFetches.length, 1);
});
