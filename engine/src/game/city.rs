// The city expansion (`city=true`; RULES.md "The city expansion"). Lodges make commodities (Paper on
// Forest, Cloth on Paddy, Coin on Quarry: 1 resource + 1 commodity; Fields and Mines 2 of their
// resource), commodities buy improvements on three tracks (Science: Paper, Trade: Cloth, Politics:
// Coin), and the event die brings raiders and progress cards. No projects and no Most Pilots; win
// at 13.
//
// - Walls (2 Copper, under a Lodge, at most 3): +2 to the hand limit each.
// - Improvements: level n costs n of the track's commodity; needs a Lodge. Level 3: Science gives a
//   resource of your choice when a roll gives you nothing (not a 7), Trade trades commodities 2:1,
//   Politics may promote to mighty knights. The first to level 4 takes the track's metropolis
//   (+2 Renown, can't be pillaged); the first to 5 takes it for good.
// - Knights: 2 basic, 2 strong, 2 mighty each. Recruit (Fish + Stone; a basic knight, inactive, on a
//   free corner by your Path, no spacing rule), promote (Fish + Stone, once per knight per turn),
//   activate (Grain). An active knight that was active when the turn began may move along your routes,
//   displace a weaker knight of another player (who moves theirs), or chase the squall from a corner of
//   its hex; each action makes it inactive. Knights block Paths, Hut spots and others' Longest Path.
// - Raiders: the event die's 3 ship faces move them 1 step; at the end of the track (city.raiders,
//   7) they land: strength = Lodges on the board, defence = active knights' levels. Raiders stronger:
//   the lowest contributors with a Lodge that can be pillaged each lose one (it becomes a Hut, its
//   wall goes). Defence as strong or stronger: the single best contributor gets a Renown token; tied
//   best each draw a progress card of their choice. Then every knight is inactive and the raiders go
//   back. Before the first landing the squall (and the pirate) is off the board: a 7 only discards.
// - Progress cards: the event die's 3 gate faces with the red die: everyone whose level on that track
//   is at least 1 and red die <= level + 1 draws one. At most 4 in hand (off your turn: drop at once;
//   on it: by the end of the turn). Played on your turn after the roll, any number; Fixed Roll only
//   before rolling; Renown cards (Printing Press, Charter) are shown at once.
//
// Simplified (RULES.md): a metropolis goes on your Lodge with the best dice; Treason places your
// strongest knight allowed; a pillaged Lodge with no Hut left in hand becomes a Hut anyway.
use super::*;

pub const PAPER: usize = 5;
pub const CLOTH: usize = 6;
pub const COIN: usize = 7;
pub const COMMODITIES: [&str; 3] = ["Paper", "Cloth", "Coin"];
pub const TRACKS: [&str; 3] = ["science", "trade", "politics"];

pub const WALL_COST: Hand = [0, 0, 0, 2, 0];       // 2 Copper
pub const KNIGHT_COST: Hand = [0, 1, 0, 0, 1];     // Fish + Stone (recruit or promote)
pub const ACTIVATE_COST: Hand = [0, 0, 1, 0, 0];   // Grain
pub const MEDICINE_COST: Hand = [0, 2, 1, 0, 0];   // a Lodge with Medicine: 2 Stone + 1 Grain

// Progress cards: (name, deck: 0 science, 1 trade, 2 politics, copies)
pub const CARDS: [(&str, u8, u8); 25] = [
    ("fixedRoll", 0, 2),      // before rolling: choose the production dice
    ("crane", 0, 2),          // the next improvement costs 1 commodity less
    ("engineer", 0, 1),       // a wall for free
    ("numberSwap", 0, 2),     // swap two number discs (not 2, 6, 8, 12)
    ("irrigation", 0, 2),     // 2 Grain for each Fields hex by your buildings
    ("medicine", 0, 2),       // a Lodge for 2 Stone + 1 Grain
    ("quarrying", 0, 2),      // 2 Stone for each Quarry hex by your buildings
    ("printingPress", 0, 1),  // 1 Renown, shown at once
    ("pathBuilders", 0, 2),   // 2 Paths (or ships) for free
    ("smithing", 0, 2),       // promote up to 2 knights for free
    ("harbourDeal", 1, 2),    // offer each player a resource; they give a commodity of their choice
    ("guildDues", 1, 2),      // take 2 cards of your choice from a player with more Renown
    ("merchant", 1, 6),       // place the merchant by your building: 1 Renown, its resource 2:1
    ("tradeFleet", 1, 2),     // name a card kind: 2:1 with the bank this turn
    ("monopoly", 1, 4),       // name a resource: everyone gives you 2 (or their 1)
    ("goodsMonopoly", 1, 2),  // name a commodity: everyone gives you 1
    ("charter", 2, 1),        // 1 Renown, shown at once
    ("diplomacy", 2, 2),      // remove an open Path (your own: build one free)
    ("rally", 2, 2),          // activate all your knights for free
    ("spy", 2, 3),            // look at a player's progress cards and take one
    ("intrigue", 2, 2),       // push off another player's knight touching your routes
    ("taxes", 2, 2),          // move the squall and take a card from everyone there (after the first landing)
    ("treason", 2, 2),        // a player removes a knight; you may place one of yours no stronger
    ("wedding", 2, 2),        // everyone with more Renown gives you 2 cards of their choice
    ("sabotage", 2, 2),       // everyone with as much Renown or more discards half
];
pub const FIXED_ROLL: u8 = 0;
pub const CRANE: u8 = 1;
pub const ENGINEER: u8 = 2;
pub const NUMBER_SWAP: u8 = 3;
pub const IRRIGATION: u8 = 4;
pub const MEDICINE: u8 = 5;
pub const QUARRYING: u8 = 6;
pub const PRINTING: u8 = 7;
pub const PATH_BUILDERS: u8 = 8;
pub const SMITHING: u8 = 9;
pub const HARBOUR: u8 = 10;
pub const GUILD_DUES: u8 = 11;
pub const MERCHANT: u8 = 12;
pub const TRADE_FLEET: u8 = 13;
pub const MONOPOLY: u8 = 14;
pub const GOODS_MONOPOLY: u8 = 15;
pub const CHARTER: u8 = 16;
pub const DIPLOMACY: u8 = 17;
pub const RALLY: u8 = 18;
pub const SPY: u8 = 19;
pub const INTRIGUE: u8 = 20;
pub const TAXES: u8 = 21;
pub const TREASON: u8 = 22;
pub const WEDDING: u8 = 23;
pub const SABOTAGE: u8 = 24;

