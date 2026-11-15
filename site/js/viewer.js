// The island viewer: makes islands with the real engine (settlers.wasm) and draws them plainly. The
// seed and any changed settings are kept in the address (?seed=...&islets.count=3), so an island can
// be shared by link.
import { loadEngine } from './engine.js';

const SHOWN = /^(island|islets|markets)\./;     // the settings that change the island
const SIZE = 34;                                 // hex size in pixels, centre to corner
const CORNERS = [[0, -2], [1, -1], [1, 1], [0, 2], [-1, 1], [-1, -1]];   // engine/src/hex.rs
const X = SIZE * Math.sqrt(3) / 2, Y = SIZE / 2;
const SVG = 'http://www.w3.org/2000/svg';
const RESOURCE = { forest: 'Timber', quarry: 'Stone', fields: 'Grain', mine: 'Copper', paddy: 'Fish' };
const MARKET = { any: '3:1 any', timber: '2:1 Timber', stone: '2:1 Stone', grain: '2:1 Grain', copper: '2:1 Copper', fish: '2:1 Fish' };

const $ = id => document.getElementById(id);
const engine = await loadEngine(await (await fetch('/js/settlers.wasm')).arrayBuffer());
const table = engine.settingsTable().filter(s => SHOWN.test(s.name));

function readAddress() {
  const params = new URLSearchParams(location.search);
  const settings = {};
  for (const s of table) {
    if (!params.has(s.name)) continue;
    const text = params.get(s.name);
    settings[s.name] = s.kind === 'switch' ? text === 'true' : Number(text);
  }
  return { seed: params.get('seed') || randomSeed(), settings };
}

function writeAddress(seed, settings) {
  const params = new URLSearchParams({ seed });
  for (const [k, v] of Object.entries(settings)) params.set(k, v);
  history.replaceState(null, '', `?${params}`);
}

function randomSeed() {
  const chars = 'abcdefghjkmnpqrstuvwxyz23456789';
  return Array.from(crypto.getRandomValues(new Uint8Array(8)), b => chars[b % chars.length]).join('');
}

function buildControls(settings) {
  const box = $('settings');
  box.replaceChildren();
  for (const s of table) {
    const label = document.createElement('label');
    label.title = s.about;
    label.textContent = s.about.replace(/ \(.*\)$/, '');
    const input = document.createElement('input');
    input.name = s.name;
    if (s.kind === 'switch') {
      input.type = 'checkbox';
      input.checked = settings[s.name] ?? s.default;
      label.classList.add('switch');
    } else {
      input.type = 'number';
      input.min = s.min;
      input.max = s.max;
      input.step = s.kind === 'whole' ? 1 : 0.1;
      input.value = settings[s.name] ?? s.default;
    }
    label.append(input);
    box.append(label);
  }
  syncControls();
}

// The random shape's size means nothing while the island is a hexagon
function syncControls() {
  const hexagon = document.querySelector('input[name="island.hexagon"]');
  const size = document.querySelector('input[name="island.size"]');
  if (hexagon && size) size.disabled = hexagon.checked;
}

function currentSettings() {
  const settings = {};
  for (const s of table) {
    const input = document.querySelector(`input[name="${s.name}"]`);
    const value = s.kind === 'switch' ? input.checked : Number(input.value);
    if (value !== s.default) settings[s.name] = value;
  }
  return settings;
}

