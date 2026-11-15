// Drawing the island, shared by the game and the island viewer. Plain and functional for now:
// illustrations come later. Positions follow engine/src/hex.rs: a hex's centre is (2q + r, 3r) on a
// lattice, corners are at fixed offsets from it, and the lattice is scaled by X and Y to pixels.
export const SIZE = 34;                                                    // pixels, centre to corner
export const X = SIZE * Math.sqrt(3) / 2, Y = SIZE / 2;
export const CORNERS = [[0, -2], [1, -1], [1, 1], [0, 2], [-1, 1], [-1, -1]];
export const RESOURCES = ['Timber', 'Stone', 'Grain', 'Copper', 'Fish'];
export const TERRAIN_RESOURCE = { forest: 'Timber', quarry: 'Stone', fields: 'Grain', mine: 'Copper', paddy: 'Fish' };
const MARKET = { any: '3:1 any', timber: '2:1 Timber', stone: '2:1 Stone', grain: '2:1 Grain', copper: '2:1 Copper', fish: '2:1 Fish' };
const SVG = 'http://www.w3.org/2000/svg';

export function el(name, attrs = {}, text) {
  const node = document.createElementNS(SVG, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  if (text !== undefined) node.textContent = text;
  return node;
}

export const pips = n => 6 - Math.abs(7 - n);
export const centre = h => ({ x: (2 * h.q + h.r) * X, y: 3 * h.r * Y });
export const point = ([x, y]) => ({ x: x * X, y: y * Y });
export const hexPoints = h => {
  const c = centre(h);
  return CORNERS.map(([dx, dy]) => `${c.x + dx * X},${c.y + dy * Y}`).join(' ');
};

export function terrainName(t) {
  return `${t[0].toUpperCase()}${t.slice(1)}`;
}

// The island as an SVG element: hexes, numbers, markets. Layers for pieces go on top of it.
// Which sea hexes to draw: one clean ring around all land, plus any sea hex mostly surrounded by
// drawn hexes (the water between islets and the main island), so the sea has no holes
export function shownHexes(island) {
  const key = h => `${h.q},${h.r}`;
  const at = new Map(island.hexes.map((h, i) => [key(h), i]));
  const near = h => [[1, 0], [1, -1], [0, -1], [-1, 0], [-1, 1], [0, 1]].map(([dq, dr]) => at.get(`${h.q + dq},${h.r + dr}`)).filter(i => i !== undefined);
  const land = island.hexes.filter(h => h.terrain !== 'sea');
  const steps = (a, b) => (Math.abs(a.q - b.q) + Math.abs(a.r - b.r) + Math.abs(a.q + a.r - b.q - b.r)) / 2;
  const shown = new Set();
  island.hexes.forEach((h, i) => {
    if (h.terrain !== 'sea' || near(h).some(n => island.hexes[n].terrain !== 'sea')) { shown.add(i); return; }
    // The straits: within 2 of two different pieces of land (the main island, or an islet)
    const masses = new Set(land.filter(l => steps(l, h) <= 2).map(l => l.islet ?? 'main'));
    if (masses.size >= 2) shown.add(i);
  });
  for (let added = true; added;) {
    added = false;
    island.hexes.forEach((h, i) => {
      if (!shown.has(i) && near(h).filter(n => shown.has(n)).length >= 4) { shown.add(i); added = true; }
    });
  }
  return shown;
}

export function islandSvg(island) {
  // Frame exactly the hexes drawn (land and one ring of sea), with a little room around them
  const shown = shownHexes(island);
  const xs = [], ys = [];
  for (const i of shown) {
    const c = centre(island.hexes[i]);
    xs.push(c.x);
    ys.push(c.y);
  }
  const padX = X + 4, padY = SIZE + 4;
  const minX = Math.min(...xs) - padX, minY = Math.min(...ys) - padY;
  const svg = el('svg', { viewBox: `${minX} ${minY} ${Math.max(...xs) + padX - minX} ${Math.max(...ys) + padY - minY}`, role: 'img', 'aria-label': `Island ${island.seed}` });

  island.hexes.forEach((h, i) => {
    if (!shown.has(i)) return;
    const c = centre(h);
    const resource = TERRAIN_RESOURCE[h.terrain];
    const title = h.terrain === 'sea' ? 'Sea' : `${terrainName(h.terrain)}${resource ? ` (${resource})` : ''}${h.number ? `, ${h.number}` : ''}${h.islet !== null ? `, islet ${h.islet + 1}` : ''}`;
    const poly = el('polygon', { points: hexPoints(h), class: `hex t-${h.terrain}${h.islet !== null ? ' islet' : ''}`, 'data-terrain': h.terrain, 'data-hex': i });
    poly.append(el('title', {}, title));
    svg.append(poly);
    if (h.number) {
      svg.append(el('circle', { cx: c.x, cy: c.y, r: SIZE * 0.42, class: 'token' }));
      svg.append(el('text', { x: c.x, y: c.y - 2, class: `number${pips(h.number) === 5 ? ' strong' : ''}` }, h.number));
      svg.append(el('text', { x: c.x, y: c.y + 12, class: 'pips' }, '•'.repeat(pips(h.number))));
    }
  });

  // Markets: a tag just out to sea from the middle of their edge
  for (const m of island.markets) {
    const c = centre(island.hexes[m.hex]);
    const [a, b] = [CORNERS[m.side], CORNERS[(m.side + 1) % 6]];
    const mid = { x: c.x + (a[0] + b[0]) / 2 * X, y: c.y + (a[1] + b[1]) / 2 * Y };
    // Far enough out that the tag never covers the edge's corners (where Huts go)
    const out = { x: c.x + (mid.x - c.x) * 1.9, y: c.y + (mid.y - c.y) * 1.9 };
    svg.append(el('line', { x1: mid.x, y1: mid.y, x2: out.x, y2: out.y, class: 'market-line' }));
    const g = el('g', { 'data-market': m.kind });
    g.append(el('rect', { x: out.x - 27, y: out.y - 8, width: 54, height: 16, rx: 4, class: 'market' }));
    g.append(el('text', { x: out.x, y: out.y, class: 'market-text' }, MARKET[m.kind]));
    g.append(el('title', {}, m.kind === 'any' ? 'Market: 3 of anything for 1' : `Market: 2 ${m.kind} for 1`));
    svg.append(g);
  }
  return svg;
}
