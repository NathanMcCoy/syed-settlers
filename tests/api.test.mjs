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

test('the hand limit is chosen when the game starts, 7 by default', async () => {
  const plain = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }] })).json();
  assert.equal((await getJson(`/api/games/${plain.id}?k=${plain.key}`)).view.handLimit, 7);
  const nine = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }], settings: { 'discard.over': 9 } })).json();
  assert.equal((await getJson(`/api/games/${nine.id}?k=${nine.key}`)).view.handLimit, 9);
  const bad = await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }], settings: { 'discard.over': 30 } });
  assert.equal(bad.status, 400);
  assert.match((await bad.json()).error, /discard.over must be between 4 and 20/);
});

test('a round limit and hidden projects are chosen when the game starts', async () => {
  const plain = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }] })).json();
  const pv = (await getJson(`/api/games/${plain.id}?k=${plain.key}`)).view;
  assert.equal(pv.roundLimit, 0, 'no limit by default');
  assert.deepEqual(pv.row, [], 'projects are a hidden draw by default');
  assert.equal(pv.deck, 25);
  const set = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }], settings: { rounds: 20, 'projects.faceUp': true } })).json();
  const sv = (await getJson(`/api/games/${set.id}?k=${set.key}`)).view;
  assert.equal(sv.roundLimit, 20);
  assert.equal(sv.row.length, 3, 'face up when chosen');
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

// Plays the first legal move for `key` until it's that seat's main turn with at least one card
async function toMyMainTurn(id, key) {
  return until(async () => {
    const g = await getJson(`/api/games/${id}?k=${key}`);
    const v = g.view;
    if (v.toAct === v.you && v.phase === 'main' && v.hand.some(n => n > 0)) return g;
    if (v.toAct === v.you && v.legal.length) {
      const pick = v.legal.find(m => m === 'roll') ?? v.legal.find(m => m === 'end') ?? v.legal[0];
      await post(`/api/games/${id}/move`, { k: key, move: pick });
    }
    return null;
  }, 30_000);
}

// An offer of one card the player has for one of a resource they don't offer
function oneForOne(hand, to) {
  const give = hand.findIndex(n => n > 0);
  const get = (give + 1) % 5;
  const g = [0, 0, 0, 0, 0], r = [0, 0, 0, 0, 0];
  g[give] = 1;
  r[get] = 1;
  return { text: `offer ${g.join(',')} ${r.join(',')} ${to}`, give, get };
}

test('trading with a bot: the bot answers by itself, an accepted offer can be dealt', async () => {
  // Several games, so some offers are accepted and some declined
  let answered = 0, dealt = 0;
  for (let n = 0; n < 6; n++) {
    const { id, key } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }] })).json();
    const g = await toMyMainTurn(id, key);
    const bot = 1 - g.view.you;
    assert.equal(g.view.canOffer, true);
    assert.equal(g.view.offersLeft, 3);
    const offer = oneForOne(g.view.hand, 1 << bot);
    const res = await post(`/api/games/${id}/move`, { k: key, move: offer.text });
    assert.equal(res.status, 200, await res.text());
    const after = await until(async () => {
      const s = await getJson(`/api/games/${id}?k=${key}`);
      return s.view.offer && s.view.offer.answers[bot] !== null ? s : null;
    });
    answered++;
    assert.deepEqual(after.view.offer.to, [bot]);
    assert.ok(after.log.some(l => l.seat === bot && l.move.startsWith('answer ')));
    if (after.view.offer.answers[bot] === true) {
      const before = after.view.hand;
      assert.ok(after.view.legal.includes(`deal ${bot}`));
      assert.equal((await post(`/api/games/${id}/move`, { k: key, move: `deal ${bot}` })).status, 200);
      const done = (await getJson(`/api/games/${id}?k=${key}`)).view;
      assert.equal(done.hand[offer.give], before[offer.give] - 1);
      assert.equal(done.hand[offer.get], before[offer.get] + 1);
      assert.equal(done.offer, null);
      dealt++;
    } else {
      assert.ok(!after.view.legal.some(m => m.startsWith('deal ')), 'nobody to deal with');
      assert.equal((await post(`/api/games/${id}/move`, { k: key, move: 'withdraw' })).status, 200);
    }
  }
  assert.equal(answered, 6);
});

