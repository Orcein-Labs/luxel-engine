// Native layered-character reference profiles. This is not a Cubism reader/compiler.
export const PROJECT_LIMIT = 8000000;
const ID = /^[a-zA-Z][a-zA-Z0-9_-]{0,63}$/;
const COLOR = /^(#[0-9a-fA-F]{6}|none)$/;
const PROPERTIES = new Set(['x', 'y', 'rotate', 'scaleX', 'scaleY', 'opacity']);
const ARITY = { M: 2, L: 2, Q: 4, C: 6, Z: 0 };
const identity = [1, 0, 0, 1, 0, 0];
function requireValue(ok, message) { if (!ok) throw new Error(message); }
function finite(value, label, limit = 10000) {
  requireValue(typeof value === 'number' && Number.isFinite(value) && Math.abs(value) <= limit, `Invalid ${label}`);
}
function list(value, label, max) { requireValue(Array.isArray(value) && value.length <= max, `Invalid ${label}`); }
function name(value) { requireValue(typeof value === 'string' && ID.test(value), 'Invalid identifier'); }
function point(value) { list(value, 'point', 2); requireValue(value.length === 2, 'Point requires two coordinates'); value.forEach(v => finite(v, 'coordinate')); }
function unique(items, label) {
  const map = new Map();
  for (const item of items) { name(item.id); requireValue(!map.has(item.id), `Duplicate ${label}: ${item.id}`); map.set(item.id, item); }
  return map;
}
function visitGraph(map, edge, label) {
  const done = new Set(), pending = new Set(), order = [];
  function visit(id) {
    requireValue(!pending.has(id), `${label} cycle`);
    if (done.has(id)) return;
    pending.add(id);
    const target = edge(map.get(id));
    if (target !== null && target !== undefined) { requireValue(map.has(target), `Missing ${label}: ${target}`); visit(target); }
    pending.delete(id); done.add(id); order.push(id);
  }
  for (const id of map.keys()) visit(id);
  return order;
}
function allowedKeys(object, keys, label) {
  requireValue(object && typeof object === 'object' && !Array.isArray(object), `Invalid ${label}`);
  for (const key of Object.keys(object)) requireValue(keys.includes(key), `Unsupported ${label} field: ${key}`);
}
function channels(owner, parameters) {
  list(owner.channels, 'channels', 32);
  const extrema = new Map();
  for (const channel of owner.channels) {
    allowedKeys(channel, ['parameter', 'property', 'gain'], 'channel');
    requireValue(parameters.has(channel.parameter), `Unknown parameter: ${channel.parameter}`);
    requireValue(PROPERTIES.has(channel.property), 'Unsupported transform');
    finite(channel.gain, 'gain', 100);
    const p = parameters.get(channel.parameter), range = [(p.min - p.default) * channel.gain, (p.max - p.default) * channel.gain];
    const totals = extrema.get(channel.property) ?? [0, 0];
    extrema.set(channel.property, [totals[0] + Math.min(...range), totals[1] + Math.max(...range)]);
  }
  for (const prop of ['scaleX', 'scaleY']) requireValue(1 + (extrema.get(prop)?.[0] ?? 0) > 0, 'Transform can collapse or invert');
}

export function compileProject(input) {
  const text = typeof input === 'string' ? input : JSON.stringify(input);
  requireValue(typeof text === 'string' && text.length <= PROJECT_LIMIT, 'Project exceeds size limit');
  const project = JSON.parse(text);
  const textured = project.schema === 'luxel.layered-character/v2';
  requireValue(textured || project.schema === 'luxel.layered-character/v1', 'Unsupported project schema'); name(project.id);
  requireValue(textured || text.length <= 1000000, 'Vector project exceeds size limit');
  allowedKeys(project, ['schema', 'id', 'label', 'canvas', 'parameters', 'nodes', 'parts', 'clips', 'bindings', 'provenance', ...(textured ? ['textures'] : [])], 'project');
  if (textured) list(project.textures, 'textures', 4);
  const textures = unique(project.textures ?? [], 'texture');
  for (const texture of textures.values()) {
    allowedKeys(texture, ['id', 'data', 'size'], 'texture'); point(texture.size);
    requireValue(texture.size.every(v => Number.isInteger(v) && v > 0 && v <= 4096), 'Invalid texture dimensions');
    requireValue(typeof texture.data === 'string' && texture.data.length <= 6000000 && /^data:image\/png;base64,[A-Za-z0-9+/]+={0,2}$/.test(texture.data), 'Only bounded embedded PNG textures are supported');
    const header = Uint8Array.from(atob(texture.data.slice(22, 66)), c => c.charCodeAt(0));
    requireValue(header.length >= 24 && [137,80,78,71,13,10,26,10,0,0,0,13,73,72,68,82].every((v,i) => header[i] === v), 'Invalid PNG header');
    const dimensions = new DataView(header.buffer);
    requireValue(dimensions.getUint32(16) === texture.size[0] && dimensions.getUint32(20) === texture.size[1], 'PNG dimensions disagree with source');
  }
  requireValue(typeof project.label === 'string' && project.label.length <= 120, 'Invalid project label');
  point(project.canvas); requireValue(project.canvas.every(v => v >= 16 && v <= 4096), 'Invalid canvas');
  list(project.parameters, 'parameters', 32); list(project.nodes, 'nodes', 64); list(project.parts, 'parts', 128); list(project.clips, 'clips', 16);
  requireValue(project.parts.length > 0 && project.nodes.length > 0, 'Empty character');
  const parameters = unique(project.parameters, 'parameter'), nodes = unique(project.nodes, 'node'), parts = unique(project.parts, 'part');
  for (const p of parameters.values()) {
    allowedKeys(p, ['id', 'label', 'min', 'max', 'default'], 'parameter');
    requireValue(typeof p.label === 'string' && p.label.length <= 80, 'Invalid parameter label');
    [p.min, p.max, p.default].forEach(v => finite(v, 'parameter range'));
    requireValue(p.min < p.max && p.default >= p.min && p.default <= p.max, 'Invalid parameter range');
  }
  for (const node of nodes.values()) {
    allowedKeys(node, ['id', 'parent', 'pivot', 'channels'], 'node'); point(node.pivot); channels(node, parameters);
  }
  const nodeOrder = visitGraph(nodes, n => n.parent, 'parent');
  let coordinateCount = 0;
  for (const part of parts.values()) {
    allowedKeys(part, ['id', 'label', 'node', 'pivot', 'order', 'visible', 'fill', 'stroke', 'strokeWidth', 'path', 'channels', 'deforms', 'mask', ...(textured ? ['raster', 'opacity', 'maskSampling'] : [])], 'part');
    requireValue(part.maskSampling === undefined || ['pixel', 'coverage'].includes(part.maskSampling), 'Unsupported mask sampling');
    if (part.opacity !== undefined) { finite(part.opacity, 'opacity', 1); requireValue(part.opacity >= 0, 'Negative opacity'); }
    requireValue(typeof part.label === 'string' && part.label.length <= 80, 'Invalid part label');
    requireValue(nodes.has(part.node), 'Unknown part node'); point(part.pivot); finite(part.order, 'draw order');
    requireValue(typeof part.visible === 'boolean', 'Invalid visibility');
    requireValue(COLOR.test(part.fill) && COLOR.test(part.stroke), 'Invalid color');
    finite(part.strokeWidth, 'stroke width', 16); requireValue(part.strokeWidth >= 0, 'Negative stroke');
    list(part.path, 'path', 512); requireValue(part.path.length > 0 && part.path[0][0] === 'M', 'Path must start with M');
    let count = 0;
    for (const command of part.path) {
      requireValue(Array.isArray(command) && Object.hasOwn(ARITY, command[0]) && command.length === 1 + ARITY[command[0]], 'Invalid path command');
      command.slice(1).forEach(v => finite(v, 'path coordinate')); count += command.length - 1;
    }
    coordinateCount += count; requireValue(coordinateCount <= 32000, 'Too many path coordinates');
    channels(part, parameters); list(part.deforms, 'deformers', 16);
    if (part.raster) {
      allowedKeys(part.raster, ['texture', 'source', 'rect'], 'raster');
      const texture = textures.get(part.raster.texture); requireValue(texture, 'Unknown texture');
      const s = part.raster.source, r = part.raster.rect;
      for (const rect of [s, r]) { list(rect, 'raster rectangle', 4); requireValue(rect.length === 4, 'Rectangle requires four coordinates'); rect.forEach(v => finite(v, 'raster rectangle')); requireValue(rect[2] > 0 && rect[3] > 0, 'Empty raster rectangle'); }
      requireValue(s.every(Number.isInteger) && s[0] >= 0 && s[1] >= 0 && s[0]+s[2] <= texture.size[0] && s[1]+s[3] <= texture.size[1], 'Raster source outside texture');
      requireValue(JSON.stringify(part.path) === JSON.stringify(rectanglePath(r)) && part.deforms.length === 0, 'Raster layers require rectangular placement; mesh deformation is unsupported');
    }
    for (const deform of part.deforms) {
      allowedKeys(deform, ['parameter', 'negative', 'positive'], 'deformer');
      requireValue(parameters.has(deform.parameter), 'Unknown deformer parameter');
      for (const deltas of [deform.negative, deform.positive]) {
        list(deltas, 'deformation coordinates', count); requireValue(deltas.length === count, 'Deformation coordinate count mismatch');
        deltas.forEach(v => finite(v, 'deformation'));
      }
    }
  }
  visitGraph(parts, p => p.mask, 'mask');
  const clips = unique(project.clips, 'clip');
  for (const clip of clips.values()) {
    allowedKeys(clip, ['id', 'duration', 'loop', 'tracks'], 'clip'); finite(clip.duration, 'duration', 3600);
    requireValue(clip.duration > 0 && typeof clip.loop === 'boolean', 'Invalid clip'); list(clip.tracks, 'tracks', 32);
    const seen = new Set();
    for (const track of clip.tracks) {
      allowedKeys(track, ['parameter', 'keys', ...(textured ? ['interpolation'] : [])], 'track'); const p = parameters.get(track.parameter);
      requireValue(track.interpolation === undefined || ['linear', 'smooth'].includes(track.interpolation), 'Unsupported interpolation');
      requireValue(p && !seen.has(track.parameter), 'Unknown or duplicate clip parameter'); seen.add(track.parameter);
      list(track.keys, 'keys', 256); requireValue(track.keys.length > 0, 'Empty track'); let previous = -1;
      for (const key of track.keys) {
        point(key); requireValue(key[0] > previous && key[0] >= 0 && key[0] <= clip.duration && key[1] >= p.min && key[1] <= p.max, 'Invalid animation key'); previous = key[0];
      }
    }
  }
  allowedKeys(project.bindings, ['gazeX', 'gazeY', 'blinkLeft', 'blinkRight', 'speech', 'head', 'body', 'secondary'], 'bindings');
  for (const id of Object.values(project.bindings)) requireValue(parameters.has(id), 'Unknown semantic binding');
  return { project, parameters, nodes, parts, nodeOrder, clips, textures };
}

export function rectanglePath([x, y, w, h]) { return [['M',x,y],['L',x+w,y],['L',x+w,y+h],['L',x,y+h],['Z']]; }

export function multiply2d(a, b) {
  return [a[0]*b[0]+a[2]*b[1], a[1]*b[0]+a[3]*b[1], a[0]*b[2]+a[2]*b[3], a[1]*b[2]+a[3]*b[3], a[0]*b[4]+a[2]*b[5]+a[4], a[1]*b[4]+a[3]*b[5]+a[5]];
}
export function transformPoint(matrix, p) { return [matrix[0]*p[0]+matrix[2]*p[1]+matrix[4], matrix[1]*p[0]+matrix[3]*p[1]+matrix[5]]; }
function local(owner, values, parameters) {
  const v = { x: 0, y: 0, rotate: 0, scaleX: 1, scaleY: 1, opacity: owner.opacity ?? 1 };
  for (const c of owner.channels) v[c.property] += c.gain * (values[c.parameter] - parameters.get(c.parameter).default);
  const angle = v.rotate * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), [x, y] = owner.pivot;
  const m = [c*v.scaleX, s*v.scaleX, -s*v.scaleY, c*v.scaleY, 0, 0];
  m[4] = x + v.x - m[0]*x - m[2]*y; m[5] = y + v.y - m[1]*x - m[3]*y;
  return { matrix: m, opacity: Math.max(0, Math.min(1, v.opacity)) };
}
export function defaultValues(character) { return Object.fromEntries([...character.parameters].map(([id, p]) => [id, p.default])); }

