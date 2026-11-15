// The game page. With no game in the address it shows "New game"; with ?g=ID it shows that game,
// live. The server runs every game and sends this page its seat's view (server/games.mjs); the page
// only draws it and sends moves as text. On your turn, the moves the server lists as legal light up.
//
// Your seat's key comes in the link's #k=... part (never sent to the server in the address), then is
// kept in this browser (localStorage) so a reload finds your seat again.
import { islandSvg, el, centre, point, hexPoints, RESOURCES, SIZE } from './board.js';

const $ = id => document.getElementById(id);
const COLOURS = ['Red', 'Blue', 'Orange', 'Purple'];
const PROJECTS = { pilot: 'Pilot', lighthouse: 'Lighthouse', shipwright: 'Shipwright', harvest: 'Good Harvest', winds: 'Fair Winds' };
const PROJECT_HELP = {
  pilot: 'Move the squall to any land hex, then steal',
  lighthouse: '1 Renown, counted at once',
  shipwright: '2 free Paths or Boats',
  harvest: 'Take any 2 resources',
  winds: 'This turn, trade 2:1 for one resource',
};

const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* private mode: the link still works */ } },
};

// ---------- New game ----------

function home() {
  $('home').hidden = false;
  // Each other seat: Bot / Friend / Nobody, as a row of buttons (no <select>: WebKit styles those
  // with a built-in stylesheet that the site's security policy blocks)
  const rows = $('seat-rows');
  const kinds = [];
  for (let i = 1; i <= 3; i++) {
    const row = document.createElement('div');
    row.className = 'seat-row';
    const label = document.createElement('span');
    label.textContent = `Seat ${i + 1}`;
    const group = document.createElement('div');
    group.className = 'pick';
    group.setAttribute('role', 'group');
    group.setAttribute('aria-label', `Seat ${i + 1}`);
    const name = document.createElement('input');
    name.name = `name${i}`;
    name.maxLength = 20;
    name.placeholder = "Friend's name";
    kinds[i] = i <= 2 ? 'bot' : '';
    const choose = kind => {
      kinds[i] = kind;
      for (const b of group.children) b.setAttribute('aria-pressed', String(b.dataset.kind === kind));
      name.hidden = kind !== 'person';
    };
    for (const [kind, text] of [['bot', 'Bot'], ['person', 'Friend'], ['', 'Nobody']]) {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = text;
      b.dataset.kind = kind;
      b.dataset.seat = i;
      b.addEventListener('click', () => choose(kind));
      group.append(b);
    }
    choose(kinds[i]);
    row.append(label, group, name);
    rows.append(row);
  }
  const saved = store.get('settlers.name');
  if (saved) $('new-game').elements.name.value = saved;

  $('new-game').addEventListener('submit', async e => {
    e.preventDefault();
    const f = e.target.elements;
    const name = f.name.value.trim();
    store.set('settlers.name', name);
    const seats = [{ kind: 'person', name }];
    for (let i = 1; i <= 3; i++) {
      const kind = kinds[i];
      if (kind) seats.push({ kind, name: kind === 'person' ? f[`name${i}`].value.trim() : undefined });
    }
    const settings = {};
    if (Number(f.win.value) !== 10) settings.win = Number(f.win.value);
    $('home-error').textContent = '';
    try {
      const res = await fetch('/api/games', { method: 'POST', body: JSON.stringify({ seats, settings }) });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error);
      store.set(`settlers.key.${data.id}`, data.key);
      store.set(`settlers.invites.${data.id}`, JSON.stringify(data.invites));
      location.href = `/?g=${data.id}`;
    } catch (err) {
      $('home-error').textContent = err.message;
    }
  });
}

// ---------- The game ----------

let id, key, board, snap, layers, pendingEdge = null, sending = false;

async function game(gameId) {
  id = gameId;
  const fromLink = new URLSearchParams(location.hash.slice(1)).get('k');
  if (fromLink) {
    store.set(`settlers.key.${id}`, fromLink);
    history.replaceState(null, '', `/?g=${id}`);
  }
  key = store.get(`settlers.key.${id}`) || '';
  $('game').hidden = false;
  document.body.classList.add('in-game');
  const res = await fetch(`/api/games/${id}?k=${encodeURIComponent(key)}`);
  const data = await res.json();
  if (!res.ok) {
    $('status').textContent = data.error;
    return;
  }
  board = data.board;
  drawBoard();
  showInvites();
  render(data);
  const live = new EventSource(`/api/games/${id}/live?k=${encodeURIComponent(key)}`);
  live.onmessage = e => render(JSON.parse(e.data));
  $('trade').addEventListener('click', () => {
    if (trade.give !== null && trade.get !== null) send(`trade ${trade.give} ${trade.get}`);
  });
}

