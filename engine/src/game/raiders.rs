// The variants box's scenario Raider attack (`box.scenario=4`; RULES.md "The variants box: scenarios").
// The Keep stands in the island's middle (no number), the Dunes in the ring round it; the 12 coastal
// hexes take the raiders. No squall: a 7 means discards, then the roller robs anyone. Your second setup
// building is a Lodge.
//
// - Every Hut or Lodge built brings a landing: 3 rolls, each not a 7 and different from the others,
//   each putting a raider (from 36) on every coastal hex with that number that has fewer than 3. Three
//   raiders conquer a hex: it produces nothing, nothing is built on its corners or edges, and a
//   building touching only conquered hexes is worth no Renown and uses no market.
// - Knights stand on edges (6 each). They come from the order deck (Stone + Grain + Fish, played at
//   once): Capture 4 (a raider from a coastal hex becomes your prisoner), Knighthood 14 (a knight on an
//   edge of the Keep), Swift Knight 4 (a knight on any free edge), Turncoat 4 (2 coins, and move 2
//   raiders from two coastal hexes to two other unconquered ones).
// - At the end of your turn, move each of your knights up to 3 edges (5 for a Grain each); knights on
//   the Keep must move; none may end on the Keep or another knight. Then, clockwise round the coast, a
//   hex with raiders and more knights on its edges than raiders is freed: its raiders are prisoners
//   for the players with knights there (one player: all; several: one each if there are enough, the
//   last to the most knights, a die settling ties; too few: dice decide, and each player left out
//   takes 3 coins). Then a die picks a pair of edge directions (1/6, 2/5, 3/4): the hex's knights on
//   those edges go home, 3 coins each. Every 2 prisoners are worth 1 Renown.
// - Coins as in Rivers (bought at your rate, 2 buy a resource, twice a turn). 12 Renown to win.
//
// Simplified (RULES.md): a number on two coastal hexes lands a raider on both; the order deck is
// reshuffled from its discards; the Keep and coast are our own layout.
use super::*;

pub const RAIDERS: u8 = 36;
pub const EDGE_KNIGHTS: u8 = 6;
// Orders (the scenario's own project deck)
pub const CAPTURE: u8 = 0;
pub const KNIGHTHOOD: u8 = 1;
pub const SWIFT_KNIGHT: u8 = 2;
pub const TURNCOAT: u8 = 3;
pub const ORDERS: [(&str, u8); 4] = [("capture", 4), ("knighthood", 14), ("swiftKnight", 4), ("turncoat", 4)];

impl Game {
    pub fn raider_attack(&self) -> bool {
        self.settings().box_scenario == 4
    }

    pub(super) fn raiders_setup(&mut self) {
        let g = &self.board.graph;
        self.keep = (0..self.board.terrain.len()).find(|&h| self.board.terrain[h] == Terrain::Keep).map_or(NONE, |h| h as u16);
        self.raiders_on = vec![0; g.cells.len()];
        self.edge_knight = vec![NOBODY; g.edges()];
        self.knight_moved = vec![false; g.edges()];
        // The coast, clockwise from a random start: land hexes touching the sea
        let centre = |h: usize| {
            let c = g.cells[h];
            ((3 * c.r) as f64 * 0.5).atan2((2 * c.q + c.r) as f64 * 0.866)
        };
        let mut coast: Vec<usize> = (0..self.board.terrain.len())
            .filter(|&h| self.board.number[h] != 0 && self.board.terrain[h] != Terrain::Sea && g.hex_edges[h].iter().any(|&e| self.board.edge_sea[e as usize]))
            .collect();
        coast.sort_by(|&a, &b| centre(a).total_cmp(&centre(b)));
        let start = self.rng.below(coast.len());
        coast.rotate_left(start);
        self.coast = coast.into_iter().map(|h| h as u16).collect();
        // A raider each on the coast's 2 and 12
        let mut supply = RAIDERS;
        for &h in &self.coast {
            if matches!(self.board.number[h as usize], 2 | 12) {
                self.raiders_on[h as usize] = 1;
                supply -= 1;
            }
        }
        self.raider_supply = supply;
        let mut deck: Vec<u8> = ORDERS.iter().enumerate().flat_map(|(k, &(_, n))| std::iter::repeat_n(k as u8, n as usize)).collect();
        self.rng.shuffle(&mut deck);
        self.orders = deck;
        self.squall = NONE;
    }

    pub fn conquered(&self, h: usize) -> bool {
        self.raiders_on.get(h).is_some_and(|&n| n >= 3)
    }

