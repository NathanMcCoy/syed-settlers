// The sea scenario The Forgotten Tribe (sea.scenario 5): prizes on sea edges around the small islands
// (Renown tokens, face-down projects, markets) go to whoever builds or moves a ship onto them. A project
// counts as bought this turn; a market must be placed on the coast by one of the player's buildings
// (with an edge between markets) and is kept until there's a spot. Huts only next to numbered hexes;
// the squall only on numbered hexes.
use super::*;

// Prize kinds (Board::edge_prize / Game::prizes): a Renown token, a project, or a market of kind k (MARKET + k)
pub const PRIZE_RENOWN: u8 = 1;
pub const PRIZE_PROJECT: u8 = 2;
pub const PRIZE_MARKET: u8 = 10;

impl Game {
    pub fn tribe(&self) -> bool {
        self.sea() && self.settings().sea_scenario == 5
    }

    // A ship on edge e takes the prize there
    pub(super) fn take_prize(&mut self, p: usize, e: usize) {
        let prize = self.prizes.get(e).copied().unwrap_or(0);
        if prize == 0 {
            return;
        }
        self.prizes[e] = 0;
        match prize {
            PRIZE_RENOWN => self.give_tokens(p, Token::Renown, 1),
            PRIZE_PROJECT => {
                if let Some(k) = self.deck.pop() {
                    self.held[p][k as usize] += 1;
                    if p == self.current as usize {
                        self.bought[k as usize] += 1; // as if bought this turn
                    }
                }
            }
            m => self.held_markets[p].push(m - PRIZE_MARKET),
        }
    }

    // Where p could place a held market: a coastal edge (land one side, sea the other) at one of p's
    // buildings, with no market on it or on any edge sharing a corner with it
    pub(super) fn legal_markets(&self, p: usize, out: &mut Vec<Action>) {
        if self.held_markets[p].is_empty() {
            return;
        }
        let g = &self.board.graph;
        for e in 0..g.edges() {
            let hs: Vec<usize> = listed(&g.edge_hexes[e]).collect();
            let coast = hs.len() == 2 && (self.board.terrain[hs[0]] == Terrain::Sea) != (self.board.terrain[hs[1]] == Terrain::Sea)
                && !hs.iter().any(|&h| self.board.terrain[h] == Terrain::Fog);
            if !coast {
                continue;
            }
            let [a, b] = g.edge_corners[e];
            let ends = [a as usize, b as usize];
            if !ends.iter().any(|&c| self.corner_owner[c] as usize == p) {
                continue;
            }
            let near_market = ends.iter().any(|&c| self.board.corner_market[c] != NOBODY || listed(&g.corner_corners[c]).any(|n| self.board.corner_market[n] != NOBODY));
            if !near_market {
                out.push(Action::PlaceMarket(e as u16));
            }
        }
    }

    // Place p's oldest held market on edge e (the board gains a market)
    pub(super) fn place_market(&mut self, p: usize, e: usize) {
        let kind = self.held_markets[p].remove(0);
        let mut board = (*self.board).clone();
        let land = listed(&board.graph.edge_hexes[e]).find(|&h| board.terrain[h] != Terrain::Sea).unwrap();
        let side = board.graph.hex_edges[land].iter().position(|&x| x as usize == e).unwrap();
        board.markets.push(Market { hex: land as u16, side: side as u8, kind });
        let markers = board.corner_marker.clone();
        let Board { graph, hexes, markets, settings, .. } = board;
        let mut rebuilt = Board::from_parts(graph, hexes, markets, settings);
        rebuilt.corner_marker = markers;
        self.board = Arc::new(rebuilt);
    }
}

#[cfg(test)]
#[path = "tribe_tests.rs"]
mod tests;
