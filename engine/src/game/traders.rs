// The variants box's final scenario, Traders and raiders (`box.scenario=5`; RULES.md "The variants box:
// scenarios"). Three works replace a Paddy, a Fields and the Dunes: the Keep (wants Marble and Glass,
// supplies Tools and Sand), the Glassworks (wants Sand; supplies Glass and Tools) and the Stoneworks
// (wants Tools; supplies Sand and Marble), each with a stack of 12 goods. No 2 or 12 (rolled again), no
// squall, no Longest Path. Your second setup building is a Lodge; your wagon starts there, with 5 coins.
//
// - At the end of your turn your wagon moves, by its move points (wagon level 0-4: 4, 5, 6, 7, 7; a
//   Grain once a turn: +2): each step along an edge costs 2, 1 on your own Path, 1 + a coin (to the
//   owner) on another player's Path, 2 more with a raider on it. From a corner of a works, entering it
//   (its interior path to the plaza) costs 2 more and ends the move: a good it wants is delivered
//   (1 Renown, and coins by level: 1-5), then you pick up one of its goods (one at a time).
// - From level 1, at a corner by a raider, try to drive it off (once each a turn): a die of 6 (level
//   1), 5-6, 4-6, 3-6 moves it to any free edge.
// - Upgrade the wagon: Timber + Fish + Stone, Timber + Stone, then 2 Timber + Fish + Stone twice; level 4
//   is 1 Renown.
// - A 7: discards, then move a raider to a free edge; landing on a Path robs its owner. Projects here:
//   Knight 16 (the same raider move), Path Builders 3 (2 free Paths, as Road Crew), Swift Journey 3
//   (the wagon moves twice this turn; the Good Harvest slot), Renown 3 (as Lighthouses).
// - Coins as in Rivers. 13 Renown to win.
//
// Simplified (RULES.md): a works is entered from any of its corners for 2 move points (no Paths on its
// interior paths); the
// three raiders start on random land edges; goods are drawn at random from each works' two kinds.
use super::*;

pub const WAGON_MP: [u8; 5] = [4, 5, 6, 7, 7];
pub const DRIVE_FROM: [u8; 5] = [7, 6, 5, 4, 3]; // a die at least this drives a raider off (7: never)
pub const DELIVERY_COINS: [u8; 5] = [1, 2, 3, 4, 5];
pub const ENTER_WORKS: u8 = 2; // move points to enter a works from its corner
pub const UPGRADES: [Hand; 4] = [[1, 1, 0, 0, 1], [1, 1, 0, 0, 0], [2, 1, 0, 0, 1], [2, 1, 0, 0, 1]];
// Goods
pub const TOOLS: u8 = 0;
pub const SAND: u8 = 1;
pub const MARBLE: u8 = 2;
pub const GLASS: u8 = 3;
pub const GOODS: [&str; 4] = ["tools", "sand", "marble", "glass"];
// The works, in Terrain order Keep-like: (terrain, wants, supplies)
pub const WORKS: [(Terrain, &[u8], [u8; 2]); 3] = [
    (Terrain::Keep, &[MARBLE, GLASS], [TOOLS, SAND]),
    (Terrain::Glassworks, &[SAND], [GLASS, TOOLS]),
    (Terrain::Stoneworks, &[TOOLS], [SAND, MARBLE]),
];

impl Game {
    pub fn traders(&self) -> bool {
        self.settings().box_scenario == 5
    }

