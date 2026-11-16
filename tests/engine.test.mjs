// The engine as the server and page use it: site/js/settlers.wasm through site/js/engine.js. The
// rules themselves are tested in Rust (cargo test, in engine/src); these tests check that the .wasm
// is up to date with that source and that the WebAssembly side works.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { loadEngine } from '../site/js/engine.js';

const engine = await loadEngine(readFileSync(new URL('../site/js/settlers.wasm', import.meta.url)));

// The same fingerprint as engine/build.rs: FNV-1a (64-bit) over src/*.rs sorted by name, without \r
function sourceHash() {
  const dir = new URL('../engine/src/', import.meta.url);
  const files = readdirSync(dir).filter(f => f.endsWith('.rs')).sort();
  let h = 0xcbf29ce484222325n;
  for (const f of files) {
    for (const b of readFileSync(new URL(f, dir))) {
      if (b === 13) continue;
      h ^= BigInt(b);
      h = (h * 0x100000001b3n) & 0xffffffffffffffffn;
    }
  }
  return h.toString(16).padStart(16, '0');
}

test('settlers.wasm was built from the engine source as it is now (else: npm run build)', () => {
  assert.equal(engine.sourceHash(), sourceHash());
});

test('islands come out of WebAssembly with every part', () => {
  const island = engine.island('betta');
  assert.equal(island.version, 2);
  assert.equal(island.seed, 'betta');
  const main = island.hexes.filter(h => h.terrain !== 'sea');
  assert.equal(main.length, 19, 'the hexagon, 5 across');
  assert.equal(island.markets.length, 9);
  assert.ok(island.hexes.every(h => !('islet' in h)) && !('islets' in island.info), 'no islets any more');
  assert.equal(island.hexes[island.squall].terrain, 'dunes');
  assert.ok(main.some(h => h.terrain === 'paddy'));
  assert.deepEqual(engine.island('betta'), island, 'same seed, same island');
  assert.notDeepEqual(engine.island('bettb'), island);
});

test('settings reach the engine, and mistakes come back as errors', () => {
  const island = engine.island('x', { 'island.hexagon': false, 'island.size': 30, 'markets.any': 3 });
  assert.equal(island.info.mainSize, 30);
  assert.equal(island.markets.filter(m => m.kind === 'any').length, 3);
  assert.throws(() => engine.island('x', { 'island.dunes': 4 }), /island.dunes must be between 0 and 3/);
  assert.throws(() => engine.island('x', { 'islets.count': 2 }), /unknown setting islets.count/);
  assert.throws(() => engine.island('two\nlines'), /one line/);
});

test('the settings table lists every knob with its range', () => {
  const table = engine.settingsTable();
  const byName = Object.fromEntries(table.map(s => [s.name, s]));
  assert.deepEqual(byName['squall.range'], { name: 'squall.range', kind: 'whole', default: 0, min: 0, max: 8, about: byName['squall.range'].about });
  assert.equal(byName['projects.deck.roadCrew'].default, 2);
  assert.equal(byName['projects.deck.marketDay'].default, 2);
  assert.equal(byName['titles.mostPilots.min'].default, 3);
  assert.equal(byName['titles.longestPath.min'].default, 5);
  for (const gone of ['islets.count', 'pieces.boat', 'titles.navigator.min', 'projects.deck.winds', 'projects.deck.bargain', 'titles.mostMarkets.min', 'titles.mostMarkets.renown']) assert.ok(!byName[gone], gone);
  assert.equal(byName.win.default, 10);
  assert.equal(byName['projects.faceUp'].kind, 'switch');
});

test('text seeds with any characters work', () => {
  for (const seed of ['', 'ünïcödé 🐟', '"quoted"', 'a\\b']) {
    assert.equal(engine.island(seed).seed, seed);
  }
});