function drawBoard() {
  const svg = islandSvg(board.island);
  layers = { roads: el('g'), buildings: el('g'), squall: el('g'), targets: el('g') };
  svg.append(layers.roads, layers.buildings, layers.squall, layers.targets);
  $('board').replaceChildren(svg);
}

const cornerAt = c => point(board.corners[c]);
const edgeEnds = e => board.edges[e].map(cornerAt);
const nameOf = s => snap.seats[s]?.name ?? `Seat ${s + 1}`;
const legal = () => (snap.view.toAct === snap.view.you ? snap.view.legal : []);

// A move goes to the server. A second click is ignored only while the game hasn't changed since the
// first (a double click); once a newer state has arrived, the next click is a new move, even if the
// server's reply to the last one is still on its way (live updates can arrive first).
let version = 0, sentAt = -1;
async function send(move) {
  if (sending && sentAt === version) return;
  sending = true;
  sentAt = version;
  $('move-error').textContent = '';
  try {
    const res = await fetch(`/api/games/${id}/move`, { method: 'POST', body: JSON.stringify({ k: key, move }) });
    if (!res.ok) $('move-error').textContent = (await res.json()).error;
    pendingEdge = null;
  } catch {
    $('move-error').textContent = 'Couldn\'t reach the server. Check your connection and try again.';
  } finally {
    sending = false;
  }
}

function render(data) {
  snap = data;
  version++;
  const v = snap.view;
  drawPieces(v);
  drawTargets(v);
  $('status').textContent = status(v);
  drawChoices(v);
  drawPlayers(v);
  drawHand(v);
  drawTrade(v);
  drawProjects(v);
  drawLog();
  document.body.dataset.phase = v.phase;
  document.body.dataset.myTurn = String(v.toAct === v.you && v.phase !== 'over');
}

function drawPieces(v) {
  layers.roads.replaceChildren();
  for (const [e, p, kind] of v.roads) {
    const [a, b] = edgeEnds(e);
    layers.roads.append(el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: `road p${p}${kind === 2 ? ' boat' : ''}`, 'data-edge': e }));
  }
  layers.buildings.replaceChildren();
  for (const [c, p, kind] of v.buildings) {
    const { x, y } = cornerAt(c);
    const piece = kind === 2
      ? el('rect', { x: x - 9, y: y - 9, width: 18, height: 18, rx: 2, class: `building lodge p${p}`, 'data-corner': c })
      : el('circle', { cx: x, cy: y, r: 7.5, class: `building hut p${p}`, 'data-corner': c });
    piece.append(el('title', {}, `${nameOf(p)}'s ${kind === 2 ? 'Lodge' : 'Hut'}`));
    layers.buildings.append(piece);
  }
  const sq = centre(board.island.hexes[v.squall]);
  layers.squall.replaceChildren(el('circle', { cx: sq.x, cy: sq.y, r: SIZE * 0.6, class: 'squall' }));
}

// Clickable spots for the legal moves that are places on the board
function drawTargets(v) {
  layers.targets.replaceChildren();
  const moves = legal();
  const edges = new Map();   // edge -> ['path', 'boat']
  for (const m of moves) {
    const [word, n] = m.split(' ');
    if (word === 'hut' || word === 'lodge') {
      const { x, y } = cornerAt(Number(n));
      const t = el('circle', { cx: x, cy: y, r: 9, class: `target corner ${word}`, 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, word === 'hut' ? 'Build a Hut here' : 'Upgrade to a Lodge'));
      layers.targets.append(t);
    } else if (word === 'path' || word === 'boat') {
      if (!edges.has(n)) edges.set(n, []);
      edges.get(n).push(word);
    } else if (word === 'squall') {
      const h = board.island.hexes[Number(n)];
      const t = el('polygon', { points: hexPoints(h), class: 'target hex-target', 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, 'Move the squall here'));
      layers.targets.append(t);
    }
  }
  for (const [e, kinds] of edges) {
    const [a, b] = edgeEnds(Number(e));
    const t = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: `target edge${pendingEdge === e ? ' chosen' : ''}`, 'data-edge-target': e, tabindex: 0 });
    t.append(el('title', {}, kinds.length > 1 ? 'Build a Path or a Boat here' : `Build a ${kinds[0] === 'path' ? 'Path' : 'Boat'} here`));
    t.addEventListener('click', () => {
      if (kinds.length === 1) return send(`${kinds[0]} ${e}`);
      pendingEdge = e;   // both possible: ask which
      render(snap);
    });
    layers.targets.append(t);
  }
  for (const t of layers.targets.querySelectorAll('[data-move]')) {
    t.addEventListener('click', () => send(t.dataset.move));
    t.addEventListener('keydown', ev => { if (ev.key === 'Enter' || ev.key === ' ') send(t.dataset.move); });
  }
}

