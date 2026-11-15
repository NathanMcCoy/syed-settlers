// The game: everything in RULES.md from setup to a winner, as two operations that everything else
// uses (the page, the server, the bots and the simulations), so nobody can be allowed a different move:
//   game.legal(&mut list)  every action the player to act may take now
//   game.apply(action)     takes one (apply() checks it's legal; apply_unchecked() trusts the caller)
//
// Speed: the board (Board) is built once per game and shared, so copying a Game (bots looking ahead)
// copies only the small changing state. Nothing here searches through lists during play except a
// player's own Paths and Boats (Navigator), at most a few dozen.
use crate::hex::{Graph, listed};
use crate::island::{ANY, Island, MAINLAND, Terrain, pips};
use crate::rng::Rng;
use crate::settings::Settings;
use std::sync::Arc;

pub const MAX_PLAYERS: usize = 4;
pub const NOBODY: u8 = u8::MAX;

// Resources, in this order everywhere: Timber, Stone, Grain, Copper, Fish
pub type Hand = [u8; 5];
pub const PATH_COST: Hand = [1, 1, 0, 0, 0];
pub const BOAT_COST: Hand = [1, 0, 0, 0, 1];
pub const HUT_COST: Hand = [1, 1, 1, 0, 1];
pub const LODGE_COST: Hand = [0, 0, 2, 3, 0];
pub const PROJECT_COST: Hand = [0, 0, 1, 1, 1];

// What stands on a corner or an edge (0: nothing)
pub const HUT: u8 = 1;
pub const LODGE: u8 = 2;
pub const PATH: u8 = 1;
pub const BOAT: u8 = 2;

// A player's pieces still in hand, by this index
pub const P_PATH: usize = 0;
pub const P_BOAT: usize = 1;
pub const P_HUT: usize = 2;
pub const P_LODGE: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum Project {
    Pilot,
    Lighthouse,
    Shipwright,
    Harvest,
    Winds,
}
pub const PROJECTS: [Project; 5] = [Project::Pilot, Project::Lighthouse, Project::Shipwright, Project::Harvest, Project::Winds];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Action {
    Hut(u16),        // build (or in setup, place) a Hut on a corner
    Lodge(u16),      // upgrade your Hut on this corner
    Path(u16),       // on an edge
    Boat(u16),
    Roll,
    Discard(u8),     // lose one card of this resource (after a 7)
    Squall(u16),     // move the squall to this hex
    Steal(u8),       // from this player
    Trade(u8, u8),   // give the bank/market this resource (at your best rate) for one of that
    Buy(u8),         // a project: face up, the row's slot; hidden, always 0
    Play(Project),
    Take(u8),        // choose a resource (Good Harvest, Fair Winds)
    EndTurn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    SetupHut,
    SetupPath { hut: u16 },
    PreRoll,
    Discard,                    // players over the limit lose cards, one action per card
    MoveSquall { anywhere: bool },
    Steal,
    Main,
    FreeBuild { left: u8 },     // Shipwright
    Harvest { left: u8 },       // Good Harvest
    Winds,                      // Fair Winds: choose the resource
    Over,
}

// The board for play: the island's graph plus, per corner and edge, everything the rules ask about,
// worked out once
pub struct Board {
    pub settings: Settings,
    pub graph: Graph,
    pub terrain: Vec<Terrain>,
    pub number: Vec<u8>,
    pub resource: Vec<u8>,            // per hex: 0â€“4, or NOBODY for Sea and Dunes
    pub land_hexes: Vec<u16>,
    pub by_number: [Vec<u16>; 13],    // producing hexes for each dice total
    pub corner_land: Vec<bool>,       // touches land: can hold a building
    pub corner_main: Vec<bool>,       // touches the main island: allowed in setup
    pub corner_islet: Vec<u8>,        // which islet it touches, or NOBODY
    pub corner_market: Vec<u8>,       // which market (index into markets), or NOBODY
    pub market_kind: Vec<u8>,         // per market: 0â€“4 (2:1) or ANY (3:1)
    pub edge_land: Vec<bool>,         // can take a Path
    pub edge_sea: Vec<bool>,          // can take a Boat
    pub islets: usize,
}

