// The game page. With no game in the address it shows "New game"; with ?g=ID it shows that game,
// live. The server runs every game and sends this page its seat's view (server/games.mjs); the page
// only draws it and sends moves as text. On your turn, the moves the server lists as legal light up.
//
// Your seat's key comes in the link's #k=... part (never sent to the server in the address), then is
// kept in this browser (localStorage) so a reload finds your seat again.
import { islandSvg, el, centre, point, hexPoints, RESOURCES, SIZE } from './board.js';

const $ = id => document.getElementById(id);
const COLOURS = ['Red', 'Blue', 'Orange', 'Purple'];
const PROJECTS = { pilot: 'Pilot', lighthouse: 'Lighthouse', roadCrew: 'Road Crew', harvest: 'Good Harvest', bargain: 'Good Bargain' };
const PROJECT_HELP = {
  pilot: 'Move the squall to any land hex, then steal',
  lighthouse: '1 Renown, counted at once',
  roadCrew: '2 free Paths',
  harvest: 'Take any 2 resources',
  bargain: 'This turn, trade 2:1 for one resource',
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
    if (Number(f.over.value) !== 7) settings['discard.over'] = Number(f.over.value);
    if (Number(f.rounds.value) !== 0) settings.rounds = Number(f.rounds.value);
    if (f.faceUp.checked) settings['projects.faceUp'] = true;
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

let id, key, board, snap, layers, sending = false;
// What the player picked in the Build shop ('path', 'hut', 'lodge'): only those spots light up
let mode = null;

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
  $('make-offer').addEventListener('click', () => {
    const to = deal.to.reduce((mask, s) => mask | (1 << s), 0);
    send(`offer ${deal.give.join(',')} ${deal.get.join(',')} ${to}${$('deal-open').checked ? ' open' : ''}`);
  });
  // A counter-offer is written from the offerer's side, as the engine keeps it: what they'd give
  // (counter.theirs, what you get) and get (counter.mine, what you give)
  $('send-counter').addEventListener('click', () => {
    if (counter) send(`counter ${snap.view.you} ${counter.theirs.join(',')} ${counter.mine.join(',')}`).then(() => { counter = null; render(snap); });
  });
  $('cancel-counter').addEventListener('click', () => { counter = null; render(snap); });
  // Esc puts away what was picked in the shop
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape' && mode) { mode = null; render(snap); }
  });
  for (const t of document.querySelectorAll('[role="tab"]')) t.addEventListener('click', () => showTab(t.dataset.tab));
  // A game you just started with friends opens on its invite links, once
  if (!$('tab-invite').hidden && !store.get(`settlers.shown.${id}`)) {
    store.set(`settlers.shown.${id}`, '1');
    showTab('invite', false);
  } else {
    showTab(store.get('settlers.tab') || 'build', false);
  }
}

// ---------- Tabs (Build, Trade, Projects, Log, Invite) ----------

function showTab(name, remember = true) {
  if ($(`tab-${name}`)?.hidden) name = 'build';
  for (const t of document.querySelectorAll('[role="tab"]')) {
    const on = t.dataset.tab === name;
    t.setAttribute('aria-selected', String(on));
    t.tabIndex = on ? 0 : -1;
    $(`panel-${t.dataset.tab}`).hidden = !on;
  }
  if (remember) store.set('settlers.tab', name);
}
const currentTab = () => document.querySelector('[role="tab"][aria-selected="true"]')?.dataset.tab;

function drawBoard() {
  const svg = islandSvg(board.island);
  layers = { roads: el('g'), buildings: el('g'), squall: el('g'), targets: el('g') };
  svg.append(layers.roads, layers.buildings, layers.squall, layers.targets);
  $('board').replaceChildren(svg);
}

const cornerAt = c => point(board.corners[c]);
const edgeEnds = e => board.edges[e].map(cornerAt);
const nameOf = s => snap.seats[s]?.name ?? `Seat ${s + 1}`;
// The server sends only this seat's moves: on your turn everything you can do, otherwise only your
// answers to a trade offer made to you
const legal = () => snap.view.legal;

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
    if (res.ok && /^(hut|lodge|path) /.test(move)) mode = null;
  } catch {
    $('move-error').textContent = 'Couldn\'t reach the server. Check your connection and try again.';
  } finally {
    sending = false;
  }
}