    // A corner or edge of a conquered hex: nothing may be built there
    pub fn by_conquered_corner(&self, c: usize) -> bool {
        self.raider_attack() && listed(&self.board.graph.corner_hexes[c]).any(|h| self.conquered(h))
    }
    pub fn by_conquered_edge(&self, e: usize) -> bool {
        self.raider_attack() && listed(&self.board.graph.edge_hexes[e]).any(|h| self.conquered(h))
    }

    // A building conquered: every land hex it touches is conquered
    pub fn building_conquered(&self, c: usize) -> bool {
        if !self.raider_attack() {
            return false;
        }
        let land: Vec<usize> = listed(&self.board.graph.corner_hexes[c]).filter(|&h| self.board.terrain[h] != Terrain::Sea).collect();
        !land.is_empty() && land.iter().all(|&h| self.conquered(h))
    }

    // Renown from the raiders: 1 for every 2 prisoners, less what conquered buildings would be worth
    pub(super) fn raid_renown(&self, p: usize) -> i32 {
        if !self.raider_attack() {
            return 0;
        }
        let lost: i32 = (0..self.corner_owner.len())
            .filter(|&c| self.corner_owner[c] as usize == p && self.building_conquered(c))
            .map(|c| self.corner_kind[c] as i32)
            .sum();
        self.prisoners[p] as i32 / 2 - lost
    }

    pub fn edge_knights(&self, p: u8) -> u8 {
        self.edge_knight.iter().filter(|&&o| o == p).count() as u8
    }

    fn keep_edge(&self, e: usize) -> bool {
        self.keep != NONE && self.board.graph.hex_edges[self.keep as usize].contains(&(e as u16))
    }

    // A landing after a building: 3 different numbers (never 7), a raider on each matching coastal hex
    // with fewer than 3
    pub(super) fn raider_landing(&mut self) {
        if !self.raider_attack() {
            return;
        }
        let mut used: Vec<u8> = Vec::new();
        for _ in 0..3 {
            if self.raider_supply == 0 {
                return;
            }
            let total = loop {
                let t = self.rng.below(6) as u8 + self.rng.below(6) as u8 + 2;
                if t != 7 && !used.contains(&t) {
                    break t;
                }
            };
            used.push(total);
            for k in 0..self.coast.len() {
                let h = self.coast[k] as usize;
                if self.board.number[h] == total && self.raiders_on[h] < 3 && self.raider_supply > 0 {
                    self.raiders_on[h] += 1;
                    self.raider_supply -= 1;
                }
            }
        }
        self.last_landing = used;
    }

    // ---- Orders ----

    pub(super) fn buy_order(&mut self, p: usize) {
        pay(&mut self.hands[p], &PROJECT_COST);
        let seat = p as u8;
        // A Capture with no raider on the coast is put aside and another drawn
        for _ in 0..20 {
            if self.orders.is_empty() {
                let mut used = std::mem::take(&mut self.orders_used);
                self.rng.shuffle(&mut used);
                self.orders = used;
            }
            let Some(k) = self.orders.pop() else { return };
            self.orders_used.push(k);
            self.last_order = k;
            let raiders = self.coast.iter().any(|&h| self.raiders_on[h as usize] > 0);
            let d = match k {
                CAPTURE if !raiders => continue,
                CAPTURE => vec![Pending { seat, decision: Decision::Capture }],
                KNIGHTHOOD => vec![Pending { seat, decision: Decision::PlaceGuard { keep: true } }],
                SWIFT_KNIGHT => vec![Pending { seat, decision: Decision::PlaceGuard { keep: false } }],
                _ => {
                    self.coins[p] += 2;
                    vec![Pending { seat, decision: Decision::MoveRaider { from: NONE } }, Pending { seat, decision: Decision::MoveRaider { from: NONE } }]
                }
            };
            self.decide_then(d, Phase::Main);
            return;
        }
    }

