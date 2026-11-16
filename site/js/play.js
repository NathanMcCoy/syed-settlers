// The game page. With no game in the address it shows "New game"; with ?g=ID it shows that game,
// live. The server runs every game and sends this page its seat's view (server/games.mjs); the page
// only draws it and sends moves as text. On your turn, the moves the server lists as legal light up.
//
// Your seat's key comes in the link's #k=... part (never sent to the server in the address), then is
// kept in this browser (localStorage) so a reload finds your seat again.
import { islandSvg, el, centre, point, hexPoints, pips, terrainName, marketTag, TERRAIN_RESOURCE, RESOURCES, SIZE } from './board.js';

const $ = id => document.getElementById(id);
const COLOURS = ['Red', 'Blue', 'Orange', 'Purple', 'Green', 'Brown'];
// Titles and tokens by the names the engine uses (titles.rs, tokens.rs); unknown ones show their name
const TITLE_NAMES = { longestPath: 'Longest Path', mostPilots: 'Most Pilots', strongestPorts: 'Strongest Ports' };
const TOKEN_NAMES = { renown: ['Renown token', 'Renown tokens'], trade: ['trade token', 'trade tokens'], cloth: ['cloth', 'cloth'] };
// Event cards (the variants box)
const EVENT_NAMES = {
  beautifulDay: ['A beautiful day', 'Nothing happens.'],
  squallAttacks: ['The squall attacks', 'A 7: big hands lose half, the squall moves.'],
  squallFlees: ['The squall flees', 'It goes to the Dunes.'],
  conflict: ['Conflict', 'The player with Most Pilots (or the most played) robs anyone.'],
  tradeAdvantage: ['Trade advantage', 'Longest Path robs anyone.'],
  calmSeas: ['Calm seas', 'The most buildings on markets: a resource of their choice.'],
  tournament: ['Tournament', 'The most Pilots played: a resource of their choice.'],
  plentifulYear: ['A plentiful year', 'Everyone takes a resource of their choice.'],
  goodNeighbours: ['Good neighbours', 'Everyone gives a card to the player on their left.'],
  helpfulNeighbour: ['A helpful neighbour', 'The leader gives a card to someone behind.'],
  epidemic: ['Epidemic', 'Lodges produce 1, not 2.'],
  earthquake: ['Earthquake', 'Everyone damages one of their Paths: repair it before building Paths.'],
  newYear: ['New Year', 'The deck is shuffled again.'],
};
const PROJECTS = { pilot: 'Pilot', lighthouse: 'Lighthouse', roadCrew: 'Road Crew', harvest: 'Good Harvest', marketDay: 'Market Day' };
const PROJECT_HELP = {
  pilot: 'Move the squall to any land hex, then steal',
  lighthouse: '1 Renown, counted at once',
  roadCrew: '2 free Paths',
  harvest: 'Take any 2 resources',
  marketDay: 'Name a resource: everyone else gives you all of theirs',
};

// The city expansion (city.rs): commodities, tracks, progress cards [name, what it does]
const COMMODITIES = ['Paper', 'Cloth', 'Coin'];
const CARD_NAMES = [...RESOURCES, ...COMMODITIES];
const TRACKS = [
  ['Science', 'Paper', 'a resource of your choice when a roll gives you nothing (not a 7)'],
  ['Trade', 'Cloth', 'commodities 2:1 with the bank'],
  ['Politics', 'Coin', 'promote strong knights to mighty'],
];
const DECKS = ['Science', 'Trade', 'Politics'];
const KNIGHT_NAMES = ['', 'basic', 'strong', 'mighty'];
const PROGRESS = {
  fixedRoll: ['Fixed Roll', 'Before rolling: choose the production dice'],
  crane: ['Crane', 'Your next improvement costs 1 commodity less'],
  engineer: ['Engineer', 'A wall for free'],
  numberSwap: ['Number Swap', 'Swap two number discs (not 2, 6, 8 or 12)'],
  irrigation: ['Irrigation', '2 Grain for each Fields hex by your buildings'],
  medicine: ['Medicine', 'Your next Lodge this turn costs 2 Stone + 1 Grain'],
  quarrying: ['Quarrying', '2 Stone for each Quarry hex by your buildings'],
  printingPress: ['Printing Press', '1 Renown'],
  pathBuilders: ['Path Builders', '2 free Paths'],
  smithing: ['Smithing', 'Promote up to 2 knights for free'],
  harbourDeal: ['Harbour Deal', 'Give each player with commodities a resource; they give you a commodity'],
  guildDues: ['Guild Dues', 'Take 2 cards of your choice from a player with more Renown'],
  merchant: ['Merchant', 'Place the merchant by your building: 1 Renown, its resource 2:1'],
  tradeFleet: ['Trade Fleet', 'Name a card kind: 2:1 with the bank this turn'],
  monopoly: ['Monopoly', 'Name a resource: everyone gives you 2 (or their 1)'],
  goodsMonopoly: ['Goods Monopoly', 'Name a commodity: everyone gives you 1'],
  charter: ['Charter', '1 Renown'],
  diplomacy: ['Diplomacy', 'Remove an open Path (your own: build one free)'],
  rally: ['Rally', 'Activate all your knights for free'],
  spy: ['Spy', "Look at a player's progress cards and take one"],
  intrigue: ['Intrigue', "Push off another player's knight touching your routes"],
  taxes: ['Taxes', 'Move the squall and take a card from everyone there'],
  treason: ['Treason', 'A player removes a knight; you may place one of yours no stronger'],
  wedding: ['Wedding', 'Everyone with more Renown gives you 2 cards of their choice'],
  sabotage: ['Sabotage', 'Everyone with as much Renown or more discards half'],
};
const cardName = k => PROGRESS[k]?.[0] ?? k;
// A card of yours by kind: resources from the hand, commodities from the city part of the view
const held = (v, k) => (k < 5 ? v.hand[k] : v.city?.goods?.[k - 5] ?? 0);
const rateOf = (v, k) => (k < 5 ? v.rates[k] : v.city.goodsRates[k - 5]);
// A game with the sea (ships), whether or not the pirate is on the board yet
const seaGame = () => /(^|;)sea=true/.test(board.settings);

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
  // Up to 5 other seats (6 players); the 5th and 6th start as Nobody
  for (let i = 1; i <= 5; i++) {
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
    for (let i = 1; i <= 5; i++) {
      const kind = kinds[i];
      if (kind) seats.push({ kind, name: kind === 'person' ? f[`name${i}`].value.trim() : undefined });
    }
    const settings = {};
    if (Number(f.win.value) !== 10) settings.win = Number(f.win.value);
    if (Number(f.over.value) !== 7) settings['discard.over'] = Number(f.over.value);
    if (Number(f.rounds.value) !== 0) settings.rounds = Number(f.rounds.value);
    if (f.faceUp.checked) settings['projects.faceUp'] = true;
    const box = new FormData(e.target).get('box');
    if (box && box !== '0') settings['box.scenario'] = Number(box);
    const sea = new FormData(e.target).get('sea');
    if (sea && sea !== '0') {
      settings.sea = true;
      settings['sea.scenario'] = Number(sea);
    }
    for (const [box, name] of [['friendly', 'variant.friendly'], ['ports', 'variant.ports'], ['events', 'variant.events'], ['two', 'variant.two'], ['city', 'city']]) {
      if (f[box].checked) settings[name] = true;
    }
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
// Moving a ship (the sea expansion): the edge of the ship picked to move, before picking where it goes
let moveFrom = null;

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
    send(`offer ${countsText(deal.give)} ${countsText(deal.get)} ${to}${$('deal-open').checked ? ' open' : ''}`);
  });
  // A counter-offer is written from the offerer's side, as the engine keeps it: what they'd give
  // (counter.theirs, what you get) and get (counter.mine, what you give)
  $('send-counter').addEventListener('click', () => {
    if (counter) send(`counter ${snap.view.you} ${countsText(counter.theirs)} ${countsText(counter.mine)}`).then(() => { counter = null; render(snap); });
  });
  $('cancel-counter').addEventListener('click', () => { counter = null; render(snap); });
  // Esc puts away what was picked in the shop
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape' && mode) { mode = null; moveFrom = null; render(snap); }
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
  // Rivers (the variants box): each river from its mouth through its 3 hexes, and its bridge sites
  const rivers = board.island.rivers ?? [];
  for (let r = 0; r * 3 < rivers.length; r++) {
    const [a, b] = edgeEnds(board.island.bridgeSites[3 * r]);
    const pts = [{ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }, ...rivers.slice(3 * r, 3 * r + 3).map(h => centre(board.island.hexes[h]))];
    const line = el('polyline', { points: pts.map(p => `${p.x},${p.y}`).join(' '), class: 'river', 'data-river': r });
    line.append(el('title', {}, 'A river: building on its hexes earns coins; cross it only by a bridge'));
    svg.append(line);
  }
  // With no 2 of its own, the 12's hex pays on a 2 too
  if (rivers.length && !board.island.hexes.some(h => h.number === 2)) {
    board.island.hexes.forEach(h => {
      if (h.number !== 12) return;
      const c = centre(h);
      svg.append(el('text', { x: c.x + SIZE * 0.5, y: c.y - SIZE * 0.35, class: 'marker', 'data-shares': 2 }, '+2'));
    });
  }
  for (const e of board.island.bridgeSites ?? []) {
    const [a, b] = edgeEnds(e);
    const site = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: 'bridge-site', 'data-bridge-site': e });
    site.append(el('title', {}, 'A bridge site: only a bridge goes here (Timber + 2 Copper, 3 coins)'));
    svg.append(site);
  }
  layers = { roads: el('g'), buildings: el('g'), squall: el('g'), targets: el('g') };
  svg.append(layers.roads, layers.buildings, layers.squall, layers.targets);
  $('board').replaceChildren(svg);
}

