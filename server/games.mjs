// Games on the server: every game runs here, even one person against bots, so the server holds the
// dice and the cards, and bots keep playing whoever is connected. The engine (settlers.wasm) decides
// what's legal; this file handles seats, links, bots' pace and live updates.
//
// A seat is a person or a bot. Each person's seat has a secret key (in their link); the server keeps
// only its SHA-256, so the keys can't be read back from memory dumps or logs. Games live in memory: a
// restart ends them (saving games comes later).
import { createHash, randomBytes } from 'node:crypto';

export const LIMITS = {
  games: 200,              // most games at once; the oldest finished/idle one goes first
  idleMs: 24 * 3600e3,     // a game nobody touched for a day is removed
  log: 40,                 // recent moves kept for the page
  nameLength: 20,
};

const hash = key => createHash('sha256').update(key).digest('hex');
const newKey = () => randomBytes(18).toString('base64url');
const ID_CHARS = 'abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789';
const newId = () => Array.from(randomBytes(12), b => ID_CHARS[b % ID_CHARS.length]).join('');

export const BOT_NAMES = ['Coral', 'Kelp', 'Pebble', 'Driftwood', 'Heron', 'Lagoon'];

export function makeGames(engine, { botDelay = 700, now = () => Date.now() } = {}) {
  const games = new Map();

  function cleanName(name, fallback) {
    const text = String(name ?? '').replace(/[\u0000-\u001f\u007f<>]/g, '').trim().slice(0, LIMITS.nameLength);
    return text || fallback;
  }

  // seats: [{ kind: 'person' | 'bot', name? }], the first being whoever creates the game.
  // Returns the game's id, the creator's seat and key, and keys for the other people's seats.
  function create({ seats, settings = {} }) {
    if (!Array.isArray(seats) || seats.length < 2 || seats.length > 4) throw new Error('a game has 2 to 4 seats');
    if (seats[0]?.kind !== 'person') throw new Error('the first seat is you');
    if (!seats.every(s => s?.kind === 'person' || s?.kind === 'bot')) throw new Error('each seat is a person or a bot');
    if (typeof settings !== 'object' || settings === null || Array.isArray(settings)) throw new Error('settings must be an object');
    prune();
    if (games.size >= LIMITS.games) throw new Error('the server is full; try again later');

    // Seat order is random (RULES.md): shuffle who sits where
    const order = seats.map((s, i) => ({ s, i, r: randomBytes(4).readUInt32LE() })).sort((a, b) => a.r - b.r);
    const id = newId();
    const seed = newId();
    const engineId = engine.newGame(seed, seats.length, settings);
    const bots = [...BOT_NAMES].sort(() => randomBytes(1)[0] - 128);
    const keys = [];
    const table = order.map(({ s, i }, seat) => {
      if (s.kind === 'bot') return { kind: 'bot', name: bots.pop(), key: null, from: i };
      const key = newKey();
      keys.push({ from: i, seat, key });
      // A blank name: "Host" for whoever started the game ("You" would read wrong on friends' screens)
      return { kind: 'person', name: cleanName(s.name, i === 0 ? 'Host' : `Friend ${i}`), key: hash(key), from: i };
    });
    const game = { id, engineId, seats: table, log: [], listeners: new Set(), timer: null, touched: now(), created: now() };
    games.set(id, game);
    schedule(game);
    const mine = keys.find(k => k.from === 0);
    return {
      id,
      seat: mine.seat,
      key: mine.key,
      invites: keys.filter(k => k.from !== 0).map(({ seat, key }) => ({ seat, name: table[seat].name, key })),
    };
  }

  function get(id) {
    const game = games.get(id);
    if (!game) throw Object.assign(new Error('no such game (it may have ended or the server restarted)'), { status: 404 });
    return game;
  }

  // Which seat a key belongs to, or null (watching)
  function seatOf(game, key) {
    if (!key) return null;
    const h = hash(String(key));
    const seat = game.seats.findIndex(s => s.key === h);
    return seat < 0 ? null : seat;
  }

  function seatsPublic(game) {
    return game.seats.map(s => ({ kind: s.kind, name: s.name, online: s.kind === 'bot' || [...game.listeners].some(l => l.seat === game.seats.indexOf(s)) }));
  }

  // Everything a page needs about a game, from one seat's point of view
  function snapshot(game, seat) {
    return { id: game.id, seats: seatsPublic(game), view: engine.view(game.engineId, seat), log: game.log };
  }

  function open(id, key) {
    const game = get(id);
    const seat = seatOf(game, key);
    return { ...snapshot(game, seat), board: engine.board(game.engineId) };
  }

  function act(id, key, move) {
    const game = get(id);
    const seat = seatOf(game, key);
    if (seat === null) throw Object.assign(new Error('that link has no seat in this game'), { status: 403 });
    if (typeof move !== 'string' || move.length > 40) throw new Error('a move is short text');
    const before = engine.view(game.engineId, null).phase;
    engine.act(game.engineId, seat, move);
    record(game, seat, move, before);
    changed(game);
  }

  // `before`: the phase the move was made in (a Market Day pick also logs how many cards it took)
  function record(game, seat, move, before) {
    const v = engine.view(game.engineId, null);
    const got = before === 'marketDay' ? v.lastTake.reduce((a, b) => a + b, 0) : undefined;
    game.log.push({ seat, move, roll: move === 'roll' ? v.lastRoll : undefined, got });
    if (game.log.length > LIMITS.log) game.log.shift();
    game.touched = now();
  }

  // Tell every open page, then let bots move if it's their turn
  function changed(game) {
    for (const l of game.listeners) send(l, snapshot(game, l.seat));
    schedule(game);
  }

  // Bots' next step, after a pause: first answers to an open offer made to bots (one per pause, so
  // people see them come in), then the move of a bot whose turn it is
  function schedule(game) {
    if (game.timer) return;
    const v = engine.view(game.engineId, null);
    if (v.phase === 'over') return;
    const answering = v.offer ? v.offer.to.find(s => game.seats[s].kind === 'bot' && v.offer.answers[s] === null) : undefined;
    if (answering === undefined && game.seats[v.toAct].kind !== 'bot') return;
    game.timer = setTimeout(() => {
      game.timer = null;
      if (!games.has(game.id)) return;
      if (answering !== undefined) {
        const move = engine.botAnswer(game.engineId, answering);
        if (move) record(game, answering, move);
      } else {
        const { toAct: seat, phase } = engine.view(game.engineId, null);
        record(game, seat, engine.bot(game.engineId), phase);
      }
      changed(game);
    }, botDelay);
  }

  // Live updates: Server-Sent Events. Each listener gets its own seat's view.
  function listen(id, key, res) {
    const game = get(id);
    const listener = { seat: seatOf(game, key), res };
    game.listeners.add(listener);
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-store, no-transform', 'X-Accel-Buffering': 'no' });
    send(listener, snapshot(game, listener.seat));
    for (const l of game.listeners) if (l !== listener) send(l, snapshot(game, l.seat));   // "online" changed
    const ping = setInterval(() => res.write(': keep-alive\n\n'), 25_000);
    res.on('close', () => {
      clearInterval(ping);
      game.listeners.delete(listener);
      for (const l of game.listeners) send(l, snapshot(game, l.seat));
    });
  }

  function send(listener, data) {
    listener.res.write(`data: ${JSON.stringify(data)}\n\n`);
  }

  // Remove idle games, and make room when full (finished games first, then the longest idle)
  function prune() {
    const t = now();
    for (const g of games.values()) if (t - g.touched > LIMITS.idleMs && g.listeners.size === 0) remove(g);
    if (games.size < LIMITS.games) return;
    const list = [...games.values()].filter(g => g.listeners.size === 0);
    const over = g => engine.view(g.engineId, null).phase === 'over';
    list.sort((a, b) => (over(b) - over(a)) || a.touched - b.touched);
    if (list[0]) remove(list[0]);
  }

  function remove(game) {
    clearTimeout(game.timer);
    engine.free(game.engineId);
    games.delete(game.id);
  }

  function stop() {
    for (const g of games.values()) {
      clearTimeout(g.timer);
      for (const l of g.listeners) l.res.end();
    }
  }

  return { create, open, act, listen, stop, count: () => games.size, _games: games };
}
