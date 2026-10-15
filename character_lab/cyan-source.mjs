import { inflateSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const FEMALE_V2_SHA256 = '82da3196532fc000fdc6ce4569a8fe031c1fb260f5d5dacb3e3f1a9abd53b3b8';
export const ASSEMBLY_SCHEMA = 'luxel.cyan-source-assembly/v1';
export const REQUIRED_SURFACES = Object.freeze([
  'lids',
  'sclera',
  'pupils',
  'lips',
  'mouth_interior',
  'face_behind_bangs',
  'neck_behind_jaw_collar',
  'hair_overlaps',
]);

const PNG_SIG = [137, 80, 78, 71, 13, 10, 26, 10];
const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, '..');

function fail(message) {
  throw new Error(message);
}

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

export function sha256Bytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

export function decodePng(bytes) {
  if (bytes.length < 24) fail('PNG too small');
  for (let i = 0; i < 8; i++) if (bytes[i] !== PNG_SIG[i]) fail('Not a PNG');
  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const idat = [];
  while (offset + 12 <= bytes.length) {
    const length = bytes.readUInt32BE(offset);
    const type = bytes.toString('ascii', offset + 4, offset + 8);
    const start = offset + 8;
    const end = start + length;
    if (end + 4 > bytes.length) fail('Truncated PNG chunk');
    const chunk = bytes.subarray(start, end);
    if (type === 'IHDR') {
      width = chunk.readUInt32BE(0);
      height = chunk.readUInt32BE(4);
      bitDepth = chunk[8];
      colorType = chunk[9];
      if (bitDepth !== 8) fail('Only 8-bit PNG is supported');
      if (chunk[10] !== 0 || chunk[11] !== 0 || chunk[12] !== 0) fail('Unsupported PNG filter/interlace');
    } else if (type === 'IDAT') {
      idat.push(chunk);
    } else if (type === 'IEND') {
      break;
    }
    offset = end + 4;
  }
  if (!width || !height) fail('PNG missing IHDR');
  const channels = colorType === 6 ? 4 : colorType === 2 ? 3 : colorType === 0 ? 1 : 0;
  if (!channels) fail(`Unsupported PNG color type ${colorType}`);
  const inflated = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const rgba = new Uint8Array(width * height * 4);
  let src = 0;
  const prev = new Uint8Array(stride);
  const row = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const filter = inflated[src++];
    for (let i = 0; i < stride; i++) row[i] = inflated[src++];
    for (let i = 0; i < stride; i++) {
      const left = i >= channels ? row[i - channels] : 0;
      const up = prev[i];
      const upLeft = i >= channels ? prev[i - channels] : 0;
      let value = row[i];
      if (filter === 1) value = (value + left) & 255;
      else if (filter === 2) value = (value + up) & 255;
      else if (filter === 3) value = (value + ((left + up) >> 1)) & 255;
      else if (filter === 4) value = (value + paeth(left, up, upLeft)) & 255;
      else if (filter !== 0) fail(`Unsupported PNG filter ${filter}`);
      row[i] = value;
    }
    prev.set(row);
    for (let x = 0; x < width; x++) {
      const o = (y * width + x) * 4;
      if (channels === 4) {
        rgba[o] = row[x * 4];
        rgba[o + 1] = row[x * 4 + 1];
        rgba[o + 2] = row[x * 4 + 2];
        rgba[o + 3] = row[x * 4 + 3];
      } else if (channels === 3) {
        rgba[o] = row[x * 3];
        rgba[o + 1] = row[x * 3 + 1];
        rgba[o + 2] = row[x * 3 + 2];
        rgba[o + 3] = 255;
      } else {
        rgba[o] = rgba[o + 1] = rgba[o + 2] = row[x];
        rgba[o + 3] = 255;
      }
    }
  }
  return { width, height, rgba };
}