function status(v) {
  if (v.phase === 'over') return v.winner === v.you ? `You win with ${v.players[v.winner].renown} Renown!` : `${nameOf(v.winner)} wins with ${v.players[v.winner].renown} Renown.`;
  const who = v.toAct === v.you ? 'You' : nameOf(v.toAct);
  const mine = v.toAct === v.you;
  switch (v.phase) {
    case 'setupHut': return mine ? 'Setup: place a Hut on a lit corner.' : `Setup: ${who} is placing a Hut.`;
    case 'setupPath': return mine ? 'Setup: place a Path or Boat next to your new Hut.' : `Setup: ${who} is placing a Path.`;
    case 'preRoll': return mine ? 'Your turn: roll the dice (or play a project first).' : `${who}'s turn.`;
    case 'discard': return mine ? `A 7! You hold too many cards: choose ${v.players[v.you].discard} more to lose.` : `A 7: ${who} is choosing cards to lose.`;
    case 'moveSquall': return mine ? 'Move the squall: click a lit hex.' : `${who} is moving the squall.`;
    case 'steal': return mine ? 'Choose who to steal a card from.' : `${who} is choosing who to steal from.`;
    case 'main': return mine ? `You rolled ${v.lastRoll}. Build, trade or buy, then end your turn.` : `${who} rolled ${v.lastRoll}.`;
    case 'freeBuild': return mine ? 'Shipwright: place a free Path or Boat.' : `${who} is placing free Paths.`;
    case 'harvest': return mine ? 'Good Harvest: take a resource.' : `${who} is taking resources.`;
    case 'winds': return mine ? 'Fair Winds: choose the resource you\'ll trade 2:1.' : `${who} is choosing a wind.`;
    default: return '';
  }
}

function button(text, move, extra = '') {
  const b = document.createElement('button');
  b.type = 'button';
  b.textContent = text;
  if (extra) b.className = extra;
  b.dataset.move = move;
  b.addEventListener('click', () => send(move));
  return b;
}

// Buttons for the legal moves that aren't places on the board
function drawChoices(v) {
  const box = $('choices');
  box.replaceChildren();
  const moves = legal();
  if (pendingEdge !== null && moves.includes(`path ${pendingEdge}`)) {
    box.append(button('Build a Path', `path ${pendingEdge}`), button('Build a Boat', `boat ${pendingEdge}`));
  }
  for (const m of moves) {
    const [word, n] = m.split(' ');
    if (word === 'roll') box.append(button('Roll the dice', m, 'primary'));
    if (word === 'discard') box.append(button(`Lose ${RESOURCES[n]} (${v.hand[n]})`, m));
    if (word === 'take') box.append(button(`Take ${RESOURCES[n]}`, m));
    if (word === 'steal') box.append(button(`Steal from ${nameOf(Number(n))} (${v.players[n].cards} cards)`, m));
    if (word === 'play') box.append(button(`Play ${PROJECTS[n]}`, m));
  }
  if (moves.includes('end')) box.append(button('End turn', 'end', 'primary'));
}

function drawPlayers(v) {
  const list = $('players');
  list.replaceChildren();
  v.players.forEach((p, s) => {
    const li = document.createElement('li');
    li.className = `player p${s}${v.toAct === s && v.phase !== 'over' ? ' acting' : ''}`;
    const seat = snap.seats[s];
    const titles = [v.navigator === s ? 'Navigator' : null, v.harbourmaster === s ? 'Harbourmaster' : null].filter(Boolean);
    const dot = document.createElement('span');
    dot.className = `dot p${s}`;
    const name = document.createElement('strong');
    name.textContent = `${seat.name}${s === v.you && seat.name !== 'You' ? ' (you)' : ''}`;
    const info = document.createElement('span');
    info.className = 'info';
    info.textContent = [
      `${p.renown}/${v.win} Renown`,
      `${p.cards} cards`,
      seat.kind === 'bot' ? 'bot' : seat.online ? null : 'away',
      p.projectCount ? `${p.projectCount} projects` : null,
      titles.length ? titles.join(', ') : null,
    ].filter(Boolean).join(' · ');
    li.title = `${COLOURS[s]}`;
    li.append(dot, name, info);
    list.append(li);
  });
}