pub fn renown_card(k: u8) -> bool {
    k == PRINTING || k == CHARTER
}

// Everything the city expansion adds to a game (empty without it)
#[derive(Clone, Default)]
pub struct CityState {
    pub walls: Vec<bool>,                 // per corner
    pub levels: [[u8; 3]; MAX_PLAYERS],   // each player's level on each track (0-5)
    pub metro: [u16; 3],                  // the corner holding each track's metropolis (NONE)
    pub metro_owner: [u8; 3],
    // Knights, per corner: owner (NOBODY), level 1-3, active; activated this turn (can't act yet);
    // promoted this turn
    pub knight_owner: Vec<u8>,
    pub knight_level: Vec<u8>,
    pub knight_active: Vec<bool>,
    pub knight_fresh: Vec<bool>,
    pub promoted: Vec<bool>,
    pub raiders: u8,                      // steps taken toward landing
    pub landed: bool,                     // the raiders have landed at least once (the squall is in play)
    pub last_raid: [u8; 3],               // the last landing: [strength, defence, 1 if the raiders won] (0s: none yet)
    pub event_die: u8,                    // the last event die: 0-2 ship, 3 science, 4 trade, 5 politics (NOBODY: none)
    pub decks: [Vec<u8>; 3],              // top last
    pub progress: [Vec<u8>; MAX_PLAYERS], // progress cards held (hidden from others)
    pub card_renown: [u8; MAX_PLAYERS],   // Renown cards shown
    pub merchant: u16,
    pub merchant_owner: u8,
    pub crane: bool,                      // the next improvement is 1 cheaper (this turn)
    pub medicine: bool,                   // the next Lodge costs 2 Stone + 1 Grain (this turn)
    pub fleet: u8,                        // card kind traded 2:1 this turn (NOBODY: none)
    pub tax: bool,                        // the squall move under way is Taxes
    pub next_dice: Option<[u8; 2]>,       // Fixed Roll's production dice
    pub hut_debt: [u8; MAX_PLAYERS],      // Huts on the board beyond the pieces (pillaged with none in hand)
    pub squall_home: u16,                 // where the squall goes at the first landing
    pub pirate_home: u16,
    pub drawn: Vec<(u8, u8)>,             // the last roll's progress draws (seat, deck), for the page
}

impl Game {
    pub fn city(&self) -> bool {
        self.settings().city
    }

    // Set up the city expansion's pieces (Game::new)
    pub(super) fn city_setup(&mut self, rng_seed_shuffle: bool) {
        let corners = self.board.graph.corners();
        let cs = &mut self.cs;
        cs.walls = vec![false; corners];
        cs.knight_owner = vec![NOBODY; corners];
        cs.knight_level = vec![0; corners];
        cs.knight_active = vec![false; corners];
        cs.knight_fresh = vec![false; corners];
        cs.promoted = vec![false; corners];
        cs.metro = [NONE; 3];
        cs.metro_owner = [NOBODY; 3];
        cs.event_die = NOBODY;
        cs.merchant = NONE;
        cs.merchant_owner = NOBODY;
        cs.fleet = NOBODY;
        cs.squall_home = self.squall;
        cs.pirate_home = self.pirate;
        for (k, &(_, deck, n)) in CARDS.iter().enumerate() {
            for _ in 0..n {
                cs.decks[deck as usize].push(k as u8);
            }
        }
        if rng_seed_shuffle {
            for d in 0..3 {
                let mut deck = std::mem::take(&mut self.cs.decks[d]);
                self.rng.shuffle(&mut deck);
                self.cs.decks[d] = deck;
            }
        }
        // The squall (and the pirate) wait off the board until the raiders first land
        self.squall = NONE;
        self.pirate = NONE;
    }

    pub fn walls(&self, p: usize) -> u8 {
        (0..self.cs.walls.len()).filter(|&c| self.cs.walls[c] && self.corner_owner[c] as usize == p).count() as u8
    }

    // After a 7, holding more than this loses half
    pub fn hand_limit(&self, p: usize) -> u32 {
        self.settings().discard_over as u32 + if self.city() { 2 * self.walls(p) as u32 } else { 0 }
    }

    pub fn metros(&self, p: usize) -> u32 {
        self.cs.metro_owner.iter().filter(|&&o| o as usize == p).count() as u32
    }

    // Renown from the city expansion: metropolises (2 each), Renown cards, the merchant
    pub(super) fn city_renown(&self, p: usize) -> u32 {
        if !self.city() {
            return 0;
        }
        2 * self.metros(p) + self.cs.card_renown[p] as u32 + (self.cs.merchant_owner as usize == p) as u32
    }

