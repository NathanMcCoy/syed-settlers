// The island viewer: makes islands with the real engine (settlers.wasm) and draws them plainly. The
// seed and any changed settings are kept in the address (?seed=...&islets.count=3), so an island can
// be shared by link.
import { loadEngine } from './engine.js';
import { islandSvg, el, centre, SIZE, TERRAIN_RESOURCE as RESOURCE } from './board.js';

const SHOWN = /^(island|islets|markets)\./;     // the settings that change the island

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

function draw(island) {
  const svg = islandSvg(island);
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
