const SVG_NS = 'http://www.w3.org/2000/svg';
let mountSequence = 0;
function svgElement(parent, name, attributes = {}) {
  const element = parent.ownerDocument.createElementNS(SVG_NS, name);
  for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, String(value));
  parent.append(element); return element;
}
function pathData(path) { return path.map(command => command.join(' ')).join(' '); }

export async function prepareTextures(character) {
  await Promise.all([...character.textures.values()].map(async texture => {
    const image = new Image(); image.src = texture.data;
    await image.decode();
    if (image.naturalWidth !== texture.size[0] || image.naturalHeight !== texture.size[1]) throw new Error('Decoded texture dimensions disagree');
  }));
}

export function mountCharacter(svg, character, onSelect) {
  svg.replaceChildren(); svg.setAttribute('viewBox', `0 0 ${character.project.canvas.join(' ')}`);
  const title = svgElement(svg, 'title'); title.textContent = character.project.label;
  const defs = svgElement(svg, 'defs'), paths = new Map(), masks = new Map();
  const prefix = `mask-${character.project.id}-${++mountSequence}-`;
  for (const texture of character.textures.values()) svgElement(defs, 'image', { id: prefix + 'texture-' + texture.id, href: texture.data, width: texture.size[0], height: texture.size[1] });
  for (const part of character.project.parts) {
    const clip = svgElement(defs, 'clipPath', { id: prefix + part.id, clipPathUnits: 'userSpaceOnUse' });
    const path = svgElement(clip, 'path');
    if (part.maskSampling === 'pixel') path.setAttribute('shape-rendering', 'crispEdges');
    if (part.mask) path.setAttribute('clip-path', `url(#${prefix + part.mask})`);
    masks.set(part.id, path);
  }
  const artwork = svgElement(svg, 'g', { 'data-artwork': '' });
  for (const part of [...character.project.parts].sort((a, b) => a.order-b.order || (a.id < b.id ? -1 : 1))) {
    const path = part.raster ? svgElement(artwork, 'g', { 'data-part': part.id, 'data-raster': '' }) : svgElement(artwork, 'path', { fill: part.fill, stroke: part.stroke,
      'stroke-width': part.strokeWidth, 'stroke-linejoin': 'round', 'stroke-linecap': 'round', 'data-part': part.id });
    if (part.raster) {
      const [x,y,width,height] = part.raster.rect;
      const transform = svgElement(path, 'g', { 'data-raster-transform': '' });
      const viewport = svgElement(transform, 'svg', { x,y,width,height, viewBox: part.raster.source.join(' '), preserveAspectRatio: 'none', overflow: 'hidden' });
      svgElement(viewport, 'use', { href: '#' + prefix + 'texture-' + part.raster.texture });
    }
    if (part.mask) path.setAttribute('clip-path', `url(#${prefix + part.mask})`);
    const label = svgElement(path, 'title'); label.textContent = part.label;
    path.addEventListener('click', () => onSelect(part.id)); paths.set(part.id, path);
  }
  const overlay = svgElement(svg, 'g', { 'data-overlay': '', 'pointer-events': 'none' });
  return { svg, character, paths, masks, overlay };
}

export function paintCharacter(view, pose, { selected = null, rig = false } = {}) {
  for (const part of pose.parts) {
    const d = pathData(part.path), path = view.paths.get(part.id);
    if (view.character.parts.get(part.id).raster) path.querySelector('[data-raster-transform]').setAttribute('transform', `matrix(${part.matrix.join(' ')})`);
    else path.setAttribute('d', d);
    path.setAttribute('opacity', part.opacity);
    path.setAttribute('display', part.visible ? 'inline' : 'none');
    view.masks.get(part.id).setAttribute('d', d);
  }
  view.overlay.replaceChildren();
  if (!rig) return;
  for (const [id, pivot] of pose.pivots) {
    const node = view.character.nodes.get(id), parent = pose.pivots.get(node.parent);
    if (parent) svgElement(view.overlay, 'line', { x1: parent[0], y1: parent[1], x2: pivot[0], y2: pivot[1], class: 'rig-link' });
    svgElement(view.overlay, 'circle', { cx: pivot[0], cy: pivot[1], r: 5, class: 'rig-pivot' });
  }
  const part = pose.parts.find(p => p.id === selected);
  if (!part) return;
  const [x, y, right, bottom] = part.bounds;
  svgElement(view.overlay, 'rect', { x, y, width: right-x, height: bottom-y, class: 'part-bounds' });
  if (view.character.parts.get(part.id).raster) return;
  let index = 0;
  for (const command of part.path) for (let i = 1; i < command.length; i += 2) {
    svgElement(view.overlay, 'circle', { cx: command[i], cy: command[i+1], r: 5,
      class: 'control-handle', 'data-coordinate': index, 'pointer-events': 'all' }); index += 2;
  }
}
