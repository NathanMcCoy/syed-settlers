// The sea scenario The Pirate Islands (sea.scenario 7). No squall: a pirate fleet circles the main
// island by the lower die each roll and attacks the players with buildings on its hex. Each player
// builds one unbranching line of ships, by a shortest route, from a coastal building on the main island
// to their beachhead (where a Hut may go) and on to their pirate fortress; Pilots only turn the line's
// first normal ship into a warship. Reaching the fortress, the player attacks it at the end of each of
// their turns: more warships than the die take a lair (3 take the fortress), fewer lose 2 ships, a tie
// loses 1. A captured fortress and 10 Renown win. No Longest Path, no Most Pilots.
//
// Each player starts with a Hut and a ship on the coast facing their islet (the rulebook's pre-placed
// pieces), then sets up two Huts as usual. Simplified (noted in RULES.md): all of a player's ships are
// their one line (no other routes); ships don't move; a line walled in by others may detour.
use super::*;

pub const PIRATES: u8 = 200; // corner_owner of a fortress still held by the pirates

impl Game {
    pub fn pirate_islands(&self) -> bool {
        self.sea() && self.settings().sea_scenario == 7
    }

    // Distances along free sea edges to a corner (for "a shortest route": the shortest still open;
    // a line walled in by others detours, see line_ships)
    fn sea_distance(&self, target: usize) -> Vec<u32> {
        let g = &self.board.graph;
        let mut dist = vec![u32::MAX; g.corners()];
        dist[target] = 0;
        let mut queue = std::collections::VecDeque::from([target]);
        while let Some(c) = queue.pop_front() {
            for (k, n) in listed(&g.corner_corners[c]).enumerate() {
                let e = g.corner_edges[c][k] as usize;
                if self.board.edge_sea[e] && self.edge_owner[e] == NOBODY && dist[n] == u32::MAX {
                    dist[n] = dist[c] + 1;
                    queue.push_back(n);
                }
            }
        }
        dist
    }

    // Where p's line goes next: the beachhead, then the fortress (None once the fortress is reached)
    pub fn line_target(&self, p: usize) -> Option<usize> {
        let tip = self.line_tip[p];
        if tip == self.fort_corner[p] {
            None
        } else if self.beach_reached[p] {
            Some(self.fort_corner[p] as usize)
        } else {
            Some(self.beach_corner[p] as usize)
        }
    }

    // The ships that may extend p's line: from its tip (or, to start, from a coastal building of p's on
    // the main island), one step closer to the target along a shortest route
    pub(super) fn line_ships(&self, p: u8, out: &mut Vec<Action>) {
        let pu = p as usize;
        let Some(target) = self.line_target(pu) else { return };
        let dist = self.sea_distance(target);
        let g = &self.board.graph;
        let starts: Vec<usize> = if self.line_tip[pu] == NONE {
            // A new line starts from p's main-island building nearest the target
            let own: Vec<usize> = (0..g.corners()).filter(|&c| self.corner_owner[c] == p && self.board.corner_region[c] == 0 && dist[c] != u32::MAX).collect();
            let best = own.iter().map(|&c| dist[c]).min().unwrap_or(u32::MAX);
            own.into_iter().filter(|&c| dist[c] == best).collect()
        } else {
            vec![self.line_tip[pu] as usize]
        };
        for c in starts {
            if dist[c] == 0 {
                continue;
            }
            if dist[c] == u32::MAX {
                // Walled in by other lines: any free sea edge on from the tip (a detour, not a block)
                for (k, n) in listed(&g.corner_corners[c]).enumerate() {
                    let e = g.corner_edges[c][k] as usize;
                    if self.board.edge_sea[e] && self.edge_owner[e] == NOBODY && self.corner_owner[n] == NOBODY {
                        out.push(Action::Ship(e as u16));
                    }
                }
                continue;
            }
            for (k, n) in listed(&g.corner_corners[c]).enumerate() {
                let e = g.corner_edges[c][k] as usize;
                if self.board.edge_sea[e] && self.edge_owner[e] == NOBODY && dist[n] + 1 == dist[c] && (self.corner_owner[n] == NOBODY || n == target) {
                    out.push(Action::Ship(e as u16));
                }
            }
        }
    }

    // The rulebook's starting pieces: each player's Hut on the main island's coast nearest their
    // beachhead (keeping the spacing), and the first ship of their line from it
    pub(super) fn pirate_start_pieces(&mut self) {
        for p in 0..self.players {
            let dist = self.sea_distance(self.beach_corner[p] as usize);
            let g = &self.board.graph;
            let best = (0..g.corners())
                .filter(|&c| self.board.corner_region[c] == 0 && self.spaced(c) && dist[c] != u32::MAX && listed(&g.corner_edges[c]).any(|e| self.board.edge_sea[e]))
                .min_by_key(|&c| (dist[c], c));
            let Some(c) = best else { continue };
            self.corner_owner[c] = p as u8;
            self.corner_kind[c] = HUT;
            self.pieces[p][P_HUT] -= 1;
            let mut next = Vec::new();
            self.line_ships(p as u8, &mut next);
            if let Some(&Action::Ship(e)) = next.first() {
                let e = e as usize;
                self.edge_owner[e] = p as u8;
                self.edge_kind[e] = SHIP;
                self.pieces[p][P_SHIP] -= 1;
                self.extend_line_to(p, e);
            }
        }
    }

