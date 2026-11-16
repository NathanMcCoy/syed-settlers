// The sea expansion (EXPANSIONS.md, B): ships, moving one ship a turn, the pirate, gold fields and
// the bonus for reaching a small island. Everything here is off unless the settings say `sea`.
use super::*;

pub const SHIP: u8 = 2; // edge_kind of a ship (PATH is 1)
pub const P_SHIP: usize = 3; // ships still in hand, in `pieces`
pub const SHIP_COST: Hand = [1, 0, 0, 0, 1]; // Timber + Fish
pub const GOLD: u8 = 5; // Board::resource of a gold field: a resource of the player's choice

impl Game {
    pub fn sea(&self) -> bool {
        self.settings().sea
    }

    // One of p's pieces of this kind (PATH or SHIP) touches this corner
    pub(super) fn touches_own(&self, p: u8, c: usize, kind: u8) -> bool {
        listed(&self.board.graph.corner_edges[c]).any(|e| self.edge_owner[e] == p && self.edge_kind[e] == kind)
    }

    // The edge touches the pirate's hex (no ship may be built, moved to or moved from there)
    pub fn by_pirate(&self, e: usize) -> bool {
        self.pirate != NONE && listed(&self.board.graph.edge_hexes[e]).any(|h| h == self.pirate as usize)
    }

    // A ship could go on edge e for p: a free sea edge, not by the pirate, joined at one end to p's
    // building, or to p's ship where no other player's building stands. `without`: pretend the ship on
    // that edge isn't there (moving it).
    fn ship_spot(&self, p: u8, e: usize, without: usize) -> bool {
        if !self.board.edge_sea[e] || self.edge_owner[e] != NOBODY || self.by_pirate(e) {
            return false;
        }
        self.board.graph.edge_corners[e].iter().any(|&c| {
            let c = c as usize;
            self.corner_owner[c] == p
                || (self.corner_owner[c] == NOBODY
                    && listed(&self.board.graph.corner_edges[c])
                        .any(|f| f != without && self.edge_owner[f] == p && self.edge_kind[f] == SHIP))
        })
    }

    pub(super) fn legal_ships(&self, p: u8, out: &mut Vec<Action>) {
        for e in 0..self.edge_owner.len() {
            if self.ship_spot(p, e, NONE as usize) {
                out.push(Action::Ship(e as u16));
            }
        }
    }

    // Can p move the ship on edge e? Not built this phase, not by the pirate, and one end open: no ship
    // or building of p's there. (The rulebook also forbids moving a ship in a line joining two of p's
    // buildings, even if another player has built on it since; such a ship never has an open end, so
    // that rule needs no check of its own.)
    pub fn ship_movable(&self, p: u8, e: usize) -> bool {
        if self.edge_owner[e] != p || self.edge_kind[e] != SHIP || self.ships_new.contains(&(e as u16)) || self.by_pirate(e) {
            return false;
        }
        let g = &self.board.graph;
        let open = |c: usize| {
            self.corner_owner[c] != p && !listed(&g.corner_edges[c]).any(|f| f != e && self.edge_owner[f] == p && self.edge_kind[f] == SHIP)
        };
        let [a, b] = g.edge_corners[e];
        open(a as usize) || open(b as usize)
    }

    // Every legal ship move for p: a movable ship to any other legal ship spot (with it lifted off)
    pub(super) fn legal_ship_moves(&self, p: u8, out: &mut Vec<Action>) {
        for from in 0..self.edge_owner.len() {
            if !self.ship_movable(p, from) {
                continue;
            }
            for to in 0..self.edge_owner.len() {
                if to != from && self.ship_spot(p, to, from) {
                    out.push(Action::MoveShip(from as u16, to as u16));
                }
            }
        }
    }

    // Players p could steal from with the pirate on its hex: others with a ship on its edges and a card
    pub(super) fn pirate_victims(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(3);
        for &e in &self.board.graph.hex_edges[self.pirate as usize] {
            let o = self.edge_owner[e as usize];
            if o != NOBODY && self.edge_kind[e as usize] == SHIP && o != self.current && self.cards(o as usize) > 0 && !out.contains(&o) {
                out.push(o);
            }
        }
        out
    }

    // The first Hut on a small island earns its owner the island bonus (once per island per player)
    pub(super) fn island_bonus(&mut self, p: usize, c: usize) {
        let region = self.board.corner_region[c];
        let bonus = self.settings().sea_island_bonus as u16;
        if !self.sea() || region == 0 || region == SEA || bonus == 0 || region >= 16 {
            return;
        }
        let bit = 1u16 << region;
        if self.islands_reached[p] & bit == 0 {
            self.islands_reached[p] |= bit;
            self.give_tokens(p, Token::Renown, bonus);
        }
    }

    // Gold owed after a production roll (or a setup Hut), as decisions: each player picks that many
    // resources, starting with the player whose turn it is
    pub(super) fn gold_decisions(&self, owed: &[u8; MAX_PLAYERS]) -> Vec<Pending> {
        (0..self.players)
            .map(|k| (self.current as usize + k) % self.players)
            .filter(|&p| owed[p] > 0)
            .map(|p| Pending { seat: p as u8, decision: Decision::Gold { left: owed[p] } })
            .collect()
    }
}

#[cfg(test)]
#[path = "sea_tests.rs"]
mod tests;