let previous = null;   // the last snapshot drawn, to show what changed since
function render(data) {
  const before = previous;
  snap = data;
  previous = data;
  version++;
  const v = snap.view;
  // A pick in the shop lasts while it can still be built
  if (mode && !legal().some(m => m.startsWith(`${mode} `))) mode = null;
  drawPieces(v, before?.view);
  drawTargets(v);
  const offered = v.offer && v.legal.some(m => m.startsWith('answer '));
  $('status').textContent = mode ? `Click a lit spot to build your ${SHOP.find(s => s.key === mode).name} (Esc to cancel).`
    : offered ? `${nameOf(v.offer.by)} offers you a trade: accept or decline it in the Trade tab.` : status(v);
  // An offer to you, new since last time: open the Trade tab so you see it
  if (offered && !(before?.view.offer && before.view.legal.some(m => m.startsWith('answer ')))) showTab('trade', false);
  $('trade-badge').hidden = !offered;
  drawDice(v);
  showRoll(v, before);
  showBought(v, before);
  drawChoices(v);
  drawPlayers(v);
  drawHand(v);
  drawShop(v);
  drawOffer(v);
  drawTrade(v);
  drawDeal(v);
  drawProjects(v);
  drawLog();
  $('trade-empty').hidden = !$('offer-box').hidden || !$('trade-box').hidden || !$('deal-box').hidden;
  document.body.dataset.phase = v.phase;
  document.body.dataset.myTurn = String(v.toAct === v.you && v.phase !== 'over');
}

// Pieces placed since the last update glow for a moment, so other players' moves are easy to follow
function drawPieces(v, before) {
  const had = new Set(before ? [...before.roads.map(r => `e${r[0]}:${r[2]}`), ...before.buildings.map(b => `c${b[0]}:${b[2]}`)] : []);
  const glow = k => (before && !had.has(k) ? ' new' : '');
  layers.roads.replaceChildren();
  for (const [e, p, kind] of v.roads) {
    const [a, b] = edgeEnds(e);
    layers.roads.append(el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: `road p${p}${glow(`e${e}:${kind}`)}`, 'data-edge': e }));
  }
  layers.buildings.replaceChildren();
  for (const [c, p, kind] of v.buildings) {
    const { x, y } = cornerAt(c);
    const piece = kind === 2
      ? el('rect', { x: x - 9, y: y - 9, width: 18, height: 18, rx: 2, class: `building lodge p${p}${glow(`c${c}:${kind}`)}`, 'data-corner': c })
      : el('circle', { cx: x, cy: y, r: 7.5, class: `building hut p${p}${glow(`c${c}:${kind}`)}`, 'data-corner': c });
    piece.append(el('title', {}, `${nameOf(p)}'s ${kind === 2 ? 'Lodge' : 'Hut'}`));
    layers.buildings.append(piece);
  }
  const sq = centre(board.island.hexes[v.squall]);
  layers.squall.replaceChildren(el('circle', { cx: sq.x, cy: sq.y, r: SIZE * 0.6, class: `squall${before && before.squall !== v.squall ? ' new' : ''}` }));
}