impl Board {
    pub fn new(island: &Island, settings: &Settings) -> Board {
        let graph = island.graph();
        let hexes = &island.hexes;
        let land = |h: usize| hexes[h].terrain != Terrain::Sea;
        let mut by_number: [Vec<u16>; 13] = Default::default();
        for (i, h) in hexes.iter().enumerate() {
            if h.number != 0 {
                by_number[h.number as usize].push(i as u16);
            }
        }
        let mut corner_market = vec![NOBODY; graph.corners()];
        for (m, market) in island.markets.iter().enumerate() {
            let e = graph.hex_edges[market.hex as usize][market.side as usize];
            for &c in &graph.edge_corners[e as usize] {
                corner_market[c as usize] = m as u8;
            }
        }
        Board {
            settings: settings.clone(),
            terrain: hexes.iter().map(|h| h.terrain).collect(),
            number: hexes.iter().map(|h| h.number).collect(),
            resource: hexes.iter().map(|h| h.terrain.resource().map_or(NOBODY, |r| r as u8)).collect(),
            land_hexes: (0..hexes.len()).filter(|&h| land(h)).map(|h| h as u16).collect(),
            by_number,
            corner_land: (0..graph.corners()).map(|c| listed(&graph.corner_hexes[c]).any(land)).collect(),
            corner_main: (0..graph.corners()).map(|c| listed(&graph.corner_hexes[c]).any(|h| land(h) && hexes[h].islet == MAINLAND)).collect(),
            corner_islet: (0..graph.corners())
                .map(|c| listed(&graph.corner_hexes[c]).find(|&h| land(h) && hexes[h].islet != MAINLAND).map_or(NOBODY, |h| hexes[h].islet))
                .collect(),
            corner_market,
            market_kind: island.markets.iter().map(|m| m.kind).collect(),
            edge_land: (0..graph.edges()).map(|e| listed(&graph.edge_hexes[e]).any(land)).collect(),
            edge_sea: (0..graph.edges()).map(|e| listed(&graph.edge_hexes[e]).any(|h| !land(h))).collect(),
            islets: island.info.islets.len(),
            graph,
        }
    }

    fn hex_distance(&self, a: u16, b: u16) -> i32 {
        self.graph.cells[a as usize].distance(self.graph.cells[b as usize])
    }
}

#[derive(Clone)]
pub struct Game {
    pub board: Arc<Board>,
    pub players: usize,
    pub phase: Phase,
    pub back: Phase,                  // where a squall move or a project returns to (PreRoll or Main)
    pub current: u8,
    pub setup_step: u8,
    pub turn: u32,                    // 0 during setup, then 1, 2, ...
    pub corner_owner: Vec<u8>,
    pub corner_kind: Vec<u8>,
    pub edge_owner: Vec<u8>,
    pub edge_kind: Vec<u8>,
    pub hands: [Hand; MAX_PLAYERS],
    pub pieces: [[u8; 4]; MAX_PLAYERS],
    pub held: [[u8; 5]; MAX_PLAYERS], // projects held, by Project (Lighthouses count for Renown)
    pub bought: [u8; 5],              // projects the current player bought this turn
    pub project_used: bool,
    pub deck: Vec<u8>,                // the rest of the deck, top last
    pub row: Vec<u8>,                 // face-up projects (empty when face up is off)
    pub islet_owner: Vec<u8>,
    pub navigator: u8,
    pub harbourmaster: u8,
    pub line: [u8; MAX_PLAYERS],      // each player's longest line of Paths and Boats
    pub markets: [u8; MAX_PLAYERS],   // how many different markets each player touches
    pub squall: u16,
    pub discards: [u8; MAX_PLAYERS],  // cards each player still has to lose
    pub winds: u8,                    // Fair Winds' resource this turn, or NOBODY
    pub last_roll: u8,
    pub winner: u8,
    pub rng: Rng,
}

fn afford(hand: &Hand, cost: &Hand) -> bool {
    hand.iter().zip(cost).all(|(h, c)| h >= c)
}
fn pay(hand: &mut Hand, cost: &Hand) {
    for (h, c) in hand.iter_mut().zip(cost) {
        *h -= c;
    }
}

