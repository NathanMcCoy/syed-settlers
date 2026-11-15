// Games through the WebAssembly, as the server will run them
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { loadEngine } from '../site/js/engine.js';

const engine = await loadEngine(readFileSync(new URL('../site/js/settlers.wasm', import.meta.url)));

test('a game of bots plays to the end, every view consistent', () => {
  const id = engine.newGame('wasm-bots', 3);
  const board = engine.board(id);
  assert.equal(board.players, 3);
  assert.ok(board.corners.length > 50 && board.edges.length > board.corners.length);
  assert.equal(board.island.seed, 'wasm-bots');
  let moves = 0;
  for (; moves < 5000; moves++) {
    const watcher = engine.view(id, null);
    if (watcher.phase === 'over') break;
    const seat = watcher.toAct;
    const mine = engine.view(id, seat);
    assert.ok(mine.legal.length > 0, `seat ${seat} has moves in ${mine.phase}`);
    for (let s = 0; s < 3; s++) if (s !== seat) assert.deepEqual(engine.view(id, s).legal, [], 'only the seat to act has moves');
    const move = engine.bot(id);
    assert.ok(mine.legal.includes(move), `the bot's ${move} was legal`);
  }
  const end = engine.view(id, null);
  assert.equal(end.phase, 'over');
  assert.ok(end.players[end.winner].renown >= end.win);
  engine.free(id);
  assert.throws(() => engine.view(id, 0), /no game/);
});

test('people move by text; wrong seats and illegal moves are refused', () => {
  const id = engine.newGame('wasm-people', 2);
  const v = engine.view(id, 0);
  assert.equal(v.phase, 'setupHut');
  assert.equal(v.you, 0);
  assert.throws(() => engine.act(id, 1, v.legal[0]), /isn't your move/);
  assert.throws(() => engine.act(id, 0, 'roll'), /isn't allowed now/);
  assert.throws(() => engine.act(id, 0, 'fly 3'), /unknown move/);
  engine.act(id, 0, v.legal[0]);
  const after = engine.view(id, 0);
  assert.equal(after.phase, 'setupPath');
  assert.equal(after.buildings.length, 1);
  assert.deepEqual(after.buildings[0], [Number(v.legal[0].split(' ')[1]), 0, 1]);
  engine.free(id);
});

test('hands are private: other seats and watchers see counts', () => {
  const id = engine.newGame('wasm-private', 3);
  while (engine.view(id, null).phase !== 'preRoll') engine.bot(id);
  const v0 = engine.view(id, 0), v1 = engine.view(id, 1), w = engine.view(id, null);
  assert.ok(Array.isArray(v0.hand) && Array.isArray(v1.hand));
  assert.equal(w.hand, null);
  assert.equal(v0.players[1].cards, v1.hand.reduce((a, b) => a + b, 0));
  assert.ok(!JSON.stringify(v0).includes(`"hand":${JSON.stringify(v1.hand)}`) || JSON.stringify(v0.hand) === JSON.stringify(v1.hand));
  engine.free(id);
});

test('bad game settings are refused', () => {
  assert.throws(() => engine.newGame('x', 5), /5 players isn't allowed/);
  assert.throws(() => engine.newGame('x', 3, { 'win': 99 }), /win must be between/);
});