    pub fn is_metro(&self, c: usize) -> bool {
        self.cs.metro.contains(&(c as u16))
    }

    // ---- Knights ----

    pub fn has_knight(&self, c: usize) -> bool {
        self.cs.knight_owner.get(c).is_some_and(|&o| o != NOBODY)
    }

    // Another player's knight on corner c (blocks p's Paths and Longest Path)
    pub fn foreign_knight(&self, p: u8, c: usize) -> bool {
        self.cs.knight_owner.get(c).is_some_and(|&o| o != NOBODY && o != p)
    }

    // p's knights of this level still in hand (2 of each)
    pub fn knights_left(&self, p: u8, level: u8) -> u8 {
        let on = (0..self.cs.knight_owner.len()).filter(|&c| self.cs.knight_owner[c] == p && self.cs.knight_level[c] == level).count() as u8;
        2u8.saturating_sub(on)
    }

    // The strength p's active knights give against the raiders
    pub fn defence(&self, p: usize) -> u32 {
        (0..self.cs.knight_owner.len())
            .filter(|&c| self.cs.knight_owner[c] as usize == p && self.cs.knight_active[c])
            .map(|c| self.cs.knight_level[c] as u32)
            .sum()
    }

    // The raiders' strength: every Lodge on the board
    pub fn raider_strength(&self) -> u32 {
        (0..self.corner_owner.len()).filter(|&c| self.corner_kind[c] == LODGE && (self.corner_owner[c] as usize) < self.players).count() as u32
    }

    // A free corner by one of p's Paths or ships where a knight may stand
    fn knight_spot(&self, p: u8, c: usize) -> bool {
        self.corner_owner[c] == NOBODY && !self.has_knight(c) && self.touches_own_edge(p, c)
    }

    fn place_knight(&mut self, p: u8, c: usize, level: u8, active: bool) {
        self.cs.knight_owner[c] = p;
        self.cs.knight_level[c] = level;
        self.cs.knight_active[c] = active;
        self.cs.knight_fresh[c] = false;
        self.update_titles(); // a knight breaks other players' lines
    }

    fn lift_knight(&mut self, c: usize) -> (u8, u8, bool) {
        let k = (self.cs.knight_owner[c], self.cs.knight_level[c], self.cs.knight_active[c]);
        self.cs.knight_owner[c] = NOBODY;
        self.cs.knight_level[c] = 0;
        self.cs.knight_active[c] = false;
        self.cs.knight_fresh[c] = false;
        self.cs.promoted[c] = false;
        self.update_titles();
        k
    }

    // Corners a knight of p's starting at `from` can reach along p's routes (Paths and ships),
    // passing p's own pieces but not others': (free corners, corners holding others' knights)
    pub fn knight_reach(&self, p: u8, from: usize) -> (Vec<usize>, Vec<usize>) {
        let g = &self.board.graph;
        let mut seen = vec![false; g.corners()];
        seen[from] = true;
        let mut queue = vec![from];
        let (mut free, mut knights) = (Vec::new(), Vec::new());
        while let Some(c) = queue.pop() {
            for (k, n) in listed(&g.corner_corners[c]).enumerate() {
                let e = g.corner_edges[c][k] as usize;
                if seen[n] || self.edge_owner[e] != p {
                    continue;
                }
                seen[n] = true;
                let building = self.corner_owner[n];
                if building != NOBODY && building != p {
                    continue; // another player's building: no further
                }
                if self.foreign_knight(p, n) {
                    knights.push(n);
                    continue;
                }
                if building == NOBODY && !self.has_knight(n) {
                    free.push(n);
                }
                queue.push(n);
            }
        }
        (free, knights)
    }

    // An active knight that may act: active since before this turn
    fn ready(&self, p: u8, c: usize) -> bool {
        self.cs.knight_owner[c] == p && self.cs.knight_active[c] && !self.cs.knight_fresh[c]
    }

    fn can_promote(&self, p: u8, c: usize) -> bool {
        let level = self.cs.knight_level[c];
        self.cs.knight_owner[c] == p
            && !self.cs.promoted[c]
            && level < 3
            && self.knights_left(p, level + 1) > 0
            && (level < 2 || self.cs.levels[p as usize][2] >= 3)
    }

    // ---- Improvements ----

    // What the next level on track t costs p (in its commodity)
    pub fn improve_cost(&self, p: usize, t: usize) -> u8 {
        (self.cs.levels[p][t] + 1).saturating_sub(self.cs.crane as u8)
    }

    // Would p's next level on t take that track's metropolis?
    fn takes_metro(&self, p: usize, t: usize) -> bool {
        let next = self.cs.levels[p][t] + 1;
        let holder = self.cs.metro_owner[t];
        (next == 4 && holder == NOBODY) || (next == 5 && holder as usize != p && (holder == NOBODY || self.cs.levels[holder as usize][t] < 5))
    }

    // p's Lodge for a new metropolis: the one with the best dice that doesn't hold one
    fn metro_spot(&self, p: usize) -> Option<usize> {
        (0..self.corner_owner.len())
            .filter(|&c| self.corner_owner[c] as usize == p && self.corner_kind[c] == LODGE && !self.is_metro(c))
            .max_by_key(|&c| (corner_pips(&self.board, c, NONE), std::cmp::Reverse(c)))
    }