    pub(super) fn traders_setup(&mut self) {
        let g = &self.board.graph;
        self.barbarian = vec![false; g.edges()];
        self.works = WORKS.iter().map(|w| (0..self.board.terrain.len()).find(|&h| self.board.terrain[h] == w.0).map_or(NONE, |h| h as u16)).collect();
        for k in 0..3 {
            let mut stack: Vec<u8> = [WORKS[k].2[0], WORKS[k].2[1]].iter().flat_map(|&t| [t; 6]).collect();
            self.rng.shuffle(&mut stack);
            self.goods_stacks[k] = stack;
        }
        // Three raiders on random land edges away from the works
        let edges: Vec<usize> = (0..g.edges()).filter(|&e| self.board.edge_land[e] && !self.board.edge_sea[e]).collect();
        let mut placed = 0;
        while placed < 3 && !edges.is_empty() {
            let e = *self.rng.pick(&edges);
            if !self.barbarian[e] {
                self.barbarian[e] = true;
                placed += 1;
            }
        }
        self.wagon_corner = [NONE; MAX_PLAYERS];
        self.cargo = [NOBODY; MAX_PLAYERS];
        self.coins = [5; MAX_PLAYERS];
        self.squall = NONE;
    }

    // Which works (0-2) corner c belongs to, if any
    pub fn works_at(&self, c: usize) -> Option<usize> {
        (0..self.works.len()).find(|&k| self.works[k] != NONE && self.board.graph.hex_corners[self.works[k] as usize].contains(&(c as u16)))
    }

    // What one step along edge e costs p in move points and coins (None: can't, no coin for a toll)
    pub fn step_cost(&self, p: u8, e: usize) -> Option<(u8, u8)> {
        let owner = self.edge_owner[e];
        let (mut mp, coin) = if owner == p {
            (1, 0)
        } else if owner != NOBODY && (owner as usize) < self.players {
            (1, 1)
        } else {
            (2, 0)
        };
        if self.barbarian[e] {
            mp += 2;
        }
        (coin == 0 || self.coins[p as usize] > 0).then_some((mp, coin))
    }

    pub(super) fn legal_drive(&self, p: u8, out: &mut Vec<Action>) {
        let pu = p as usize;
        let g = &self.board.graph;
        let c = self.wagon_corner[pu] as usize;
        for (k, n) in listed(&g.corner_corners[c]).enumerate() {
            let e = g.corner_edges[c][k] as usize;
            if !self.board.edge_land[e] {
                continue;
            }
            if let Some((mp, _)) = self.step_cost(p, e) {
                if mp <= self.wagon_mp {
                    out.push(Action::WagonStep(n as u16));
                }
            }
        }
        // Enter the works here: with a good it wants, or an empty wagon to load
        if let Some(w) = self.works_at(c) {
            let cargo = self.cargo[pu];
            let worth = cargo == NOBODY || WORKS[w].1.contains(&cargo);
            if worth && self.wagon_mp >= ENTER_WORKS {
                out.push(Action::EnterWorks);
            }
        }
        if self.wagon_level[pu] >= 1 {
            for e in listed(&g.corner_edges[c]) {
                if self.barbarian[e] && !self.driven.contains(&(e as u16)) {
                    out.push(Action::DriveOff(e as u16));
                }
            }
        }
        if !self.boosted && self.hands[pu][2] > 0 {
            out.push(Action::Boost);
        }
        out.push(Action::Skip);
    }

    // The end of a turn: the wagon moves (once, or twice with Swift Journey)
    pub(super) fn start_drive(&mut self) -> bool {
        if !self.traders() || self.wagon_corner[self.current as usize] == NONE {
            return false;
        }
        self.wagon_mp = WAGON_MP[self.wagon_level[self.current as usize] as usize];
        self.driven.clear();
        self.decide_then([Pending { seat: self.current, decision: Decision::Drive }], Phase::TurnEnd);
        true
    }

