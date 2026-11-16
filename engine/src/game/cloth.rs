// The sea scenario Cloth (sea.scenario 6): villages on small islands (hexes of Terrain::Village with a
// number and 5 cloth each) trade cloth to everyone whose Paths and ships link one of their buildings
// to the village. 2 cloth = 1 Renown. No Huts on the small islands, no Longest Path, three setup Huts
// (only the third collects), and the game also ends when 5 villages are empty.
use super::*;

pub const VILLAGE_CLOTH: u8 = 5;
pub const GENERAL_CLOTH: u8 = 10;

impl Game {
    pub fn cloth(&self) -> bool {
        self.sea() && self.settings().sea_scenario == 6
    }

    // The villages (hex numbers), in board order
    pub fn villages(&self) -> Vec<usize> {
        (0..self.board.terrain.len()).filter(|&h| self.board.terrain[h] == Terrain::Village).collect()
    }

    // Corners p's network reaches from p's buildings along p's Paths and ships (not through another
    // player's building)
    fn network(&self, p: u8, skip: usize) -> Vec<bool> {
        let g = &self.board.graph;
        let mut seen = vec![false; g.corners()];
        let mut todo: Vec<usize> = (0..g.corners()).filter(|&c| self.corner_owner[c] == p).collect();
        for &c in &todo {
            seen[c] = true;
        }
        while let Some(c) = todo.pop() {
            if self.corner_owner[c] != NOBODY && self.corner_owner[c] != p {
                continue;
            }
            for e in listed(&g.corner_edges[c]) {
                if e == skip || self.edge_owner[e] != p {
                    continue;
                }
                let [a, b] = g.edge_corners[e];
                let n = if a as usize == c { b } else { a } as usize;
                if !seen[n] {
                    seen[n] = true;
                    todo.push(n);
                }
            }
        }
        seen
    }

    // New trade relations after p built or moved: each village p's network now touches (once ever)
    pub(super) fn check_villages(&mut self, p: usize) {
        if !self.cloth() {
            return;
        }
        let reach = self.network(p as u8, usize::MAX);
        for (k, h) in self.villages().into_iter().enumerate() {
            let bit = 1u16 << k;
            if self.village_links[p] & bit == 0 && self.board.graph.hex_corners[h].iter().any(|&c| reach[c as usize]) {
                self.village_links[p] |= bit;
                if self.village_cloth[k] > 0 {
                    self.village_cloth[k] -= 1;
                    self.give_tokens(p, Token::Cloth, 1);
                }
            }
        }
    }

    // A village whose number is rolled sends 1 cloth to each linked player (from its own supply, then
    // the general one); an empty village sends nothing
    pub(super) fn village_production(&mut self, total: u8) {
        if !self.cloth() {
            return;
        }
        for (k, h) in self.villages().into_iter().enumerate() {
            if self.board.number[h] != total || self.village_cloth[k] == 0 || h as u16 == self.squall {
                continue;
            }
            for p in 0..self.players {
                if self.village_links[p] & (1 << k) == 0 {
                    continue;
                }
                if self.village_cloth[k] > 0 {
                    self.village_cloth[k] -= 1;
                } else if self.general_cloth > 0 {
                    self.general_cloth -= 1;
                } else {
                    continue;
                }
                self.give_tokens(p, Token::Cloth, 1);
            }
        }
    }

    // Is the ship on edge e part of a line from one of p's buildings to a village? (It can't move.)
    pub(super) fn on_village_route(&self, p: u8, e: usize) -> bool {
        if !self.cloth() {
            return false;
        }
        let g = &self.board.graph;
        let without = self.network(p, e);
        let village_corner = |c: usize| listed(&g.corner_hexes[c]).any(|h| self.board.terrain[h] == Terrain::Village);
        // From the building side (still reached without the ship) on to a village beyond it
        let [a, b] = g.edge_corners[e];
        let beyond = |from: usize, to: usize| {
            if !without[from] {
                return false;
            }
            // Corners reachable from `to` along p's pieces, not using e
            let mut seen = vec![to];
            let mut todo = vec![to];
            while let Some(c) = todo.pop() {
                if village_corner(c) {
                    return true;
                }
                for f in listed(&g.corner_edges[c]) {
                    if f == e || self.edge_owner[f] != p {
                        continue;
                    }
                    let [x, y] = g.edge_corners[f];
                    let n = if x as usize == c { y } else { x } as usize;
                    if !seen.contains(&n) {
                        seen.push(n);
                        todo.push(n);
                    }
                }
            }
            false
        };
        beyond(a as usize, b as usize) || beyond(b as usize, a as usize)
    }

    // The pirate can't move until the player has a trade relation
    pub(super) fn pirate_free(&self, p: usize) -> bool {
        !self.cloth() || self.village_links[p] != 0
    }

    // At the end of a turn: with 5 or more empty villages, the most Renown wins (then the most cloth)
    pub(super) fn cloth_ending(&mut self) -> bool {
        if !self.cloth() || self.village_cloth.iter().filter(|&&n| n == 0).count() < 5 {
            return false;
        }
        let score = |g: &Game, p: usize| (g.renown(p), g.tokens_of(p, Token::Cloth));
        let best = (0..self.players).map(|p| score(self, p)).max().unwrap();
        let leaders: Vec<usize> = (0..self.players).filter(|&p| score(self, p) == best).collect();
        self.winner = if leaders.len() == 1 { leaders[0] as u8 } else { NOBODY };
        self.cloth_ended = true;
        self.phase = Phase::Over;
        self.offer = None;
        true
    }
}