const cornerAt = c => point(board.corners[c]);
const edgeEnds = e => board.edges[e].map(cornerAt);
// (Owners past the seats are the two-player variant's neutral colours)
const nameOf = s => snap.seats[s]?.name ?? (s >= snap.seats.length ? 'Neutral' : `Seat ${s + 1}`);
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
    if (res.ok && /^(hut|lodge|path|ship|moveship|wall|recruit|promote|activate|knightmove|displace|chase) /.test(move)) { mode = null; moveFrom = null; }
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
  if (mode && !legal().some(m => m.startsWith(`${mode} `))) { mode = null; moveFrom = null; }
  applyRevealed(v);
  drawPieces(v, before?.view);
  drawTargets(v);
  const offered = v.offer && v.legal.some(m => m.startsWith('answer '));
  const picking = mode === 'moveship'
    ? (moveFrom === null ? 'Click a lit ship to move it (Esc to cancel).' : 'Click where the ship goes (Esc to cancel).')
    : mode && `Click a lit spot to build your ${SHOP.find(s => s.key === mode).name} (Esc to cancel).`;
  $('status').textContent = mode ? picking
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
  drawCity(v);
  drawCoins(v);
  drawLog();
  $('trade-empty').hidden = !$('offer-box').hidden || !$('trade-box').hidden || !$('deal-box').hidden || !$('coin-box').hidden;
  document.body.dataset.phase = v.phase;
  document.body.dataset.myTurn = String(v.toAct === v.you && v.phase !== 'over');
}

// Fog Islands: hexes revealed since the island was drawn get their terrain and number
function applyRevealed(v) {
  for (const [h, terrain, number] of v.revealed ?? []) {
    const hex = board.island.hexes[h];
    if (hex.terrain === terrain) continue;
    hex.terrain = terrain;
    hex.number = number;
    const poly = document.querySelector(`#board polygon[data-hex="${h}"]`);
    if (!poly) continue;
    poly.setAttribute('class', `hex t-${terrain} revealed`);
    poly.dataset.terrain = terrain;
    const resource = TERRAIN_RESOURCE[terrain];
    poly.querySelector('title').textContent = terrain === 'sea' ? 'Sea' : `${terrainName(terrain)}${resource ? ` (${resource})` : ''}${number ? `, ${number}` : ''}`;
    if (number) {
      const c = centre(hex);
      poly.after(
        el('circle', { cx: c.x, cy: c.y, r: SIZE * 0.42, class: 'token', 'data-number': number, 'data-token': h }),
        el('text', { x: c.x, y: c.y - 2, class: `number${pips(number) === 5 ? ' strong' : ''}` }, number),
        el('text', { x: c.x, y: c.y + 12, class: 'pips' }, '•'.repeat(pips(number))),
      );
    }
  }
}

// Pieces placed since the last update glow for a moment, so other players' moves are easy to follow
function drawPieces(v, before) {
  const had = new Set(before ? [...before.roads.map(r => `e${r[0]}:${r[2]}`), ...before.buildings.map(b => `c${b[0]}:${b[2]}`)] : []);
  const glow = k => (before && !had.has(k) ? ' new' : '');
  layers.roads.replaceChildren();
  for (const [e, p, kind] of v.roads) {
    const [a, b] = edgeEnds(e);
    const broken = (v.damaged ?? []).includes(e) ? ' damaged' : '';
    const bridge = (v.rivers?.bridges ?? []).includes(e);
    const line = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: `road${kind === 2 ? ' ship' : ''}${bridge ? ' bridge' : ''}${broken} p${p}${glow(`e${e}:${kind}`)}`, 'data-edge': e });
    line.append(el('title', {}, `${nameOf(p)}'s ${bridge ? 'bridge' : kind === 2 ? 'ship' : 'Path'}`));
    layers.roads.append(line);
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
  drawCityPieces(v);
  layers.squall.replaceChildren();
  // (The Pirate Islands has no squall)
  if (board.island.hexes[v.squall]) {
    const sq = centre(board.island.hexes[v.squall]);
    layers.squall.append(el('circle', { cx: sq.x, cy: sq.y, r: SIZE * 0.6, class: `squall${before && before.squall !== v.squall ? ' new' : ''}` }));
  }
  // The Pirate Islands: the fleet, each fortress (lairs left) and beachhead, and warships
  if (v.fleet !== null && v.fleet !== undefined) {
    const fc = centre(board.island.hexes[v.fleet]);
    const fleet = el('rect', { x: fc.x - 11, y: fc.y - 11, width: 22, height: 22, rx: 4, class: `fleet${before && before.fleet !== v.fleet ? ' new' : ''}`, 'data-fleet': v.fleet });
    fleet.append(el('title', {}, 'The pirate fleet: it sails by the lower die and attacks the players with buildings on its hex'));
    layers.squall.append(fleet);
  }
  (v.forts ?? []).forEach(([fort, lairs, taken, beach], s) => {
    const f = cornerAt(fort);
    if (!taken) {
      const mark = el('text', { x: f.x, y: f.y - 10, class: `marker fort p${s}` }, `☠${lairs}`);
      mark.append(el('title', {}, `${nameOf(s)}'s pirate fortress: ${lairs} lair${lairs === 1 ? '' : 's'} left`));
      layers.squall.append(mark);
    }
    const b = cornerAt(beach);
    const flag = el('text', { x: b.x + 9, y: b.y - 6, class: `marker beach p${s}` }, '⚑');
    flag.append(el('title', {}, `${nameOf(s)}'s beachhead: a Hut may go here once their line reaches it`));
    layers.squall.append(flag);
  });
  for (const e of v.warships ?? []) {
    const line = layers.roads.querySelector(`[data-edge="${e}"]`);
    if (line) line.classList.add('warship');
  }
  // Wonders: the wall (W), bridge (B) and no-setup (×) markers on their corners
  for (const [c, kind] of v.markers ?? []) {
    const { x, y } = cornerAt(c);
    const mark = el('text', { x: x + 9, y: y - 6, class: 'marker' }, ['', 'W', 'B', '×'][kind]);
    mark.append(el('title', {}, ['', 'Wall marker (the Great Wall)', 'Bridge marker (the Great Bridge)', 'No setup Hut here'][kind]));
    layers.squall.append(mark);
  }
  // The Forgotten Tribe: prizes on sea edges (R a Renown token, P a project, M a market), and markets
  // placed during the game
  for (const [e, kind] of v.prizes ?? []) {
    const [a, b] = edgeEnds(e);
    const label = kind === 1 ? 'R' : kind === 2 ? 'P' : 'M';
    const mark = el('text', { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 + 4, class: 'marker prize', 'data-prize': e }, label);
    mark.append(el('title', {}, kind === 1 ? 'A Renown token for the first ship here' : kind === 2 ? 'A project for the first ship here' : 'A market for the first ship here'));
    layers.squall.append(mark);
  }
  for (const [hex, side, kind] of (v.markets ?? []).slice(board.island.markets.length)) {
    layers.squall.append(...marketTag(board.island, { hex, side, kind }));
  }
  // Cloth: what each village has left, and who trades with it
  for (const [h, cloth, linked] of v.villages ?? []) {
    const c = centre(board.island.hexes[h]);
    const label = el('text', { x: c.x, y: c.y + SIZE * 0.72, class: 'village-cloth', 'data-village': h }, `${cloth} cloth`);
    label.append(el('title', {}, linked.length ? `Trades with ${linked.map(nameOf).join(', ')}` : 'Nobody trades with this village yet'));
    layers.squall.append(label);
  }
  // The sea expansion: the pirate
  if (v.pirate !== null && v.pirate !== undefined) {
    const pc = centre(board.island.hexes[v.pirate]);
    const pirate = el('circle', { cx: pc.x, cy: pc.y, r: SIZE * 0.45, class: `pirate${before && before.pirate !== v.pirate ? ' new' : ''}`, 'data-pirate': v.pirate });
    pirate.append(el('title', {}, 'The pirate: no ships may be built or moved on this hex\'s edges'));
    layers.squall.append(pirate);
  }
}