    fn improve(&mut self, p: usize, t: usize) {
        let cost = self.improve_cost(p, t);
        self.hands[p][PAPER + t] -= cost;
        self.cs.crane = false;
        let take = self.takes_metro(p, t);
        self.cs.levels[p][t] += 1;
        if take {
            if let Some(c) = self.metro_spot(p) {
                self.cs.metro[t] = c as u16;
                self.cs.metro_owner[t] = p as u8;
            }
        }
    }

    // ---- Legal actions in the main phase ----

    pub(super) fn legal_city(&self, p: u8, out: &mut Vec<Action>) {
        if !self.city() {
            return;
        }
        let pu = p as usize;
        let hand = &self.hands[pu];
        let g = &self.board.graph;
        if afford(hand, &WALL_COST) && self.walls(pu) < 3 {
            for c in 0..g.corners() {
                if self.corner_owner[c] == p && self.corner_kind[c] == LODGE && !self.cs.walls[c] {
                    out.push(Action::Wall(c as u16));
                }
            }
        }
        if self.lodges(pu) > 0 {
            for t in 0..3 {
                if self.cs.levels[pu][t] < 5
                    && hand[PAPER + t] >= self.improve_cost(pu, t)
                    && (!self.takes_metro(pu, t) || self.metro_spot(pu).is_some())
                {
                    out.push(Action::Improve(t as u8));
                }
            }
        }
        if afford(hand, &KNIGHT_COST) {
            if self.knights_left(p, 1) > 0 {
                for c in 0..g.corners() {
                    if self.knight_spot(p, c) {
                        out.push(Action::Recruit(c as u16));
                    }
                }
            }
            for c in 0..g.corners() {
                if self.can_promote(p, c) {
                    out.push(Action::Promote(c as u16));
                }
            }
        }
        let grain = afford(hand, &ACTIVATE_COST);
        for c in 0..g.corners() {
            if self.cs.knight_owner[c] != p {
                continue;
            }
            if grain && !self.cs.knight_active[c] {
                out.push(Action::Activate(c as u16));
            }
            if self.ready(p, c) {
                let (free, knights) = self.knight_reach(p, c);
                for n in free {
                    out.push(Action::KnightMove(c as u16, n as u16));
                }
                for n in knights {
                    if self.cs.knight_level[n] < self.cs.knight_level[c] {
                        out.push(Action::Displace(c as u16, n as u16));
                    }
                }
                // Chase the squall (or the pirate) from a corner of its hex
                let by = |h: u16| h != NONE && g.hex_corners[h as usize].contains(&(c as u16));
                if self.cs.landed && (by(self.squall) || by(self.pirate)) {
                    out.push(Action::Chase(c as u16));
                }
            }
        }
        let mut seen = 0u32;
        for &k in &self.cs.progress[pu] {
            if seen & (1 << k) != 0 || k == FIXED_ROLL || (k == TAXES && !self.cs.landed) {
                continue;
            }
            seen |= 1 << k;
            out.push(Action::PlayCard(k));
        }
    }

    // Progress cards over the limit at the end of the turn must be dropped first
    pub fn must_drop(&self, p: usize) -> bool {
        self.city() && self.cs.progress[p].len() > 4
    }

    pub(super) fn legal_drops(&self, p: usize, out: &mut Vec<Action>) {
        let mut seen = 0u32;
        for &k in &self.cs.progress[p] {
            if seen & (1 << k) == 0 {
                seen |= 1 << k;
                out.push(Action::DropCard(k));
            }
        }
    }

    pub(super) fn legal_fixed_roll(&self, p: usize, out: &mut Vec<Action>) {
        if self.city() && self.cs.progress[p].contains(&FIXED_ROLL) {
            for a in 1..=6 {
                for b in 1..=6 {
                    out.push(Action::FixedRoll(a, b));
                }
            }
        }
    }

    // ---- The roll ----

    // The production dice (Fixed Roll's, if played), then the event die and what it brings: the
    // raiders and progress draws. Returns the decisions they need (before production).
    pub(super) fn city_roll(&mut self) -> (u8, Vec<Pending>) {
        let dice = match self.cs.next_dice.take() {
            Some(d) => d,
            None => [self.rng.below(6) as u8 + 1, self.rng.below(6) as u8 + 1],
        };
        self.last_dice = dice;
        let die = self.rng.below(6) as u8;
        self.cs.event_die = die;
        self.cs.drawn.clear();
        let mut decisions = Vec::new();
        if die < 3 {
            self.cs.raiders += 1;
            if self.cs.raiders >= self.settings().city_raiders {
                decisions.extend(self.raid());
            }
        } else {
            let t = (die - 3) as usize;
            let red = dice[0];
            for k in 0..self.players {
                let s = (self.current as usize + k) % self.players;
                let level = self.cs.levels[s][t];
                if level >= 1 && red <= level + 1 {
                    if let Some(d) = self.draw_progress(s, t) {
                        decisions.push(d);
                    }
                }
            }
        }
        (dice[0] + dice[1], decisions)
    }

    // p draws the top card of deck t: a Renown card is shown at once; off p's turn, over 4 means
    // dropping one (the decision returned)
    fn draw_progress(&mut self, p: usize, t: usize) -> Option<Pending> {
        let k = self.cs.decks[t].pop()?;
        self.cs.drawn.push((p as u8, t as u8));
        if renown_card(k) {
            self.cs.card_renown[p] += 1;
            return None;
        }
        self.cs.progress[p].push(k);
        (p != self.current as usize && self.cs.progress[p].len() > 4).then_some(Pending { seat: p as u8, decision: Decision::DropProgress })
    }

