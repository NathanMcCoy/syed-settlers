// The variants box's variants (EXPANSIONS.md, A): small rule switches that mix with any game.
//   variant.friendly  the squall leaves alone players with only 2 Renown
//   variant.ports     Strongest Ports: a title for Renown in buildings on markets; 1 more to win
// (Event cards and the two-player variant have their own files.)
use super::*;

impl Game {
    // Renown everyone can see (hidden Lighthouses don't count): what "only 2 Renown" is about
    pub fn public_renown(&self, p: usize) -> u32 {
        let hidden = if self.settings().projects_face_up { 0 } else { self.held[p][Project::Lighthouse as usize] as u32 };
        self.renown(p) - hidden
    }

    // Friendly squall: is p protected (a building of theirs keeps the squall away)?
    pub(super) fn protected(&self, p: usize) -> bool {
        self.settings().variant_friendly && self.public_renown(p) <= 2
    }

    // Friendly squall: may the squall go to hex h? Not next to a protected player's building
    pub(super) fn squall_allowed(&self, h: usize) -> bool {
        !self.settings().variant_friendly
            || self.board.graph.hex_corners[h].iter().all(|&c| {
                let o = self.corner_owner[c as usize];
                o == NOBODY || !self.protected(o as usize)
            })
    }

    // Friendly squall: with nowhere allowed, the squall goes to the Dunes (any land hex without one)
    pub(super) fn squall_fallback(&self) -> Vec<u16> {
        let dunes: Vec<u16> = self.board.land_hexes.iter().copied().filter(|&h| self.board.terrain[h as usize] == Terrain::Dunes && h != self.squall).collect();
        if dunes.is_empty() { self.board.land_hexes.iter().copied().filter(|&h| h != self.squall).collect() } else { dunes }
    }

    // Strongest Ports: Renown in p's buildings on markets (a Hut 1, a Lodge 2)
    pub(super) fn port_renown(&self, p: usize) -> u8 {
        (0..self.corner_owner.len())
            .filter(|&c| self.corner_owner[c] as usize == p && self.board.corner_market[c] != NOBODY)
            .map(|c| self.corner_kind[c])
            .sum()
    }

    // Worked out again whenever a building is placed or upgraded
    pub(super) fn refresh_ports(&mut self) {
        if self.settings().variant_ports {
            self.refresh_title(Title::StrongestPorts);
        }
    }
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod tests;