// The city expansion on the board: walls round Lodges, metropolises (★), knights (their level, filled
// when active) and the merchant
function drawCityPieces(v) {
  if (!v.city) return;
  for (const c of v.city.walls) {
    const { x, y } = cornerAt(c);
    const owner = v.buildings.find(b => b[0] === c)?.[1] ?? 0;
    const wall = el('rect', { x: x - 13, y: y - 13, width: 26, height: 26, rx: 3, class: `wall p${owner}`, 'data-wall': c });
    wall.append(el('title', {}, `${nameOf(owner)}'s wall: +2 to their hand limit`));
    layers.buildings.prepend(wall);
  }
  v.city.metro.forEach((m, t) => {
    if (!m) return;
    const { x, y } = cornerAt(m[0]);
    const star = el('text', { x, y: y + 4, class: 'metro', 'data-metro': t }, '★');
    star.append(el('title', {}, `${nameOf(m[1])}'s ${TRACKS[t][0]} metropolis: +2 Renown, can't be pillaged`));
    layers.buildings.append(star);
  });
  for (const [c, p, level, active, fresh] of v.city.knights) {
    const { x, y } = cornerAt(c);
    const g = el('g', { class: `knight p${p}${active ? ' active' : ''}`, 'data-knight': c });
    g.append(el('circle', { cx: x, cy: y, r: 8 }), el('text', { x, y: y + 4 }, String(level)));
    g.append(el('title', {}, `${nameOf(p)}'s ${KNIGHT_NAMES[level]} knight (${active ? (fresh ? 'active from next turn' : 'active') : 'inactive'})`));
    layers.buildings.append(g);
  }
  if (v.city.merchant) {
    const [h, owner] = v.city.merchant;
    const c = centre(board.island.hexes[h]);
    const m = el('text', { x: c.x + SIZE * 0.42, y: c.y - SIZE * 0.3, class: `marker merchant p${owner}`, 'data-merchant': h }, 'M');
    m.append(el('title', {}, `${nameOf(owner)}'s merchant: 1 Renown, this hex's resource 2:1 for them`));
    layers.squall.append(m);
  }
}