    // The raiders land
    fn raid(&mut self) -> Vec<Pending> {
        let strength = self.raider_strength();
        let defence: Vec<u32> = (0..self.players).map(|p| self.defence(p)).collect();
        let total: u32 = defence.iter().sum();
        let order: Vec<usize> = (0..self.players).map(|k| (self.current as usize + k) % self.players).collect();
        let mut decisions = Vec::new();
        if strength > total {
            // The lowest contributors with a Lodge that can be pillaged each lose one
            let mut levels: Vec<u32> = defence.clone();
            levels.sort();
            levels.dedup();
            for level in levels {
                let hit: Vec<usize> = order.iter().copied().filter(|&p| defence[p] == level && self.pillageable(p)).collect();
                if !hit.is_empty() {
                    decisions.extend(hit.into_iter().map(|p| Pending { seat: p as u8, decision: Decision::Pillage }));
                    break;
                }
            }
        } else {
            let best = *defence.iter().max().unwrap();
            let top: Vec<usize> = order.iter().copied().filter(|&p| defence[p] == best).collect();
            if top.len() == 1 {
                self.give_tokens(top[0], Token::Renown, 1);
            } else {
                decisions.extend(top.into_iter().map(|p| Pending { seat: p as u8, decision: Decision::DrawCard }));
            }
        }
        self.cs.last_raid = [strength.min(255) as u8, total.min(255) as u8, (strength > total) as u8];
        for a in self.cs.knight_active.iter_mut() {
            *a = false;
        }
        self.cs.raiders = 0;
        if !self.cs.landed {
            self.cs.landed = true;
            self.squall = self.cs.squall_home;
            self.pirate = self.cs.pirate_home;
        }
        decisions
    }

    fn pillageable(&self, p: usize) -> bool {
        (0..self.corner_owner.len()).any(|c| self.corner_owner[c] as usize == p && self.corner_kind[c] == LODGE && !self.is_metro(c))
    }

    // A Lodge becomes a Hut again (the raiders): its wall goes
    fn pillage(&mut self, p: usize, c: usize) {
        self.corner_kind[c] = HUT;
        self.cs.walls[c] = false;
        self.pieces[p][P_LODGE] += 1;
        if self.pieces[p][P_HUT] > 0 {
            self.pieces[p][P_HUT] -= 1;
        } else {
            self.cs.hut_debt[p] += 1;
        }
        self.refresh_ports();
    }

    // Production: what a Lodge on hex h (resource r) gives in the city expansion: 1 resource + 1
    // commodity on Forest, Paddy and Quarry; 2 of the resource on Fields and Mines
    pub(super) fn lodge_goods(r: usize) -> Option<usize> {
        match r {
            0 => Some(PAPER), // Forest: Timber + Paper
            4 => Some(CLOTH), // Paddy: Fish + Cloth
            1 => Some(COIN),  // Quarry: Stone + Coin
            _ => None,
        }
    }

    // ---- Decisions ----