    pub(super) fn trade_decided(&mut self, a: Action) {
        let p = self.to_act();
        let pu = p as usize;
        match a {
            Action::WagonStep(n) => {
                let c = self.wagon_corner[pu] as usize;
                let g = &self.board.graph;
                let k = listed(&g.corner_corners[c]).position(|x| x == n as usize).unwrap();
                let e = g.corner_edges[c][k] as usize;
                let (mp, coin) = self.step_cost(p, e).unwrap();
                self.wagon_mp -= mp;
                if coin > 0 {
                    self.coins[pu] -= 1;
                    self.coins[self.edge_owner[e] as usize] += 1;
                }
                self.wagon_corner[pu] = n;
            }
            Action::EnterWorks => {
                // The works' plaza: deliver, then pick up; the move ends
                self.wagon_mp -= ENTER_WORKS;
                if let Some(w) = self.works_at(self.wagon_corner[pu] as usize) {
                    let cargo = self.cargo[pu];
                    if cargo != NOBODY && WORKS[w].1.contains(&cargo) {
                        self.delivered[pu] += 1;
                        self.coins[pu] += DELIVERY_COINS[self.wagon_level[pu] as usize] as u16;
                        self.cargo[pu] = NOBODY;
                    }
                    if self.cargo[pu] == NOBODY {
                        if self.goods_stacks[w].is_empty() {
                            let mut refill: Vec<u8> = WORKS[w].2.iter().flat_map(|&t| [t; 6]).collect();
                            self.rng.shuffle(&mut refill);
                            self.goods_stacks[w] = refill;
                        }
                        self.cargo[pu] = self.goods_stacks[w].pop().unwrap();
                    }
                    self.end_drive();
                }
            }
            Action::DriveOff(e) => {
                self.driven.push(e);
                let die = self.rng.below(6) as u8 + 1;
                if die >= DRIVE_FROM[self.wagon_level[pu] as usize] {
                    // Driven off: the player puts it on any free edge (no robbery)
                    self.pending.insert(0, Pending { seat: p, decision: Decision::MoveBarbarian { from: e, steal: false } });
                }
            }
            Action::Boost => {
                self.hands[pu][2] -= 1;
                self.boosted = true;
                self.wagon_mp += 2;
            }
            Action::Skip => self.end_drive(),
            Action::BarbarianTo(from, to) => {
                let steal = matches!(self.deciding().map(|d| d.decision), Some(Decision::MoveBarbarian { steal: true, .. }));
                self.barbarian[from as usize] = false;
                self.barbarian[to as usize] = true;
                let owner = self.edge_owner[to as usize];
                self.decided(None);
                if steal && owner != NOBODY && owner != p && (owner as usize) < self.players {
                    self.rob(pu, owner as usize); // landing on a Path robs its owner
                }
            }
            Action::UpgradeWagon => {
                let lvl = self.wagon_level[pu] as usize;
                pay(&mut self.hands[pu], &UPGRADES[lvl]);
                self.wagon_level[pu] += 1;
            }
            _ => unreachable!(),
        }
    }

    // The wagon stops; with Swift Journey played, it moves once more with fresh points
    fn end_drive(&mut self) {
        if self.swift {
            self.swift = false;
            self.wagon_mp = WAGON_MP[self.wagon_level[self.current as usize] as usize];
            return; // the Drive decision stays
        }
        self.decided(None);
    }

    pub(super) fn legal_barbarian(&self, d: Pending, out: &mut Vec<Action>) {
        let Decision::MoveBarbarian { from, .. } = d.decision else { return };
        let froms: Vec<u16> = if from == NONE { (0..self.barbarian.len()).filter(|&e| self.barbarian[e]).map(|e| e as u16).collect() } else { vec![from] };
        let free: Vec<u16> = (0..self.barbarian.len()).filter(|&e| !self.barbarian[e] && self.board.edge_land[e]).map(|e| e as u16).collect();
        for &f in &froms {
            for &t in &free {
                out.push(Action::BarbarianTo(f, t));
            }
        }
    }

    pub(super) fn legal_upgrade(&self, p: usize, out: &mut Vec<Action>) {
        if self.traders() && self.wagon_level[p] < 4 && afford(&self.hands[p], &UPGRADES[self.wagon_level[p] as usize]) {
            out.push(Action::UpgradeWagon);
        }
    }

    // Renown from trading: 1 per delivery, 1 for a fully upgraded wagon
    pub(super) fn trade_renown(&self, p: usize) -> u32 {
        if !self.traders() {
            return 0;
        }
        self.delivered[p] as u32 + (self.wagon_level[p] == 4) as u32
    }
}

#[cfg(test)]
#[path = "traders_tests.rs"]
mod tests;
