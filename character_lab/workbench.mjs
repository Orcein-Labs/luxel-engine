import { compileProject, defaultValues, evaluateCharacter, sampleClip, rectanglePath, PROJECT_LIMIT } from './runtime.mjs';
import { mountCharacter, paintCharacter, prepareTextures } from './renderer.mjs';
import { BUILTINS } from './fixtures.mjs';

const root = document.getElementById('luxel-workbench');
const input = name => root.querySelector(`[data-input="${name}"]`);
const output = name => root.querySelector(`[data-output="${name}"]`);
const action = name => root.querySelector(`[data-action="${name}"]`);
const svg = root.querySelector('[data-view="character"]');
const library = new Map(BUILTINS.map(project => { const c = compileProject(project); return [c.project.id, c]; }));
let character, view, pose, selected, selectedNode, values, running = false, time = 0, revision = 0;
let frame = null, anchorTime = null, anchorSeconds = 0, drag = null;
let history = [];

function option(select, value, label) {
  const element = document.createElement('option'); element.value = value; element.textContent = label; select.append(element);
}
function subjectOptions() {
  input('subject').replaceChildren();
  for (const [id, c] of library) option(input('subject'), id, c.project.label);
}
function stop() {
  running = false; if (frame !== null) cancelAnimationFrame(frame); frame = null;
  action('play').textContent = 'Play demo'; action('play').setAttribute('aria-pressed', 'false');
}
function reportError(error) { output('error').textContent = error.message; }
function showPose(sync = true, evaluated = null) {
  pose = evaluated ?? evaluateCharacter(character, values);
  paintCharacter(view, pose, { selected, rig: input('rig').checked });
  if (sync) for (const p of character.parameters.values()) {
    root.querySelector(`[data-parameter="${p.id}"]`).value = String(values[p.id]);
    root.querySelector(`[data-value="${p.id}"]`).textContent = values[p.id].toFixed(2);
  }
  output('time').textContent = `${time.toFixed(2)} s`; input('time').value = String(time);
}
function preview(nextValues, nextTime = time) {
  try {
    const evaluated = evaluateCharacter(character, nextValues);
    values = nextValues; time = nextTime; showPose(true, evaluated); output('error').textContent = ''; return true;
  } catch (error) { stop(); reportError(error); showPose(); return false; }
}
function inspector() {
  const part = character.parts.get(selected), node = character.nodes.get(selectedNode);
  input('part').value = selected; input('visible').checked = part.visible;
  input('pivotX').value = part.pivot[0]; input('pivotY').value = part.pivot[1]; input('order').value = part.order;
  input('fill').disabled = part.fill === 'none'; input('fill').value = part.fill === 'none' ? '#000000' : part.fill;
  root.querySelector('[data-view="placement"]').hidden = !part.raster;
  if (part.raster) ['layerX','layerY','layerW','layerH'].forEach((name,i) => { input(name).value = part.raster.rect[i]; });
  input('node').value = selectedNode; input('nodeX').value = node.pivot[0]; input('nodeY').value = node.pivot[1];
  input('parent').replaceChildren(); option(input('parent'), '', 'None');
  for (const id of character.nodes.keys()) if (id !== selectedNode) option(input('parent'), id, id);
  input('parent').value = node.parent ?? '';
  const prior = input('channel').value; input('channel').replaceChildren();
  part.channels.forEach((c, i) => option(input('channel'), `part:${i}`, `Layer: ${c.parameter} · ${c.property}`));
  node.channels.forEach((c, i) => option(input('channel'), `node:${i}`, `Rig: ${c.parameter} · ${c.property}`));
  const hasChannels = part.channels.length + node.channels.length > 0;
  if (!hasChannels) option(input('channel'), '', 'No controls on this layer or node');
  if (hasChannels && [...input('channel').options].some(o => o.value === prior)) input('channel').value = prior;
  input('channel').disabled = !hasChannels; refreshGain();
}
function selectedChannel() {
  const [kind, index] = input('channel').value.split(':');
  const owner = kind === 'node' ? character.nodes.get(selectedNode) : character.parts.get(selected);
  return { kind, index: Number(index), channel: owner.channels[Number(index)] };
}
function refreshGain() {
  const { channel } = selectedChannel();
  input('gain').disabled = !channel; input('gain').value = channel?.gain ?? '';
}
function selectPart(id) {
  selected = id; selectedNode = character.parts.get(id).node; input('channel').value = ''; inspector(); showPose(false);
}
function mount() {
  view = mountCharacter(svg, character, selectPart);
  input('part').replaceChildren(); for (const p of character.project.parts) option(input('part'), p.id, p.label);
  input('node').replaceChildren(); for (const id of character.nodes.keys()) option(input('node'), id, id);
  const controls = root.querySelector('[data-view="parameters"]'); controls.replaceChildren();
  for (const p of character.parameters.values()) {
    const label = document.createElement('label'), value = document.createElement('output'), slider = document.createElement('input');
    label.append(document.createTextNode(p.label)); value.dataset.value = p.id; label.append(value);
    slider.type = 'range'; slider.min = p.min; slider.max = p.max; slider.step = '0.01'; slider.dataset.parameter = p.id;
    slider.setAttribute('aria-label', p.label);
    slider.addEventListener('input', () => { stop(); preview({ ...values, [p.id]: Number(slider.value) }); });
    label.append(slider); controls.append(label);
  }
  const clip = character.project.clips[0]; input('time').max = String(clip?.duration ?? 1);
  input('time').disabled = !clip; action('play').disabled = !clip;
  inspector(); showPose(); root.dataset.project = character.project.id;
  output('status').textContent = `${character.project.parts.length} editable layers · ${character.project.nodes.length} rig nodes · revision ${revision}`;
  action('undo').disabled = history.length === 0;
  output('caption').textContent = character.textures.size ? 'Layered illustration pilot · Edit placement and rig; pixels are not vector points' : 'Vector diagnostic · Drag selected points with rig visible';
}
function activate(next) {
  history = [];
  stop(); character = next; values = defaultValues(character); selected = character.project.parts.find(p => p.id === 'face')?.id ?? character.project.parts[0].id;
  selectedNode = character.parts.get(selected).node; time = 0; input('subject').value = character.project.id; mount(); output('error').textContent = '';
}
function edit(change) {
  try {
    const project = structuredClone(character.project); change(project);
    const next = compileProject(project); evaluateCharacter(next, values);
    stop(); history.push(character); if (history.length > 20) history.shift();
    character = next; library.set(next.project.id, next); revision++; mount(); output('error').textContent = '';
    return true;
  } catch (error) { reportError(error); inspector(); return false; }
}
function editPart(change) { return edit(project => change(project.parts.find(p => p.id === selected))); }
function editNode(change) { return edit(project => change(project.nodes.find(n => n.id === selectedNode))); }
function tick(now) {
  if (!running) return;
  if (anchorTime === null) anchorTime = now;
  const clip = character.project.clips[0], elapsed = anchorSeconds + (now-anchorTime)/1000;
  const nextTime = clip.loop ? elapsed % clip.duration : Math.min(elapsed, clip.duration);
  if (!preview(sampleClip(character, clip.id, nextTime), nextTime)) return;
  if (!clip.loop && elapsed >= clip.duration) { stop(); return; }
  frame = requestAnimationFrame(tick);
}
input('subject').addEventListener('change', () => activate(library.get(input('subject').value)));
input('part').addEventListener('change', () => selectPart(input('part').value));
input('node').addEventListener('change', () => { selectedNode = input('node').value; input('channel').value = ''; inspector(); });
input('channel').addEventListener('change', refreshGain);
input('rig').addEventListener('change', () => showPose(false));
for (const [index,name] of ['layerX','layerY','layerW','layerH'].entries()) input(name).addEventListener('change', () => editPart(p => { p.raster.rect[index] = Number(input(name).value); p.path = rectanglePath(p.raster.rect); }));
for (const [name, axis] of [['pivotX', 0], ['pivotY', 1]]) input(name).addEventListener('change', () => editPart(p => { p.pivot[axis] = Number(input(name).value); }));
for (const [name, axis] of [['nodeX', 0], ['nodeY', 1]]) input(name).addEventListener('change', () => editNode(n => { n.pivot[axis] = Number(input(name).value); }));
input('visible').addEventListener('change', () => editPart(p => { p.visible = input('visible').checked; }));
input('order').addEventListener('change', () => editPart(p => { p.order = Number(input('order').value); }));
input('fill').addEventListener('input', () => editPart(p => { p.fill = input('fill').value; }));
input('parent').addEventListener('change', () => editNode(n => { n.parent = input('parent').value || null; }));
input('gain').addEventListener('change', () => {
  const { kind, index } = selectedChannel(), gain = Number(input('gain').value);
  (kind === 'node' ? editNode : editPart)(owner => { owner.channels[index].gain = gain; });
});
input('time').addEventListener('input', () => {
  stop(); const nextTime = Number(input('time').value), clip = character.project.clips[0];
  preview(sampleClip(character, clip.id, nextTime === clip.duration && clip.loop ? nextTime-1e-9 : nextTime), nextTime);
});
action('play').addEventListener('click', () => {
  if (running) { stop(); return; }
  running = true; anchorTime = null; anchorSeconds = time;
  action('play').textContent = 'Pause'; action('play').setAttribute('aria-pressed', 'true'); frame = requestAnimationFrame(tick);
});
action('neutral').addEventListener('click', () => { stop(); values = defaultValues(character); time = 0; showPose(); });
action('undo').addEventListener('click', () => {
  if (!history.length) return; stop(); character = history.pop(); library.set(character.project.id, character); revision++; mount();
});
action('export').addEventListener('click', () => {
  const blob = new Blob([JSON.stringify(character.project, null, 2)+'\n'], { type: 'application/json' });
  const url = URL.createObjectURL(blob), link = document.createElement('a');
  link.href = url; link.download = `${character.project.id}.json`; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
});
input('import').addEventListener('change', async () => {
  try {
    const file = input('import').files[0]; if (!file) return;
    if (file.size > PROJECT_LIMIT) throw new Error('Project exceeds size limit');
    const next = compileProject(await file.text()); evaluateCharacter(next);
    await prepareTextures(next);
    library.set(next.project.id, next); subjectOptions(); activate(next);
  } catch (error) { reportError(error); }
  finally { input('import').value = ''; }
});
function canvasPoint(event) {
  const point = svg.createSVGPoint(); point.x = event.clientX; point.y = event.clientY;
  const mapped = point.matrixTransform(svg.getScreenCTM().inverse()); return [mapped.x, mapped.y];
}
svg.addEventListener('pointerdown', event => {
  const coordinate = event.target.getAttribute('data-coordinate'); if (coordinate === null) return;
  stop(); drag = { index: Number(coordinate), point: canvasPoint(event) }; svg.setPointerCapture(event.pointerId); event.preventDefault();
});
svg.addEventListener('pointermove', event => {
  if (!drag) return;
  const point = canvasPoint(event), [dx, dy] = [point[0]-drag.point[0], point[1]-drag.point[1]];
  const [a, b, c, d] = pose.parts.find(p => p.id === selected).matrix, determinant = a*d-b*c;
  const localX = (d*dx-c*dy)/determinant, localY = (-b*dx+a*dy)/determinant;
  if (editPart(part => {
    let offset = 0;
    for (const command of part.path) for (let i = 1; i < command.length; i += 2) {
      if (offset === drag.index) { command[i] += localX; command[i+1] += localY; }
      offset += 2;
    }
  })) drag.point = point;
});
svg.addEventListener('pointerup', () => { drag = null; });
svg.addEventListener('pointercancel', () => { drag = null; });
document.addEventListener('visibilitychange', () => { if (document.hidden) stop(); });
Promise.all([...library.values()].map(prepareTextures)).then(() => { subjectOptions(); activate(library.values().next().value); }).catch(reportError);
