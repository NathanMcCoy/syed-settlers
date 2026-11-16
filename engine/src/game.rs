// The game: everything in RULES.md from setup to a winner, as two operations that everything else
// uses (the page, the server, the bots and the simulations), so nobody can be allowed a different move:
//   game.legal(&mut list)  every action the player to act may take now
//   game.apply(action)     takes one (apply() checks it's legal; apply_unchecked() trusts the caller)
//
// Speed: the board (Board) is built once per game and shared, so copying a Game (bots looking ahead)
// copies only the small changing state. Nothing here searches through lists during play except a
// player's own Paths (Longest Path), at most a few dozen.
use crate::hex::{Graph, listed};
use crate::hex::NONE;
use crate::island::{ANY, Island, SEA, Terrain, pips};
use crate::rng::Rng;
use crate::settings::Settings;
use std::sync::Arc;

mod pending;
mod sea;
mod titles;
mod tokens;
mod events;
mod turns;
mod two;
mod variants;
pub use two::NEUTRAL;
pub use events::{EVENTS, Event, event_card};
pub use pending::{Decision, Pending};
pub use sea::{GOLD, P_SHIP, SHIP, SHIP_COST};
pub use titles::{TITLE_COUNT, TITLES, Title, title};
pub use tokens::{TOKEN_COUNT, TOKENS, Token};

pub const MAX_PLAYERS: usize = 6;
pub const NOBODY: u8 = u8::MAX;

// Resources, in this order everywhere: Timber, Stone, Grain, Copper, Fish
pub type Hand = [u8; 5];
pub const PATH_COST: Hand = [1, 0, 0, 1, 0];     // Timber + Copper
pub const HUT_COST: Hand = [1, 0, 1, 1, 1];      // Timber + Grain + Copper + Fish
pub const LODGE_COST: Hand = [0, 3, 2, 0, 0];    // 3 Stone + 2 Grain
pub const PROJECT_COST: Hand = [0, 1, 1, 0, 1];  // Stone + Grain + Fish

// What stands on a corner or an edge (0: nothing)
pub const HUT: u8 = 1;
pub const LODGE: u8 = 2;
pub const PATH: u8 = 1;