// The last roll as two dice faces (pips on a 3 × 3 grid)
const PIPS = { 1: [4], 2: [0, 8], 3: [0, 4, 8], 4: [0, 2, 6, 8], 5: [0, 2, 4, 6, 8], 6: [0, 2, 3, 5, 6, 8] };
function drawDice(v) {
  const box = $('dice');
  const parts = [];
  if (v.event) {
    const [title, text] = EVENT_NAMES[v.event.name] ?? [v.event.name, ''];
    const card = Object.assign(document.createElement('span'), { className: 'event-card', title: text });
    card.append(Object.assign(document.createElement('strong'), { textContent: v.event.number }), ` ${title}`);
    parts.push(card);
    box.setAttribute('aria-label', `Event card: ${v.event.number}, ${title}. ${text}`);
  } else if (v.dice) {
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
    // The city expansion's event die: a ship (the raiders come closer) or a track's gate
    if (v.city && v.city.eventDie !== null) {
      const ship = v.city.eventDie < 3;
      const face = Object.assign(document.createElement('span'), {
        className: `event-die${ship ? ' ship' : ` gate t${v.city.eventDie - 3}`}`,
        textContent: ship ? '⛵' : DECKS[v.city.eventDie - 3][0],
        title: ship ? 'Event die: the raiders come a step closer' : `Event die: the ${DECKS[v.city.eventDie - 3]} gate (red die ${v.dice[0]})`,
      });
      parts.push(face);
    }
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
  if (v.city) text += cityRollText(v, before.view);
  if (total === 7) text += v.city && !v.city.landed ? ' Big hands lose half (no squall before the raiders land).' : ' The squall moves.';
  else if (v.hand && before.view.hand) {
    const cards = view => [...view.hand, ...(view.city?.goods ?? [])];
    const was = cards(before.view);
    const got = cards(v).map((n, k) => n - was[k]);
    const parts = got.map((n, k) => (n > 0 ? `+${n} ${CARD_NAMES[k]}` : null)).filter(Boolean);
    text += parts.length ? ` You got ${parts.join(', ')}.` : ' Nothing for you this time.';
  }
  if (v.short?.length) {
    text += ` The bank ran short of ${v.short.map(r => RESOURCES[r]).join(' and ')}: nobody got ${v.short.length > 1 ? 'those' : 'it'}, unless only one player was owed it (they got what was left).`;
  }
  toast(text);
}

// The event die's news: the raiders' step or landing, and who drew progress cards
function cityRollText(v, was) {
  const c = v.city;
  if (c.eventDie === null) return '';
  if (c.eventDie < 3) {
    if (c.raiders === 0 && was.city && was.city.raiders > 0) {
      const [strength, defence, won] = c.lastRaid ?? [0, 0, 0];
      return won
        ? ` The raiders landed (${strength} against ${defence}) and pillaged the weakest defenders.`
        : ` The raiders landed (${strength} against ${defence}) and were driven off.`;
    }
    return ` The raiders came closer (${c.raiders} of ${c.raidersAt}).`;
  }
  const deck = DECKS[c.eventDie - 3];
  const who = c.drawn.map(([s]) => (s === v.you ? 'you' : nameOf(s)));
  return who.length ? ` ${deck} gate: ${who.join(', ')} drew a progress card.` : ` ${deck} gate: nobody drew.`;
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
  // (Placing a market you took is always shown: it isn't bought in the shop)
  const moves = v.phase === 'main' ? legal().filter(m => (mode && m.startsWith(`${mode} `)) || m.startsWith('placemarket ')) : legal();
  // Two-step moves: first what moves (a ship, a knight, a number), then where it goes. A decision with
  // only one starting point (a displaced knight) skips the first step.
  const word = TWO_STEP[mode] ? mode : moves.length && moves.every(m => TWO_STEP[m.split(' ')[0]] && m.split(' ')[0] === moves[0].split(' ')[0]) ? moves[0].split(' ')[0] : null;
  if (word) {
    const pairs = moves.filter(m => m.startsWith(`${word} `)).map(m => m.split(' ').slice(1).map(Number));
    const starts = [...new Set(pairs.map(([a]) => a))];
    const from = moveFrom ?? (starts.length === 1 && word !== 'swap' ? starts[0] : null);
    const lit = from === null ? starts : pairs.filter(([a]) => a === from).map(([, b]) => b);
    const [kind, first, second] = TWO_STEP[word];
    for (const n of lit) {
      const t = targetShape(kind, n, from === null ? ' from' : '', word === 'moveship');
      t.dataset[word] = n;
      t.append(el('title', {}, from === null ? first : second));
      const pick = () => {
        if (from === null) { moveFrom = n; render(snap); } else { moveFrom = null; send(`${word} ${from} ${n}`); }
      };
      t.addEventListener('click', pick);
      t.addEventListener('keydown', ev => { if (ev.key === 'Enter' || ev.key === ' ') pick(); });
      layers.targets.append(t);
    }
    return;
  }
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
    } else if (word === 'neutralhut') {
      const { x, y } = cornerAt(Number(m.split(' ')[2]));
      const t = el('circle', { cx: x, cy: y, r: 9, class: 'target corner', 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, 'A Hut for the neutral colour'));
      layers.targets.append(t);
    } else if (word === 'placemarket') {
      const [a, b] = edgeEnds(Number(n));
      const t = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: 'target edge', 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, 'Place your market here'));
      layers.targets.append(t);
    } else if (word === 'neutralpath' || word === 'damage' || word === 'repair') {
      const e = Number(m.split(' ')[word === 'neutralpath' ? 2 : 1]);
      const [a, b] = edgeEnds(e);
      const t = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: 'target edge', 'data-move': m, 'data-edge-target': e, tabindex: 0 });
      t.append(el('title', {}, word === 'damage' ? 'Damage this Path' : word === 'repair' ? 'Repair this Path' : 'A Path for the neutral colour'));
      layers.targets.append(t);
    } else if (word === 'ship') {
      // Drawn a little toward the sea, so a coastal edge's Path and ship spots don't overlap
      const [a, b] = seaEdgeEnds(Number(n));
      const t = el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: 'target edge sea', 'data-move': m, 'data-edge-target': n, tabindex: 0 });
      t.append(el('title', {}, 'Build a ship here'));
      layers.targets.append(t);
    } else if (CORNER_WORDS[word]) {
      const t = targetShape('corner', Number(n));
      t.dataset.move = m;
      t.append(el('title', {}, CORNER_WORDS[word]));
      layers.targets.append(t);
    } else if (word === 'bridge') {
      const t = targetShape('edge', Number(n));
      t.dataset.move = m;
      t.append(el('title', {}, 'Build a bridge here'));
      layers.targets.append(t);
    } else if (word === 'removepath') {
      const t = targetShape('edge', Number(n));
      t.dataset.move = m;
      t.append(el('title', {}, 'Remove this Path'));
      layers.targets.append(t);
    } else if (word === 'merchant') {
      const t = targetShape('hex', Number(n));
      t.dataset.move = m;
      t.append(el('title', {}, 'Place the merchant here'));
      layers.targets.append(t);
    } else if (word === 'squall' || word === 'pirate') {
      const h = board.island.hexes[Number(n)];
      const t = el('polygon', { points: hexPoints(h), class: 'target hex-target', 'data-move': m, tabindex: 0 });
      t.append(el('title', {}, word === 'squall' ? 'Move the squall here' : 'Move the pirate here'));
      layers.targets.append(t);
    }
  }
  for (const t of layers.targets.querySelectorAll('[data-move]')) {
    t.addEventListener('click', () => send(t.dataset.move));
    t.addEventListener('keydown', ev => { if (ev.key === 'Enter' || ev.key === ' ') send(t.dataset.move); });
  }
}

// Moves picked in two steps: [what kind of spot, the first step's tip, the second's]
const TWO_STEP = {
  moveship: ['edge', 'Move this ship', 'Move the ship here'],
  knightmove: ['corner', 'Move this knight', 'Move the knight here'],
  displace: ['corner', 'Use this knight', 'Push off this knight'],
  swap: ['hex', 'Swap this number', 'with this one'],
};
// Moves on a corner (the city expansion), with their tips
const CORNER_WORDS = {
  wall: 'Build a wall under this Lodge',
  recruit: 'A knight here',
  promote: 'Promote this knight',
  activate: 'Activate this knight',
  chase: 'This knight chases the squall',
  pillage: 'The raiders take this Lodge',
  expel: 'Push off this knight',
  removeknight: 'Remove this knight',
};
function targetShape(kind, n, extra = '', sea = false) {
  if (kind === 'corner') {
    const { x, y } = cornerAt(n);
    return el('circle', { cx: x, cy: y, r: 11, class: `target corner${extra}`, tabindex: 0 });
  }
  if (kind === 'hex') return el('polygon', { points: hexPoints(board.island.hexes[n]), class: `target hex-target${extra}`, tabindex: 0 });
  const [a, b] = sea ? seaEdgeEnds(n) : edgeEnds(n);
  return el('line', { x1: a.x, y1: a.y, x2: b.x, y2: b.y, class: `target edge${sea ? ' sea' : ''}${extra}`, tabindex: 0 });
}

// An edge's ends moved a few pixels toward its sea hex (ship spots next to a Path spot on the coast)
function seaEdgeEnds(e) {
  const [a, b] = edgeEnds(e);
  const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
  let near = null, best = Infinity;
  for (const h of board.island.hexes) {
    if (h.terrain !== 'sea') continue;
    const c = centre(h);
    const d = (c.x - mid.x) ** 2 + (c.y - mid.y) ** 2;
    if (d < best) { best = d; near = c; }
  }
  if (!near) return [a, b];
  const len = Math.hypot(near.x - mid.x, near.y - mid.y) || 1;
  const dx = (near.x - mid.x) / len * 5, dy = (near.y - mid.y) / len * 5;
  return [{ x: a.x + dx, y: a.y + dy }, { x: b.x + dx, y: b.y + dy }];
}