function blit(dest, destW, destH, src, origin) {
  const [ox, oy] = origin;
  for (let y = 0; y < src.height; y++) {
    const dy = y + oy;
    if (dy < 0 || dy >= destH) continue;
    for (let x = 0; x < src.width; x++) {
      const dx = x + ox;
      if (dx < 0 || dx >= destW) continue;
      const si = (y * src.width + x) * 4;
      if (src.rgba[si + 3] === 0) continue;
      const di = (dy * destW + dx) * 4;
      dest[di] = src.rgba[si];
      dest[di + 1] = src.rgba[si + 1];
      dest[di + 2] = src.rgba[si + 2];
      dest[di + 3] = src.rgba[si + 3];
    }
  }
}

function layerPixels(layer) {
  return layer.image.rgba;
}

export function loadCyanAssembly(recipePath = join(repoRoot, 'assets/avatars/cyan/layered/recipe.json')) {
  const recipe = JSON.parse(readFileSync(recipePath, 'utf8'));
  if (recipe.schema !== ASSEMBLY_SCHEMA) fail('Unsupported cyan source assembly schema');
  const sourcePath = join(repoRoot, recipe.source);
  const sourceBytes = readFileSync(sourcePath);
  if (sha256Bytes(sourceBytes) !== FEMALE_V2_SHA256) fail('female-v2 digest mismatch');
  if (recipe.source_digest !== FEMALE_V2_SHA256) fail('recipe source_digest does not freeze female-v2');
  const source = decodePng(sourceBytes);
  if (source.width !== recipe.canvas[0] || source.height !== recipe.canvas[1]) fail('canvas mismatch');
  const base = dirname(recipePath);
  const layers = recipe.layers.map((layer) => {
    const image = decodePng(readFileSync(join(base, layer.png)));
    if (image.width !== layer.size[0] || image.height !== layer.size[1]) fail(`layer ${layer.id} size mismatch`);
    return { ...layer, image };
  });
  const silhouette = decodePng(readFileSync(join(base, 'silhouette.png')));
  return { recipe, source, layers, silhouette, sourceBytes };
}

export function assembleRest(assembly, { kinds = ['visible'] } = {}) {
  const [width, height] = assembly.recipe.canvas;
  const rgba = new Uint8Array(width * height * 4);
  const owner = new Int16Array(width * height).fill(-1);
  const wanted = new Set(kinds);
  assembly.recipe.part_stack.forEach((id) => {
    const index = assembly.layers.findIndex((layer) => layer.id === id);
    const layer = assembly.layers[index];
    if (!layer || !wanted.has(layer.kind)) return;
    const [ox, oy] = layer.origin;
    const img = layer.image;
    for (let y = 0; y < img.height; y++) {
      const dy = y + oy;
      if (dy < 0 || dy >= height) continue;
      for (let x = 0; x < img.width; x++) {
        const dx = x + ox;
        if (dx < 0 || dx >= width) continue;
        const si = (y * img.width + x) * 4;
        if (img.rgba[si + 3] === 0) continue;
        const di = dy * width + dx;
        owner[di] = index;
        const pi = di * 4;
        rgba[pi] = img.rgba[si];
        rgba[pi + 1] = img.rgba[si + 1];
        rgba[pi + 2] = img.rgba[si + 2];
        rgba[pi + 3] = img.rgba[si + 3];
      }
    }
  });
  return { width, height, rgba, owner };
}

export function coverageLedger(assembly) {
  const listed = assembly.recipe.coverage;
  if (!Array.isArray(listed) || listed.length !== REQUIRED_SURFACES.length) fail('coverage ledger is incomplete');
  const bySurface = new Map(listed.map((row) => [row.surface, row]));
  return REQUIRED_SURFACES.map((surface) => {
    const row = bySurface.get(surface);
    if (!row) fail(`coverage ledger missing ${surface}`);
    const layers = assembly.layers.filter((layer) => (layer.surfaces || []).includes(surface));
    const pixels = layers.reduce((sum, layer) => {
      let count = 0;
      const data = layerPixels(layer);
      for (let i = 3; i < data.length; i += 4) if (data[i] > 0) count++;
      return sum + count;
    }, 0);
    const present = pixels > 0 && row.status === 'present' && row.layer_ids.length > 0;
    return { surface, status: present ? 'present' : 'missing', layer_ids: row.layer_ids, pixels, finding: row.finding };
  });
}

