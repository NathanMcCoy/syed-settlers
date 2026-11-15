// The game API, against the real server: starting games, seats and keys, moves, bots, live updates
import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import { start } from '../server/server.mjs';

const server = await start({ port: 0, host: '127.0.0.1', botDelay: 0 });
const base = `http://127.0.0.1:${server.address().port}`;
after(() => server.close());

const post = (path, data) => fetch(base + path, { method: 'POST', body: JSON.stringify(data) });
const getJson = async path => (await fetch(base + path)).json();
const until = async (check, ms = 5000) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const v = await check();
    if (v) return v;
    await new Promise(r => setTimeout(r, 20));
  }
  throw new Error('timed out');
};

test('a person against bots: bots play their turns, the person moves by key', async () => {
  const res = await post('/api/games', { seats: [{ kind: 'person', name: 'Tester' }, { kind: 'bot' }, { kind: 'bot' }] });
  assert.equal(res.status, 201);
  const { id, seat, key, invites } = await res.json();
  assert.match(id, /^[A-Za-z0-9]{12}$/);
  assert.deepEqual(invites, []);
  // Bots place until it's the person's turn
  const mine = await until(async () => {
    const g = await getJson(`/api/games/${id}?k=${key}`);
    return g.view.toAct === seat && g.view.phase !== 'over' ? g : null;
  });
  assert.equal(mine.view.you, seat);
  assert.equal(mine.seats[seat].name, 'Tester');
  assert.equal(mine.seats.filter(s => s.kind === 'bot').length, 2);
  assert.ok(mine.board.corners.length > 0 && mine.view.legal.length > 0);

  // A wrong key can't move; the right one can
  assert.equal((await post(`/api/games/${id}/move`, { k: 'nope', move: mine.view.legal[0] })).status, 403);
  const bad = await post(`/api/games/${id}/move`, { k: key, move: 'roll' });
  assert.equal(bad.status, 400);
  assert.match((await bad.json()).error, /isn't allowed now/);
  const ok = await post(`/api/games/${id}/move`, { k: key, move: mine.view.legal[0] });
  assert.equal(ok.status, 200);
  const after1 = await getJson(`/api/games/${id}?k=${key}`);
  assert.ok(after1.log.some(l => l.seat === seat && l.move === mine.view.legal[0]));
});

test('friends get their own keys; watchers see no hand', async () => {
  const { id, key, invites } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'person', name: 'Matt' }, { kind: 'bot' }] })).json();
  assert.equal(invites.length, 1);
  assert.equal(invites[0].name, 'Matt');
  const friend = await getJson(`/api/games/${id}?k=${invites[0].key}`);
  assert.equal(friend.view.you, invites[0].seat);
  const watcher = await getJson(`/api/games/${id}`);
  assert.equal(watcher.view.you, null);
  assert.equal(watcher.view.hand, null);
  const me = await getJson(`/api/games/${id}?k=${key}`);
  assert.notEqual(me.view.you, friend.view.you);
});

test('bad requests are refused clearly', async () => {
  for (const [data, why] of [
    [{ seats: [{ kind: 'bot' }, { kind: 'bot' }] }, /first seat is you/],
    [{ seats: [{ kind: 'person' }] }, /2 to 4 seats/],
    [{ seats: [{ kind: 'person' }, { kind: 'dragon' }] }, /person or a bot/],
    [{ seats: [{ kind: 'person' }, { kind: 'bot' }], settings: { win: 50 } }, /win must be between/],
  ]) {
    const r = await post('/api/games', data);
    assert.equal(r.status, 400);
    assert.match((await r.json()).error, why);
  }
  assert.equal((await fetch(`${base}/api/games/nosuchgame0`)).status, 404);
  const big = await fetch(`${base}/api/games`, { method: 'POST', body: 'x'.repeat(10000) });
  assert.equal(big.status, 413);
  assert.equal((await fetch(`${base}/api/games`, { method: 'POST', body: '{nope' })).status, 400);
});

test('names are cleaned', async () => {
  const { id, key } = await (await post('/api/games', { seats: [{ kind: 'person', name: '<b>Al\u0007</b> with a very long name indeed' }, { kind: 'bot' }] })).json();
  const g = await getJson(`/api/games/${id}?k=${key}`);
  const name = g.seats[g.view.you].name;
  assert.ok(!/[<>\u0007]/.test(name) && name.length <= 20, name);
});

test('live updates arrive for each move, with the seat\'s own view', async () => {
  const { id, key } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }] })).json();
  const controller = new AbortController();
  const res = await fetch(`${base}/api/games/${id}/live?k=${key}`, { signal: controller.signal });
  assert.equal(res.headers.get('content-type'), 'text/event-stream');
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let text = '';
  const next = async () => {
    while (!text.includes('\n\n')) {
      const { value, done } = await reader.read();
      if (done) throw new Error('stream ended');
      text += decoder.decode(value);
    }
    const [event, rest] = [text.slice(0, text.indexOf('\n\n')), text.slice(text.indexOf('\n\n') + 2)];
    text = rest;
    return event.startsWith('data: ') ? JSON.parse(event.slice(6)) : next();
  };
  let snap = await next();
  assert.equal(snap.id, id);
  // Play setup moves whenever it's our turn until the first roll is ours or a bot's
  for (let i = 0; i < 40 && snap.view.phase.startsWith('setup'); i++) {
    if (snap.view.toAct === snap.view.you) await post(`/api/games/${id}/move`, { k: key, move: snap.view.legal[0] });
    snap = await next();
    assert.ok(Array.isArray(snap.view.hand), 'every update carries our own hand');
  }
  assert.ok(!snap.view.phase.startsWith('setup'), 'setup finished with live updates');
  controller.abort();
});

test('a whole game of bots through the server finishes', async () => {
  // The person seat plays only "the first legal move", the bots the rest
  const { id, key } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }, { kind: 'bot' }, { kind: 'bot' }] })).json();
  const end = await until(async () => {
    const g = await getJson(`/api/games/${id}?k=${key}`);
    if (g.view.phase === 'over') return g;
    if (g.view.toAct === g.view.you && g.view.legal.length) {
      const pick = g.view.legal.find(m => m === 'roll') ?? g.view.legal.find(m => m === 'end') ?? g.view.legal[0];
      await post(`/api/games/${id}/move`, { k: key, move: pick });
    }
    return null;
  }, 60_000);
  assert.ok(end.view.players[end.view.winner].renown >= end.view.win);
});