function status(v) {
  if (v.phase === 'over') {
    const limit = v.byRounds ? `The ${v.roundLimit} rounds are up. ` : v.clothEnded ? 'Five villages have run out of cloth. ' : '';
    if (v.winner === null) return `${limit}It's a draw: equal ${v.clothEnded ? 'Renown and cloth' : 'Renown and Lodges'} at the top.`;
    return limit + (v.winner === v.you ? `You win with ${v.players[v.winner].renown} Renown!` : `${nameOf(v.winner)} wins with ${v.players[v.winner].renown} Renown.`);
  }
  const who = v.toAct === v.you ? 'You' : nameOf(v.toAct);
  const mine = v.toAct === v.you;
  switch (v.phase) {
    case 'setupHut': return mine ? 'Setup: place a Hut on a lit corner.' : `Setup: ${who} is placing a Hut.`;
    case 'setupPath': return mine ? (v.pirate !== null ? 'Setup: place a Path next to your new Hut, or a ship on the coast (dashed spots).' : 'Setup: place a Path next to your new Hut.') : `Setup: ${who} is placing a Path.`;
    case 'preRoll': return mine ? (v.city ? `Your turn: roll the dice${legal().some(m => m.startsWith('fixedroll')) ? ' (or use Fixed Roll in the City tab)' : ''}.` : 'Your turn: roll the dice (or play a project first).') : `${who}'s turn.`;
    case 'discard': return mine ? (v.lastRoll === 7 ? `A 7! You hold too many cards: choose ${v.players[v.you].discard} more to lose.` : `Choose ${v.players[v.you].discard} more cards to lose.`) : `${who} is choosing cards to lose.`;
    case 'moveSquall': return mine ? (v.pirate !== null ? 'Move the squall (a lit land hex) or the pirate (a lit sea hex).' : 'Move the squall: click a lit hex.') : `${who} is moving the ${v.pirate !== null ? 'squall or the pirate' : 'squall'}.`;
    case 'gold': return mine ? 'A gold field paid you: choose a resource.' : `${who} is choosing resources from a gold field.`;
    case 'pick': return mine ? 'The event card gives you a resource: choose one.' : `${who} is choosing a resource.`;
    case 'gift': return mine ? 'The event card: choose a card to give.' : `${who} is choosing a card to give.`;
    case 'rob': return mine ? 'The event card lets you rob someone: choose who.' : `${who} is choosing who to rob.`;
    case 'damage': return mine ? 'Earthquake: click one of your Paths to damage.' : `${who} is choosing a Path to damage.`;
    case 'neutral': return mine ? 'Now build one for a neutral colour: click a lit spot.' : `${who} is building for a neutral colour.`;
    case 'secondRoll': return mine ? 'Roll again: two productions a turn (the second a different number).' : `${who} is rolling again.`;
    case 'steal': return mine ? 'Choose who to steal a card from.' : `${who} is choosing who to steal from.`;
    // 5-6 players: the player 3 seats on builds after each turn (no roll, no trading with players)
    case 'main': if (v.paired) return mine ? `Your extra build phase (after ${nameOf(v.turnOf)}'s turn): build, buy or play a project, trade with the bank or markets (not with players), then end it.` : `${who} has an extra build phase after ${nameOf(v.turnOf)}'s turn.`;
      if (mine && v.city && !legal().includes('end')) return 'You hold more than 4 progress cards: play or drop one in the City tab before ending your turn.';
      return mine ? `You rolled ${v.lastRoll}. Pick something to build, trade or buy, then end your turn.` : `${who} rolled ${v.lastRoll}.`;
    case 'freeBuild': return mine ? (v.pirate !== null ? 'Road Crew: place a free Path or ship.' : 'Road Crew: place a free Path.') : `${who} is placing free Paths.`;
    case 'harvest': return mine ? 'Good Harvest: take a resource.' : `${who} is taking resources.`;
    case 'marketDay': return mine ? 'Market Day: choose the resource everyone else hands over.' : `${who} is calling a Market Day.`;
    // The city expansion's decisions
    case 'dropProgress': return mine ? 'You hold more than 4 progress cards: drop one.' : `${who} is dropping a progress card.`;
    case 'pillage': return mine ? 'The raiders won: click the Lodge they take (it becomes a Hut).' : `The raiders are pillaging ${who === 'You' ? 'you' : `${who}'s`} Lodge.`;
    case 'displaced': return mine ? 'Your knight was pushed off: click where it goes.' : `${who} is moving a displaced knight.`;
    case 'drawCard': return mine ? 'You tied as the best defender: draw a progress card from a deck.' : `${who} is drawing a progress card.`;
    case 'name': return mine ? 'Name a card kind for your progress card.' : `${who} is naming a card kind.`;
    case 'target': return mine ? 'Choose a player for your progress card.' : `${who} is choosing a player.`;
    case 'takeCards': return mine ? 'Guild Dues: take cards of your choice.' : `${who} is taking cards (Guild Dues).`;
    case 'spy': return mine ? `Spy: their progress cards are ${v.city.spied.map(cardName).join(', ')}. Take one, or none.` : `${who} is spying.`;
    case 'harbour': return mine ? 'Harbour Deal: offer a resource to each player with commodities (they pay a commodity), or stop.' : `${who} is making Harbour Deals.`;
    case 'payGoods': return mine ? 'Harbour Deal: give a commodity of your choice back.' : `${who} is paying a commodity.`;
    case 'smith': return mine ? 'Smithing: click a knight to promote for free, or stop.' : `${who} is promoting knights.`;
    case 'freeWall': return mine ? 'Engineer: click a Lodge for a free wall.' : `${who} is building a free wall.`;
    case 'swap': return mine ? 'Number Swap: click two hexes to swap their numbers.' : `${who} is swapping numbers.`;
    case 'placeMerchant': return mine ? 'Merchant: click a hex by your building.' : `${who} is placing the merchant.`;
    case 'removePath': return mine ? 'Diplomacy: click an open Path to remove.' : `${who} is removing a Path.`;
    case 'expel': return mine ? 'Intrigue: click a knight touching your Paths to push it off.' : `${who} is pushing off a knight.`;
    case 'giveUpKnight': return mine ? 'Treason: click one of your knights to remove.' : `${who} is removing a knight (Treason).`;
    case 'treasonKnight': return mine ? 'Treason: place a knight of yours (no stronger), or skip.' : `${who} is placing a knight.`;
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
    if (word === 'discard') box.append(button(`Lose ${CARD_NAMES[n]} (${held(v, Number(n))})`, m));
    if (word === 'take') box.append(button(v.phase === 'marketDay' ? `Take everyone's ${RESOURCES[n]}` : `Take ${RESOURCES[n]}`, m));
    // The city expansion
    if (word === 'choose') box.append(button(chooseLabel(v, Number(n)), m));
    if (word === 'takecard') box.append(button(`Take a ${CARD_NAMES[m.split(' ')[2]]}`, m));
    if (word === 'takeprogress') box.append(button(`Take ${cardName(m.split(' ')[2])}`, m));
    if (word === 'dropcard' && v.phase !== 'main') box.append(button(`Drop ${cardName(n)}`, m));
    if (word === 'skip') box.append(button(v.phase === 'treasonKnight' ? 'Place no knight' : v.phase === 'spy' ? 'Take nothing' : 'Done', m));
    // Fishing: spend fish, pass the old boot
    if (word === 'fish') box.append(button(`${v.fishing.costs[n]} fish: ${FISH_ACTIONS[n]}`, m));
    if (word === 'giveboot') box.append(button(`Give ${nameOf(Number(n))} the old boot`, m));
    if (word === 'steal') box.append(button(`Steal from ${nameOf(Number(n))} (${v.players[n].cards} cards)`, m));
    // Event cards and the two-player variant
    if (word === 'gift') box.append(button(v.phase === 'harbour' ? `Offer ${nameOf(Number(n))} a ${CARD_NAMES[m.split(' ')[2]]}` : `Give ${nameOf(Number(n))} a ${CARD_NAMES[m.split(' ')[2]]}`, m));
    if (word === 'rob') box.append(button(`Rob ${nameOf(Number(n))} (${v.players[n].cards} cards)`, m));
    if (word === 'cashpilot') box.append(button('Cash in a played Pilot (+2 trade tokens)', m));
    if (word === 'forcetrade') box.append(button(`Forced trade (${v.players[v.you].renown <= v.players[1 - v.you].renown ? 1 : 2} token)`, m));
    if (word === 'squallhome') box.append(button('Send the squall to the Dunes (trade tokens)', m));
    // Projects are played from Your projects, under the hand (drawMyProjects)
  }
  if (moves.includes('end')) box.append(button('End turn', 'end', 'primary'));
}

// Fishing's spending (engine fishing.rs FISH_COSTS order)
const FISH_ACTIONS = ['send the squall off the island', 'steal a card', 'take a resource', 'a free Path', 'a free project'];

// What a "choose" picks, by the decision: a deck, a card kind or a player
function chooseLabel(v, x) {
  switch (v.phase) {
    case 'drawCard': return `Draw a ${DECKS[x]} card (${v.city.decks[x]} left)`;
    case 'name': return `Name ${CARD_NAMES[x]}`;
    default: return `Choose ${nameOf(x)}`;
  }
}