function drawHand(v) {
  $('hand-box').hidden = v.hand === null;
  if (v.hand === null) return;
  $('hand').replaceChildren(...RESOURCES.map((r, i) => {
    const li = document.createElement('li');
    li.className = `card r${i}`;
    li.textContent = `${r} ${v.hand[i]}`;
    return li;
  }));
}

// Trading: pick what to give (only what you can afford, at your rate) and what to get
const trade = { give: null, get: null };
function drawTrade(v) {
  const trades = legal().filter(m => m.startsWith('trade '));
  $('trade-box').hidden = trades.length === 0;
  if (!trades.length) return;
  const gives = [...new Set(trades.map(m => Number(m.split(' ')[1])))];
  if (!gives.includes(trade.give)) trade.give = gives[0];
  if (trade.get === trade.give || trade.get === null) trade.get = RESOURCES.findIndex((_, i) => i !== trade.give);
  const pick = (box, list, chosen, label, onPick) => {
    box.replaceChildren(...list.map(r => {
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = label(r);
      b.className = `card r${r}`;
      b.setAttribute('aria-pressed', String(r === chosen));
      b.addEventListener('click', () => { onPick(r); drawTrade(v); });
      return b;
    }));
  };
  pick($('give'), gives, trade.give, r => `${v.rates[r]} ${RESOURCES[r]}`, r => { trade.give = r; });
  pick($('get'), [0, 1, 2, 3, 4].filter(r => r !== trade.give), trade.get, r => RESOURCES[r], r => { trade.get = r; });
}

function drawProjects(v) {
  const buys = legal().filter(m => m.startsWith('buy '));
  $('row').replaceChildren(...v.row.map((k, slot) => {
    const li = document.createElement('li');
    li.className = 'project';
    const name = document.createElement('strong');
    name.textContent = PROJECTS[k];
    const help = document.createElement('span');
    help.textContent = PROJECT_HELP[k];
    li.append(name, help);
    if (buys.includes(`buy ${slot}`)) li.append(button('Buy', `buy ${slot}`));
    return li;
  }));
  if (!v.row.length && buys.includes('buy 0')) $('row').append(button('Buy a project (unseen)', 'buy 0'));
  const mine = v.you === null ? null : v.players[v.you].projects;
  $('held').textContent = mine
    ? `Yours: ${Object.keys(PROJECTS).map((k, i) => (mine[i] ? `${PROJECTS[k]} ×${mine[i]}` : null)).filter(Boolean).join(', ') || 'none'} · ${v.deck} left in the deck`
    : `${v.deck} left in the deck`;
}

function describe(entry) {
  const who = entry.seat === snap.view.you ? 'You' : nameOf(entry.seat);
  const [word, a, b] = entry.move.split(' ');
  switch (word) {
    case 'hut': return `${who} built a Hut`;
    case 'lodge': return `${who} built a Lodge`;
    case 'path': return `${who} built a Path`;
    case 'boat': return `${who} built a Boat`;
    case 'roll': return `${who} rolled ${entry.roll}`;
    case 'discard': return `${who} lost a ${RESOURCES[a]}`;
    case 'squall': return `${who} moved the squall`;
    case 'steal': return `${who} stole a card from ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'trade': return `${who} traded ${RESOURCES[a]} for ${RESOURCES[b]}`;
    case 'buy': return `${who} bought a project`;
    case 'play': return `${who} played ${PROJECTS[a]}`;
    case 'take': return `${who} took ${RESOURCES[a]}`;
    case 'end': return `${who} ended the turn`;
    default: return `${who}: ${entry.move}`;
  }
}

function drawLog() {
  $('log').replaceChildren(...snap.log.slice(-12).reverse().map(entry => {
    const li = document.createElement('li');
    li.className = `p${entry.seat}`;
    li.textContent = describe(entry);
    return li;
  }));
}

function showInvites() {
  let invites = [];
  try { invites = JSON.parse(store.get(`settlers.invites.${id}`) || '[]'); } catch { /* none */ }
  $('invite-box').hidden = invites.length === 0;
  $('invites').replaceChildren(...invites.map(inv => {
    const li = document.createElement('li');
    const link = `${location.origin}/?g=${id}#k=${inv.key}`;
    const label = document.createElement('span');
    label.textContent = `${inv.name}: `;
    const input = document.createElement('input');
    input.readOnly = true;
    input.value = link;
    const copy = document.createElement('button');
    copy.type = 'button';
    copy.textContent = 'Copy';
    copy.addEventListener('click', async () => {
      try { await navigator.clipboard.writeText(link); copy.textContent = 'Copied'; } catch { input.select(); }
    });
    li.append(label, input, copy);
    return li;
  }));
}

const gameId = new URLSearchParams(location.search).get('g');
if (gameId) game(gameId); else home();