export function ownershipReport(assembly) {
  const rest = assembleRest(assembly);
  const [width, height] = assembly.recipe.canvas;
  const visible = assembly.layers.map((layer) => (layer.kind === 'visible' ? 0 : -1));
  let owned = 0;
  let silhouette = 0;
  let unowned = 0;
  let outside = 0;
  const sil = assembly.silhouette.rgba;
  for (let i = 0; i < rest.owner.length; i++) {
    const inSil = sil[i * 4] > 127;
    if (inSil) silhouette++;
    if (rest.owner[i] >= 0) {
      owned++;
      if (!inSil) outside++;
    } else if (inSil) {
      unowned++;
    }
  }
  const collisions = visibleCollisions(assembly, width, height);
  return { width, height, owned, silhouette, unowned, outside, collisions, exclusive: collisions === 0 };
}

function visibleCollisions(assembly, width, height) {
  const counts = new Uint8Array(width * height);
  for (const layer of assembly.layers) {
    if (layer.kind !== 'visible') continue;
    const [ox, oy] = layer.origin;
    const img = layer.image;
    for (let y = 0; y < img.height; y++) {
      const dy = y + oy;
      if (dy < 0 || dy >= height) continue;
      for (let x = 0; x < img.width; x++) {
        if (img.rgba[(y * img.width + x) * 4 + 3] === 0) continue;
        const dx = x + ox;
        if (dx < 0 || dx >= width) continue;
        const i = dy * width + dx;
        if (counts[i] < 255) counts[i]++;
      }
    }
  }
  let collisions = 0;
  for (let i = 0; i < counts.length; i++) if (counts[i] > 1) collisions++;
  return collisions;
}

export function isolationReport(assembly) {
  const rest = assembleRest(assembly);
  const sil = assembly.silhouette.rgba;
  let leaked = 0;
  for (let i = 0; i < rest.owner.length; i++) {
    if (rest.owner[i] >= 0 && sil[i * 4] <= 127) leaked++;
  }
  return { leaked, isolated: leaked === 0 };
}

export function fidelityReport(assembly, { boundaryPx = 2 } = {}) {
  const rest = assembleRest(assembly);
  const src = assembly.source.rgba;
  const sil = assembly.silhouette.rgba;
  const width = rest.width;
  const height = rest.height;
  let interior = 0;
  let changed = 0;
  let boundary = 0;
  let boundaryChanged = 0;
  const policy = boundaryPx;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = y * width + x;
      if (sil[i * 4] <= 127) continue;
      let edge = false;
      for (let dy = -policy; dy <= policy && !edge; dy++) {
        for (let dx = -policy; dx <= policy; dx++) {
          const nx = x + dx;
          const ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= width || ny >= height) {
            edge = true;
            break;
          }
          if (sil[(ny * width + nx) * 4] <= 127) {
            edge = true;
            break;
          }
        }
      }
      const pi = i * 4;
      const delta = Math.max(
        Math.abs(rest.rgba[pi] - src[pi]),
        Math.abs(rest.rgba[pi + 1] - src[pi + 1]),
        Math.abs(rest.rgba[pi + 2] - src[pi + 2]),
      );
      if (edge) {
        boundary++;
        if (delta > 8) boundaryChanged++;
        continue;
      }
      interior++;
      if (delta > 8) changed++;
    }
  }
  return { interior, changed, boundary, boundaryChanged, passed: changed === 0 };
}

export function cloneAssembly(assembly) {
  return {
    recipe: structuredClone(assembly.recipe),
    source: assembly.source,
    silhouette: assembly.silhouette,
    sourceBytes: assembly.sourceBytes,
    layers: assembly.layers.map((layer) => ({
      ...layer,
      image: { width: layer.image.width, height: layer.image.height, rgba: layer.image.rgba.slice() },
    })),
  };
}