    pub(super) fn legal_city_decision(&self, d: Pending, out: &mut Vec<Action>) {
        let p = d.seat;
        let pu = p as usize;
        let g = &self.board.graph;
        match d.decision {
            Decision::DropProgress => {
                if self.cs.progress[pu].len() > 4 {
                    self.legal_drops(pu, out);
                }
            }
            Decision::Pillage => {
                for c in 0..g.corners() {
                    if self.corner_owner[c] == p && self.corner_kind[c] == LODGE && !self.is_metro(c) {
                        out.push(Action::Pillage(c as u16));
                    }
                }
            }
            Decision::Displaced { at, .. } => {
                for n in self.knight_reach(p, at as usize).0 {
                    out.push(Action::KnightMove(at, n as u16));
                }
            }
            Decision::DrawCard => {
                for t in 0..3 {
                    if !self.cs.decks[t].is_empty() {
                        out.push(Action::Choose(t as u8));
                    }
                }
            }
            Decision::Name { card } => {
                let kinds = match card {
                    MONOPOLY => 0..5,
                    GOODS_MONOPOLY => 5..8,
                    _ => 0..8, // Trade Fleet
                };
                out.extend(kinds.map(|k| Action::Choose(k as u8)));
            }
            Decision::Target { card } => {
                let mine = self.renown(pu);
                for o in 0..self.players {
                    if o == pu {
                        continue;
                    }
                    let ok = match card {
                        GUILD_DUES => self.renown(o) > mine && self.cards(o) > 0,
                        SPY => self.cs.progress[o].iter().any(|&k| !renown_card(k)),
                        _ => self.cs.knight_owner.iter().any(|&k| k as usize == o), // Treason
                    };
                    if ok {
                        out.push(Action::Choose(o as u8));
                    }
                }
            }
            Decision::TakeCards { from, .. } => {
                for k in 0..CARD_KINDS {
                    if self.hands[from as usize][k] > 0 {
                        out.push(Action::TakeCard(from, k as u8));
                    }
                }
            }
            Decision::Spy { from } => {
                let mut seen = 0u32;
                for &k in &self.cs.progress[from as usize] {
                    if seen & (1 << k) == 0 && !renown_card(k) {
                        seen |= 1 << k;
                        out.push(Action::TakeProgress(from, k));
                    }
                }
                if !out.is_empty() {
                    out.push(Action::Skip);
                }
            }
            Decision::Harbour { asked } => {
                for o in 0..self.players {
                    let goods: u8 = self.hands[o][5..].iter().sum();
                    if o == pu || asked & (1 << o) != 0 || goods == 0 {
                        continue;
                    }
                    for r in 0..5 {
                        if self.hands[pu][r] > 0 {
                            out.push(Action::Gift(o as u8, r as u8));
                        }
                    }
                }
                if !out.is_empty() {
                    out.push(Action::Skip);
                }
            }
            Decision::PayGoods { to } => {
                for k in 5..CARD_KINDS {
                    if self.hands[pu][k] > 0 {
                        out.push(Action::Gift(to, k as u8));
                    }
                }
            }
            Decision::Smith { .. } => {
                for c in 0..g.corners() {
                    if self.can_promote(p, c) {
                        out.push(Action::Promote(c as u16));
                    }
                }
                if !out.is_empty() {
                    out.push(Action::Skip);
                }
            }
            Decision::FreeWall => {
                if self.walls(pu) < 3 {
                    for c in 0..g.corners() {
                        if self.corner_owner[c] == p && self.corner_kind[c] == LODGE && !self.cs.walls[c] {
                            out.push(Action::Wall(c as u16));
                        }
                    }
                }
            }
            Decision::Swap => {
                let ok: Vec<usize> = (0..self.board.number.len()).filter(|&h| !matches!(self.board.number[h], 0 | 2 | 6 | 8 | 12)).collect();
                for (i, &a) in ok.iter().enumerate() {
                    for &b in &ok[i + 1..] {
                        if self.board.number[a] != self.board.number[b] {
                            out.push(Action::SwapNumbers(a as u16, b as u16));
                        }
                    }
                }
            }
            Decision::PlaceMerchant => {
                for &h in &self.board.land_hexes {
                    let r = self.board.resource[h as usize];
                    let mine = g.hex_corners[h as usize].iter().any(|&c| self.corner_owner[c as usize] == p);
                    if r < 5 && mine {
                        out.push(Action::Merchant(h));
                    }
                }
            }
            Decision::RemovePath => {
                for e in 0..self.edge_owner.len() {
                    if self.open_path(e) {
                        out.push(Action::RemovePath(e as u16));
                    }
                }
            }
            Decision::Expel => {
                for c in 0..g.corners() {
                    if self.foreign_knight(p, c) && self.touches_own_edge(p, c) {
                        out.push(Action::Expel(c as u16));
                    }
                }
            }
            Decision::GiveUpKnight { .. } => {
                for c in 0..g.corners() {
                    if self.cs.knight_owner[c] == p {
                        out.push(Action::RemoveKnight(c as u16));
                    }
                }
            }
            Decision::TreasonKnight { level, .. } => {
                if (1..=level).any(|l| self.knights_left(p, l) > 0) {
                    for c in 0..g.corners() {
                        if self.knight_spot(p, c) {
                            out.push(Action::Recruit(c as u16));
                        }
                    }
                }
                if !out.is_empty() {
                    out.push(Action::Skip);
                }
            }
            _ => {}
        }
    }

    // A Path is "open" if one of its ends has none of its owner's other Paths, buildings or knights
    fn open_path(&self, e: usize) -> bool {
        let o = self.edge_owner[e];
        if o == NOBODY || o as usize >= self.players || self.edge_kind[e] != PATH {
            return false;
        }
        let g = &self.board.graph;
        g.edge_corners[e].iter().any(|&c| {
            let c = c as usize;
            self.corner_owner[c] != o
                && self.cs.knight_owner[c] != o
                && !listed(&g.corner_edges[c]).any(|x| x != e && self.edge_owner[x] == o)
        })
    }

    // ---- Doing things ----