// The last roll as two dice faces (pips on a 3 × 3 grid)
const PIPS = { 1: [4], 2: [0, 8], 3: [0, 4, 8], 4: [0, 2, 6, 8], 5: [0, 2, 4, 6, 8], 6: [0, 2, 3, 5, 6, 8] };
function drawDice(v) {
  const box = $('dice');
  const parts = [];
  if (v.dice) {
    box.setAttribute('aria-label', `Last roll: ${v.dice[0]} and ${v.dice[1]}, ${v.dice[0] + v.dice[1]}`);
    parts.push(...v.dice.map(n => {
      const die = document.createElement('span');
      die.className = 'die';
      for (let i = 0; i < 9; i++) {
        const pip = document.createElement('span');
        if (PIPS[n].includes(i)) pip.className = 'pip';
        die.append(pip);
      }
      return die;
    }), Object.assign(document.createElement('span'), { className: 'total', textContent: v.dice[0] + v.dice[1] }));
  }
  // With a round limit, where the game stands ("15 rounds" during setup)
  if (v.roundLimit) {
    const text = v.round ? `Round ${Math.min(v.round, v.roundLimit)} of ${v.roundLimit}` : `${v.roundLimit} rounds`;
    parts.push(Object.assign(document.createElement('span'), { className: 'round', textContent: text }));
  }
  box.replaceChildren(...parts);
  box.hidden = parts.length === 0;
}

// Right after a roll: the hexes that produced flash, and you see what you got
function showRoll(v, before) {
  // Only for a roll that's new in this update (the same snapshot can be sent again, e.g. when
  // someone opens the page)
  const last = snap.log[snap.log.length - 1];
  const prevLast = before?.log[before.log.length - 1];
  const same = prevLast && JSON.stringify(prevLast) === JSON.stringify(last) && before.log.length === snap.log.length;
  if (!before || !last || last.move !== 'roll' || same) return;
  const total = last.roll;
  for (const t of document.querySelectorAll('.token.flash')) t.classList.remove('flash');
  if (total !== 7) {
    for (const t of document.querySelectorAll(`.token[data-number="${total}"]`)) {
      if (Number(t.dataset.token) === v.squall) continue;
      void t.getBoundingClientRect();   // restart the animation
      t.classList.add('flash');
    }
  }
  const who = last.seat === v.you ? 'You' : nameOf(last.seat);
  let text = `${who} rolled ${total}.`;
  if (total === 7) text += ' The squall moves.';
  else if (v.hand && before.view.hand) {
    const got = v.hand.map((n, r) => n - before.view.hand[r]);
    const parts = got.map((n, r) => (n > 0 ? `+${n} ${RESOURCES[r]}` : null)).filter(Boolean);
    text += parts.length ? ` You got ${parts.join(', ')}.` : ' Nothing for you this time.';
  }
  if (v.short?.length) {
    text += ` The bank ran out of ${v.short.map(r => RESOURCES[r]).join(' and ')}, so nobody got ${v.short.length > 1 ? 'those' : 'it'}.`;
  }
  toast(text);
}

let toastTimer = null;
function toast(text) {
  const t = $('toast');
  t.textContent = text;
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { t.hidden = true; }, 4000);
}

// Clickable spots for the legal moves that are places on the board
function drawTargets(v) {
  layers.targets.replaceChildren();
  // On your main turn, only what you picked in the shop lights up; in setup, Road Crew's free
  // Paths and squall moves, every spot does
  const moves = v.phase === 'main' ? legal().filter(m => mode && m.startsWith(`${mode} `)) : legal();
  for (const m of moves) {
    const [word, n] = m.split(' ');
    if (word === 'hut' || word === 'lodge') {
      const { x, y } = cornerAt(Number(n));
      // A Lodge spot is a ring big enough to surround the Hut it upgrades
      const t = el('circle', { cx: x, cy: y, r: word === 'lodge' ? 13 : 9, class: `target corner ${word}`, 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, word === 'hut' ? 'Build a Hut here' : 'Upgrade to a Lodge'));
      layers.targets.append(t);
    } else if (word === 'path') {
      const [a, b] = edgeEnds(Number(n));
      const t = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: 'target edge', 'data-move': m, 'data-edge-target': n, tabindex: 0 });
      t.append(el('title', {}, 'Build a Path here'));
      layers.targets.append(t);
    } else if (word === 'squall') {
      const h = board.island.hexes[Number(n)];
      const t = el('polygon', { points: hexPoints(h), class: 'target hex-target', 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, 'Move the squall here'));
      layers.targets.append(t);
    }
  }
  for (const t of layers.targets.querySelectorAll('[data-move]')) {
    t.addEventListener('click', () => send(t.dataset.move));
    t.addEventListener('keydown', ev => { if (ev.key === 'Enter' || ev.key === ' ') send(t.dataset.move); });
  }
}

