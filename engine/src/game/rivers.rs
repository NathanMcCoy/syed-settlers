// The variants box's scenario Rivers (`box.scenario=2`; RULES.md "The variants box: scenarios"). Two
// rivers run in from the coast, each through 3 hexes, one of them a Swamp (the Swamps replace the
// Dunes: no number). Where a river crosses an edge (between its hexes, and at its mouth) is a bridge
// site: no Path there, only a bridge (2 Copper + Timber, 3 each, counts as a Path for Longest Path,
// joins like a Path, not with Road Crew).
//
// Coins: 1 for each Hut or Path built on a river hex (setup too), 3 for each bridge; bought from the
// bank like a resource (at your rate for what you give); 2 coins buy a resource of your choice, twice a
// turn. Coins aren't cards (no hand limit, no stealing). The single player with the most coins is the
// Wealthiest (+1 Renown); everyone tied for the fewest is Poor (-2 Renown). The 2 shares the 12's hex.
// 10 Renown to win.
//
// Simplified (RULES.md): coins aren't traded between players; the rivers are our own layout.
use super::*;

pub const BRIDGE_COST: Hand = [1, 0, 0, 2, 0]; // Timber + 2 Copper
pub const BRIDGES: u8 = 3;

impl Game {
    pub fn rivers(&self) -> bool {
        self.settings().box_scenario == 2
    }

    // Set the rivers up from the island (Game::new): river hexes and bridge sites
    pub(super) fn rivers_setup(&mut self, island: &Island) {
        let g = &self.board.graph;
        self.river_hex = vec![false; g.cells.len()];
        self.bridge_site = vec![false; g.edges()];
        self.bridge = vec![false; g.edges()];
        for &h in &island.rivers {
            self.river_hex[h as usize] = true;
        }
        for &e in &island.bridge_sites {
            self.bridge_site[e as usize] = true;
        }
        self.bridges_left = [BRIDGES; MAX_PLAYERS];
    }

    pub fn is_bridge_site(&self, e: usize) -> bool {
        self.bridge_site.get(e).copied().unwrap_or(false)
    }

    // Coins for a building on corner c or a Path on edge e touching a river hex
    pub(super) fn river_coins_corner(&mut self, p: usize, c: usize) {
        if self.rivers() && listed(&self.board.graph.corner_hexes[c]).any(|h| self.river_hex[h]) {
            self.coins[p] += 1;
        }
    }
    pub(super) fn river_coins_edge(&mut self, p: usize, e: usize) {
        if self.rivers() && listed(&self.board.graph.edge_hexes[e]).any(|h| self.river_hex[h]) {
            self.coins[p] += 1;
        }
    }

    // Renown from wealth: +1 for the single richest, -2 for everyone tied poorest
    pub(super) fn wealth_renown(&self, p: usize) -> i32 {
        if !self.rivers() {
            return 0;
        }
        let coins = &self.coins[..self.players];
        let most = *coins.iter().max().unwrap();
        let least = *coins.iter().min().unwrap();
        let mut r = 0;
        if coins[p] == most && coins.iter().filter(|&&c| c == most).count() == 1 {
            r += 1;
        }
        if coins[p] == least {
            r -= 2;
        }
        r
    }

    pub fn wealthiest(&self) -> u8 {
        (0..self.players).find(|&p| self.wealth_renown(p) > 0).map_or(NOBODY, |p| p as u8)
    }

    pub fn poor(&self, p: usize) -> bool {
        self.wealth_renown(p) < 0
    }

    pub(super) fn legal_rivers(&self, p: u8, out: &mut Vec<Action>) {
        if !self.rivers() {
            return;
        }
        let pu = p as usize;
        let hand = &self.hands[pu];
        if afford(hand, &BRIDGE_COST) && self.bridges_left[pu] > 0 && !self.paths_blocked(p) {
            for e in 0..self.edge_owner.len() {
                if self.bridge_site[e] && self.edge_connected(p, e) {
                    out.push(Action::Bridge(e as u16));
                }
            }
        }
        for r in 0..5 {
            if hand[r] >= self.rate(p, r) {
                out.push(Action::BuyCoin(r as u8));
            }
        }
        if self.coins[pu] >= 2 && self.coin_buys < 2 {
            for r in 0..5 {
                if self.bank(r) > 0 {
                    out.push(Action::SpendCoins(r as u8));
                }
            }
        }
    }

    pub(super) fn build_bridge(&mut self, p: usize, e: usize) {
        pay(&mut self.hands[p], &BRIDGE_COST);
        self.bridges_left[p] -= 1;
        self.edge_owner[e] = p as u8;
        self.edge_kind[e] = PATH; // a bridge counts as a Path
        self.bridge[e] = true;
        self.coins[p] += 3;
        self.update_titles();
    }
}

#[cfg(test)]
#[path = "rivers_tests.rs"]
mod tests;