    // The city expansion's actions (returns false for anything else)
    pub(super) fn apply_city(&mut self, a: Action) -> bool {
        let p = self.to_act();
        let pu = p as usize;
        let deciding = self.deciding().map(|d| d.decision);
        match a {
            Action::Wall(c) => {
                if deciding == Some(Decision::FreeWall) {
                    self.cs.walls[c as usize] = true;
                    self.decided(None);
                } else {
                    pay(&mut self.hands[pu], &WALL_COST);
                    self.cs.walls[c as usize] = true;
                }
            }
            Action::Improve(t) => self.improve(pu, t as usize),
            Action::Recruit(c) => {
                if let Some(Decision::TreasonKnight { level, active }) = deciding {
                    let l = (1..=level).rev().find(|&l| self.knights_left(p, l) > 0).unwrap();
                    self.place_knight(p, c as usize, l, active);
                    self.decided(None);
                } else {
                    pay(&mut self.hands[pu], &KNIGHT_COST);
                    self.place_knight(p, c as usize, 1, false);
                }
            }
            Action::Promote(c) => {
                let c = c as usize;
                self.cs.knight_level[c] += 1;
                self.cs.promoted[c] = true;
                if let Some(Decision::Smith { left }) = deciding {
                    self.decided(if left > 1 { Some(Decision::Smith { left: left - 1 }) } else { None });
                } else {
                    pay(&mut self.hands[pu], &KNIGHT_COST);
                }
            }
            Action::Activate(c) => {
                pay(&mut self.hands[pu], &ACTIVATE_COST);
                self.cs.knight_active[c as usize] = true;
                self.cs.knight_fresh[c as usize] = true;
            }
            Action::KnightMove(from, to) => {
                if let Some(Decision::Displaced { level, active, .. }) = deciding {
                    // A displaced knight finds a new corner (its status unchanged)
                    self.place_knight(p, to as usize, level, active);
                    self.decided(None);
                } else {
                    let (_, level, _) = self.lift_knight(from as usize);
                    self.place_knight(p, to as usize, level, false);
                }
            }
            Action::Displace(from, to) => {
                let (_, level, _) = self.lift_knight(from as usize);
                let (other, their_level, their_active) = self.lift_knight(to as usize);
                self.place_knight(p, to as usize, level, false);
                self.displaced(other, to, their_level, their_active);
            }
            Action::Expel(c) => {
                let (other, level, active) = self.lift_knight(c as usize);
                self.decided(None);
                self.displaced(other, c, level, active);
            }
            Action::Chase(c) => {
                self.cs.knight_active[c as usize] = false;
                self.back = Phase::Main;
                self.phase = Phase::MoveSquall { anywhere: true };
            }
            Action::Pillage(c) => {
                self.pillage(pu, c as usize);
                self.decided(None);
            }
            Action::RemoveKnight(c) => {
                let (_, level, active) = self.lift_knight(c as usize);
                let Some(Decision::GiveUpKnight { to }) = deciding else { unreachable!() };
                self.pending[0] = Pending { seat: to, decision: Decision::TreasonKnight { level, active } };
                self.decided_keep();
            }
            Action::DropCard(k) => {
                self.return_card(pu, k);
                if deciding == Some(Decision::DropProgress) {
                    self.decided(if self.cs.progress[pu].len() > 4 { Some(Decision::DropProgress) } else { None });
                }
            }
            Action::FixedRoll(a, b) => {
                self.return_card(pu, FIXED_ROLL);
                self.cs.next_dice = Some([a, b]);
                self.apply_unchecked(Action::Roll);
            }
            Action::PlayCard(k) => self.play_card(pu, k),
            Action::Choose(x) => self.choose(pu, deciding.expect("a decision"), x),
            Action::TakeCard(from, k) => {
                self.hands[from as usize][k as usize] -= 1;
                self.hands[pu][k as usize] += 1;
                let Some(Decision::TakeCards { left, .. }) = deciding else { unreachable!() };
                self.decided(if left > 1 { Some(Decision::TakeCards { from, left: left - 1 }) } else { None });
            }
            Action::TakeProgress(from, k) => {
                let hand = &mut self.cs.progress[from as usize];
                let i = hand.iter().position(|&x| x == k).unwrap();
                hand.remove(i);
                self.cs.progress[pu].push(k);
                self.decided(None);
            }
            Action::SwapNumbers(a, b) => {
                let mut hexes = self.board.hexes.clone();
                let (na, nb) = (hexes[a as usize].number, hexes[b as usize].number);
                hexes[a as usize].number = nb;
                hexes[b as usize].number = na;
                self.rebuild_board(hexes);
                self.decided(None);
            }
            Action::Merchant(h) => {
                self.cs.merchant = h;
                self.cs.merchant_owner = p;
                self.decided(None);
            }
            Action::RemovePath(e) => {
                let e = e as usize;
                let owner = self.edge_owner[e];
                self.edge_owner[e] = NOBODY;
                self.edge_kind[e] = 0;
                self.pieces[owner as usize][P_PATH] += 1;
                self.update_titles();
                if owner == p && self.pieces[pu][P_PATH] > 0 {
                    self.after = Phase::FreeBuild { left: 1 }; // your own: build one free
                }
                self.decided(None);
                self.finish_free_build();
            }
            Action::Skip => self.decided(None),
            _ => return false,
        }
        true
    }

    // The first decision was replaced in place (pending[0]): go on as decided() does
    // (it may have no choices: then it goes)
    fn decided_keep(&mut self) {
        let mut list = Vec::new();
        self.legal_decision(self.pending[0], &mut list);
        if list.is_empty() {
            self.decided(None);
        }
    }

    // `other`'s knight was pushed off corner `at`: they move it along their routes, or it goes back to
    // their hand when it can't move
    fn displaced(&mut self, other: u8, at: u16, level: u8, active: bool) {
        if self.knight_reach(other, at as usize).0.is_empty() {
            return;
        }
        let after = if self.phase == Phase::Decide { self.after } else { self.phase };
        let mut queue = std::mem::take(&mut self.pending);
        queue.insert(0, Pending { seat: other, decision: Decision::Displaced { at, level, active } });
        self.decide_then(queue, after);
    }

    // A played or dropped card goes under its deck
    fn return_card(&mut self, p: usize, k: u8) {
        let hand = &mut self.cs.progress[p];
        let i = hand.iter().position(|&x| x == k).expect("the card is held");
        hand.remove(i);
        self.cs.decks[CARDS[k as usize].1 as usize].insert(0, k);
    }

    fn rebuild_board(&mut self, hexes: Vec<Hex>) {
        let b = (*self.board).clone();
        let markers = b.corner_marker.clone();
        let Board { graph, markets, settings, .. } = b;
        let mut rebuilt = Board::from_parts(graph, hexes, markets, settings);
        rebuilt.corner_marker = markers;
        self.board = Arc::new(rebuilt);
    }

    // Others ranked against p: every other player, in turn order from p
    fn others(&self, p: usize) -> Vec<usize> {
        (1..self.players).map(|k| (p + k) % self.players).collect()
    }