function el(name, attrs, text) {
  const node = document.createElementNS(SVG, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  if (text !== undefined) node.textContent = text;
  return node;
}

const pips = n => 6 - Math.abs(7 - n);
const centre = h => ({ x: (2 * h.q + h.r) * X, y: 3 * h.r * Y });

function draw(island) {
  const xs = [], ys = [];
  for (const h of island.hexes) { const c = centre(h); xs.push(c.x); ys.push(c.y); }
  const pad = SIZE * 1.1;
  const minX = Math.min(...xs) - pad, minY = Math.min(...ys) - pad;
  const svg = el('svg', { viewBox: `${minX} ${minY} ${Math.max(...xs) + pad - minX} ${Math.max(...ys) + pad - minY}`, role: 'img', 'aria-label': `Island ${island.seed}` });

  for (const h of island.hexes) {
    const c = centre(h);
    const points = CORNERS.map(([dx, dy]) => `${c.x + dx * X},${c.y + dy * Y}`).join(' ');
    const title = h.terrain === 'sea' ? 'Sea' : `${h.terrain[0].toUpperCase()}${h.terrain.slice(1)}${RESOURCE[h.terrain] ? ` (${RESOURCE[h.terrain]})` : ''}${h.number ? `, ${h.number}` : ''}${h.islet !== null ? `, islet ${h.islet + 1}` : ''}`;
    const poly = el('polygon', { points, class: `hex t-${h.terrain}${h.islet !== null ? ' islet' : ''}`, 'data-terrain': h.terrain });
    poly.append(el('title', {}, title));
    svg.append(poly);
    if (h.number) {
      svg.append(el('circle', { cx: c.x, cy: c.y, r: SIZE * 0.42, class: 'token' }));
      svg.append(el('text', { x: c.x, y: c.y - 2, class: `number${pips(h.number) === 5 ? ' strong' : ''}` }, h.number));
      svg.append(el('text', { x: c.x, y: c.y + 12, class: 'pips' }, '•'.repeat(pips(h.number))));
    }
  }

  // Markets: a label just out to sea from the middle of their edge
  for (const m of island.markets) {
    const h = island.hexes[m.hex];
    const c = centre(h);
    const [a, b] = [CORNERS[m.side], CORNERS[(m.side + 1) % 6]];
    const mid = { x: c.x + (a[0] + b[0]) / 2 * X, y: c.y + (a[1] + b[1]) / 2 * Y };
    const out = { x: c.x + (mid.x - c.x) * 1.45, y: c.y + (mid.y - c.y) * 1.45 };
    svg.append(el('line', { x1: mid.x, y1: mid.y, x2: out.x, y2: out.y, stroke: '#fffaf0', 'stroke-width': 2 }));
    const g = el('g', { 'data-market': m.kind });
    g.append(el('rect', { x: out.x - 27, y: out.y - 8, width: 54, height: 16, rx: 4, class: 'market' }));
    g.append(el('text', { x: out.x, y: out.y, class: 'market-text' }, MARKET[m.kind]));
    g.append(el('title', {}, m.kind === 'any' ? 'Market: 3 of anything for 1' : `Market: 2 ${m.kind} for 1`));
    svg.append(g);
  }

  const sq = centre(island.hexes[island.squall]);
  svg.append(el('circle', { cx: sq.x, cy: sq.y, r: SIZE * 0.6, class: 'squall' }));

  $('board').replaceChildren(svg);
}

function describe(island) {
  const i = island.info;
  const parts = [`Seed ${island.seed}`, `${i.mainSize} hexes`];
  if (i.islets.length) {
    parts.push(`islets ${i.islets.map(x => `${x.size} hex${x.size > 1 ? 'es' : ''} at ${x.gap}`).join(', ')} (average gap ${i.averageGap})`);
    parts.push(`islet resources ${Math.round(i.isletResources * 100)}% of the main island`);
  } else parts.push('no islets');
  return parts.join(' · ');
}

function show(seed, settings) {
  const status = $('status');
  try {
    const started = performance.now();
    const island = engine.island(seed, settings);
    draw(island);
    status.className = '';
    status.textContent = `${describe(island)} · made in ${(performance.now() - started).toFixed(1)} ms`;
    writeAddress(seed, settings);
  } catch (e) {
    status.className = 'error';
    status.textContent = e.message;
  }
}

const start = readAddress();
$('seed').value = start.seed;
buildControls(start.settings);
const legend = ['forest', 'quarry', 'fields', 'mine', 'paddy', 'dunes', 'sea'].map(t => {
  const li = document.createElement('li');
  const swatch = document.createElement('span');
  swatch.className = `k-${t}`;
  li.append(swatch, `${t[0].toUpperCase()}${t.slice(1)}${RESOURCE[t] ? `: ${RESOURCE[t]}` : ''}`);
  return li;
});
const squallKey = document.createElement('li');
squallKey.textContent = 'Dashed ring: the squall · dashed outline: islet · red numbers: 6 and 8';
const marketKey = document.createElement('li');
marketKey.textContent = 'Tags on the coast: markets. "3:1 any": 3 of one resource for 1 of anything. "2:1 Stone": 2 Stone for 1 of anything. Use one with a Hut or Lodge on either corner of its edge.';
$('legend').replaceChildren(...legend, squallKey, marketKey);

const refresh = () => {
  syncControls();
  show($('seed').value.trim() || 'default', currentSettings());
};
$('controls').addEventListener('change', refresh);
$('controls').addEventListener('submit', e => { e.preventDefault(); refresh(); });
$('random').addEventListener('click', () => { $('seed').value = randomSeed(); refresh(); });
$('reset').addEventListener('click', () => { buildControls({}); refresh(); });
show(start.seed, start.settings);