function findLayer(assembly, id) {
  const layer = assembly.layers.find((item) => item.id === id);
  if (!layer) fail(`missing layer ${id}`);
  return layer;
}

export function displacedEyeFragment(assembly) {
  const next = cloneAssembly(assembly);
  const layer = findLayer(next, 'iris_left');
  const shifted = new Uint8Array(layer.image.rgba.length);
  const w = layer.image.width;
  const h = layer.image.height;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const si = (y * w + x) * 4;
      const dx = x + 24;
      const dy = y + 18;
      if (dx >= w || dy >= h) continue;
      shifted.set(layer.image.rgba.subarray(si, si + 4), (dy * w + dx) * 4);
    }
  }
  layer.image.rgba = shifted;
  return next;
}

export function omittedCollar(assembly) {
  const next = cloneAssembly(assembly);
  next.layers = next.layers.filter((layer) => layer.id !== 'collar');
  next.recipe.part_stack = next.recipe.part_stack.filter((id) => id !== 'collar');
  next.recipe.layers = next.recipe.layers.filter((layer) => layer.id !== 'collar');
  next.recipe.coverage = next.recipe.coverage.map((row) => {
    if (!row.layer_ids.includes('collar')) return row;
    return { ...row, status: 'missing', layer_ids: row.layer_ids.filter((id) => id !== 'collar'), finding: 'collar omitted' };
  });
  return next;
}

export function duplicateUnderlay(assembly) {
  const next = cloneAssembly(assembly);
  const underlay = {
    id: 'flattened_underlay',
    label: 'flattened underlay',
    kind: 'visible',
    surfaces: [],
    png: 'flattened.png',
    origin: [0, 0],
    size: assembly.recipe.canvas,
    image: { width: assembly.source.width, height: assembly.source.height, rgba: assembly.source.rgba.slice() },
  };
  next.layers.unshift(underlay);
  next.recipe.part_stack = ['flattened_underlay', ...next.recipe.part_stack];
  next.recipe.layers = [{ id: 'flattened_underlay', kind: 'visible', surfaces: [], origin: [0, 0], size: assembly.recipe.canvas }, ...next.recipe.layers];
  return next;
}

export function leakedBackground(assembly) {
  const next = cloneAssembly(assembly);
  const layer = findLayer(next, 'face');
  const [ox, oy] = layer.origin;
  const w = layer.image.width;
  const h = layer.image.height;
  for (let y = 0; y < Math.min(24, h); y++) {
    for (let x = 0; x < Math.min(24, w); x++) {
      const canvasX = x + ox;
      const canvasY = y + oy;
      if (canvasX > 80 && canvasY > 80) continue;
      const i = (y * w + x) * 4;
      layer.image.rgba[i] = 255;
      layer.image.rgba[i + 1] = 0;
      layer.image.rgba[i + 2] = 255;
      layer.image.rgba[i + 3] = 255;
    }
  }
  // Stamp into the top-left of the full canvas by moving origin if the face crop does not cover it.
  if (ox > 40 || oy > 40) {
    const canvas = { width: next.recipe.canvas[0], height: next.recipe.canvas[1], rgba: new Uint8Array(next.recipe.canvas[0] * next.recipe.canvas[1] * 4) };
    blit(canvas.rgba, canvas.width, canvas.height, layer.image, layer.origin);
    for (let y = 0; y < 32; y++) {
      for (let x = 0; x < 32; x++) {
        const i = (y * canvas.width + x) * 4;
        canvas.rgba[i] = 255;
        canvas.rgba[i + 1] = 0;
        canvas.rgba[i + 2] = 255;
        canvas.rgba[i + 3] = 255;
      }
    }
    layer.image = canvas;
    layer.origin = [0, 0];
    layer.size = [canvas.width, canvas.height];
  }
  return next;
}

export function collarCoveragePresent(assembly) {
  const collar = assembly.layers.find((layer) => layer.id === 'collar');
  if (!collar) return false;
  const data = collar.image.rgba;
  for (let i = 3; i < data.length; i += 4) if (data[i] > 0) return true;
  return false;
}