// Your projects, under your hand: each one, and a Use button on those you can play now. Projects
// bought this turn wait for your next turn; a Lighthouse is never played, it counts at once.
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
      const usable = !fresh && legal().includes(`play ${k}`);
      if (k === 'lighthouse') when.textContent = '1 Renown, counted';
      else if (usable) when.textContent = PROJECT_HELP[k];
      else if (fresh) when.textContent = 'Playable from your next turn';
      else when.textContent = 'Playable on your turn (one project a turn)';
      when.className = 'when';
      li.title = PROJECT_HELP[k];
      const text = document.createElement('div');
      text.className = 'text';
      text.append(name, when);
      li.append(text);
      if (usable) {
        li.classList.add('usable');
        li.append(button('Use', `play ${k}`, 'use'));
      }
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
  // The sea expansion only
  { key: 'repair', name: 'Repair a Path', cost: [1, 0, 0, 1, 0], piece: 0, note: 'An earthquake damaged it: no new Paths until it\'s repaired', repair: true },
  { key: 'ship', name: 'Ship', cost: [1, 0, 0, 0, 1], piece: 3, note: 'Sails on from your buildings and ships', sea: true },
  { key: 'moveship', name: 'Move a ship', cost: [0, 0, 0, 0, 0], piece: 3, note: 'Once a turn: a ship with an open end, not built this turn', sea: true, free: true },
  // Rivers only
  { key: 'bridge', name: 'Bridge', cost: [1, 0, 0, 2, 0], note: 'On a bridge site where a river crosses: 3 coins, counts as a Path', rivers: true, why: 'No bridge site by your Paths, or no bridge left' },
  // The city expansion only
  { key: 'wall', name: 'Wall', cost: [0, 0, 0, 2, 0], note: 'Under a Lodge: +2 to your hand limit (3 at most)', city: true, why: 'No Lodge without a wall' },
  { key: 'recruit', name: 'Knight', cost: [0, 1, 0, 0, 1], note: 'A basic knight on a free corner by your Path (starts inactive)', city: true, why: 'No free corner by your Paths, or no basic knight left' },
  { key: 'activate', name: 'Activate a knight', cost: [0, 0, 1, 0, 0], note: 'Active knights defend against the raiders, and can act from next turn', city: true, why: 'No inactive knight' },
  { key: 'promote', name: 'Promote a knight', cost: [0, 1, 0, 0, 1], note: 'Basic to strong; strong to mighty needs Politics 3. Once a turn each', city: true, why: 'No knight can be promoted' },
  { key: 'knightmove', name: 'Move a knight', cost: [0, 0, 0, 0, 0], note: 'An active knight along your Paths (it becomes inactive)', city: true, free: true, verb: 'Move', why: 'No knight ready (active since before this turn)' },
  { key: 'displace', name: 'Displace a knight', cost: [0, 0, 0, 0, 0], note: 'Push a weaker knight off with an active one', city: true, free: true, verb: 'Pick', why: 'No weaker knight in reach' },
  { key: 'chase', name: 'Chase the squall', cost: [0, 0, 0, 0, 0], note: 'An active knight by the squall moves it, then steals', city: true, free: true, verb: 'Pick', why: 'No ready knight by the squall' },
];

// Why an item can't be built now, or null if it can
function shopReason(v, item) {
  if (v.you === null) return 'Watching';
  if (legal().some(m => m.startsWith(`${item.key} `))) return null;
  if (v.toAct !== v.you || v.phase === 'over') return 'Not your turn';
  if (v.phase === 'preRoll') return 'After you roll';
  if (v.phase !== 'main') return 'Not now';
  if (item.key === 'moveship') return v.shipMoved ? 'Already moved one this turn' : 'No ship can move';
  if (item.piece !== undefined && v.players[v.you].pieces[item.piece] === 0) return 'None left';
  const short = item.cost.map((n, r) => Math.max(0, n - v.hand[r]));
  if (short.some(n => n)) return `Need ${short.map((n, r) => (n ? `${n} ${RESOURCES[r]}` : null)).filter(Boolean).join(', ')}`;
  if (item.why) return item.why;
  return item.key === 'lodge' ? (v.city?.medicine ? 'No Hut to upgrade' : 'No Hut to upgrade') : 'No spot you can reach';
}

function drawShop(v) {
  if (v.you === null) {
    const li = document.createElement('li');
    li.className = 'note';
    li.textContent = 'You\'re watching this game.';
    $('shop').replaceChildren(li);
    return;
  }
  const myDamaged = (v.damaged ?? []).some(e => v.roads.some(([re, p]) => re === e && p === v.you));
  const pieces = SHOP.filter(item => (!item.sea || seaGame()) && (!item.city || v.city) && (!item.rivers || v.rivers) && (!item.repair || myDamaged)).map(item => {
    const reason = shopReason(v, item);
    const b = document.createElement('button');
    b.type = 'button';
    b.dataset.build = item.key;
    if (mode === item.key) {
      b.textContent = 'Cancel';
      b.addEventListener('click', () => { mode = null; render(snap); });
    } else {
      b.textContent = item.verb ?? (item.free ? 'Move' : 'Build');
      b.disabled = reason !== null;
      b.addEventListener('click', () => { mode = item.key; moveFrom = null; render(snap); });
    }
    return shopRow(v, { key: item.key, name: item.name, note: item.note, cost: item.cost, picked: mode === item.key }, b, reason);
  });
  $('shop').replaceChildren(...pieces, ...wonderItems(v), ...projectItems(v));
}

// One row of the shop: name, note, cost chips (greyed where you're short), its button, and why not
// Wonders (the sea scenario): each wonder with its requirement, owner and level, and Claim or Build
const WONDER_TEXT = {
  greatBridge: ['Great Bridge', 'Needs a building on a bridge marker (B)'],
  grandTheater: ['Grand Theater', 'Needs 2 Lodges'],
  greatWall: ['Great Wall', 'Needs a building on a wall marker (W)'],
  grandMonument: ['Grand Monument', 'Needs a Lodge on a market and a line of 5 Paths or ships'],
  grandCastle: ['Grand Castle', 'Needs a Lodge and 6 Renown'],
};
function wonderItems(v) {
  return (v.wonders ?? []).map(([name, owner, level, cost], w) => {
    const [title, need] = WONDER_TEXT[name] ?? [name, ''];
    const claim = `claimwonder ${w}`;
    const mineNow = owner === v.you;
    const move = mineNow ? 'buildwonder' : claim;
    const b = document.createElement('button');
    b.type = 'button';
    b.textContent = mineNow ? `Build level ${level + 1}` : 'Claim';
    b.dataset.move = move;
    const ok = legal().includes(move);
    b.disabled = !ok;
    b.addEventListener('click', () => send(move));
    const state = owner === null ? need : `${owner === v.you ? 'Yours' : nameOf(owner)}: level ${level} of 4`;
    const reason = ok ? null : owner !== null && !mineNow ? 'Claimed' : mineNow && level === 4 ? 'Finished' : mineNow ? 'Need the cards' : 'Requirement not met';
    return shopRow(v, { key: `wonder-${name}`, name: title, note: `${state}. Each of 4 levels costs:`, cost, picked: false }, b, reason);
  });
}

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
        note: `Drawn unseen: Pilot, Lighthouse, Road Crew, Good Harvest or Market Day · ${v.deck} left`,
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
const cardsText = h => h.map((n, r) => (n ? `${n} ${CARD_NAMES[r]}` : null)).filter(Boolean).join(' + ');
// Counts as move text: 5 (resources), or 8 when commodities are in it (the city expansion)
const countsText = h => (h.slice(5).some(n => n) ? h : h.slice(0, 5)).join(',');