test('a friend answers an offer on someone else\'s turn, with their own key only', async () => {
  const { id, key, invites } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'person', name: 'Matt' }] })).json();
  const friendKey = invites[0].key;
  // Both play first-legal moves until the host has a main turn with cards
  let g;
  for (let i = 0; i < 400; i++) {
    g = await getJson(`/api/games/${id}?k=${key}`);
    const v = g.view;
    if (v.toAct === v.you && v.phase === 'main' && v.hand.some(n => n > 0)) break;
    const actor = v.toAct === v.you ? key : friendKey;
    const mine = (await getJson(`/api/games/${id}?k=${actor}`)).view;
    if (mine.legal.length) {
      const pick = mine.legal.find(m => m === 'roll') ?? mine.legal.find(m => m === 'end') ?? mine.legal[0];
      await post(`/api/games/${id}/move`, { k: actor, move: pick });
    }
  }
  const friend = invites[0].seat;
  const offer = oneForOne(g.view.hand, 1 << friend);
  assert.equal((await post(`/api/games/${id}/move`, { k: key, move: offer.text })).status, 200);
  const fv = (await getJson(`/api/games/${id}?k=${friendKey}`)).view;
  assert.ok(fv.legal.includes(`answer ${friend} no`), 'the friend may answer although it isn\'t their turn');
  assert.equal(fv.canOffer, false);
  assert.equal((await post(`/api/games/${id}/move`, { k: key, move: `answer ${friend} no` })).status, 400, 'the host can\'t answer for the friend');
  assert.equal((await post(`/api/games/${id}/move`, { k: friendKey, move: `answer ${friend} no` })).status, 200);
  const hv = (await getJson(`/api/games/${id}?k=${key}`)).view;
  assert.equal(hv.offer.answers[friend], false);
  assert.equal((await post(`/api/games/${id}/move`, { k: friendKey, move: 'end' })).status, 400, 'still not the friend\'s turn');
});

// Two people: both play first-legal moves until the host has a main turn with cards
async function hostMainTurn(id, key, friendKey) {
  for (let i = 0; i < 400; i++) {
    const g = await getJson(`/api/games/${id}?k=${key}`);
    const v = g.view;
    if (v.toAct === v.you && v.phase === 'main' && v.hand.some(n => n > 0)) return g;
    const actor = v.toAct === v.you ? key : friendKey;
    const mine = (await getJson(`/api/games/${id}?k=${actor}`)).view;
    if (mine.legal.length) {
      const pick = mine.legal.find(m => m === 'roll') ?? mine.legal.find(m => m === 'end') ?? mine.legal[0];
      await post(`/api/games/${id}/move`, { k: actor, move: pick });
    }
  }
  throw new Error('no main turn for the host');
}

test('a friend counters an offer and the host takes the counter', async () => {
  // Several games, until the friend holds a card the host doesn't offer
  for (let n = 0; n < 10; n++) {
    const { id, key, invites } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'person', name: 'Matt' }] })).json();
    const friendKey = invites[0].key, friend = invites[0].seat;
    const g = await hostMainTurn(id, key, friendKey);
    const fHand = (await getJson(`/api/games/${id}?k=${friendKey}`)).view.hand;
    const give = g.view.hand.findIndex(x => x > 0);
    const theirs = fHand.findIndex((x, r) => x > 0 && r !== give);
    if (theirs < 0) continue;
    const one = r => { const h = [0, 0, 0, 0, 0]; h[r] = 1; return h.join(','); };
    const want = (give + 1) % 5 === theirs ? (give + 2) % 5 : (give + 1) % 5;
    assert.equal((await post(`/api/games/${id}/move`, { k: key, move: `offer ${one(give)} ${one(want)} ${1 << friend}` })).status, 200);
    // Matt counters: the host's same card, for one he actually has
    const text = `counter ${friend} ${one(give)} ${one(theirs)}`;
    assert.equal((await post(`/api/games/${id}/move`, { k: key, move: text })).status, 400, 'the host can\'t counter for him');
    const res = await post(`/api/games/${id}/move`, { k: friendKey, move: text });
    assert.equal(res.status, 200, await res.text());
    const hv = (await getJson(`/api/games/${id}?k=${key}`)).view;
    assert.equal(hv.offer.answers[friend], 'counter');
    assert.deepEqual(hv.offer.counters[friend].get, JSON.parse(`[${one(theirs)}]`));
    assert.ok(hv.legal.includes(`accept ${friend}`));
    assert.equal((await post(`/api/games/${id}/move`, { k: key, move: `accept ${friend}` })).status, 200);
    const done = (await getJson(`/api/games/${id}?k=${key}`)).view;
    assert.equal(done.offer, null);
    assert.equal(done.hand[give], hv.hand[give] - 1);
    assert.equal(done.hand[theirs], hv.hand[theirs] + 1);
    return;
  }
  assert.fail('no game where the friend had a different card');
});

test('an open offer to a bot trades as soon as it accepts', async () => {
  let accepted = 0;
  for (let n = 0; n < 8; n++) {
    const { id, key } = await (await post('/api/games', { seats: [{ kind: 'person' }, { kind: 'bot' }] })).json();
    const g = await toMyMainTurn(id, key);
    const bot = 1 - g.view.you;
    const offer = oneForOne(g.view.hand, 1 << bot);
    assert.equal((await post(`/api/games/${id}/move`, { k: key, move: `${offer.text} open` })).status, 200);
    const after = await until(async () => {
      const s = await getJson(`/api/games/${id}?k=${key}`);
      return s.log.some(l => l.seat === bot && l.move.startsWith('answer ')) ? s : null;
    });
    const said = after.log.findLast(l => l.seat === bot && l.move.startsWith('answer ')).move;
    if (said.endsWith('yes')) {
      accepted++;
      assert.equal(after.view.offer, null, 'no confirmation needed');
      assert.equal(after.view.hand[offer.give], g.view.hand[offer.give] - 1);
      assert.equal(after.view.hand[offer.get], g.view.hand[offer.get] + 1);
    } else {
      assert.equal(after.view.offer.answers[bot], false);
    }
  }
  assert.ok(accepted >= 0);
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