    fn play_card(&mut self, p: usize, k: u8) {
        self.return_card(p, k);
        self.back = Phase::Main;
        let seat = p as u8;
        let one = |d: Decision| vec![Pending { seat, decision: d }];
        let decisions: Vec<Pending> = match k {
            CRANE => {
                self.cs.crane = true;
                vec![]
            }
            MEDICINE => {
                self.cs.medicine = true;
                vec![]
            }
            ENGINEER => one(Decision::FreeWall),
            NUMBER_SWAP => one(Decision::Swap),
            IRRIGATION | QUARRYING => {
                let (terrain, r) = if k == IRRIGATION { (Terrain::Fields, 2) } else { (Terrain::Quarry, 1) };
                let g = &self.board.graph;
                let hexes = (0..self.board.terrain.len())
                    .filter(|&h| self.board.terrain[h] == terrain && g.hex_corners[h].iter().any(|&c| self.corner_owner[c as usize] == seat))
                    .count() as u32;
                let n = (2 * hexes).min(self.bank(r));
                self.hands[p][r] += n as u8;
                vec![]
            }
            PATH_BUILDERS => {
                self.phase = Phase::FreeBuild { left: 2 };
                self.finish_free_build();
                return;
            }
            SMITHING => one(Decision::Smith { left: 2 }),
            HARBOUR => one(Decision::Harbour { asked: 0 }),
            GUILD_DUES | SPY | TREASON => one(Decision::Target { card: k }),
            MERCHANT => one(Decision::PlaceMerchant),
            TRADE_FLEET | MONOPOLY | GOODS_MONOPOLY => one(Decision::Name { card: k }),
            DIPLOMACY => one(Decision::RemovePath),
            RALLY => {
                for c in 0..self.cs.knight_owner.len() {
                    if self.cs.knight_owner[c] == seat && !self.cs.knight_active[c] {
                        self.cs.knight_active[c] = true;
                        self.cs.knight_fresh[c] = true;
                    }
                }
                vec![]
            }
            INTRIGUE => one(Decision::Expel),
            TAXES => {
                self.cs.tax = true;
                self.phase = Phase::MoveSquall { anywhere: true };
                return;
            }
            WEDDING => {
                let mine = self.renown(p);
                self.others(p)
                    .into_iter()
                    .filter(|&o| self.renown(o) > mine)
                    .flat_map(|o| [Pending { seat: o as u8, decision: Decision::Gift { to: seat } }; 2])
                    .collect()
            }
            SABOTAGE => {
                let mine = self.renown(p);
                self.others(p)
                    .into_iter()
                    .filter(|&o| self.renown(o) >= mine && self.cards(o) >= 2)
                    .map(|o| Pending { seat: o as u8, decision: Decision::Discard { left: (self.cards(o) / 2) as u8 } })
                    .collect()
            }
            _ => vec![],
        };
        self.decide_then(decisions, Phase::Main);
    }

    fn choose(&mut self, p: usize, d: Decision, x: u8) {
        match d {
            Decision::DrawCard => {
                let drop = self.draw_progress(p, x as usize);
                match drop {
                    Some(next) => {
                        self.pending[0] = next;
                        self.decided_keep();
                    }
                    None => self.decided(None),
                }
            }
            Decision::Name { card: TRADE_FLEET } => {
                self.cs.fleet = x;
                self.decided(None);
            }
            Decision::Name { card } => {
                // Monopoly: 2 of a resource (or the 1 they have); goods: 1 of a commodity
                let most = if card == MONOPOLY { 2 } else { 1 };
                for o in self.others(p) {
                    let n = self.hands[o][x as usize].min(most);
                    self.hands[o][x as usize] -= n;
                    self.hands[p][x as usize] += n;
                }
                self.decided(None);
            }
            Decision::Target { card } => {
                let me = p as u8;
                self.pending[0] = match card {
                    GUILD_DUES => Pending { seat: me, decision: Decision::TakeCards { from: x, left: 2 } },
                    SPY => Pending { seat: me, decision: Decision::Spy { from: x } },
                    _ => Pending { seat: x, decision: Decision::GiveUpKnight { to: me } },
                };
                self.decided_keep();
            }
            _ => unreachable!("Choose in {d:?}"),
        }
    }

    // Harbour Deal: p gives `to` a resource; `to` pays a commodity of their choice at once, then p may
    // offer the next player
    pub(super) fn harbour_gift(&mut self, p: usize, to: u8, r: u8) {
        self.hands[p][r as usize] -= 1;
        self.hands[to as usize][r as usize] += 1;
        let Some(Decision::Harbour { asked }) = self.deciding().map(|d| d.decision) else { unreachable!() };
        self.pending[0].decision = Decision::Harbour { asked: asked | (1 << to) };
        self.pending.insert(0, Pending { seat: to, decision: Decision::PayGoods { to: p as u8 } });
    }

    // After the roll's production: Science level 3 gives a resource of your choice to each player who
    // got nothing (not on a 7)
    pub(super) fn science_picks(&self, before: &[u32; MAX_PLAYERS]) -> Vec<Pending> {
        (0..self.players)
            .map(|k| (self.current as usize + k) % self.players)
            .filter(|&p| self.cs.levels[p][0] >= 3 && self.cards(p) == before[p])
            .map(|p| Pending { seat: p as u8, decision: Decision::Pick { left: 1 } })
            .collect()
    }

    // The end of a phase: this turn's card effects and knight marks end
    pub(super) fn city_end_phase(&mut self) {
        if !self.city() {
            return;
        }
        let cs = &mut self.cs;
        cs.crane = false;
        cs.medicine = false;
        cs.fleet = NOBODY;
        cs.tax = false;
        for f in cs.knight_fresh.iter_mut() {
            *f = false;
        }
        for f in cs.promoted.iter_mut() {
            *f = false;
        }
    }
}

#[cfg(test)]
#[path = "city_tests.rs"]
mod tests;
