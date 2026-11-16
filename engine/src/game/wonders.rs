// The sea scenario Wonders (sea.scenario 8): claim one of five wonders by meeting its requirement,
// then build its 4 levels (each level costs the same 5 resources). You win by finishing it, or with 10
// Renown and a higher level than anyone else. Wall and bridge markers sit on main-island corners.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Wonder {
    GreatBridge,   // a building on a bridge marker
    GrandTheater,  // 2 Lodges
    GreatWall,     // a building on a wall marker
    GrandMonument, // a Lodge on a market and a line of 5+ Paths/ships
    GrandCastle,   // a Lodge and 6+ Renown
}
pub const WONDERS: [Wonder; 5] = [Wonder::GreatBridge, Wonder::GrandTheater, Wonder::GreatWall, Wonder::GrandMonument, Wonder::GrandCastle];

// Corner markers (Board::corner_marker)
pub const WALL: u8 = 1;
pub const BRIDGE: u8 = 2;
pub const NO_SETUP: u8 = 3; // the "X" beside a bridge marker: no setup Hut

impl Wonder {
    pub fn name(self) -> &'static str {
        ["greatBridge", "grandTheater", "greatWall", "grandMonument", "grandCastle"][self as usize]
    }
    // Each level's cost, from the tiles (read off the rulebook's picture): Timber, Stone, Grain, Copper, Fish
    pub fn cost(self) -> Hand {
        match self {
            Wonder::GreatBridge => [1, 0, 1, 3, 0],
            Wonder::GrandTheater => [0, 2, 3, 0, 0],
            Wonder::GreatWall => [3, 0, 1, 0, 1],
            Wonder::GrandMonument => [1, 0, 0, 1, 3],
            Wonder::GrandCastle => [0, 3, 1, 1, 0],
        }
    }
}

impl Game {
    pub fn wonders_on(&self) -> bool {
        self.sea() && self.settings().sea_scenario == 8
    }

    // Does p meet this wonder's requirement?
    pub fn meets(&self, p: usize, w: Wonder) -> bool {
        let mine = |c: usize| self.corner_owner[c] as usize == p;
        let lodge = |c: usize| mine(c) && self.corner_kind[c] == LODGE;
        let corners = 0..self.corner_owner.len();
        match w {
            Wonder::GreatBridge => corners.clone().any(|c| mine(c) && self.board.corner_marker[c] == BRIDGE),
            Wonder::GreatWall => corners.clone().any(|c| mine(c) && self.board.corner_marker[c] == WALL),
            Wonder::GrandTheater => self.lodges(p) >= 2,
            Wonder::GrandMonument => corners.clone().any(|c| lodge(c) && self.board.corner_market[c] != NOBODY) && self.line[p] >= 5,
            Wonder::GrandCastle => self.lodges(p) >= 1 && self.renown(p) >= 6,
        }
    }

    pub(super) fn legal_wonders(&self, p: usize, out: &mut Vec<Action>) {
        if !self.wonders_on() {
            return;
        }
        match self.wonder_of[p] {
            NOBODY => {
                for w in WONDERS {
                    if self.wonder_owner[w as usize] == NOBODY && self.meets(p, w) {
                        out.push(Action::ClaimWonder(w as u8));
                    }
                }
            }
            w => {
                if self.wonder_level[p] < 4 && afford(&self.hands[p], &WONDERS[w as usize].cost()) {
                    out.push(Action::BuildWonder);
                }
            }
        }
    }

    // Wonders win: a finished wonder, or 10 Renown and the highest level (strictly)
    pub(super) fn wonder_win(&self, p: usize) -> bool {
        if !self.wonders_on() {
            return false;
        }
        let level = self.wonder_level[p];
        level == 4 || (level > 0 && self.renown(p) >= 10 && (0..self.players).all(|o| o == p || self.wonder_level[o] < level))
    }
}

#[cfg(test)]
#[path = "wonders_tests.rs"]
mod tests;