// A player's pieces still in hand, by this index
pub const P_PATH: usize = 0;
pub const P_HUT: usize = 1;
pub const P_LODGE: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum Project {
    Pilot,
    Lighthouse,
    RoadCrew,
    Harvest,
    MarketDay,
}
pub const PROJECTS: [Project; 5] = [Project::Pilot, Project::Lighthouse, Project::RoadCrew, Project::Harvest, Project::MarketDay];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Action {
    Hut(u16),        // build (or in setup, place) a Hut on a corner
    Ship(u16),       // the sea expansion: build (or in setup, place) a ship on a sea edge
    MoveShip(u16, u16), // move your ship from this edge to that one (once a turn)
    Pirate(u16),     // move the pirate to this sea hex (instead of the squall)
    Gift(u8, u8),    // event cards: give this player one card of this resource
    Rob(u8),         // event cards: take a random card from this player
    Damage(u16),     // event cards (an earthquake): damage your own Path on this edge
    Repair(u16),     // repair your damaged Path on this edge (Timber + Copper)
    NeutralHut(u8, u16),  // the two-player variant: a Hut for this neutral colour on this corner
    NeutralPath(u8, u16), // ... a Path for this neutral colour on this edge
    CashPilot,       // discard a played Pilot for 2 trade tokens
    ForceTrade,      // spend trade tokens: take 2 random cards from the other player, give 2 back
    SquallHome,      // spend trade tokens: send the squall to the Dunes
    Lodge(u16),      // upgrade your Hut on this corner
    Path(u16),       // on an edge
    Roll,
    Discard(u8),     // lose one card of this resource (after a 7)
    Squall(u16),     // move the squall to this hex
    Steal(u8),       // from this player
    Trade(u8, u8),   // give the bank/market this resource (at your best rate) for one of that
    Buy(u8),         // a project: face up, the row's slot; hidden, always 0
    Play(Project),
    Take(u8),        // choose a resource (Good Harvest, Market Day)
    EndTurn,
    // Trading between players (RULES.md "Trading"): the player whose turn it is offers cards for
    // cards to some of the others (`to`: one bit per seat); each answers; the offerer completes the
    // deal with one who accepted, or withdraws. Offers aren't in legal() (too many to list): see
    // offer_ok().
    // `open`: the first to accept trades at once, without the offerer choosing
    Offer { give: Hand, get: Hand, to: u8, open: bool },
    Answer(u8, bool), // this seat accepts (true) or declines the offer
    Deal(u8),         // trade with this seat, who accepted
    Withdraw,
    // A seat offered the trade proposes a changed one instead: what the OFFERER would give and get
    Counter { seat: u8, give: Hand, get: Hand },
    Accept(u8),       // the offerer takes this seat's counter-offer
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    SetupHut,
    SetupPath { hut: u16 },
    PreRoll,
    Decide,                     // decisions by one or more players (pending.rs), e.g. a 7's discards
    Rolled { total: u8 },       // only as `after`: produce this number once the decisions are made
    SecondRoll,                 // the two-player variant: roll again (a different number)
    MoveSquall { anywhere: bool },
    Steal,
    Main,
    FreeBuild { left: u8 },     // Road Crew
    Harvest { left: u8 },       // Good Harvest
    MarketDay,                  // Market Day: choose the resource everyone hands over
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
    pub corner_market: Vec<u8>,       // which market (index into markets), or NOBODY
    pub market_kind: Vec<u8>,         // per market: 0â€“4 (2:1) or ANY (3:1)
    pub edge_land: Vec<bool>,         // can take a Path
    pub edge_sea: Vec<bool>,          // touches the sea: can take a ship (the sea expansion)
    pub corner_region: Vec<u8>,       // which island the corner's land is on (0 main), or SEA
    pub sea_hexes: Vec<u16>,          // where the pirate may go
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
            resource: hexes.iter().map(|h| match h.terrain {
                Terrain::Gold => GOLD,
                t => t.resource().map_or(NOBODY, |r| r as u8),
            }).collect(),
            land_hexes: (0..hexes.len()).filter(|&h| land(h)).map(|h| h as u16).collect(),
            by_number,
            corner_land: (0..graph.corners()).map(|c| listed(&graph.corner_hexes[c]).any(land)).collect(),
            corner_market,
            market_kind: island.markets.iter().map(|m| m.kind).collect(),
            edge_land: (0..graph.edges()).map(|e| listed(&graph.edge_hexes[e]).any(land)).collect(),
            edge_sea: (0..graph.edges()).map(|e| listed(&graph.edge_hexes[e]).any(|h| !land(h))).collect(),
            corner_region: (0..graph.corners())
                .map(|c| listed(&graph.corner_hexes[c]).filter(|&h| land(h)).map(|h| hexes[h].region).min().unwrap_or(SEA))
                .collect(),
            sea_hexes: (0..hexes.len()).filter(|&h| !land(h)).map(|h| h as u16).collect(),
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
    pub pieces: [[u8; 4]; MAX_PLAYERS], // Paths, Huts, Lodges, ships still in hand (P_PATH, P_HUT, P_LODGE, P_SHIP)
    pub held: [[u8; 5]; MAX_PLAYERS], // projects held, by Project (Lighthouses count for Renown)
    pub bought: [u8; 5],              // projects the current player bought this turn
    pub project_used: bool,
    pub deck: Vec<u8>,                // the rest of the deck, top last
    pub row: Vec<u8>,                 // face-up projects (empty when face up is off)
    pub holders: [u8; TITLE_COUNT],   // who holds each title (titles.rs), or NOBODY
    pub tokens: [[u16; TOKEN_COUNT]; MAX_PLAYERS], // each player's tokens (tokens.rs)
    pub line: [u8; MAX_PLAYERS],      // each player's longest line of Paths
    pub pilots: [u8; MAX_PLAYERS],    // Pilots each player has played (everyone sees them)
    pub squall: u16,
    // The sea expansion (sea.rs): the pirate's hex (NONE without the sea), whether the steal now is
    // the pirate's, ships built this phase (they can't move), whether a ship moved this phase, and
    // the small islands each player has a Hut on (a bit per region)
    pub pirate: u16,
    pub at_sea: bool,
    pub ships_new: Vec<u16>,
    pub ship_moved: bool,
    pub islands_reached: [u16; MAX_PLAYERS],
    // Event cards (events.rs): the deck (top last), the last card's event (NOBODY before the first),
    // whether this production is an epidemic, and Paths an earthquake damaged
    pub event_deck: Vec<u8>,
    pub last_event: u8,
    pub epidemic: bool,
    pub damaged: Vec<bool>,
    // The two-player variant (two.rs): the first roll's number this turn, whether it's been rolled,
    // whether trade tokens were spent and a Pilot cashed in this turn
    pub first_roll: u8,
    pub rolled_once: bool,
    pub token_used: bool,
    pub pilot_cashed: bool,
    pub pending: Vec<Pending>,        // decisions waiting (pending.rs); the first is being made
    pub after: Phase,                 // where the game goes once they're all made
    pub last_take: [u8; MAX_PLAYERS], // what each player handed over on the last Market Day
    pub last_roll: u8,
    pub winner: u8,
    pub last_dice: [u8; 2],           // the last roll's two dice (0, 0 before the first)
    pub short: u8,                    // resources the bank couldn't pay on the last roll (a bit each)
    pub by_rounds: bool,              // the game ended at the round limit (the winner may be NOBODY: a draw)
    pub offer: Option<Offer>,         // the current player's open offer to other players
    pub offers_made: u8,              // this turn (counts up to 255; only a limit compares with it)
    // Paired turns (5-6 players, turns.paired): after the turn of `owner`, the player 3 seats on
    // (`current` then) takes an extra build phase (`paired`): Main without the roll or trading with
    // players. `owner` is whose turn it is; `current` is who's acting.
    pub owner: u8,
    pub paired: bool,
    pub rng: Rng,
}