    pub(super) fn legal_raider_decision(&self, d: Pending, out: &mut Vec<Action>) {
        let p = d.seat;
        let g = &self.board.graph;
        match d.decision {
            Decision::Capture => {
                for &h in &self.coast {
                    if self.raiders_on[h as usize] > 0 {
                        out.push(Action::Capture(h));
                    }
                }
            }
            Decision::PlaceGuard { keep } => {
                if self.edge_knights(p) < EDGE_KNIGHTS {
                    for e in 0..g.edges() {
                        let ok = if keep { self.keep_edge(e) } else { self.board.edge_land[e] && !self.keep_edge(e) };
                        if ok && self.edge_knight[e] == NOBODY {
                            out.push(Action::PlaceGuard(e as u16));
                        }
                    }
                }
            }
            Decision::MoveRaider { from: before } => {
                let on_board: Vec<u16> = self.coast.iter().copied().filter(|&h| self.raiders_on[h as usize] > 0 && h != before).collect();
                let froms: Vec<u16> = if on_board.is_empty() && self.raider_supply > 0 { vec![NONE] } else { on_board };
                for &f in &froms {
                    for &t in &self.coast {
                        if t != f && !self.conquered(t as usize) {
                            out.push(Action::MoveRaider(f, t));
                        }
                    }
                }
            }
            Decision::MoveGuards => {
                let grain = self.hands[p as usize][2] > 0;
                let mut must = false;
                // (two players: either may move the neutral knight, after their own)
                let own_left = (0..g.edges()).any(|e| self.edge_knight[e] == p && !self.knight_moved[e]);
                for e in 0..g.edges() {
                    let neutral = self.two() && self.edge_knight[e] == NEUTRAL[0] && !own_left;
                    if (self.edge_knight[e] != p && !neutral) || self.knight_moved[e] {
                        continue;
                    }
                    let (near, far) = self.guard_reach(e);
                    must |= self.keep_edge(e) && !near.is_empty();
                    for &t in &near {
                        out.push(Action::GuardMove(e as u16, t as u16));
                    }
                    if grain {
                        for &t in &far {
                            out.push(Action::GuardRun(e as u16, t as u16));
                        }
                    }
                }
                if !must {
                    out.push(Action::Skip);
                }
            }
            _ => {}
        }
    }

    // Free edges a knight on edge e reaches in up to 3 steps, and in 4 or 5 (the edges are joined at
    // corners; knights ignore everything on the way)
    pub fn guard_reach(&self, e: usize) -> (Vec<usize>, Vec<usize>) {
        let g = &self.board.graph;
        let mut dist = vec![u8::MAX; g.edges()];
        dist[e] = 0;
        let mut queue = std::collections::VecDeque::from([e]);
        while let Some(x) = queue.pop_front() {
            if dist[x] == 5 {
                continue;
            }
            for &c in &g.edge_corners[x] {
                for y in listed(&g.corner_edges[c as usize]) {
                    if dist[y] == u8::MAX && self.board.edge_land[y] {
                        dist[y] = dist[x] + 1;
                        queue.push_back(y);
                    }
                }
            }
        }
        let ok = |y: usize| y != e && self.edge_knight[y] == NOBODY && !self.keep_edge(y);
        let near = (0..g.edges()).filter(|&y| dist[y] <= 3 && ok(y)).collect();
        let far = (0..g.edges()).filter(|&y| (4..=5).contains(&dist[y]) && ok(y)).collect();
        (near, far)
    }

    pub(super) fn move_guard(&mut self, from: usize, to: usize, run: bool) {
        let p = self.edge_knight[from];
        if run {
            let mover = self.current as usize; // (the neutral knight's run is paid by whoever moves it)
            self.hands[mover][2] -= 1;
        }
        self.edge_knight[from] = NOBODY;
        self.edge_knight[to] = p;
        self.knight_moved[to] = true;
    }

    // The end of a turn: knights move (if the player has any), then the raiders are driven off
    // (returns false when there's nothing to decide: expel() has run)
    pub(super) fn start_guard_moves(&mut self) -> bool {
        if !self.raider_attack() {
            return false;
        }
        self.knight_moved.iter_mut().for_each(|m| *m = false);
        if self.edge_knights(self.current) == 0 && !(self.two() && self.neutral_placed) {
            self.expel();
            return false;
        }
        self.decide_then([Pending { seat: self.current, decision: Decision::MoveGuards }], Phase::ExpelDone);
        true
    }