function status(v) {
  if (v.phase === 'over') {
    const limit = v.byRounds ? `The ${v.roundLimit} rounds are up. ` : '';
    if (v.winner === null) return `${limit}It's a draw: equal Renown and Lodges at the top.`;
    return limit + (v.winner === v.you ? `You win with ${v.players[v.winner].renown} Renown!` : `${nameOf(v.winner)} wins with ${v.players[v.winner].renown} Renown.`);
  }
  const who = v.toAct === v.you ? 'You' : nameOf(v.toAct);
  const mine = v.toAct === v.you;
  switch (v.phase) {
    case 'setupHut': return mine ? 'Setup: place a Hut on a lit corner.' : `Setup: ${who} is placing a Hut.`;
    case 'setupPath': return mine ? 'Setup: place a Path next to your new Hut.' : `Setup: ${who} is placing a Path.`;
    case 'preRoll': return mine ? 'Your turn: roll the dice (or play a project first).' : `${who}'s turn.`;
    case 'discard': return mine ? `A 7! You hold too many cards: choose ${v.players[v.you].discard} more to lose.` : `A 7: ${who} is choosing cards to lose.`;
    case 'moveSquall': return mine ? 'Move the squall: click a lit hex.' : `${who} is moving the squall.`;
    case 'steal': return mine ? 'Choose who to steal a card from.' : `${who} is choosing who to steal from.`;
    case 'main': return mine ? `You rolled ${v.lastRoll}. Pick something to build, trade or buy, then end your turn.` : `${who} rolled ${v.lastRoll}.`;
    case 'freeBuild': return mine ? 'Road Crew: place a free Path.' : `${who} is placing free Paths.`;
    case 'harvest': return mine ? 'Good Harvest: take a resource.' : `${who} is taking resources.`;
    case 'bargain': return mine ? 'Good Bargain: choose the resource you\'ll trade 2:1.' : `${who} is choosing a bargain.`;
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

// Your projects, under your hand: each one, and whether you can play it now. Projects bought this
// turn wait for your next turn; a Lighthouse is never played, it counts at once.
function drawMyProjects(v) {
  const held = v.players[v.you].projects;
  const any = held && held.some(n => n > 0);
  $('my-projects').hidden = !any;
  if (!any) return;
  const keys = Object.keys(PROJECTS);
  const items = [];
  keys.forEach((k, i) => {
    for (let n = 0; n < held[i]; n++) {
      const li = document.createElement('li');
      li.className = `my-project ${k}`;
      const name = document.createElement('strong');
      name.textContent = PROJECTS[k];
      const when = document.createElement('span');
      const fresh = n < (v.bought?.[i] ?? 0);
      if (k === 'lighthouse') when.textContent = '1 Renown, counted';
      else if (v.legal.includes(`play ${k}`)) when.textContent = 'Play now';
      else if (fresh) when.textContent = 'Playable from your next turn';
      else when.textContent = 'Playable on your turn (one project a turn)';
      when.className = 'when';
      li.title = PROJECT_HELP[k];
      li.append(name, when);
      items.push(li);
    }
  });
  $('my-projects-list').replaceChildren(...items);
}

// Right after you buy a project: what you drew
function showBought(v, before) {
  if (!before || v.you === null) return;
  const now = v.players[v.you].projects, was = before.view.players[v.you]?.projects;
  if (!now || !was) return;
  const last = snap.log[snap.log.length - 1];
  if (!last || last.seat !== v.you || !last.move.startsWith('buy ')) return;
  const i = now.findIndex((n, r) => n > was[r]);
  if (i < 0) return;
  const k = Object.keys(PROJECTS)[i];
  toast(k === 'lighthouse'
    ? 'You drew a Lighthouse: +1 Renown, counted now.'
    : `You drew a ${PROJECTS[k]}: ${PROJECT_HELP[k].toLowerCase()}. Playable from your next turn.`);
}

// ---------- The Build shop ----------

const SHOP = [
  { key: 'path', name: 'Path', cost: [1, 0, 0, 1, 0], piece: 0, note: 'Joins your buildings' },
  { key: 'hut', name: 'Hut', cost: [1, 0, 1, 1, 1], piece: 1, note: '1 Renown' },
  { key: 'lodge', name: 'Lodge', cost: [0, 3, 2, 0, 0], piece: 2, note: '2 Renown, upgrades a Hut' },
];

// Why an item can't be built now, or null if it can
function shopReason(v, item) {
  if (v.you === null) return 'Watching';
  if (legal().some(m => m.startsWith(`${item.key} `))) return null;
  if (v.toAct !== v.you || v.phase === 'over') return 'Not your turn';
  if (v.phase === 'preRoll') return 'After you roll';
  if (v.phase !== 'main') return 'Not now';
  if (v.players[v.you].pieces[item.piece] === 0) return 'None left';
  const short = item.cost.map((n, r) => Math.max(0, n - v.hand[r]));
  if (short.some(n => n)) return `Need ${short.map((n, r) => (n ? `${n} ${RESOURCES[r]}` : null)).filter(Boolean).join(', ')}`;
  return item.key === 'lodge' ? 'No Hut to upgrade' : 'No spot you can reach';
}

function drawShop(v) {
  if (v.you === null) {
    const li = document.createElement('li');
    li.className = 'note';
    li.textContent = 'You\'re watching this game.';
    $('shop').replaceChildren(li);
    return;
  }
  const pieces = SHOP.map(item => {
    const reason = shopReason(v, item);
    const b = document.createElement('button');
    b.type = 'button';
    b.dataset.build = item.key;
    if (mode === item.key) {
      b.textContent = 'Cancel';
      b.addEventListener('click', () => { mode = null; render(snap); });
    } else {
      b.textContent = 'Build';
      b.disabled = reason !== null;
      b.addEventListener('click', () => { mode = item.key; render(snap); });
    }
    return shopRow(v, { key: item.key, name: item.name, note: item.note, cost: item.cost, picked: mode === item.key }, b, reason);
  });
  $('shop').replaceChildren(...pieces, ...projectItems(v));
}

// One row of the shop: name, note, cost chips (greyed where you're short), its button, and why not
function shopRow(v, item, button, reason) {
  const li = document.createElement('li');
  li.className = `shop-item${item.picked ? ' picked' : ''}${item.project ? ' project-item' : ''}`;
  li.dataset.item = item.key;
  const name = document.createElement('strong');
  name.textContent = item.name;
  const note = document.createElement('span');
  note.className = 'note';
  note.textContent = item.note;
  const cost = document.createElement('span');
  cost.className = 'cost-chips';
  item.cost.forEach((n, r) => {
    for (let i = 0; i < n; i++) {
      const chip = document.createElement('span');
      chip.className = `chip r${r}${(v.hand?.[r] ?? 0) > i ? '' : ' short'}`;
      chip.textContent = RESOURCES[r];
      cost.append(chip);
    }
  });
  const why = document.createElement('span');
  why.className = 'why';
  why.textContent = reason && v.toAct === v.you && v.phase === 'main' ? reason : '';
  li.append(name, note, cost, button, why);
  return li;
}

// Project cards in the shop: hidden (the default), one card drawn unseen; face up, each card showing
const PROJECT_COST = [0, 1, 1, 0, 1];   // Stone + Grain + Fish (engine: PROJECT_COST)
function projectItems(v) {
  const cards = v.row.length
    ? v.row.map((k, slot) => ({ key: `project-${slot}`, move: `buy ${slot}`, name: PROJECTS[k], note: PROJECT_HELP[k] }))
    : [{
        key: 'project', move: 'buy 0', name: 'Project card',
        note: `Drawn unseen: Pilot, Lighthouse, Road Crew, Good Harvest or Good Bargain · ${v.deck} left`,
      }];
  return cards.map(card => {
    const reason = projectReason(v, card.move);
    const b = button('Buy', card.move);
    b.dataset.buy = card.move;
    b.disabled = reason !== null;
    return shopRow(v, { ...card, cost: PROJECT_COST, project: true }, b, reason);
  });
}

function projectReason(v, move) {
  if (legal().includes(move)) return null;
  if (v.toAct !== v.you || v.phase === 'over') return 'Not your turn';
  if (v.phase === 'preRoll') return 'After you roll';
  if (v.phase !== 'main') return 'Not now';
  if (!v.deck && !v.row.length) return 'None left';
  const short = PROJECT_COST.map((n, r) => Math.max(0, n - v.hand[r]));
  if (short.some(n => n)) return `Need ${short.map((n, r) => (n ? `${n} ${RESOURCES[r]}` : null)).filter(Boolean).join(', ')}`;
  return 'Not now';
}

// ---------- Trading with players ----------

const hasCards = h => h.some(n => n > 0);
const cardsText = h => h.map((n, r) => (n ? `${n} ${RESOURCES[r]}` : null)).filter(Boolean).join(' + ');

// +/− steppers for the five resources: obj[side][r] counts, at most max(r); raising one clears the
// same resource on the other side (a resource is on one side of a trade only)
function steppers(box, obj, side, other, max, redraw) {
  box.replaceChildren(...RESOURCES.map((name, r) => {
    const row = document.createElement('span');
    row.className = `stepper card r${r}`;
    const less = document.createElement('button');
    less.type = 'button';
    less.textContent = '−';
    less.setAttribute('aria-label', `One less ${name}`);
    less.disabled = obj[side][r] === 0;
    less.addEventListener('click', () => { obj[side][r]--; redraw(); });
    const count = document.createElement('span');
    count.textContent = `${name} ${obj[side][r]}`;
    const more = document.createElement('button');
    more.type = 'button';
    more.textContent = '+';
    more.setAttribute('aria-label', `One more ${name}`);
    more.disabled = obj[side][r] >= max(r);
    more.addEventListener('click', () => { obj[side][r]++; obj[other][r] = 0; redraw(); });
    row.append(less, count, more);
    return row;
  }));
}

// The counter-offer being written by this player (null: none). mine = what I give, theirs = what
// the offerer gives me.
let counter = null;

// The open offer: what it is, everyone's answers and counter-offers, and your buttons (answer,
// counter, or as the offerer: trade with someone who accepted, take a counter, withdraw)
function drawOffer(v) {
  const o = v.offer;
  $('offer-box').hidden = !o;
  if (!o) { counter = null; $('counter-box').hidden = true; return; }
  const by = o.by === v.you ? 'You offer' : `${nameOf(o.by)} offers`;
  const to = o.to.map(s => (s === v.you ? 'you' : nameOf(s))).join(', ');
  $('offer-text').textContent = `${by} ${cardsText(o.give)} for ${cardsText(o.get)}, to ${to}.${o.open ? ' Open offer: the first to accept gets it.' : ''}`;
  $('offer-answers').replaceChildren(...o.to.map(s => {
    const li = document.createElement('li');
    li.className = `p${s}`;
    const who = s === v.you ? 'You' : nameOf(s);
    const c = o.counters[s];
    if (o.answers[s] === 'counter' && c) {
      // Said from the reader's side: the offerer reads "you give", the others read names
      li.textContent = o.by === v.you
        ? `${who} counters: you give ${cardsText(c.give)} for ${cardsText(c.get)}`
        : `${who} countered: ${cardsText(c.get)} for ${nameOf(o.by)}'s ${cardsText(c.give)}`;
    } else {
      li.textContent = `${who}: ${o.answers[s] === true ? 'accepted' : o.answers[s] === false ? 'declined' : 'thinking…'}`;
    }
    return li;
  }));
  const box = $('offer-buttons');
  box.replaceChildren();
  let canCounter = false;
  for (const m of v.legal) {
    const [word, a, b] = m.split(' ');
    if (word === 'answer' && b === 'yes') box.append(button(o.open ? 'Accept (trades now)' : 'Accept', m, 'primary'));
    if (word === 'answer' && b === 'no') { box.append(button('Decline', m)); canCounter = true; }
    if (word === 'deal') box.append(button(`Trade with ${nameOf(Number(a))}`, m, 'primary'));
    if (word === 'accept') box.append(button(`Take ${nameOf(Number(a))}'s counter`, m, 'primary'));
    if (word === 'withdraw') box.append(button('Withdraw offer', m));
  }
  // A counter-offer: start from the offer as it stands, seen from your side
  if (canCounter && !counter) {
    const c = document.createElement('button');
    c.type = 'button';
    c.textContent = 'Counter…';
    c.addEventListener('click', () => { counter = { mine: [...o.get], theirs: [...o.give] }; render(snap); });
    box.append(c);
  }
  $('counter-box').hidden = !counter;
  if (counter) {
    counter.mine = counter.mine.map((n, r) => Math.min(n, v.hand[r]));
    const redraw = () => drawOffer(snap.view);
    steppers($('counter-mine'), counter, 'mine', 'theirs', r => v.hand[r], redraw);
    steppers($('counter-theirs'), counter, 'theirs', 'mine', () => 9, redraw);
    $('send-counter').disabled = !hasCards(counter.mine) || !hasCards(counter.theirs);
  }
}

// Building an offer: steppers for what you give (up to your hand) and get, and who it's to
const deal = { give: [0, 0, 0, 0, 0], get: [0, 0, 0, 0, 0], to: [] };
function drawDeal(v) {
  const show = v.canOffer && !v.offer;
  $('deal-box').hidden = !show;
  if (!show) return;
  $('offers-left').textContent = `${v.offersLeft} offer${v.offersLeft === 1 ? '' : 's'} left this turn`;
  deal.give = deal.give.map((n, r) => Math.min(n, v.hand[r]));
  const others = v.players.map((_, s) => s).filter(s => s !== v.you);
  deal.to = deal.to.filter(s => others.includes(s));
  if (!deal.to.length) deal.to = [...others];
  steppers($('deal-give'), deal, 'give', 'get', r => v.hand[r], () => drawDeal(snap.view));
  steppers($('deal-get'), deal, 'get', 'give', () => 9, () => drawDeal(snap.view));
  $('deal-to').replaceChildren(...others.map(s => {
    const b = document.createElement('button');
    b.type = 'button';
    b.textContent = nameOf(s);
    b.className = `p${s}`;
    b.setAttribute('aria-pressed', String(deal.to.includes(s)));
    b.addEventListener('click', () => {
      deal.to = deal.to.includes(s) ? deal.to.filter(x => x !== s) : [...deal.to, s];
      drawDeal(snap.view);
    });
    return b;
  }));
  $('make-offer').disabled = !hasCards(deal.give) || !hasCards(deal.get) || !deal.to.length;
}

// Players as compact cards: name, a Renown bar toward the win, cards held, titles as badges
function drawPlayers(v) {
  $('players').replaceChildren(...v.players.map((p, s) => {
    const seat = snap.seats[s];
    const li = document.createElement('li');
    li.className = `player p${s}${v.toAct === s && v.phase !== 'over' ? ' acting' : ''}${v.winner === s ? ' winner' : ''}`;
    li.title = COLOURS[s];
    const top = document.createElement('div');
    top.className = 'player-top';
    const dot = document.createElement('span');
    dot.className = `dot p${s}`;
    const name = document.createElement('strong');
    name.textContent = `${seat.name}${s === v.you ? ' (you)' : ''}`;
    const tag = document.createElement('span');
    tag.className = 'tag';
    tag.textContent = seat.kind === 'bot' ? 'bot' : seat.online || s === v.you ? '' : 'away';
    const renown = document.createElement('span');
    renown.className = 'renown';
    renown.textContent = `${p.renown}/${v.win}`;
    top.append(dot, name, tag, renown);
    // The bar's width is set through the style object, which the security policy allows
    const bar = document.createElement('div');
    bar.className = 'bar';
    const fill = document.createElement('div');
    fill.className = `fill p${s}`;
    fill.style.width = `${Math.min(100, (100 * p.renown) / v.win)}%`;
    bar.append(fill);
    const info = document.createElement('div');
    info.className = 'info';
    const facts = [`${p.cards} card${p.cards === 1 ? '' : 's'}`];
    if (p.projectCount) facts.push(`${p.projectCount} project${p.projectCount === 1 ? '' : 's'}`);
    info.append(facts.join(' · '));
    for (const [holder, title] of [[v.longestPath, 'Longest Path'], [v.mostMarkets, 'Most Markets']]) {
      if (holder !== s) continue;
      const badge = document.createElement('span');
      badge.className = 'title-badge';
      badge.textContent = title;
      info.append(' ', badge);
    }
    li.append(top, bar, info);
    return li;
  }));
}

// Your hand as cards (count big, name small), and what you could build with it right now
function drawHand(v) {
  $('hand-box').hidden = v.hand === null;
  if (v.hand === null) return;
  $('hand').replaceChildren(...RESOURCES.map((r, i) => {
    const li = document.createElement('li');
    li.className = `card r${i}${v.hand[i] ? '' : ' none'}`;
    li.dataset.count = v.hand[i];
    li.dataset.resource = r;
    const n = document.createElement('b');
    n.textContent = v.hand[i];
    const label = document.createElement('span');
    label.textContent = r;
    li.append(n, label);
    return li;
  }));
  const affordable = SHOP.filter(item => item.cost.every((n, r) => v.hand[r] >= n)).map(item => item.name);
  if (v.row.length || v.deck) {
    if (PROJECT_COST.every((n, r) => v.hand[r] >= n)) affordable.push('a project');
  }
  $('can-build').textContent = affordable.length ? `Enough for: ${affordable.join(', ')}` : 'Not enough for anything yet.';
  drawMyProjects(v);
  const held = v.hand.reduce((a, b) => a + b, 0);
  $('hand-limit').textContent = `On a 7, holding more than ${v.handLimit} cards loses half${held > v.handLimit ? ` (you hold ${held})` : ''}.`;
  $('hand-limit').classList.toggle('warn', held > v.handLimit);
}

// Trading: pick what to give (only what you can afford, at your rate) and what to get
const trade = { give: null, get: null };
function drawTrade(v) {
  // What's left in the bank (a game has 20 of each resource; null: it never runs out)
  $('bank-left').textContent = v.bank ? `Left in the bank: ${v.bank.map((n, r) => `${RESOURCES[r]} ${n}`).join(' · ')}` : '';
  $('bank-left').hidden = !v.bank;
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
    case 'roll': return `${who} rolled ${entry.roll}`;
    case 'discard': return `${who} lost a ${RESOURCES[a]}`;
    case 'squall': return `${who} moved the squall`;
    case 'steal': return `${who} stole a card from ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'trade': return `${who} traded ${RESOURCES[a]} for ${RESOURCES[b]}`;
    case 'buy': return `${who} bought a project`;
    case 'play': return `${who} played ${PROJECTS[a]}`;
    case 'take': return `${who} took ${RESOURCES[a]}`;
    case 'end': return `${who} ended the turn`;
    case 'offer': return `${who} offered ${cardsText(a.split(',').map(Number))} for ${cardsText(b.split(',').map(Number))}`;
    case 'answer': return `${Number(a) === snap.view.you ? 'You' : nameOf(Number(a))} ${b === 'yes' ? 'accepted' : 'declined'} the offer`;
    case 'deal': return `${who} traded with ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'counter': return `${Number(a) === snap.view.you ? 'You' : nameOf(Number(a))} made a counter-offer`;
    case 'accept': return `${who} took ${Number(a) === snap.view.you ? 'your' : `${nameOf(Number(a))}'s`} counter-offer`;
    case 'withdraw': return `${who} withdrew the offer`;
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
  $('tab-invite').hidden = invites.length === 0;
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
