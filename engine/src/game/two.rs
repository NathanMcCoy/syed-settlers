// The two-player variant (the variants box, variant.two; only with 2 players):
// - two neutral colours (owners NEUTRAL[0], NEUTRAL[1]) start with a Hut each; every Path a player
//   builds makes them build a Path for a neutral, every Hut a neutral Hut (or a Path if no Hut fits)
// - two production rolls a turn, the second a different number
// - trade tokens: 2 for a Hut by the Dunes, 1 for a Hut on the coast, 2 for cashing in a played Pilot
//   (once a turn); spend once a turn (1, or 2 when ahead) on a forced trade or sending the squall to
//   the Dunes
// - a neutral can hold Longest Path
use super::*;

pub const NEUTRAL: [u8; 2] = [2, 3];

impl Game {
    pub fn two(&self) -> bool {
        self.settings().variant_two
    }

    // Who titles are counted over: the players, and with the two-player variant the neutrals too
    pub(super) fn title_owners(&self) -> usize {
        if self.two() { 4 } else { self.players }
    }

    // At the start: each neutral's Hut on a random spaced corner of the main island
    pub(super) fn place_neutrals(&mut self) {
        for n in NEUTRAL {
            let spots: Vec<usize> = (0..self.corner_owner.len())
                .filter(|&c| self.spaced(c) && self.board.corner_region[c] == 0 && !self.board.graph.corner_hexes[c].iter().any(|&h| h != NONE && self.board.terrain[h as usize] == Terrain::Sea))
                .collect();
            let c = *self.rng.pick(&spots);
            self.corner_owner[c] = n;
            self.corner_kind[c] = HUT;
        }
    }

    // Trade tokens for a new Hut of p's: 2 by the Dunes, 1 on the coast
    pub(super) fn hut_tokens(&mut self, p: usize, c: usize) {
        if !self.two() {
            return;
        }
        let hexes: Vec<usize> = listed(&self.board.graph.corner_hexes[c]).collect();
        let dunes = hexes.iter().any(|&h| self.board.terrain[h] == Terrain::Dunes);
        let coast = hexes.iter().any(|&h| self.board.terrain[h] == Terrain::Sea) || hexes.len() < 3;
        let n = 2 * dunes as u16 + coast as u16;
        if n > 0 {
            self.give_tokens(p, Token::Trade, n);
        }
    }

    // The neutral build a player owes after building (a Hut owes a Hut, a Path a Path)
    pub(super) fn owe_neutral(&mut self, p: u8, hut: bool) -> Option<Pending> {
        if !self.two() {
            return None;
        }
        Some(Pending { seat: p, decision: Decision::Neutral { hut } })
    }

    pub(super) fn legal_neutral(&self, hut: bool, out: &mut Vec<Action>) {
        if hut {
            for n in NEUTRAL {
                for c in 0..self.corner_owner.len() {
                    if self.spaced(c) && self.touches_own(n, c, PATH) {
                        out.push(Action::NeutralHut(n, c as u16));
                    }
                }
            }
            if !out.is_empty() {
                return;
            }
        }
        // A Path (or a Hut owed with no spot: a Path instead)
        for n in NEUTRAL {
            for e in 0..self.edge_owner.len() {
                let joined = self.board.graph.edge_corners[e].iter().any(|&c| {
                    let c = c as usize;
                    self.corner_owner[c] == n || (self.corner_owner[c] == NOBODY && self.touches_own(n, c, PATH))
                });
                if self.board.edge_land[e] && self.edge_owner[e] == NOBODY && joined {
                    out.push(Action::NeutralPath(n, e as u16));
                }
            }
        }
    }

    // The cost of spending trade tokens: 1, or 2 when ahead of the other player (public Renown)
    pub fn token_cost(&self, p: usize) -> u16 {
        let other = 1 - p;
        if self.public_renown(p) <= self.public_renown(other) { 1 } else { 2 }
    }

    // Trade-token actions open to the player acting (before the roll or in the main phase)
    pub(super) fn legal_tokens(&self, p: usize, out: &mut Vec<Action>) {
        if !self.two() || self.paired {
            return;
        }
        if !self.pilot_cashed && self.pilots[p] > 0 {
            out.push(Action::CashPilot);
        }
        if !self.token_used && self.tokens_of(p, Token::Trade) >= self.token_cost(p) {
            if self.cards(1 - p) > 0 {
                out.push(Action::ForceTrade);
            }
            if self.board.terrain[self.squall as usize] != Terrain::Dunes && self.board.land_hexes.iter().any(|&h| self.board.terrain[h as usize] == Terrain::Dunes) {
                out.push(Action::SquallHome);
            }
        }
    }

    // A forced trade: take 2 random cards from the other player, then give 2 of your choice
    pub(super) fn force_trade(&mut self, p: usize) {
        let other = 1 - p;
        let cost = self.token_cost(p);
        self.spend_tokens(p, Token::Trade, cost);
        self.token_used = true;
        let taking = self.cards(other).min(2);
        for _ in 0..taking {
            self.rob(p, other);
        }
        let back = self.phase;
        let gifts = (0..2).map(|_| Pending { seat: p as u8, decision: Decision::Gift { to: other as u8 } });
        self.decide_then(gifts, back);
    }
}

#[cfg(test)]
#[path = "two_tests.rs"]
mod tests;