impl Game {
    // A new game for `players` seats (seat 0 places first), with its own random numbers from `seed`
    pub fn new(island: &Island, settings: &Settings, players: usize, seed: u32) -> Game {
        assert!((2..=MAX_PLAYERS).contains(&players), "2 to 4 players");
        let board = Arc::new(Board::new(island, settings));
        let mut rng = Rng::new(seed);
        let s = settings;
        let counts = [s.deck_pilot, s.deck_lighthouse, s.deck_shipwright, s.deck_harvest, s.deck_winds];
        let mut deck: Vec<u8> = (0..5u8).flat_map(|k| std::iter::repeat_n(k, counts[k as usize] as usize)).collect();
        rng.shuffle(&mut deck);
        let mut row = Vec::new();
        if s.projects_face_up {
            for _ in 0..s.projects_row {
                if let Some(k) = deck.pop() {
                    row.push(k);
                }
            }
        }
        let g = &board.graph;
        Game {
            players,
            phase: Phase::SetupHut,
            back: Phase::PreRoll,
            current: 0,
            setup_step: 0,
            turn: 0,
            corner_owner: vec![NOBODY; g.corners()],
            corner_kind: vec![0; g.corners()],
            edge_owner: vec![NOBODY; g.edges()],
            edge_kind: vec![0; g.edges()],
            hands: [[0; 5]; MAX_PLAYERS],
            pieces: [[s.pieces_path, s.pieces_boat, s.pieces_hut, s.pieces_lodge]; MAX_PLAYERS],
            held: [[0; 5]; MAX_PLAYERS],
            bought: [0; 5],
            project_used: false,
            deck,
            row,
            islet_owner: vec![NOBODY; board.islets],
            navigator: NOBODY,
            harbourmaster: NOBODY,
            line: [0; MAX_PLAYERS],
            markets: [0; MAX_PLAYERS],
            squall: island.squall,
            discards: [0; MAX_PLAYERS],
            winds: NOBODY,
            last_roll: 0,
            winner: NOBODY,
            rng,
            board,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.board.settings
    }

    // Who has to act now (during a 7's discards, the first player still over the limit)
    pub fn to_act(&self) -> u8 {
        if self.phase == Phase::Discard {
            (0..self.players).find(|&p| self.discards[p] > 0).unwrap_or(self.current as usize) as u8
        } else {
            self.current
        }
    }

    pub fn over(&self) -> bool {
        self.phase == Phase::Over
    }

    pub fn cards(&self, p: usize) -> u32 {
        self.hands[p].iter().map(|&n| n as u32).sum()
    }

    // ---- Renown ----

    pub fn huts(&self, p: usize) -> u8 {
        self.settings().pieces_hut - self.pieces[p][P_HUT]
    }
    pub fn lodges(&self, p: usize) -> u8 {
        self.settings().pieces_lodge - self.pieces[p][P_LODGE]
    }

    pub fn renown(&self, p: usize) -> u32 {
        let s = self.settings();
        let mut r = self.huts(p) as u32 + 2 * self.lodges(p) as u32 + self.held[p][Project::Lighthouse as usize] as u32;
        r += self.islet_owner.iter().filter(|&&o| o as usize == p).count() as u32 * s.islet_bonus as u32;
        if self.navigator as usize == p {
            r += s.navigator_renown as u32;
        }
        if self.harbourmaster as usize == p {
            r += s.harbourmaster_renown as u32;
        }
        r
    }

    // ---- What may go where ----

    // Free, on land, and no building on any neighbouring corner
    pub fn spaced(&self, c: usize) -> bool {
        let g = &self.board.graph;
        self.board.corner_land[c]
            && self.corner_owner[c] == NOBODY
            && listed(&g.corner_corners[c]).all(|n| self.corner_owner[n] == NOBODY)
    }

    // One of p's Paths or Boats touches this corner
    fn touches_own_edge(&self, p: u8, c: usize) -> bool {
        listed(&self.board.graph.corner_edges[c]).any(|e| self.edge_owner[e] == p)
    }

    // An edge p could extend to: free, and one of its ends is p's building, or is free of others'
    // buildings and touches p's Path or Boat
    fn edge_connected(&self, p: u8, e: usize) -> bool {
        self.edge_owner[e] == NOBODY
            && self.board.graph.edge_corners[e].iter().any(|&c| {
                let c = c as usize;
                self.corner_owner[c] == p || (self.corner_owner[c] == NOBODY && self.touches_own_edge(p, c))
            })
    }

    // The bank's rate for giving this resource: 4, 3 with a general market, 2 with its own market
    // or Fair Winds on it
    pub fn rate(&self, p: u8, r: usize) -> u8 {
        if self.winds as usize == r && self.current == p {
            return 2;
        }
        let mut best = 4;
        for c in 0..self.corner_owner.len() {
            if self.corner_owner[c] != p {
                continue;
            }
            let m = self.board.corner_market[c];
            if m == NOBODY {
                continue;
            }
            let kind = self.board.market_kind[m as usize];
            if kind == r as u8 {
                return 2;
            }
            if kind == ANY {
                best = 3;
            }
        }
        best
    }

    fn playable(&self, p: usize, k: Project) -> bool {
        k != Project::Lighthouse && !self.project_used && self.held[p][k as usize] > self.bought[k as usize]
    }

    // ---- Legal actions ----

    pub fn legal(&self, out: &mut Vec<Action>) {
        out.clear();
        let g = &self.board.graph;
        let p = self.to_act();
        let pu = p as usize;
        match self.phase {
            Phase::SetupHut => {
                for c in 0..g.corners() {
                    if self.board.corner_main[c] && self.spaced(c) {
                        out.push(Action::Hut(c as u16));
                    }
                }
            }
            Phase::SetupPath { hut } => {
                for e in listed(&g.corner_edges[hut as usize]) {
                    if self.edge_owner[e] != NOBODY {
                        continue;
                    }
                    if self.board.edge_land[e] && self.pieces[pu][P_PATH] > 0 {
                        out.push(Action::Path(e as u16));
                    }
                    if self.board.edge_sea[e] && self.pieces[pu][P_BOAT] > 0 {
                        out.push(Action::Boat(e as u16));
                    }
                }
            }
            Phase::PreRoll => {
                out.push(Action::Roll);
                self.legal_plays(pu, out);
            }
            Phase::Discard => {
                for r in 0..5 {
                    if self.hands[pu][r] > 0 {
                        out.push(Action::Discard(r as u8));
                    }
                }
            }
            Phase::MoveSquall { anywhere } => {
                let range = self.settings().squall_range as i32;
                for &h in &self.board.land_hexes {
                    if h != self.squall && (anywhere || self.board.hex_distance(h, self.squall) <= range) {
                        out.push(Action::Squall(h));
                    }
                }
            }
            Phase::Steal => {
                for v in self.victims() {
                    out.push(Action::Steal(v));
                }
            }
            Phase::Main => {
                let hand = &self.hands[pu];
                let pieces = &self.pieces[pu];
                if afford(hand, &LODGE_COST) && pieces[P_LODGE] > 0 {
                    for c in 0..g.corners() {
                        if self.corner_owner[c] == p && self.corner_kind[c] == HUT {
                            out.push(Action::Lodge(c as u16));
                        }
                    }
                }
                if afford(hand, &HUT_COST) && pieces[P_HUT] > 0 {
                    for c in 0..g.corners() {
                        if self.spaced(c) && self.touches_own_edge(p, c) {
                            out.push(Action::Hut(c as u16));
                        }
                    }
                }
                let path = afford(hand, &PATH_COST) && pieces[P_PATH] > 0;
                let boat = afford(hand, &BOAT_COST) && pieces[P_BOAT] > 0;
                if path || boat {
                    self.legal_edges(p, path, boat, out);
                }
                for give in 0..5 {
                    if hand[give] >= self.rate(p, give) {
                        for get in 0..5 {
                            if get != give {
                                out.push(Action::Trade(give as u8, get as u8));
                            }
                        }
                    }
                }
                if afford(hand, &PROJECT_COST) {
                    if self.settings().projects_face_up {
                        for slot in 0..self.row.len() {
                            out.push(Action::Buy(slot as u8));
                        }
                    } else if !self.deck.is_empty() {
                        out.push(Action::Buy(0));
                    }
                }
                self.legal_plays(pu, out);
                out.push(Action::EndTurn);
            }
            Phase::FreeBuild { .. } => {
                let p = self.current;
                self.legal_edges(p, self.pieces[p as usize][P_PATH] > 0, self.pieces[p as usize][P_BOAT] > 0, out);
            }
            Phase::Harvest { .. } | Phase::Winds => {
                for r in 0..5 {
                    out.push(Action::Take(r));
                }
            }
            Phase::Over => {}
        }
    }

    fn legal_edges(&self, p: u8, path: bool, boat: bool, out: &mut Vec<Action>) {
        for e in 0..self.edge_owner.len() {
            if !self.edge_connected(p, e) {
                continue;
            }
            if path && self.board.edge_land[e] {
                out.push(Action::Path(e as u16));
            }
            if boat && self.board.edge_sea[e] {
                out.push(Action::Boat(e as u16));
            }
        }
    }

    fn legal_plays(&self, p: usize, out: &mut Vec<Action>) {
        for k in [Project::Pilot, Project::Shipwright, Project::Harvest, Project::Winds] {
            if self.playable(p, k) {
                out.push(Action::Play(k));
            }
        }
    }

    // Players the current player can steal from at the squall's hex: others with a building there
    // and at least one card
    pub fn victims(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(3);
        for &c in &self.board.graph.hex_corners[self.squall as usize] {
            let o = self.corner_owner[c as usize];
            if o != NOBODY && o != self.current && self.cards(o as usize) > 0 && !out.contains(&o) {
                out.push(o);
            }
        }
        out
    }

    // ---- Doing things ----

    // Takes the action if it's legal now; otherwise changes nothing and says why
    pub fn apply(&mut self, a: Action) -> Result<(), String> {
        let mut list = Vec::new();
        self.legal(&mut list);
        if !list.contains(&a) {
            return Err(format!("{a:?} isn't allowed now ({:?})", self.phase));
        }
        self.apply_unchecked(a);
        Ok(())
    }

    // Takes an action the caller knows is legal (from legal()); for bots and simulations
    pub fn apply_unchecked(&mut self, a: Action) {
        let p = self.to_act();
        let pu = p as usize;
        match a {
            Action::Hut(c) => self.place_hut(p, c as usize),
            Action::Lodge(c) => {
                pay(&mut self.hands[pu], &LODGE_COST);
                self.corner_kind[c as usize] = LODGE;
                self.pieces[pu][P_LODGE] -= 1;
                self.pieces[pu][P_HUT] += 1;
            }
            Action::Path(e) => self.place_edge(p, e as usize, PATH),
            Action::Boat(e) => self.place_edge(p, e as usize, BOAT),
            Action::Roll => self.roll(),
            Action::Discard(r) => {
                self.hands[pu][r as usize] -= 1;
                self.discards[pu] -= 1;
                if self.discards.iter().all(|&d| d == 0) {
                    self.phase = Phase::MoveSquall { anywhere: false };
                }
            }
            Action::Squall(h) => {
                self.squall = h;
                let victims = self.victims();
                match victims.len() {
                    0 => self.phase = self.back,
                    1 => self.steal(victims[0]),
                    _ => self.phase = Phase::Steal,
                }
            }
            Action::Steal(v) => self.steal(v),
            Action::Trade(give, get) => {
                let rate = self.rate(p, give as usize);
                self.hands[pu][give as usize] -= rate;
                self.hands[pu][get as usize] += 1;
            }
            Action::Buy(slot) => {
                pay(&mut self.hands[pu], &PROJECT_COST);
                let k = if self.settings().projects_face_up {
                    let k = self.row.remove(slot as usize);
                    if let Some(next) = self.deck.pop() {
                        self.row.insert(slot as usize, next);
                    }
                    k
                } else {
                    self.deck.pop().unwrap()
                };
                self.held[pu][k as usize] += 1;
                self.bought[k as usize] += 1;
            }
            Action::Play(k) => {
                self.held[pu][k as usize] -= 1;
                self.project_used = true;
                self.back = self.phase;
                self.phase = match k {
                    Project::Pilot => Phase::MoveSquall { anywhere: true },
                    Project::Shipwright => Phase::FreeBuild { left: 2 },
                    Project::Harvest => Phase::Harvest { left: 2 },
                    Project::Winds => Phase::Winds,
                    Project::Lighthouse => unreachable!("Lighthouses aren't played"),
                };
                self.finish_free_build();
            }
            Action::Take(r) => match self.phase {
                Phase::Harvest { left } => {
                    self.hands[pu][r as usize] += 1;
                    self.phase = if left > 1 { Phase::Harvest { left: left - 1 } } else { self.back };
                }
                _ => {
                    self.winds = r;
                    self.phase = self.back;
                }
            },
            Action::EndTurn => {
                self.current = ((self.current as usize + 1) % self.players) as u8;
                self.turn += 1;
                self.bought = [0; 5];
                self.project_used = false;
                self.winds = NOBODY;
                self.phase = Phase::PreRoll;
            }
        }
        // You win the moment you have enough Renown during your own turn
        if self.turn > 0 && self.phase != Phase::Over && self.renown(self.current as usize) >= self.settings().win as u32 {
            self.winner = self.current;
            self.phase = Phase::Over;
        }
    }

    fn place_hut(&mut self, p: u8, c: usize) {
        let pu = p as usize;
        let setup = self.phase == Phase::SetupHut;
        if !setup {
            pay(&mut self.hands[pu], &HUT_COST);
        }
        self.corner_owner[c] = p;
        self.corner_kind[c] = HUT;
        self.pieces[pu][P_HUT] -= 1;
        let islet = self.board.corner_islet[c];
        if islet != NOBODY && self.islet_owner[islet as usize] == NOBODY {
            self.islet_owner[islet as usize] = p;
        }
        if setup {
            // The second round's Hut collects one of each resource around it
            if self.setup_step as usize >= self.players {
                for h in listed(&self.board.graph.corner_hexes[c]) {
                    let r = self.board.resource[h];
                    if r != NOBODY {
                        self.hands[pu][r as usize] += 1;
                    }
                }
            }
            self.phase = Phase::SetupPath { hut: c as u16 };
        }
        // A new building can break other players' lines, and may touch a market
        self.update_titles();
    }

    fn place_edge(&mut self, p: u8, e: usize, kind: u8) {
        let pu = p as usize;
        match self.phase {
            Phase::Main => pay(&mut self.hands[pu], if kind == PATH { &PATH_COST } else { &BOAT_COST }),
            Phase::FreeBuild { left } => self.phase = Phase::FreeBuild { left: left - 1 },
            _ => {}
        }
        self.edge_owner[e] = p;
        self.edge_kind[e] = kind;
        self.pieces[pu][if kind == PATH { P_PATH } else { P_BOAT }] -= 1;
        self.update_titles();
        if let Phase::SetupPath { .. } = self.phase {
            self.setup_step += 1;
            let n = self.players as u8;
            if self.setup_step == 2 * n {
                self.current = 0;
                self.turn = 1;
                self.phase = Phase::PreRoll;
            } else {
                self.current = if self.setup_step < n { self.setup_step } else { 2 * n - 1 - self.setup_step };
                self.phase = Phase::SetupHut;
            }
        }
        self.finish_free_build();
    }

    // Shipwright ends when both pieces are down, or when nothing more can be placed
    fn finish_free_build(&mut self) {
        if let Phase::FreeBuild { left } = self.phase {
            let mut list = Vec::new();
            if left > 0 {
                self.legal(&mut list);
            }
            if list.is_empty() {
                self.phase = self.back;
            }
        }
    }

    fn roll(&mut self) {
        let total = (self.rng.below(6) + self.rng.below(6) + 2) as u8;
        self.rolled(total);
    }

    // What a dice total does (separate from the dice so tests can choose the total)
    fn rolled(&mut self, total: u8) {
        self.last_roll = total;
        self.back = Phase::Main;
        if total == 7 {
            let over = self.settings().discard_over as u32;
            for p in 0..self.players {
                let n = self.cards(p);
                self.discards[p] = if n > over { (n / 2) as u8 } else { 0 };
            }
            self.phase = if self.discards.iter().any(|&d| d > 0) { Phase::Discard } else { Phase::MoveSquall { anywhere: false } };
            return;
        }
        self.produce(total);
        self.phase = Phase::Main;
    }

    // Every hex with this number (except under the squall) pays 1 per Hut and 2 per Lodge around it
    fn produce(&mut self, total: u8) {
        let g = &self.board.graph;
        for &h in &self.board.by_number[total as usize] {
            if h == self.squall {
                continue;
            }
            let r = self.board.resource[h as usize] as usize;
            for &c in &g.hex_corners[h as usize] {
                let o = self.corner_owner[c as usize];
                if o != NOBODY {
                    self.hands[o as usize][r] += self.corner_kind[c as usize];
                }
            }
        }
    }

    fn steal(&mut self, v: u8) {
        let n = self.cards(v as usize);
        if n > 0 {
            let mut pick = self.rng.below(n as usize) as u32;
            for r in 0..5 {
                let have = self.hands[v as usize][r] as u32;
                if pick < have {
                    self.hands[v as usize][r] -= 1;
                    self.hands[self.current as usize][r] += 1;
                    break;
                }
                pick -= have;
            }
        }
        self.phase = self.back;
    }

    // ---- Titles ----

    fn update_titles(&mut self) {
        let s = self.settings();
        let (nav_min, harbour_min) = (s.navigator_min, s.harbourmaster_min);
        for p in 0..self.players {
            self.line[p] = self.longest_line(p as u8);
            self.markets[p] = self.markets_touched(p as u8);
        }
        self.navigator = title(self.navigator, &self.line[..self.players], nav_min);
        self.harbourmaster = title(self.harbourmaster, &self.markets[..self.players], harbour_min);
    }

    // The longest unbroken line of p's Paths and Boats: no edge twice, and it can't pass through a
    // corner holding another player's building (it may end there)
    pub fn longest_line(&self, p: u8) -> u8 {
        let g = &self.board.graph;
        let mut used: Vec<u16> = Vec::with_capacity(32);
        let mut best = 0;
        for e in 0..self.edge_owner.len() {
            if self.edge_owner[e] != p {
                continue;
            }
            for &c in &g.edge_corners[e] {
                // Start each line at an end of one of p's edges
                best = best.max(self.extend_line(p, c as usize, &mut used, 0));
            }
        }
        best
    }

    fn extend_line(&self, p: u8, c: usize, used: &mut Vec<u16>, len: u8) -> u8 {
        let owner = self.corner_owner[c];
        if len > 0 && owner != NOBODY && owner != p {
            return len;
        }
        let g = &self.board.graph;
        let mut best = len;
        for e in listed(&g.corner_edges[c]) {
            if self.edge_owner[e] != p || used.contains(&(e as u16)) {
                continue;
            }
            let [a, b] = g.edge_corners[e];
            let next = if a as usize == c { b } else { a } as usize;
            used.push(e as u16);
            best = best.max(self.extend_line(p, next, used, len + 1));
            used.pop();
        }
        best
    }

    fn markets_touched(&self, p: u8) -> u8 {
        let mut seen = 0u64;
        for c in 0..self.corner_owner.len() {
            let m = self.board.corner_market[c];
            if self.corner_owner[c] == p && m != NOBODY {
                seen |= 1 << m;
            }
        }
        seen.count_ones() as u8
    }
}

// Who holds a title now: the holder keeps it while nobody has strictly more; otherwise the one player
// strictly ahead takes it; with a tie for the lead (or nobody at the minimum) nobody holds it
fn title(holder: u8, values: &[u8], min: u8) -> u8 {
    let best = *values.iter().max().unwrap();
    if best < min {
        return NOBODY;
    }
    if holder != NOBODY && values[holder as usize] == best {
        return holder;
    }
    let mut leaders = values.iter().enumerate().filter(|&(_, &v)| v == best);
    let first = leaders.next().unwrap().0 as u8;
    if leaders.next().is_some() { NOBODY } else { first }
}

// Total pips of the producing hexes around a corner (how often it produces), for bots and the page
pub fn corner_pips(board: &Board, c: usize, squall: u16) -> u32 {
    listed(&board.graph.corner_hexes[c])
        .filter(|&h| h as u16 != squall && board.resource[h] != NOBODY)
        .map(|h| pips(board.number[h]))
        .sum()
}

#[cfg(test)]
mod tests;
