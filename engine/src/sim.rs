// Whole games between bots, for balance checks and speed: `cli sim` runs thousands of these.
use crate::bot::Greedy;
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
}

pub fn renown_sources(g: &Game, p: usize) -> [u32; 5] {
    let s = g.settings();
    [
        g.huts(p) as u32,
        2 * g.lodges(p) as u32,
        g.held[p][Project::Lighthouse as usize] as u32,
        if g.longest as usize == p { s.longest_renown as u32 } else { 0 },
        if g.most_pilots as usize == p { s.most_pilots_renown as u32 } else { 0 },
    ]
}

// One game of Greedy bots on the island made from seed `sim<seed>`
pub fn play(settings: &Settings, seed: u32, players: usize, max_turns: u32) -> Result<Outcome, String> {
    let island = generate(settings, &format!("sim{seed}"))?;
    let mut g = Game::new(&island, settings, players, seed.wrapping_mul(2654435761));
    let mut bots: Vec<Greedy> = (0..players).map(|i| Greedy::new(seed ^ (i as u32 + 1).wrapping_mul(0x9E3779B9))).collect();
    let mut legal = Vec::with_capacity(256);
    let mut actions = 0;
    while !g.over() && g.turn <= max_turns {
        g.legal(&mut legal);
        let p = g.to_act() as usize;
        let a = bots[p].choose(&g, &legal);
        g.apply_unchecked(a);
        actions += 1;
    }
    let mut sources = [[0; 5]; MAX_PLAYERS];
    for (p, row) in sources.iter_mut().enumerate().take(players) {
        *row = renown_sources(&g, p);
    }
    Ok(Outcome { winner: g.winner, turns: g.turn, actions, sources })
}
