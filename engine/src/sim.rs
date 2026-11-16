// Whole games between bots, for balance checks and speed: `cli sim` runs thousands of these.
use crate::bot::{Bot, Greedy, RandomBot};
use crate::game::*;
use crate::island::generate;
use crate::settings::Settings;

// Where a player's Renown came from: Huts, Lodges, Lighthouses, Longest Path, Most Pilots
pub const SOURCES: [&str; 5] = ["huts", "lodges", "lighthouses", "longest path", "most pilots"];

#[derive(Clone, Debug)]
pub struct Outcome {
    pub winner: u8, // NOBODY if it hit the turn limit
    pub turns: u32,
    pub actions: u32,
    pub sources: [[u32; 5]; MAX_PLAYERS],
    pub token_renown: [u32; MAX_PLAYERS], // Renown from tokens (e.g. the sea's island bonus); not in `sources`
}

pub fn renown_sources(g: &Game, p: usize) -> [u32; 5] {
    let s = g.settings();
    [
        g.huts(p) as u32,
        2 * g.lodges(p) as u32,
        g.held[p][Project::Lighthouse as usize] as u32,
        if g.holder(Title::LongestPath) as usize == p { s.longest_renown as u32 } else { 0 },
        if g.holder(Title::MostPilots) as usize == p { s.most_pilots_renown as u32 } else { 0 },
    ]
}

// The kinds of bot the simulator can seat
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotKind {
    Greedy,
    Random,
}

impl BotKind {
    pub fn parse(name: &str) -> Result<BotKind, String> {
        match name {
            "greedy" => Ok(BotKind::Greedy),
            "random" => Ok(BotKind::Random),
            _ => Err(format!("unknown bot {name} (greedy or random)")),
        }
    }
    fn make(self, seed: u32) -> Box<dyn Bot> {
        match self {
            BotKind::Greedy => Box::new(Greedy::new(seed)),
            BotKind::Random => Box::new(RandomBot::new(seed)),
        }
    }
}

// One game of Greedy bots on the island made from seed `sim<seed>`
pub fn play(settings: &Settings, seed: u32, players: usize, max_turns: u32) -> Result<Outcome, String> {
    play_with(settings, seed, players, max_turns, &[BotKind::Greedy])
}

// The same, with these kinds of bot in the seats (repeated round the table: [Greedy, Random] alternates)
pub fn play_with(settings: &Settings, seed: u32, players: usize, max_turns: u32, kinds: &[BotKind]) -> Result<Outcome, String> {
    let island = generate(settings, &format!("sim{seed}"))?;
    let mut g = Game::new(&island, settings, players, seed.wrapping_mul(2654435761));
    let mut bots: Vec<Box<dyn Bot>> =
        (0..players).map(|i| kinds[i % kinds.len()].make(seed ^ (i as u32 + 1).wrapping_mul(0x9E3779B9))).collect();
    let mut legal = Vec::with_capacity(256);
    let mut actions = 0;
    while !g.over() && g.turn <= max_turns {
        g.legal(&mut legal);
        let p = g.to_act() as usize;
        // Never stuck: a game that isn't over always has something to do
        if legal.is_empty() {
            return Err(format!("stuck: nothing legal for seat {p} in {:?} (seed {seed})", g.phase));
        }
        let a = bots[p].choose(&g, &legal);
        g.apply_unchecked(a);
        actions += 1;
    }
    let mut sources = [[0; 5]; MAX_PLAYERS];
    for (p, row) in sources.iter_mut().enumerate().take(players) {
        *row = renown_sources(&g, p);
    }
    let mut token_renown = [0; MAX_PLAYERS];
    for (p, t) in token_renown.iter_mut().enumerate().take(players) {
        *t = g.token_renown(p);
    }
    Ok(Outcome { winner: g.winner, turns: g.turn, actions, sources, token_renown })
}