// +/− steppers for the five resources (and the city expansion's commodities): obj[side][r] counts, at
// most max(r); raising one clears the same kind on the other side (a kind is on one side of a trade only)
function steppers(box, obj, side, other, max, redraw) {
  box.replaceChildren(...(snap.view.city ? CARD_NAMES : RESOURCES).map((name, r) => {
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
    counter.mine = counter.mine.map((n, r) => Math.min(n, held(v, r)));
    const redraw = () => drawOffer(snap.view);
    steppers($('counter-mine'), counter, 'mine', 'theirs', r => held(v, r), redraw);
    steppers($('counter-theirs'), counter, 'theirs', 'mine', () => 9, redraw);
    $('send-counter').disabled = !hasCards(counter.mine) || !hasCards(counter.theirs);
  }
}

// Building an offer: steppers for what you give (up to your hand) and get, and who it's to
const deal = { give: [0, 0, 0, 0, 0, 0, 0, 0], get: [0, 0, 0, 0, 0, 0, 0, 0], to: [] };
function drawDeal(v) {
  const show = v.canOffer && !v.offer;
  $('deal-box').hidden = !show;
  if (!show) return;
  $('offers-left').textContent = v.offersLeft === null ? '' : `${v.offersLeft} offer${v.offersLeft === 1 ? '' : 's'} left this turn`;
  deal.give = deal.give.map((n, r) => Math.min(n, held(v, r)));
  const others = v.players.map((_, s) => s).filter(s => s !== v.you);
  deal.to = deal.to.filter(s => others.includes(s));
  if (!deal.to.length) deal.to = [...others];
  steppers($('deal-give'), deal, 'give', 'get', r => held(v, r), () => drawDeal(snap.view));
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
    if (p.pilots) facts.push(`${p.pilots} Pilot${p.pilots === 1 ? '' : 's'} played`);
    // The city expansion: knights' strength, the three tracks, progress cards held
    if (v.city) {
      facts.push(`tracks ${v.city.levels[s].join('/')}`);
      facts.push(`knights ${v.city.defence[s]} active`);
      if (v.city.progressCounts[s]) facts.push(`${v.city.progressCounts[s]} progress`);
    }
    // Rivers: coins, and the Wealthiest / Poor tiles
    if (v.rivers) {
      facts.push(`${v.rivers.coins[s]} coin${v.rivers.coins[s] === 1 ? '' : 's'}`);
      if (v.rivers.wealthiest === s) facts.push('Wealthiest +1');
      if (v.rivers.poor.includes(s)) facts.push('Poor −2');
    }
    // Fishing: fish tokens held (their values only for your own), and the old boot
    if (v.fishing) {
      const n = v.fishing.tokens[s];
      if (n) facts.push(s === v.you ? `${v.fishing.fish} fish (${n} token${n === 1 ? '' : 's'})` : `${n} fish token${n === 1 ? '' : 's'}`);
      if (v.fishing.boot === s) facts.push('the old boot (+1 to win)');
    }
    const wonder = (v.wonders ?? []).find(([, owner]) => owner === s);
    if (wonder) facts.push(`${(WONDER_TEXT[wonder[0]] ?? [wonder[0]])[0]} ${wonder[2]}/4`);
    for (const [kind, n] of Object.entries(p.tokens ?? {})) {
      const [one, many] = TOKEN_NAMES[kind] ?? [`${kind} token`, `${kind} tokens`];
      if (n) facts.push(`${n} ${n === 1 ? one : many}`);
    }
    info.append(facts.join(' · '));
    (v.city?.metro ?? []).forEach((m, t) => {
      if (m?.[1] !== s) return;
      const badge = Object.assign(document.createElement('span'), { className: 'title-badge', textContent: `★ ${TRACKS[t][0]}`, title: `${TRACKS[t][0]} metropolis: 2 Renown` });
      info.append(' ', badge);
    });
    for (const t of v.titles ?? []) {
      if (t.holder !== s) continue;
      const badge = document.createElement('span');
      badge.className = 'title-badge';
      badge.textContent = TITLE_NAMES[t.name] ?? t.name;
      badge.title = `${badge.textContent}: ${t.renown} Renown`;
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
  const kinds = v.city ? CARD_NAMES : RESOURCES;
  $('hand').replaceChildren(...kinds.map((r, i) => {
    const li = document.createElement('li');
    const count = held(v, i);
    li.className = `card r${i}${count ? '' : ' none'}`;
    li.dataset.count = count;
    li.dataset.resource = r;
    const n = document.createElement('b');
    n.textContent = count;
    const label = document.createElement('span');
    label.textContent = r;
    li.append(n, label);
    return li;
  }));
  const affordable = SHOP.filter(item => !item.free && !item.repair && (!item.sea || seaGame()) && (!item.city || v.city) && (!item.rivers || v.rivers) && item.cost.every((n, r) => v.hand[r] >= n)).map(item => item.name);
  if (v.row.length || v.deck) {
    if (PROJECT_COST.every((n, r) => v.hand[r] >= n)) affordable.push('a project');
  }
  $('can-build').textContent = affordable.length ? `Enough for: ${affordable.join(', ')}` : 'Not enough for anything yet.';
  drawMyProjects(v);
  const total = v.hand.reduce((a, b) => a + b, 0) + (v.city?.goods ?? []).reduce((a, b) => a + b, 0);
  const limit = v.city ? v.city.handLimits[v.you] : v.handLimit;
  $('hand-limit').textContent = `On a 7, holding more than ${limit} cards loses half${total > limit ? ` (you hold ${total})` : ''}.`;
  $('hand-limit').classList.toggle('warn', total > limit);
}

// Trading: pick what to give (only what you can afford, at your rate) and what to get
const trade = { give: null, get: null };
function drawTrade(v) {
  // What's left in the bank (a game has 19 of each resource; null: it never runs out)
  $('bank-left').textContent = v.bank ? `Left in the bank: ${v.bank.map((n, r) => `${RESOURCES[r]} ${n}`).join(' · ')}` : '';
  $('bank-left').hidden = !v.bank;
  const trades = legal().filter(m => m.startsWith('trade '));
  $('trade-box').hidden = trades.length === 0;
  if (!trades.length) return;
  const gives = [...new Set(trades.map(m => Number(m.split(' ')[1])))];
  if (!gives.includes(trade.give)) trade.give = gives[0];
  const kinds = v.city ? CARD_NAMES : RESOURCES;
  if (trade.get === trade.give || trade.get === null) trade.get = kinds.findIndex((_, i) => i !== trade.give);
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
  pick($('give'), gives, trade.give, r => `${rateOf(v, r)} ${kinds[r]}`, r => { trade.give = r; });
  pick($('get'), kinds.map((_, i) => i).filter(r => r !== trade.give), trade.get, r => kinds[r], r => { trade.get = r; });
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

// Rivers' coins, in the Trade tab: buy one with a resource (at your rate), or spend 2 for a resource
function drawCoins(v) {
  $('coin-box').hidden = !v.rivers || v.you === null;
  if (!v.rivers || v.you === null) return;
  const r = v.rivers;
  const standing = r.wealthiest === v.you ? ' You are the Wealthiest (+1 Renown).' : r.poor.includes(v.you) ? ' You are Poor (−2 Renown): get more coins than someone.' : '';
  $('coin-note').textContent = `You have ${r.coins[v.you]} coin${r.coins[v.you] === 1 ? '' : 's'}.${standing} Spend 2 for a resource, ${r.coinBuysLeft} more this turn.`;
  const buys = legal().filter(m => m.startsWith('buycoin '));
  const spends = legal().filter(m => m.startsWith('spendcoins '));
  $('coin-buy').replaceChildren(...buys.map(m => { const k = Number(m.split(' ')[1]); return button(`${v.rates[k]} ${RESOURCES[k]} → 1 coin`, m); }));
  $('coin-spend').replaceChildren(...spends.map(m => button(`2 coins → ${RESOURCES[Number(m.split(' ')[1])]}`, m)));
}

// The City tab (the city expansion): the raiders, your tracks, Fixed Roll and your progress cards
let fixed = [1, 1];
function drawCity(v) {
  $('tab-city').hidden = !v.city;
  $('tab-projects').hidden = !!v.city; // no projects in the city expansion
  if (!v.city) return;
  const c = v.city;
  const raid = c.lastRaid ? ` Last landing: ${c.lastRaid[0]} against ${c.lastRaid[1]}, ${c.lastRaid[2] ? 'the raiders won' : 'driven off'}.` : '';
  $('raiders').textContent = `Raiders: ${c.raiders} of ${c.raidersAt} steps. Strength ${c.strength} (Lodges on the board) against ${c.defence.reduce((a, b) => a + b, 0)} (active knights).${c.landed ? '' : ' The squall waits until they first land.'}${raid}`;
  $('tracks').replaceChildren(...TRACKS.map(([name, goods, power], t) => {
    const li = document.createElement('li');
    li.className = 'track';
    const level = v.you === null ? 0 : c.levels[v.you][t];
    const holder = c.metro[t] ? ` · ★ ${nameOf(c.metro[t][1])}` : '';
    const head = Object.assign(document.createElement('strong'), { textContent: `${name}: level ${level} of 5${holder}` });
    const help = Object.assign(document.createElement('span'), {
      textContent: `${level < 5 && c.improveCost ? `Next: ${c.improveCost[t]} ${goods}. ` : ''}Level 3: ${power}. First to 4: the metropolis.`,
    });
    li.append(head, help);
    if (legal().includes(`improve ${t}`)) li.append(button('Improve', `improve ${t}`));
    return li;
  }));
  // Fixed Roll, before rolling: the red and the yellow die
  const canFix = legal().some(m => m.startsWith('fixedroll '));
  $('fixed-roll').hidden = !canFix;
  if (canFix) {
    const row = (i, label) => {
      const g = document.createElement('div');
      g.className = 'pick';
      g.append(Object.assign(document.createElement('span'), { textContent: label }));
      for (let n = 1; n <= 6; n++) {
        const b = Object.assign(document.createElement('button'), { type: 'button', textContent: n });
        b.setAttribute('aria-pressed', String(fixed[i] === n));
        b.addEventListener('click', () => { fixed[i] = n; drawCity(snap.view); });
        g.append(b);
      }
      return g;
    };
    $('fixed-dice').replaceChildren(row(0, 'Red'), row(1, 'Yellow'));
    $('fixed-go').textContent = `Roll with ${fixed[0] + fixed[1]}`;
    $('fixed-go').onclick = () => send(`fixedroll ${fixed[0]} ${fixed[1]}`);
  }
  const mine = c.progress ?? [];
  $('progress-cards').replaceChildren(...mine.map(k => {
    const li = document.createElement('li');
    li.className = 'project';
    li.append(Object.assign(document.createElement('strong'), { textContent: cardName(k) }), Object.assign(document.createElement('span'), { textContent: PROGRESS[k]?.[1] ?? '' }));
    if (legal().includes(`card ${k}`)) li.append(button('Play', `card ${k}`));
    if (legal().includes(`dropcard ${k}`)) li.append(button('Drop', `dropcard ${k}`));
    return li;
  }));
  $('progress-note').textContent = `${mine.length ? '' : 'No progress cards. '}At most 4 in hand. Decks: ${DECKS.map((d, t) => `${d} ${c.decks[t]}`).join(' · ')}.${c.crane ? ' Crane: your next improvement is 1 cheaper.' : ''}${c.medicine ? ' Medicine: your next Lodge costs 2 Stone + 1 Grain.' : ''}${c.fleet !== null ? ` Trade Fleet: ${CARD_NAMES[c.fleet]} 2:1 this turn.` : ''}`;
}

function describe(entry) {
  const who = entry.seat === snap.view.you ? 'You' : nameOf(entry.seat);
  const [word, a, b] = entry.move.split(' ');
  switch (word) {
    case 'hut': return `${who} built a Hut`;
    case 'lodge': return `${who} built a Lodge`;
    case 'path': return `${who} built a Path`;
    case 'ship': return `${who} built a ship`;
    case 'moveship': return `${who} moved a ship`;
    case 'pirate': return `${who} moved the pirate`;
    case 'gift': return `${who} gave ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))} a card`;
    case 'rob': return `${who} robbed ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'damage': return `${who}'s Path was damaged by the earthquake`;
    case 'repair': return `${who} repaired a Path`;
    case 'claimwonder': return `${who} claimed a wonder`;
    case 'placemarket': return `${who} placed a market`;
    case 'buildwonder': return `${who} built a wonder level`;
    case 'neutralhut': return `${who} built a neutral Hut`;
    case 'neutralpath': return `${who} built a neutral Path`;
    case 'cashpilot': return `${who} cashed in a Pilot for trade tokens`;
    case 'forcetrade': return `${who} forced a trade`;
    case 'squallhome': return `${who} sent the squall to the Dunes`;
    case 'roll': return `${who} rolled ${entry.roll}`;
    case 'discard': return `${who} lost a ${CARD_NAMES[a]}`;
    case 'squall': return `${who} moved the squall`;
    case 'steal': return `${who} stole a card from ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'trade': return `${who} traded ${CARD_NAMES[a]} for ${CARD_NAMES[b]}`;
    case 'buy': return `${who} bought a project`;
    case 'play': return `${who} played ${PROJECTS[a]}`;
    case 'take': return entry.got === undefined
      ? `${who} took ${RESOURCES[a]}`
      : `${who} called a Market Day and took ${entry.got} ${RESOURCES[a]} from the others`;
    case 'end': return `${who} ended the turn`;
    case 'offer': return `${who} offered ${cardsText(a.split(',').map(Number))} for ${cardsText(b.split(',').map(Number))}`;
    case 'answer': return `${Number(a) === snap.view.you ? 'You' : nameOf(Number(a))} ${b === 'yes' ? 'accepted' : 'declined'} the offer`;
    case 'deal': return `${who} traded with ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'counter': return `${Number(a) === snap.view.you ? 'You' : nameOf(Number(a))} made a counter-offer`;
    case 'accept': return `${who} took ${Number(a) === snap.view.you ? 'your' : `${nameOf(Number(a))}'s`} counter-offer`;
    case 'withdraw': return `${who} withdrew the offer`;
    case 'wall': return `${who} built a wall`;
    case 'improve': return `${who} improved ${TRACKS[a][0]}`;
    case 'recruit': return `${who} placed a knight`;
    case 'promote': return `${who} promoted a knight`;
    case 'activate': return `${who} activated a knight`;
    case 'knightmove': return `${who} moved a knight`;
    case 'displace': return `${who} displaced a knight`;
    case 'chase': return `${who} chased the squall`;
    case 'card': return `${who} played ${cardName(a)}`;
    case 'fixedroll': return `${who} played Fixed Roll`;
    case 'dropcard': return `${who} dropped a progress card`;
    case 'choose': return `${who} chose`;
    case 'takecard': return `${who} took a card from ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'takeprogress': return `${who} took a progress card from ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))}`;
    case 'swap': return `${who} swapped two numbers`;
    case 'removepath': return `${who} removed a Path`;
    case 'merchant': return `${who} placed the merchant`;
    case 'expel': return `${who} pushed off a knight`;
    case 'removeknight': return `${who} removed a knight`;
    case 'pillage': return `The raiders pillaged ${who === 'You' ? 'your' : `${who}'s`} Lodge`;
    case 'skip': return `${who} finished`;
    case 'fish': return `${who} spent fish: ${FISH_ACTIONS[a]}`;
    case 'bridge': return `${who} built a bridge (3 coins)`;
    case 'buycoin': return `${who} bought a coin with ${RESOURCES[a]}`;
    case 'spendcoins': return `${who} spent 2 coins on ${RESOURCES[a]}`;
    case 'giveboot': return `${who} gave ${Number(a) === snap.view.you ? 'you' : nameOf(Number(a))} the old boot`;
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