    // (For bots: the same list)
    pub fn legal_line_ships(&self, p: u8, out: &mut Vec<Action>) {
        self.line_ships(p, out);
    }

    // After p's ship on edge e: the line's tip moves to its far end
    pub(super) fn extend_line_to(&mut self, p: usize, e: usize) {
        let [a, b] = self.board.graph.edge_corners[e];
        let (a, b) = (a as usize, b as usize);
        let tip = self.line_tip[p];
        let far = if tip != NONE && a == tip as usize { b } else if tip != NONE { a } else if self.corner_owner[a] as usize == p { b } else { a };
        self.line_tip[p] = far as u16;
        self.ship_line.push((p as u8, e as u16));
        if far == self.beach_corner[p] as usize {
            self.beach_reached[p] = true;
        }
    }

    pub fn warships(&self, p: usize) -> u32 {
        self.ship_line.iter().filter(|&&(o, e)| o as usize == p && self.warship[e as usize]).count() as u32
    }

    // A Pilot: p's normal ship nearest the line's start becomes a warship
    pub(super) fn make_warship(&mut self, p: usize) {
        if let Some(&(_, e)) = self.ship_line.iter().find(|&&(o, e)| o as usize == p && !self.warship[e as usize]) {
            self.warship[e as usize] = true;
        }
    }

    // The fleet sails `steps` hexes round its ring, then attacks the players with buildings on its hex
    // (strength `steps` against each one's warships). Returns the players who beat it (they each take
    // a resource of their choice).
    pub(super) fn fleet_sails(&mut self, steps: u8) -> Vec<Pending> {
        if !self.pirate_islands() || self.fleet.is_empty() || (0..self.players).all(|p| self.fort_taken[p]) {
            return Vec::new();
        }
        self.fleet_at = ((self.fleet_at as usize + steps as usize) % self.fleet.len()) as u8;
        let h = self.fleet[self.fleet_at as usize] as usize;
        let mut owners: Vec<usize> = Vec::new();
        for &c in &self.board.graph.hex_corners[h] {
            let o = self.corner_owner[c as usize] as usize;
            if o < self.players && !owners.contains(&o) {
                owners.push(o);
            }
        }
        let mut winners = Vec::new();
        for o in owners {
            let mine = self.warships(o);
            if steps as u32 > mine {
                // Lose 1 random card, plus 1 per Lodge
                for _ in 0..1 + self.lodges(o) {
                    let n = self.cards(o);
                    if n == 0 {
                        break;
                    }
                    let mut pick = self.rng.below(n as usize) as u32;
                    for r in 0..CARD_KINDS {
                        let have = self.hands[o][r] as u32;
                        if pick < have {
                            self.hands[o][r] -= 1;
                            break;
                        }
                        pick -= have;
                    }
                }
            } else if (steps as u32) < mine {
                winners.push(Pending { seat: o as u8, decision: Decision::Pick { left: 1 } });
            }
        }
        winners
    }

    // At the end of p's turn with the line at the fortress: attack it
    pub(super) fn attack_fortress(&mut self, p: usize) {
        if !self.pirate_islands() || self.fort_taken[p] || self.line_tip[p] != self.fort_corner[p] {
            return;
        }
        let die = self.rng.below(6) as u32 + 1;
        let mine = self.warships(p);
        self.last_attack = [p as u8, die as u8, mine as u8];
        if mine > die {
            self.lairs[p] -= 1;
            if self.lairs[p] == 0 {
                let c = self.fort_corner[p] as usize;
                self.fort_taken[p] = true;
                self.corner_owner[c] = p as u8;
                self.corner_kind[c] = HUT;
                if self.pieces[p][P_HUT] > 0 {
                    self.pieces[p][P_HUT] -= 1;
                } else {
                    self.give_tokens(p, Token::Renown, 1);
                }
            }
        } else {
            // Lose the ships nearest the fortress: 2, or 1 on a tie
            for _ in 0..if mine == die { 1 } else { 2 } {
                let Some(k) = self.ship_line.iter().rposition(|&(o, _)| o as usize == p) else { break };
                let (_, e) = self.ship_line.remove(k);
                let e = e as usize;
                self.edge_owner[e] = NOBODY;
                self.edge_kind[e] = 0;
                self.warship[e] = false;
                self.pieces[p][P_SHIP] += 1;
                // The tip goes back to the remaining end of that ship
                let [a, b] = self.board.graph.edge_corners[e];
                self.line_tip[p] = if a == self.line_tip[p] { b } else { a };
            }
            if !self.ship_line.iter().any(|&(o, _)| o as usize == p) {
                self.line_tip[p] = NONE;
            }
            self.beach_reached[p] = self.line_tip[p] != NONE && self.ship_line.iter().any(|&(o, e)| {
                o as usize == p && self.board.graph.edge_corners[e as usize].contains(&self.beach_corner[p])
            });
        }
    }
}

#[cfg(test)]
#[path = "pirates_tests.rs"]
mod tests;
