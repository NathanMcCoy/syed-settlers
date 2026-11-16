// The variants box's scenario Merchant trains (`box.scenario=3`; RULES.md "The variants box:
// scenarios"). A watering hole (no number) replaces the Dunes in the island's middle; three trains may
// start at alternate corners of it, each on the edge leading away. Building any Hut or Lodge in a phase
// earns a wagon (one a phase), placed at the end of that phase after a vote: everyone, from the player
// acting, bids Fish and Grain (the cards are spent); a player with more than half the votes places it,
// otherwise the one with the most, otherwise (a tie) the player acting. A wagon starts a new train or
// goes on the front of one, nose to tail, on a land edge without a wagon; trains don't branch, and a
// train ends when its front meets another wagon. 22 wagons. A building with 2 or more wagons at its
// corner is worth +1 Renown; a wagon on a Path counts 1 more for its owner's Longest Path. 12 Renown
// to win. The squall starts off the island.
//
// Simplified (RULES.md): no negotiation without a majority (the most votes decide, then the player
// acting); bids of up to 9 of each.
use super::*;

pub const WAGONS: u8 = 22;
pub const MOST_BID: u8 = 9;

impl Game {
    pub fn merchant_trains(&self) -> bool {
        self.settings().box_scenario == 3
    }

    pub(super) fn merchant_setup(&mut self) {
        let g = &self.board.graph;
        self.wagon = vec![false; g.edges()];
        self.oasis = (0..self.board.terrain.len()).find(|&h| self.board.terrain[h] == Terrain::Oasis).map_or(NONE, |h| h as u16);
        self.wagons_left = WAGONS;
        self.trains = Vec::new();
        self.squall = NONE;
    }

    // The three start corners: corners 0, 2 and 4 of the watering hole
    pub fn train_starts(&self) -> Vec<usize> {
        if self.oasis == NONE {
            return Vec::new();
        }
        let corners = &self.board.graph.hex_corners[self.oasis as usize];
        [0, 2, 4].iter().map(|&k| corners[k] as usize).collect()
    }

    fn wagon_at(&self, c: usize) -> bool {
        listed(&self.board.graph.corner_edges[c]).any(|e| self.wagon[e])
    }

    // Where the next wagon may go: (edge, which train it extends or NONE for a new one, the corner it
    // reaches)
    pub fn wagon_spots(&self) -> Vec<(usize, u16, usize)> {
        let g = &self.board.graph;
        let mut out = Vec::new();
        if self.wagons_left == 0 {
            return out;
        }
        let oasis_edges: &[u16] = if self.oasis == NONE { &[] } else { &g.hex_edges[self.oasis as usize] };
        // A new train: the edge leading away from an unused start corner
        for c in self.train_starts() {
            if self.wagon_at(c) || self.trains.iter().any(|t| t.0 == c as u16) {
                continue;
            }
            for (k, n) in listed(&g.corner_corners[c]).enumerate() {
                let e = g.corner_edges[c][k];
                if !oasis_edges.contains(&e) && self.board.edge_land[e as usize] {
                    out.push((e as usize, NONE, n));
                }
            }
        }
        // The front of a train that hasn't ended
        for (i, &(_, front, ended)) in self.trains.iter().enumerate() {
            if ended {
                continue;
            }
            let f = front as usize;
            for (k, n) in listed(&g.corner_corners[f]).enumerate() {
                let e = g.corner_edges[f][k] as usize;
                if !self.wagon[e] && self.board.edge_land[e] && !oasis_edges.contains(&(e as u16)) {
                    out.push((e, i as u16, n));
                }
            }
        }
        out
    }

    pub(super) fn place_wagon(&mut self, e: usize) {
        let (_, train, far) = *self.wagon_spots().iter().find(|s| s.0 == e).expect("a wagon spot");
        // Does it meet another wagon at its far corner? Then this train ends (they merge)
        let meets = self.wagon_at(far);
        self.wagon[e] = true;
        self.wagons_left -= 1;
        if train == NONE {
            let [a, b] = self.board.graph.edge_corners[e];
            let start = if a as usize == far { b } else { a };
            self.trains.push((start, far as u16, meets));
        } else {
            let t = &mut self.trains[train as usize];
            t.1 = far as u16;
            t.2 = meets;
        }
        // A train that can't go on has ended too
        for i in 0..self.trains.len() {
            if !self.trains[i].2 && !self.wagon_spots().iter().any(|s| s.1 == i as u16) {
                self.trains[i].2 = true;
            }
        }
        self.update_titles();
    }

    // Renown from wagons: +1 for each of p's buildings with 2 or more wagons at its corner
    pub(super) fn wagon_renown(&self, p: usize) -> u32 {
        if !self.merchant_trains() {
            return 0;
        }
        let g = &self.board.graph;
        (0..self.corner_owner.len())
            .filter(|&c| self.corner_owner[c] as usize == p && listed(&g.corner_edges[c]).filter(|&e| self.wagon[e]).count() >= 2)
            .count() as u32
    }

    pub fn wagon_on(&self, e: usize) -> bool {
        self.wagon.get(e).copied().unwrap_or(false)
    }

    // At the end of a phase in which the player acting built: the vote, then the wagon
    // (returns false when there's no vote: the phase ends as usual)
    pub(super) fn start_vote(&mut self) -> bool {
        if !self.merchant_trains() || !self.wagon_due {
            return false;
        }
        self.wagon_due = false;
        if self.wagon_spots().is_empty() {
            return false;
        }
        self.bids = [0; MAX_PLAYERS];
        let bids: Vec<Pending> = (0..self.players)
            .map(|k| Pending { seat: ((self.current as usize + k) % self.players) as u8, decision: Decision::Bid })
            .collect();
        self.decide_then(bids, Phase::VoteDone);
        true
    }

    // Who places the wagon: more than half the votes; else the most votes; else the player acting
    pub fn vote_winner(&self) -> u8 {
        let votes = &self.bids[..self.players];
        let total: u32 = votes.iter().map(|&v| v as u32).sum();
        let best = *votes.iter().max().unwrap();
        let top: Vec<usize> = (0..self.players).filter(|&p| votes[p] == best).collect();
        if best > 0 && (2 * best as u32 > total || top.len() == 1) {
            top[0] as u8
        } else {
            self.current
        }
    }

    pub(super) fn resolve_vote(&mut self) {
        let seat = self.vote_winner();
        self.decide_then([Pending { seat, decision: Decision::PlaceWagon }], Phase::TurnEnd);
    }

    pub(super) fn bid(&mut self, p: usize, fish: u8, grain: u8) {
        self.hands[p][4] -= fish;
        self.hands[p][2] -= grain;
        self.bids[p] = fish + grain;
        self.decided(None);
    }
}

#[cfg(test)]
#[path = "merchant_tests.rs"]
mod tests;