    // Clockwise round the coast: free every hex where knights outnumber raiders
    pub(super) fn expel(&mut self) {
        let board = self.board.clone(); // (an Arc: the loop below changes the game)
        let g = &board.graph;
        for k in 0..self.coast.len() {
            let h = self.coast[k] as usize;
            let r = self.raiders_on[h];
            let edges: Vec<usize> = g.hex_edges[h].iter().map(|&e| e as usize).collect();
            let here: Vec<(usize, u8)> = edges.iter().enumerate().filter(|&(_, &e)| self.edge_knight[e] != NOBODY).map(|(side, &e)| (side, self.edge_knight[e])).collect();
            if r == 0 || here.len() as u8 <= r {
                continue;
            }
            // Prisoners for the players there
            let mut involved: Vec<u8> = Vec::new();
            for &(_, o) in &here {
                if !involved.contains(&o) {
                    involved.push(o);
                }
            }
            let count = |o: u8| here.iter().filter(|&&(_, x)| x == o).count();
            if involved.len() == 1 {
                self.gain_prisoners(involved[0], r);
            } else if r as usize >= involved.len() {
                for &o in &involved {
                    self.gain_prisoners(o, 1);
                }
                let mut left = r as usize - involved.len();
                while left > 0 {
                    let most = involved.iter().map(|&o| count(o)).max().unwrap();
                    let top: Vec<u8> = involved.iter().copied().filter(|&o| count(o) == most).collect();
                    let winner = self.dice_order(&top)[0];
                    self.gain_prisoners(winner, 1);
                    for &o in &top {
                        if o != winner {
                            self.consolation(o);
                        }
                    }
                    left -= 1;
                }
            } else {
                let order = self.dice_order(&involved);
                for (i, &o) in order.iter().enumerate() {
                    if i < r as usize {
                        self.gain_prisoners(o, 1);
                    } else {
                        self.consolation(o);
                    }
                }
            }
            self.raiders_on[h] = 0;
            // Knight losses: a die picks a pair of edge directions
            let die = self.rng.below(6) as usize + 1;
            let pair = match die {
                1 | 6 => 0,
                2 | 5 => 1,
                _ => 2,
            };
            for &(side, o) in &here {
                if side % 3 == pair && o != NEUTRAL[0] {
                    self.edge_knight[edges[side]] = NOBODY;
                    self.consolation(o);
                }
            }
        }
    }

    fn gain_prisoners(&mut self, p: u8, n: u8) {
        if (p as usize) < self.players {
            self.prisoners[p as usize] += n;
        }
    }

    // A player left out (or losing a knight): 3 coins; two players: 2 coins and a trade token
    fn consolation(&mut self, p: u8) {
        if (p as usize) >= self.players {
            return;
        }
        if self.two() {
            self.coins[p as usize] += 2;
            self.give_tokens(p as usize, Token::Trade, 1);
        } else {
            self.coins[p as usize] += 3;
        }
    }

    // Players in the order of a die roll each (rerolling ties)
    // (the neutral's "roll" is always a 3)
    fn dice_order(&mut self, players: &[u8]) -> Vec<u8> {
        let mut rolled: Vec<(u32, u8)> = players.iter().map(|&p| {
            let die = if p == NEUTRAL[0] { 2 } else { self.rng.below(6) as u32 };
            (die * 64 + self.rng.below(64) as u32, p)
        }).collect();
        rolled.sort_by(|a, b| b.0.cmp(&a.0));
        rolled.into_iter().map(|(_, p)| p).collect()
    }

    pub(super) fn raid_decided(&mut self, a: Action) {
        let p = self.to_act() as usize;
        match a {
            Action::Capture(h) => {
                self.raiders_on[h as usize] -= 1;
                self.prisoners[p] += 1;
                self.decided(None);
            }
            Action::PlaceGuard(e) => {
                let neutral = matches!(self.deciding().map(|d| d.decision), Some(Decision::PlaceGuard { keep: false })) && self.neutral_due;
                let first = self.two() && !self.neutral_placed && !neutral;
                self.edge_knight[e as usize] = if neutral { NEUTRAL[0] } else { p as u8 };
                if neutral {
                    self.neutral_due = false;
                    self.neutral_placed = true;
                }
                self.decided(None);
                // Two players: the first knight of the game brings a neutral knight (placed by its builder)
                if first {
                    self.neutral_due = true;
                    let back = self.phase;
                    self.decide_then([Pending { seat: p as u8, decision: Decision::PlaceGuard { keep: false } }], back);
                }
            }
            Action::MoveRaider(from, to) => {
                if from == NONE {
                    self.raider_supply -= 1;
                } else {
                    self.raiders_on[from as usize] -= 1;
                }
                self.raiders_on[to as usize] += 1;
                // The second move must come from another hex
                if let Some(next) = self.pending.get_mut(1) {
                    if next.decision == (Decision::MoveRaider { from: NONE }) {
                        next.decision = Decision::MoveRaider { from };
                    }
                }
                self.decided(None);
            }
            Action::GuardMove(f, t) | Action::GuardRun(f, t) => {
                self.move_guard(f as usize, t as usize, matches!(a, Action::GuardRun(..)));
                // Still deciding: more knights may move (drop_impossible ends it when none can)
                let mut list = Vec::new();
                self.legal_decision(self.pending[0], &mut list);
                if list.is_empty() || list == [Action::Skip] {
                    self.decided(None);
                }
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
#[path = "raiders_tests.rs"]
mod tests;
