// Titles (F11 in EXPANSIONS.md): "the player strictly ahead in X, with at least N, gets R Renown".
// Every title is a row here: its value per player, its minimum and Renown from the settings, and when
// it's worked out again. The base game has two; expansions add rows (Strongest Ports, Wealthiest,
// mission tiles), not new code paths.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum Title {
    LongestPath, // the longest line of Paths (worked out after a Path or a building)
    MostPilots,  // the most Pilots played (worked out when one is played)
}
pub const TITLES: [Title; 2] = [Title::LongestPath, Title::MostPilots];
pub const TITLE_COUNT: usize = TITLES.len();

impl Title {
    // The name the page and the protocol use
    pub fn name(self) -> &'static str {
        match self {
            Title::LongestPath => "longestPath",
            Title::MostPilots => "mostPilots",
        }
    }

    pub fn min(self, s: &Settings) -> u8 {
        match self {
            Title::LongestPath => s.longest_min,
            Title::MostPilots => s.most_pilots_min,
        }
    }

    pub fn renown(self, s: &Settings) -> u8 {
        match self {
            Title::LongestPath => s.longest_renown,
            Title::MostPilots => s.most_pilots_renown,
        }
    }
}

impl Game {
    // Who holds a title, or NOBODY
    pub fn holder(&self, t: Title) -> u8 {
        self.holders[t as usize]
    }

    // A player's value for a title (what "the most" counts)
    pub fn title_value(&self, t: Title, p: usize) -> u8 {
        match t {
            Title::LongestPath => self.line[p],
            Title::MostPilots => self.pilots[p],
        }
    }

    // Work out who holds this title now, from the players' current values
    pub(super) fn refresh_title(&mut self, t: Title) {
        let mut values = [0u8; MAX_PLAYERS];
        for (p, v) in values.iter_mut().enumerate().take(self.players) {
            *v = self.title_value(t, p);
        }
        self.holders[t as usize] = title(self.holders[t as usize], &values[..self.players], t.min(self.settings()));
    }

    // Renown from titles for player p
    pub fn title_renown(&self, p: usize) -> u32 {
        TITLES.iter().filter(|&&t| self.holder(t) as usize == p).map(|&t| t.renown(self.settings()) as u32).sum()
    }
}

// Who holds a title now: the holder keeps it while nobody has strictly more; otherwise the one player
// strictly ahead takes it; with a tie for the lead (or nobody at the minimum) nobody holds it
pub fn title(holder: u8, values: &[u8], min: u8) -> u8 {
    let best = *values.iter().max().unwrap();
    if best < min {
        return NOBODY;
    }
    if holder != NOBODY && values[holder as usize] == best {
        return holder;
    }
    let mut leaders = values.iter().enumerate().filter(|&(_, &v)| v == best);
    let first = leaders.next().unwrap().0 as u8;
    if leaders.next().is_some() { NOBODY } else { first }
}