export function evaluateCharacter(character, input = {}) {
  const values = defaultValues(character);
  requireValue(input && typeof input === 'object' && !Array.isArray(input), 'Invalid control values');
  for (const [id, value] of Object.entries(input)) {
    const p = character.parameters.get(id); requireValue(p, `Unknown parameter: ${id}`); finite(value, 'control');
    requireValue(value >= p.min && value <= p.max, `Control out of range: ${id}`); values[id] = value;
  }
  const transforms = new Map(), opacity = new Map(), pivots = new Map();
  for (const id of character.nodeOrder) {
    const node = character.nodes.get(id), own = local(node, values, character.parameters);
    const parent = transforms.get(node.parent) ?? identity, matrix = multiply2d(parent, own.matrix);
    transforms.set(id, matrix); opacity.set(id, own.opacity * (opacity.get(node.parent) ?? 1));
    pivots.set(id, transformPoint(matrix, node.pivot));
  }
  const parts = character.project.parts.map(part => {
    const own = local(part, values, character.parameters), matrix = multiply2d(transforms.get(part.node), own.matrix);
    const path = part.path.map(command => command.slice());
    for (const deform of part.deforms) {
      const p = character.parameters.get(deform.parameter), delta = values[p.id] - p.default;
      const ratio = delta === 0 ? 0 : delta / (delta < 0 ? p.default - p.min : p.max - p.default);
      const deltas = ratio < 0 ? deform.negative : deform.positive; let offset = 0;
      for (const command of path) for (let i = 1; i < command.length; i++) command[i] += deltas[offset++] * Math.abs(ratio);
    }
    const points = [];
    for (const command of path) for (let i = 1; i < command.length; i += 2) {
      const point = transformPoint(matrix, [command[i], command[i+1]]); point.forEach(v => finite(v, 'evaluated coordinate', 1000000));
      [command[i], command[i+1]] = point; points.push(point);
    }
    const bounds = [Math.min(...points.map(p => p[0])), Math.min(...points.map(p => p[1])), Math.max(...points.map(p => p[0])), Math.max(...points.map(p => p[1]))];
    return { id: part.id, path, bounds, matrix, opacity: own.opacity * opacity.get(part.node), visible: part.visible };
  });
  parts.sort((a, b) => character.parts.get(a.id).order - character.parts.get(b.id).order || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return { values, parts, pivots };
}

export function sampleClip(character, id, seconds) {
  finite(seconds, 'time', 1e9); requireValue(seconds >= 0, 'Negative playback time');
  const clip = character.clips.get(id); requireValue(clip, 'Unknown clip');
  const time = clip.loop ? seconds % clip.duration : Math.min(seconds, clip.duration), values = defaultValues(character);
  for (const track of clip.tracks) {
    const keys = track.keys; let value = keys[0][1];
    for (let i = 1; i < keys.length; i++) {
      if (time < keys[i][0]) {
        const a = keys[i-1], b = keys[i]; let t = Math.max(0, (time-a[0])/(b[0]-a[0]));
        if (track.interpolation === 'smooth') t = t*t*(3-2*t);
        value = a[1] + (b[1]-a[1]) * t; break;
      }
      value = keys[i][1];
    }
    values[track.parameter] = value;
  }
  return values;
}