// An offer to other players: what the offerer gives and wants, who it's to (a bit per seat), and
// each seat's answer (0 none yet, 1 accepted, 2 declined)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Offer {
    pub give: Hand,
    pub get: Hand,
    pub to: u8,
    pub open: bool,
    pub answers: [u8; MAX_PLAYERS],
    // Counter-offers, per seat: what the offerer would give and get instead
    pub counters: [Option<(Hand, Hand)>; MAX_PLAYERS],
}
pub const ACCEPTED: u8 = 1;
pub const DECLINED: u8 = 2;
pub const COUNTERED: u8 = 3;

// Is this a real trade: cards on both sides, no resource on both?
fn trade_shape_ok(give: &Hand, get: &Hand) -> Result<(), String> {
    if give.iter().all(|&n| n == 0) || get.iter().all(|&n| n == 0) {
        return Err("an offer needs cards on both sides".into());
    }
    if (0..5).any(|r| give[r] > 0 && get[r] > 0) {
        return Err("a resource can't be on both sides".into());
    }
    Ok(())
}

pub fn afford(hand: &Hand, cost: &Hand) -> bool {
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
        assert!((2..=MAX_PLAYERS).contains(&players), "2 to 6 players");
        let board = Arc::new(Board::new(island, settings));
        let mut rng = Rng::new(seed);
        let s = settings;
        let counts = [s.deck_pilot, s.deck_lighthouse, s.deck_road_crew, s.deck_harvest, s.deck_market_day];
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
        let mut g = Game {
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
            pieces: [[s.pieces_path, s.pieces_hut, s.pieces_lodge, if s.sea { s.pieces_ship } else { 0 }]; MAX_PLAYERS],
            held: [[0; 5]; MAX_PLAYERS],
            bought: [0; 5],
            project_used: false,
            deck,
            row,
            holders: [NOBODY; TITLE_COUNT],
            tokens: [[0; TOKEN_COUNT]; MAX_PLAYERS],
            line: [0; MAX_PLAYERS],
            pilots: [0; MAX_PLAYERS],
            squall: island.squall,
            pirate: island.pirate,
            at_sea: false,
            ships_new: Vec::new(),
            ship_moved: false,
            islands_reached: [0; MAX_PLAYERS],
            event_deck: Vec::new(),
            last_event: NOBODY,
            epidemic: false,
            damaged: vec![false; g.edges()],
            first_roll: 0,
            rolled_once: false,
            token_used: false,
            pilot_cashed: false,
            pending: Vec::new(),
            after: Phase::PreRoll,
            last_take: [0; MAX_PLAYERS],
            last_roll: 0,
            winner: NOBODY,
            last_dice: [0, 0],
            short: 0,
            by_rounds: false,
            offer: None,
            offers_made: 0,
            owner: 0,
            paired: false,
            rng,
            board,
        };
        if g.settings().variant_events {
            g.new_event_deck();
        }
        if g.settings().variant_two {
            g.place_neutrals();
        }
        g
    }

    // A game rebuilt from how it started and every move since (F14 in EXPANSIONS.md): the same island,
    // settings, seats and seed, then each (seat, action) in order. The dice and hidden draws come
    // from the game's own random numbers, so the result is exactly the game that was played.
    pub fn replay(island: &Island, settings: &Settings, players: usize, seed: u32, moves: &[(u8, Action)]) -> Result<Game, String> {
        let mut g = Game::new(island, settings, players, seed);
        for (i, &(seat, a)) in moves.iter().enumerate() {
            g.apply_as(seat, a).map_err(|e| format!("move {} ({a:?} by seat {seat}): {e}", i + 1))?;
        }
        Ok(g)
    }

    pub fn settings(&self) -> &Settings {
        &self.board.settings
    }

    // Who has to act now: the player making the first waiting decision (pending.rs), otherwise the
    // player whose turn or extra phase it is
    pub fn to_act(&self) -> u8 {
        match self.deciding() {
            Some(d) => d.seat,
            None => self.current,
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
        self.huts(p) as u32 + 2 * self.lodges(p) as u32 + self.held[p][Project::Lighthouse as usize] as u32
            + self.title_renown(p)
            + self.token_renown(p)
    }

    // ---- Winning (F10 in EXPANSIONS.md) ----

    // The Renown player p needs to win (rulesets will add modifiers here, e.g. +1 for an old boot)
    pub fn win_target(&self, _p: usize) -> u32 {
        let s = self.settings();
        s.win as u32 + s.variant_ports as u32
    }

    // Has p won? Enough Renown (rulesets will add their own ways here, e.g. finishing a wonder).
    // Only asked during p's own turn or extra phase.
    pub fn has_won(&self, p: usize) -> bool {
        self.renown(p) >= self.win_target(p)
    }

    // ---- What may go where ----

    // Free, on land, and no building on any neighbouring corner
    pub fn spaced(&self, c: usize) -> bool {
        let g = &self.board.graph;
        self.board.corner_land[c]
            && self.corner_owner[c] == NOBODY
            && listed(&g.corner_corners[c]).all(|n| self.corner_owner[n] == NOBODY)
    }

    // One of p's Paths touches this corner
    fn touches_own_edge(&self, p: u8, c: usize) -> bool {
        listed(&self.board.graph.corner_edges[c]).any(|e| self.edge_owner[e] == p)
    }

    // An edge p could extend to: free, and one of its ends is p's building, or is free of others'
    // buildings and touches p's Path (a Path can't continue past another player's building)
    fn edge_connected(&self, p: u8, e: usize) -> bool {
        self.edge_owner[e] == NOBODY
            && self.board.graph.edge_corners[e].iter().any(|&c| {
                let c = c as usize;
                self.corner_owner[c] == p || (self.corner_owner[c] == NOBODY && self.touches_own(p, c, PATH))
            })
    }

    // The bank's rate for giving this resource: 4, 3 with a general market, 2 with its own market
    pub fn rate(&self, p: u8, r: usize) -> u8 {
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
                let sea = self.sea();
                for c in 0..g.corners() {
                    if self.spaced(c) && (!sea || self.board.corner_region[c] == 0) {
                        out.push(Action::Hut(c as u16));
                    }
                }
            }
            Phase::SetupPath { hut } => {
                for e in listed(&g.corner_edges[hut as usize]) {
                    if self.edge_owner[e] == NOBODY && self.board.edge_land[e] && self.pieces[pu][P_PATH] > 0 {
                        out.push(Action::Path(e as u16));
                    }
                    if self.sea() && self.edge_owner[e] == NOBODY && self.board.edge_sea[e] && !self.by_pirate(e) && self.pieces[pu][P_SHIP] > 0 {
                        out.push(Action::Ship(e as u16));
                    }
                }
            }
            Phase::PreRoll => {
                out.push(Action::Roll);
                self.legal_plays(pu, out);
                self.legal_tokens(pu, out);
            }
            Phase::SecondRoll => out.push(Action::Roll),
            Phase::Decide => {
                if let Some(d) = self.deciding() {
                    self.legal_decision(d, out);
                }
            }
            Phase::MoveSquall { anywhere } => {
                // squall.range 0 (the default): any land hex after a 7 too
                let range = self.settings().squall_range as i32;
                let anywhere = anywhere || range == 0;
                let before = out.len();
                for &h in &self.board.land_hexes {
                    if h != self.squall && (anywhere || self.board.hex_distance(h, self.squall) <= range) && self.squall_allowed(h as usize) {
                        out.push(Action::Squall(h));
                    }
                }
                // Friendly squall with nowhere allowed: the Dunes
                if out.len() == before && self.settings().variant_friendly {
                    out.extend(self.squall_fallback().into_iter().map(Action::Squall));
                }
                if self.pirate != NONE {
                    for &h in &self.board.sea_hexes {
                        if h != self.pirate {
                            out.push(Action::Pirate(h));
                        }
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
                if afford(hand, &PATH_COST) && self.settings().variant_events {
                    for e in 0..self.edge_owner.len() {
                        if self.damaged[e] && self.edge_owner[e] == p {
                            out.push(Action::Repair(e as u16));
                        }
                    }
                }
                if afford(hand, &HUT_COST) && pieces[P_HUT] > 0 {
                    for c in 0..g.corners() {
                        if self.spaced(c) && self.touches_own_edge(p, c) && !self.by_damage(c) {
                            out.push(Action::Hut(c as u16));
                        }
                    }
                }
                if afford(hand, &PATH_COST) && pieces[P_PATH] > 0 {
                    self.legal_paths(p, out);
                }
                if afford(hand, &SHIP_COST) && pieces[P_SHIP] > 0 {
                    self.legal_ships(p, out);
                }
                if self.sea() && !self.ship_moved {
                    self.legal_ship_moves(p, out);
                }
                for give in 0..5 {
                    if hand[give] >= self.rate(p, give) {
                        for get in 0..5 {
                            if get != give && self.bank(get) > 0 {
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
                self.legal_tokens(pu, out);
                if let Some(o) = &self.offer {
                    out.push(Action::Withdraw);
                    for s in 0..self.players {
                        if o.answers[s] == ACCEPTED && self.deal_ok(s) {
                            out.push(Action::Deal(s as u8));
                        }
                        if self.counter_deal_ok(s) {
                            out.push(Action::Accept(s as u8));
                        }
                    }
                }
                out.push(Action::EndTurn);
            }
            Phase::FreeBuild { .. } => {
                // Road Crew: Paths, or with the sea, ships too (any mix of 2)
                let p = self.current;
                if self.pieces[p as usize][P_PATH] > 0 {
                    self.legal_paths(p, out);
                }
                if self.pieces[p as usize][P_SHIP] > 0 {
                    self.legal_ships(p, out);
                }
            }
            Phase::Harvest { .. } => {
                for r in 0..5 {
                    if self.bank(r as usize) > 0 {
                        out.push(Action::Take(r));
                    }
                }
            }
            Phase::MarketDay => {
                for r in 0..5 {
                    out.push(Action::Take(r));
                }
            }
            Phase::Over | Phase::Rolled { .. } => {}
        }
    }

    fn legal_paths(&self, p: u8, out: &mut Vec<Action>) {
        if self.paths_blocked(p) {
            return; // an earthquake: repair first
        }
        for e in 0..self.edge_owner.len() {
            if self.board.edge_land[e] && self.edge_connected(p, e) {
                out.push(Action::Path(e as u16));
            }
        }
    }

    // Every action `seat` may take now: the player to act's legal(), plus answers to an open offer
    // made to this seat (others answer while it's someone else's turn)
    pub fn legal_for(&self, seat: u8, out: &mut Vec<Action>) {
        if seat == self.to_act() {
            self.legal(out);
        } else {
            out.clear();
        }
        if let Some(o) = &self.offer {
            let s = seat as usize;
            if seat != self.current && s < self.players && o.to & (1 << s) != 0 && o.answers[s] == 0 && !self.over() {
                // Accepting an open offer trades at once, so the offerer must still hold the cards too
                if afford(&self.hands[s], &o.get) && (!o.open || self.deal_ok(s)) {
                    out.push(Action::Answer(seat, true));
                }
                out.push(Action::Answer(seat, false));
            }
        }
    }

    // May the current player make this offer now? Why not, if not.
    pub fn offer_ok(&self, give: &Hand, get: &Hand, to: u8) -> Result<(), String> {
        if self.phase != Phase::Main {
            return Err("offers are made after rolling".into());
        }
        if !self.settings().trade_players {
            return Err("trading between players is off in this game".into());
        }
        if self.paired {
            return Err("no trading with players in the extra build phase (the bank and markets are fine)".into());
        }
        if !self.offers_left() {
            return Err(format!("{} offers a turn at most", self.settings().trade_offers));
        }
        trade_shape_ok(give, get)?;
        if !afford(&self.hands[self.current as usize], give) {
            return Err("you don't have those cards".into());
        }
        let others: u8 = (0..self.players as u8).filter(|&s| s != self.current).map(|s| 1 << s).sum();
        if to == 0 || to & !others != 0 {
            return Err("offer to one or more of the other players".into());
        }
        Ok(())
    }

    // May the current player make another offer this turn? (trade.offers 0: no limit; never in a
    // paired player's extra build phase)
    pub fn offers_left(&self) -> bool {
        let s = self.settings();
        s.trade_players && !self.paired && (s.trade_offers == 0 || self.offers_made < s.trade_offers)
    }

    // Do paired turns apply to this game? (5 or 6 players, turns.paired on)
    pub fn pairs(&self) -> bool {
        self.players >= 5 && self.settings().turns_paired
    }

    // The player who takes the extra build phase after `owner`'s turn: 3 seats on
    pub fn partner(&self, owner: u8) -> u8 {
        ((owner as usize + 3) % self.players) as u8
    }

    // Can the open offer still be completed with seat s (both still hold the cards)?
    fn deal_ok(&self, s: usize) -> bool {
        self.offer.is_some_and(|o| afford(&self.hands[self.current as usize], &o.give) && afford(&self.hands[s], &o.get))
    }

    // May `seat` send this counter-offer (what the offerer would give and get) now? Why not, if not.
    pub fn counter_ok(&self, seat: u8, give: &Hand, get: &Hand) -> Result<(), String> {
        let Some(o) = &self.offer else { return Err("there's no offer to answer".into()) };
        let s = seat as usize;
        if seat == self.current || s >= self.players || o.to & (1 << s) == 0 {
            return Err("only a player offered the trade can counter it".into());
        }
        if o.answers[s] == ACCEPTED {
            return Err("you've already accepted".into());
        }
        trade_shape_ok(give, get)?;
        if !afford(&self.hands[s], get) {
            return Err("you don't have those cards".into());
        }
        Ok(())
    }

    // Can the offerer still take seat s's counter-offer (both hold the cards)?
    fn counter_deal_ok(&self, s: usize) -> bool {
        self.offer.is_some_and(|o| {
            o.counters[s].is_some_and(|(give, get)| afford(&self.hands[self.current as usize], &give) && afford(&self.hands[s], &get))
        })
    }

    // The offerer gives `give` to seat s and gets `get` from them; the offer ends
    fn swap(&mut self, s: usize, give: Hand, get: Hand) {
        let me = self.current as usize;
        pay(&mut self.hands[me], &give);
        pay(&mut self.hands[s], &get);
        for r in 0..5 {
            self.hands[me][r] += get[r];
            self.hands[s][r] += give[r];
        }
        self.offer = None;
    }

    fn legal_plays(&self, p: usize, out: &mut Vec<Action>) {
        for k in [Project::Pilot, Project::RoadCrew, Project::Harvest, Project::MarketDay] {
            if self.playable(p, k) {
                out.push(Action::Play(k));
            }
        }
    }

    // Players the current player can steal from at the squall's hex: others with a building there
    // and at least one card
    pub fn victims(&self) -> Vec<u8> {
        if self.at_sea {
            return self.pirate_victims();
        }
        let mut out = Vec::with_capacity(3);
        for &c in &self.board.graph.hex_corners[self.squall as usize] {
            let o = self.corner_owner[c as usize];
            if o != NOBODY && o != self.current && self.cards(o as usize) > 0 && !out.contains(&o) && !self.protected(o as usize) {
                out.push(o);
            }
        }
        out
    }

    // ---- Doing things ----

    // Takes the action if it's legal now; otherwise changes nothing and says why
    pub fn apply(&mut self, a: Action) -> Result<(), String> {
        self.apply_as(self.to_act(), a)
    }

    // The same, for an action by `seat` (answers to offers come from seats other than the one to act)
    pub fn apply_as(&mut self, seat: u8, a: Action) -> Result<(), String> {
        if let Action::Offer { give, get, to, .. } = a {
            if seat != self.current || self.to_act() != seat {
                return Err("it isn't your turn".into());
            }
            self.offer_ok(&give, &get, to)?;
        } else if let Action::Counter { seat: s, give, get } = a {
            if s != seat {
                return Err("nobody counters for someone else".into());
            }
            self.counter_ok(seat, &give, &get)?;
        } else {
            let mut list = Vec::new();
            self.legal_for(seat, &mut list);
            if !list.contains(&a) {
                return Err(format!("{a:?} isn't allowed now ({:?})", self.phase));
            }
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
                self.refresh_ports();
            }
            Action::Path(e) => self.place_edge(p, e as usize, PATH),
            Action::Ship(e) => self.place_edge(p, e as usize, SHIP),
            Action::MoveShip(from, to) => {
                let (from, to) = (from as usize, to as usize);
                self.edge_owner[from] = NOBODY;
                self.edge_kind[from] = 0;
                self.edge_owner[to] = p;
                self.edge_kind[to] = SHIP;
                self.ship_moved = true;
                self.update_titles();
            }
            Action::Pirate(h) => {
                self.pirate = h;
                self.at_sea = true;
                let victims = self.victims();
                match victims.len() {
                    0 => self.phase = self.back,
                    1 => self.steal(victims[0]),
                    _ => self.phase = Phase::Steal,
                }
            }
            Action::Roll if self.phase == Phase::SecondRoll => {
                // The two-player variant's second roll: a number different from the first
                let mut total = self.production_roll();
                while total == self.first_roll {
                    total = self.production_roll();
                }
                self.rolled_once = true;
                self.rolled(total);
            }
            Action::Roll if self.two() => {
                let total = self.production_roll();
                self.first_roll = total;
                self.rolled(total);
            }
            Action::NeutralHut(n, c) => {
                self.corner_owner[c as usize] = n;
                self.corner_kind[c as usize] = HUT;
                self.update_titles();
                self.decided(None);
            }
            Action::NeutralPath(n, e) => {
                self.edge_owner[e as usize] = n;
                self.edge_kind[e as usize] = PATH;
                self.update_titles();
                self.decided(None);
            }
            Action::CashPilot => {
                self.pilots[pu] -= 1;
                self.pilot_cashed = true;
                self.give_tokens(pu, Token::Trade, 2);
                self.refresh_title(Title::MostPilots);
            }
            Action::ForceTrade => self.force_trade(pu),
            Action::SquallHome => {
                let cost = self.token_cost(pu);
                self.spend_tokens(pu, Token::Trade, cost);
                self.token_used = true;
                if let Some(&h) = self.board.land_hexes.iter().find(|&&h| self.board.terrain[h as usize] == Terrain::Dunes) {
                    self.squall = h;
                }
            }
            Action::Roll => {
                if self.settings().variant_events {
                    self.draw_event();
                } else {
                    let total = self.production_roll();
                    self.rolled(total);
                }
            }
            Action::Gift(to, r) => {
                self.hands[pu][r as usize] -= 1;
                self.hands[to as usize][r as usize] += 1;
                self.decided(None);
            }
            Action::Rob(v) => {
                self.rob(pu, v as usize);
                self.decided(None);
            }
            Action::Damage(e) => {
                self.damaged[e as usize] = true;
                self.decided(None);
            }
            Action::Repair(e) => {
                pay(&mut self.hands[pu], &PATH_COST);
                self.damaged[e as usize] = false;
            }
            Action::Discard(r) => {
                self.hands[pu][r as usize] -= 1;
                let Some(Pending { decision: Decision::Discard { left }, .. }) = self.deciding() else { unreachable!("a discard is decided") };
                self.decided(if left > 1 { Some(Decision::Discard { left: left - 1 }) } else { None });
            }
            Action::Squall(h) => {
                self.squall = h;
                self.at_sea = false;
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
                    Project::Pilot => {
                        self.pilots[pu] += 1;
                        self.refresh_title(Title::MostPilots);
                        Phase::MoveSquall { anywhere: true }
                    }
                    Project::RoadCrew => Phase::FreeBuild { left: 2 },
                    Project::Harvest => Phase::Harvest { left: 2 },
                    Project::MarketDay => Phase::MarketDay,
                    Project::Lighthouse => unreachable!("Lighthouses aren't played"),
                };
                self.finish_free_build();
                self.finish_harvest();
            }
            Action::Take(r) => match self.phase {
                Phase::Decide => {
                    // A gold field or an event: one resource of the player's choice
                    self.hands[pu][r as usize] += 1;
                    let next = match self.deciding().map(|d| d.decision) {
                        Some(Decision::Gold { left }) if left > 1 => Some(Decision::Gold { left: left - 1 }),
                        Some(Decision::Pick { left }) if left > 1 => Some(Decision::Pick { left: left - 1 }),
                        _ => None,
                    };
                    self.decided(next);
                }
                Phase::Harvest { left } => {
                    self.hands[pu][r as usize] += 1;
                    self.phase = if left > 1 { Phase::Harvest { left: left - 1 } } else { self.back };
                    self.finish_harvest();
                }
                _ => {
                    // Market Day: every other player hands over all of this resource
                    let r = r as usize;
                    for o in 0..self.players {
                        let n = if o == pu { 0 } else { self.hands[o][r] };
                        self.last_take[o] = n;
                        self.hands[o][r] -= n;
                        self.hands[pu][r] += n;
                    }
                    self.phase = self.back;
                }
            },
            Action::Offer { give, get, to, open } => {
                self.offer = Some(Offer { give, get, to, open, answers: [0; MAX_PLAYERS], counters: [None; MAX_PLAYERS] });
                self.offers_made = self.offers_made.saturating_add(1);
            }
            Action::Answer(s, yes) => {
                let o = self.offer.as_mut().expect("an offer");
                o.answers[s as usize] = if yes { ACCEPTED } else { DECLINED };
                // An open offer goes to the first who accepts, at once
                if yes && o.open {
                    let (give, get) = (o.give, o.get);
                    self.swap(s as usize, give, get);
                }
            }
            Action::Deal(s) => {
                let o = self.offer.expect("an offer");
                self.swap(s as usize, o.give, o.get);
            }
            Action::Counter { seat, give, get } => {
                let o = self.offer.as_mut().expect("an offer");
                o.counters[seat as usize] = Some((give, get));
                o.answers[seat as usize] = COUNTERED;
            }
            Action::Accept(s) => {
                let (give, get) = self.offer.expect("an offer").counters[s as usize].expect("a counter-offer");
                self.swap(s as usize, give, get);
            }
            Action::Withdraw => self.offer = None,
            Action::EndTurn => {
                self.end_phase();
                if self.over() {
                    return;
                }
            }
        }
        // You win the moment you have enough Renown during your own turn
        if self.turn > 0 && self.phase != Phase::Over && self.has_won(self.current as usize) {
            self.winner = self.current;
            self.phase = Phase::Over;
            self.offer = None;
        }
    }

    // The round limit is reached: the most Renown wins; a tie goes to more Lodges; still level, a
    // draw (winner NOBODY)
    fn end_by_rounds(&mut self) {
        let score = |p: usize| (self.renown(p), self.lodges(p));
        let best = (0..self.players).map(score).max().unwrap();
        let leaders: Vec<usize> = (0..self.players).filter(|&p| score(p) == best).collect();
        self.winner = if leaders.len() == 1 { leaders[0] as u8 } else { NOBODY };
        self.by_rounds = true;
        self.phase = Phase::Over;
        self.offer = None;
    }

    // The round being played (1 for the first; 0 during setup)
    pub fn round(&self) -> u32 {
        if self.turn == 0 { 0 } else { (self.turn - 1) / self.players as u32 + 1 }
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
        let mut gold = [0u8; MAX_PLAYERS];
        if setup {
            // The second round's Hut collects one of each resource around it (a gold field: one of
            // the player's choice)
            if self.setup_step as usize >= self.players {
                for h in listed(&self.board.graph.corner_hexes[c]) {
                    let r = self.board.resource[h];
                    if r == GOLD {
                        gold[pu] += 1;
                    } else if r != NOBODY && self.bank(r as usize) > 0 {
                        self.hands[pu][r as usize] += 1;
                    }
                }
            }
            let decisions = self.gold_decisions(&gold);
            self.decide_then(decisions, Phase::SetupPath { hut: c as u16 });
            self.hut_tokens(pu, c);
        } else {
            self.island_bonus(pu, c);
            self.hut_tokens(pu, c);
        }
        // A new building can break other players' lines
        self.update_titles();
        self.refresh_ports();
        // The two-player variant: a Hut built owes a neutral Hut (or Path)
        if !setup {
            if let Some(d) = self.owe_neutral(p, true) {
                let after = self.phase;
                self.decide_then([d], after);
            }
        }
    }

    // A Path or a ship (kind PATH or SHIP): bought in the main phase, free with Road Crew or in setup
    fn place_edge(&mut self, p: u8, e: usize, kind: u8) {
        let pu = p as usize;
        let (cost, piece) = if kind == SHIP { (SHIP_COST, P_SHIP) } else { (PATH_COST, P_PATH) };
        let built = matches!(self.phase, Phase::Main | Phase::FreeBuild { .. });
        match self.phase {
            Phase::Main => pay(&mut self.hands[pu], &cost),
            Phase::FreeBuild { left } => self.phase = Phase::FreeBuild { left: left - 1 },
            _ => {}
        }
        if kind == SHIP && !matches!(self.phase, Phase::SetupPath { .. }) {
            self.ships_new.push(e as u16);
        }
        self.edge_owner[e] = p;
        self.edge_kind[e] = kind;
        self.pieces[pu][piece] -= 1;
        self.update_titles();
        if let Phase::SetupPath { .. } = self.phase {
            self.setup_step += 1;
            let n = self.players as u8;
            if self.setup_step == 2 * n {
                self.current = 0;
                self.owner = 0;
                self.turn = 1;
                self.phase = Phase::PreRoll;
            } else {
                self.current = if self.setup_step < n { self.setup_step } else { 2 * n - 1 - self.setup_step };
                self.phase = Phase::SetupHut;
            }
        }
        self.finish_free_build();
        // The two-player variant: a Path built owes a neutral Path
        if kind == PATH && built {
            if let Some(d) = self.owe_neutral(p, false) {
                let after = self.phase;
                self.decide_then([d], after);
            }
        }
    }

    // Road Crew ends when both Paths are down, or when nothing more can be placed
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



    // Every hex with this number (except under the squall) pays 1 per Hut and 2 per Lodge around it
    // With a limited bank, a resource the bank can't pay in full to everyone owed it goes to nobody
    // (that roll), unless only one player is owed it: they get what's left. `short` records the
    // resources not paid in full, for the page.
    // Returns the gold each player is owed (a resource of their choice each: pending.rs decides them)
    pub(crate) fn produce(&mut self, total: u8) -> [u8; MAX_PLAYERS] {
        let g = &self.board.graph;
        let mut owed = [[0u8; 5]; MAX_PLAYERS];
        let mut gold = [0u8; MAX_PLAYERS];
        for &h in &self.board.by_number[total as usize] {
            if h == self.squall {
                continue;
            }
            let r = self.board.resource[h as usize] as usize;
            for &c in &g.hex_corners[h as usize] {
                let o = self.corner_owner[c as usize];
                if o != NOBODY && (o as usize) < self.players {
                    // An epidemic (event cards): Lodges produce 1, not 2
                    let n = if self.epidemic { 1 } else { self.corner_kind[c as usize] };
                    if r == GOLD as usize {
                        gold[o as usize] += n;
                    } else {
                        owed[o as usize][r] += n;
                    }
                }
            }
        }
        self.short = 0;
        for r in 0..5 {
            let wanted: u32 = owed.iter().map(|o| o[r] as u32).sum();
            let left = self.bank(r);
            if wanted > left {
                self.short |= 1 << r;
                let mut owed_to = (0..self.players).filter(|&p| owed[p][r] > 0);
                if let (Some(only), None) = (owed_to.next(), owed_to.next()) {
                    self.hands[only][r] += left as u8;
                }
                continue;
            }
            for p in 0..self.players {
                self.hands[p][r] += owed[p][r];
            }
        }
        gold
    }

    // Cards of a resource left in the bank: the game's total less what players hold (u32::MAX when
    // the bank never runs out)
    pub fn bank(&self, r: usize) -> u32 {
        let cap = self.settings().bank_cards as u32;
        if cap == 0 {
            return u32::MAX;
        }
        cap.saturating_sub((0..self.players).map(|p| self.hands[p][r] as u32).sum())
    }

    // Good Harvest ends early when the bank has nothing left to take
    fn finish_harvest(&mut self) {
        if let Phase::Harvest { .. } = self.phase {
            if (0..5).all(|r| self.bank(r) == 0) {
                self.phase = self.back;
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

    // Longest Path, after a Path or a building is placed (Most Pilots changes only when a Pilot is
    // played)
    fn update_titles(&mut self) {
        for p in 0..self.title_owners() {
            self.line[p] = self.longest_line(p as u8);
        }
        self.refresh_title(Title::LongestPath);
    }

    // The longest unbroken line of p's Paths: no edge twice, and it can't pass through a corner
    // holding another player's building (it may end there)
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
                best = best.max(self.extend_line(p, c as usize, &mut used, 0, 0));
            }
        }
        best
    }

    // `came`: the kind of the edge the line arrived by (0 at the start). Paths and ships only join
    // at one of p's own buildings (the sea expansion).
    fn extend_line(&self, p: u8, c: usize, used: &mut Vec<u16>, len: u8, came: u8) -> u8 {
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
            if came != 0 && self.edge_kind[e] != came && owner != p {
                continue;
            }
            let [a, b] = g.edge_corners[e];
            let next = if a as usize == c { b } else { a } as usize;
            used.push(e as u16);
            best = best.max(self.extend_line(p, next, used, len + 1, self.edge_kind[e]));
            used.pop();
        }
        best
    }

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
