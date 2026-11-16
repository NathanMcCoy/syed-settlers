// The variants box's scenario Fishing (`box.scenario=1`; RULES.md "The variants box: scenarios"). The
// Dunes are a Lake in the island's middle, paying on 2, 3, 11 and 12; six fishing grounds (numbered sea
// hexes on the coast) pay on their numbers. Buildings there draw fish tokens (Hut 1, Lodge 2) from a bag
// of 11 one-fish, 10 two-fish and 8 three-fish tokens and the old boot. Fish aren't cards: not in the
// hand limit, never stolen or traded. At most 7 tokens (the boot aside).
//
// On your turn after rolling, spend fish (excess fish are lost): 2 sends the squall off the island
// (it comes back with the next 7 or Pilot), 3 steals a card from a player, 4 takes a resource of your
// choice, 5 builds a free Path, 7 takes a free project. The old boot's holder needs 1 more Renown to win
// and may pass it on their turn to a player with as much Renown or more. 10 Renown to win. The squall
// starts off the island; it may sit on the Lake (blocking all four numbers).
//
// Simplified (RULES.md): a fishing ground pays every building on its coast corners; spending picks the
// tokens that waste the fewest fish; a player at 7 tokens draws no more (no swap).
use super::*;

pub const LAKE_NUMBERS: [u8; 4] = [2, 3, 11, 12];
pub const FISH_COSTS: [u8; 5] = [2, 3, 4, 5, 7]; // remove the squall, steal, take, free Path, free project
pub const OLD_BOOT: u8 = 0;                       // the boot in the bag (fish tokens are 1, 2, 3)
pub const MOST_FISH: usize = 7;

impl Game {
    pub fn fishing(&self) -> bool {
        self.settings().box_scenario == 1
    }

    // The bag, the Lake, and the squall off the island (Game::new)
    pub(super) fn fish_setup(&mut self) {
        let mut bag: Vec<u8> = [(1u8, 11), (2, 10), (3, 8), (OLD_BOOT, 1)].iter().flat_map(|&(v, n)| std::iter::repeat_n(v, n)).collect();
        self.rng.shuffle(&mut bag);
        self.fish_bag = bag;
        self.lake = (0..self.board.terrain.len()).find(|&h| self.board.terrain[h] == Terrain::Lake).map_or(NONE, |h| h as u16);
        self.boot = NOBODY;
        self.squall = NONE;
    }

    pub fn fish_total(&self, p: usize) -> u32 {
        self.fish[p].iter().map(|&v| v as u32).sum()
    }

    // Is hex h fishing water: the Lake, or a fishing ground (a numbered sea hex)?
    fn fishing_water(&self, h: usize) -> bool {
        h as u16 == self.lake || (self.board.terrain[h] == Terrain::Sea && self.board.number[h] != 0)
    }

    // p draws n fish tokens, one at a time; the old boot is shown at once and doesn't count toward 7
    fn draw_fish(&mut self, p: usize, n: u8) {
        for _ in 0..n {
            if self.fish[p].len() >= MOST_FISH {
                return;
            }
            if self.fish_bag.is_empty() {
                let mut used = std::mem::take(&mut self.fish_used);
                self.rng.shuffle(&mut used);
                self.fish_bag = used;
            }
            let Some(t) = self.fish_bag.pop() else { return };
            if t == OLD_BOOT {
                self.boot = p as u8;
            } else {
                self.fish[p].push(t);
            }
        }
    }

    // A roll's fish: the Lake on its four numbers (unless the squall sits on it) and fishing grounds on
    // theirs pay every building around them, in turn order from the roller
    pub(super) fn fish_production(&mut self, total: u8) {
        if !self.fishing() {
            return;
        }
        let g = &self.board.graph;
        let mut owed = [0u8; MAX_PLAYERS];
        for h in 0..self.board.terrain.len() {
            let pays = if h as u16 == self.lake {
                LAKE_NUMBERS.contains(&total) && self.squall != self.lake
            } else {
                self.fishing_water(h) && self.board.number[h] == total
            };
            if !pays {
                continue;
            }
            for &c in &g.hex_corners[h] {
                let o = self.corner_owner[c as usize];
                if (o as usize) < self.players {
                    owed[o as usize] += self.corner_kind[c as usize];
                }
            }
        }
        for k in 0..self.players {
            let p = (self.current as usize + k) % self.players;
            if owed[p] > 0 {
                self.draw_fish(p, owed[p]);
            }
        }
    }

    // Setup: a second Hut by fishing water draws one token
    pub(super) fn setup_fish(&mut self, p: usize, c: usize) {
        if self.fishing() && listed(&self.board.graph.corner_hexes[c]).any(|h| self.fishing_water(h)) {
            self.draw_fish(p, 1);
        }
    }

    pub(super) fn legal_fish(&self, p: u8, out: &mut Vec<Action>) {
        if !self.fishing() {
            return;
        }
        let pu = p as usize;
        let total = self.fish_total(pu);
        for (k, &cost) in FISH_COSTS.iter().enumerate() {
            if total < cost as u32 {
                continue;
            }
            let possible = match k {
                0 => self.squall != NONE,
                1 => (0..self.players).any(|o| o != pu && self.cards(o) > 0),
                2 => (0..5).any(|r| self.bank(r) > 0),
                3 => self.pieces[pu][P_PATH] > 0 && {
                    let mut paths = Vec::new();
                    self.legal_paths(p, &mut paths);
                    !paths.is_empty()
                },
                _ => !self.deck.is_empty(),
            };
            if possible {
                out.push(Action::Fish(k as u8));
            }
        }
        if self.boot == p {
            let mine = self.public_renown(pu);
            for o in 0..self.players {
                if o != pu && self.public_renown(o) >= mine {
                    out.push(Action::GiveBoot(o as u8));
                }
            }
        }
    }

    // Pay at least `cost` fish with the tokens that waste the fewest (then use the fewest tokens)
    fn pay_fish(&mut self, p: usize, cost: u8) {
        let tokens = self.fish[p].clone();
        let mut best: Option<(u32, u32, u32)> = None; // (waste, count, subset)
        for mask in 1u32..(1 << tokens.len()) {
            let sum: u32 = (0..tokens.len()).filter(|&i| mask & (1 << i) != 0).map(|i| tokens[i] as u32).sum();
            if sum < cost as u32 {
                continue;
            }
            let key = (sum - cost as u32, mask.count_ones(), mask);
            if best.is_none_or(|b| (key.0, key.1) < (b.0, b.1)) {
                best = Some(key);
            }
        }
        let (_, _, mask) = best.expect("enough fish (legal_fish checked)");
        for i in (0..tokens.len()).rev() {
            if mask & (1 << i) != 0 {
                let t = self.fish[p].remove(i);
                self.fish_used.push(t);
            }
        }
    }

    pub(super) fn spend_fish(&mut self, p: usize, k: u8) {
        self.pay_fish(p, FISH_COSTS[k as usize]);
        let seat = p as u8;
        match k {
            0 => self.squall = NONE,
            1 => self.decide_then([Pending { seat, decision: Decision::Rob }], Phase::Main),
            2 => self.decide_then([Pending { seat, decision: Decision::Pick { left: 1 } }], Phase::Main),
            3 => {
                self.back = Phase::Main;
                self.phase = Phase::FreeBuild { left: 1 };
                self.finish_free_build();
            }
            _ => {
                if let Some(k) = self.deck.pop() {
                    self.held[p][k as usize] += 1;
                    self.bought[k as usize] += 1; // as if bought this turn
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "fishing_tests.rs"]
mod tests;
