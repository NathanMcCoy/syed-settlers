// Makes a new island from settings and a seed (the same pair always makes the same island). RULES.md
// "The island" says what it guarantees; the tests at the bottom check every guarantee on thousands of
// islands. When a random attempt breaks a rule, the generator simply tries again.
//
// This runs once per game, so it favours being clear over being fast, but it never iterates over a
// HashMap/HashSet: their order changes from run to run, and the same seed must give the same island.
use crate::hex::{Cell, DIRS, Graph, NONE, listed, within};
use crate::json;
use crate::rng::Rng;
use crate::settings::Settings;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Terrain {
    Sea,
    Forest,
    Quarry,
    Fields,
    Mine,
    Paddy,
    Dunes,
    Gold, // the sea expansion: produces a resource of the player's choice
    Fog,  // the sea expansion's Fog Islands: a hidden hex, turned over when a Path or ship reaches it
    Village, // the sea scenario Cloth: a village trading cloth (a number, no resource)
    Lake,    // the variants box's Fishing: fish tokens on 2, 3, 11 and 12 (no resource)
}
use Terrain::*;

pub const PRODUCING: [Terrain; 5] = [Forest, Quarry, Fields, Mine, Paddy];
pub const RESOURCES: [&str; 5] = ["timber", "stone", "grain", "copper", "fish"];
pub const ANY: u8 = 5; // a general (3:1) market; 0â€“4 are the resources' 2:1 markets

impl Terrain {
    // 0â€“4 (RESOURCES), or None for Sea and Dunes
    pub fn resource(self) -> Option<usize> {
        match self {
            Forest => Some(0),
            Quarry => Some(1),
            Fields => Some(2),
            Mine => Some(3),
            Paddy => Some(4),
            Sea | Dunes | Gold | Fog | Village | Lake => None,
        }
    }
    pub fn name(self) -> &'static str {
        ["sea", "forest", "quarry", "fields", "mine", "paddy", "dunes", "gold", "fog", "village", "lake"][self as usize]
    }
}

// How many of the 36 dice rolls give this number (2 and 12: 1, ..., 6 and 8: 5); 0 for no number
pub fn pips(n: u8) -> u32 {
    if n == 0 { 0 } else { (6 - (7 - n as i32).abs()) as u32 }
}
pub fn strong(n: u8) -> bool {
    n == 6 || n == 8
}

// Numbers for `count` producing hexes: the usual 18, minus the rarest or plus middle ones
const BASE_NUMBERS: [u8; 18] = [2, 3, 3, 4, 4, 5, 5, 6, 6, 8, 8, 9, 9, 10, 10, 11, 11, 12];
const REMOVE_ORDER: [u8; 8] = [2, 12, 3, 11, 4, 10, 5, 9];
const ADD_ORDER: [u8; 8] = [5, 9, 6, 8, 4, 10, 3, 11];
// The large island's 28 numbers (5-6 player rules): two 2s and two 12s, three of every other
const LARGE_NUMBERS: [u8; 28] = [2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12];

// The numbers for an island made with these settings: the large island's own set when it has its
// usual 28 producing hexes, otherwise number_tokens()
pub fn tokens_for(s: &Settings) -> Vec<u8> {
    let producing = main_size(s) - s.island_dunes as usize;
    if s.island_large && producing == LARGE_NUMBERS.len() { LARGE_NUMBERS.to_vec() } else { number_tokens(producing) }
}

pub fn number_tokens(count: usize) -> Vec<u8> {
    let mut list = BASE_NUMBERS.to_vec();
    let mut i = 0;
    while list.len() > count {
        let n = REMOVE_ORDER[i % REMOVE_ORDER.len()];
        if let Some(at) = list.iter().position(|&x| x == n) {
            list.remove(at);
        }
        i += 1;
    }
    let mut i = 0;
    while list.len() < count {
        list.push(ADD_ORDER[i % ADD_ORDER.len()]);
        i += 1;
    }
    list
}

// The classic number order: laid on the hexagon in a spiral, from a corner of the outer ring
// inwards, counterclockwise, skipping the Dunes
pub const SPIRAL_NUMBERS: [u8; 18] = [5, 2, 6, 3, 8, 10, 9, 12, 11, 4, 8, 10, 9, 4, 5, 6, 3, 11];

// Does this island get its numbers in the spiral? Only the hexagon with exactly 18 numbered hexes
// (The large island's spiral order isn't known: the 5-6 rulebook prints only the number sides of its
// 28 discs, so the large island always uses the fair shuffle.)
pub fn uses_spiral(s: &Settings) -> bool {
    s.island_spiral && s.island_hexagon && !s.island_large && main_size(s) - s.island_dunes as usize == SPIRAL_NUMBERS.len()
}

// The hexagon's cells in spiral order: the outer ring from the corner in direction `start`,
// counterclockwise on screen (DIRS turn counterclockwise as their index grows), then the ring
// inside it from its own corner in the same direction, then the middle
pub fn spiral_cells(start: usize) -> Vec<Cell> {
    let mut out = Vec::new();
    for radius in (1..=HEXAGON_RADIUS).rev() {
        let (dq, dr) = DIRS[start % 6];
        let mut c = Cell::new(dq * radius, dr * radius);
        for side in 0..6 {
            let (sq, sr) = DIRS[(start + 2 + side) % 6];
            for _ in 0..radius {
                out.push(c);
                c = Cell::new(c.q + sq, c.r + sr);
            }
        }
    }
    out.push(Cell::new(0, 0));
    out
}

const SEA_REACH: i32 = 2; // sea is every hex within this many steps of land
const FAIR_LOW: f64 = 0.6;
const FAIR_HIGH: f64 = 1.5;
const ATTEMPTS: u32 = 300;
pub(crate) const NUMBER_TRIES: usize = 400;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hex {
    pub cell: Cell,
    pub terrain: Terrain,
    pub number: u8, // 0: none
    pub region: u8, // which island: 0 the main one, 1, 2, ... small ones (the sea expansion); SEA for sea
}
pub const SEA: u8 = u8::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Market {
    pub hex: u16, // the land hex
    pub side: u8, // which of its sides (hex.rs's side order) faces the sea
    pub kind: u8, // 0â€“4 a resource (2:1), ANY (3:1)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub main_size: usize,
    pub main_average: f64, // pips per producing hex
    pub attempts: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Island {
    pub seed: String,
    pub hexes: Vec<Hex>, // row by row; a hex's number is its place here
    pub markets: Vec<Market>,
    pub squall: u16,
    pub pirate: u16, // the pirate's starting sea hex (the sea expansion), or NONE
    // Fog Islands: what lies under the fog (tiles and numbers, in no order: the game shuffles them)
    pub fog_tiles: Vec<Terrain>,
    pub fog_numbers: Vec<u8>,
    // Wonders: corner markers, as (hex, which corner of it 0-5, kind: 1 wall, 2 bridge, 3 no setup)
    pub markers: Vec<(u16, u8, u8)>,
    // The Forgotten Tribe: prizes on sea edges, as (hex, side, kind: 1 Renown, 2 project, 10 + market kind)
    pub prizes: Vec<(u16, u8, u8)>,
    // The Pirate Islands: per seat, its fortress and beachhead as (hex, corner of it); and the fleet's
    // ring of sea hexes round the main island, in order
    pub forts: Vec<((u16, u8), (u16, u8))>,
    pub fleet: Vec<u16>,
    pub info: Info,
}

impl Island {
    pub fn cells(&self) -> Vec<Cell> {
        self.hexes.iter().map(|h| h.cell).collect()
    }

    pub fn graph(&self) -> Graph {
        Graph::new(&self.cells())
    }

    pub fn to_json(&self) -> String {
        let sea = self.pirate != NONE;
        let hexes: Vec<String> = self.hexes.iter().map(|h| format!(
            "{{\"q\":{},\"r\":{},\"terrain\":\"{}\",\"number\":{}{}}}",
            h.cell.q, h.cell.r, h.terrain.name(),
            if h.number == 0 { "null".into() } else { h.number.to_string() },
            if !sea { String::new() } else if h.region == SEA { ",\"region\":null".into() } else { format!(",\"region\":{}", h.region) },
        )).collect();
        let markets: Vec<String> = self.markets.iter().map(|m| format!(
            "{{\"hex\":{},\"side\":{},\"kind\":\"{}\"}}",
            m.hex, m.side, if m.kind == ANY { "any" } else { RESOURCES[m.kind as usize] },
        )).collect();
        let i = &self.info;
        // Version 2: no islets (Nov 15)
        format!(
            "{{\"version\":2,\"seed\":{},\"hexes\":[{}],\"markets\":[{}],\"squall\":{}{},\"info\":{{\"mainSize\":{},\"mainAverage\":{},\"attempts\":{}}}}}",
            json::string(&self.seed), hexes.join(","), markets.join(","), self.squall,
            if sea { format!(",\"pirate\":{}", self.pirate) } else { String::new() },
            i.main_size, json::number(i.main_average), i.attempts,
        )
    }
}

pub fn generate(settings: &Settings, seed: &str) -> Result<Island, String> {
    let mut rng = Rng::from_text(seed);
    for attempt in 1..=ATTEMPTS {
        if let Some(mut island) = attempt_island(settings, &mut rng) {
            island.seed = seed.to_string();
            island.info.attempts = attempt;
            return Ok(island);
        }
    }
    Err(format!("no island met the rules after {ATTEMPTS} attempts (seed {seed}); try fewer markets"))
}

// The hexagon main island: every hex within this many steps of the middle (2: rows 3-4-5-4-3)
pub const HEXAGON_RADIUS: i32 = 2;

pub fn main_size(s: &Settings) -> usize {
    if s.island_large {
        large_cells().len()
    } else if s.island_hexagon {
        within(Cell::new(0, 0), HEXAGON_RADIUS).len()
    } else {
        s.island_size as usize
    }
}

// The large island (5-6 players): 30 hexes in rows of 3, 4, 5, 6, 5, 4, 3, centred on the middle
// row's centre. On the lattice (hex.rs) a cell's x is 2q + r; each row's x runs evenly either side of
// the middle, shifted by 1 so the 6-hex row (even length) lines up with its neighbours.
pub fn large_cells() -> Vec<Cell> {
    let mut out = Vec::new();
    for r in -3i32..=3 {
        let len = 6 - r.abs();
        let first = (-(len - 1) - r + 1) / 2;
        for q in first..first + len {
            out.push(Cell::new(q, r));
        }
    }
    out
}

fn attempt_island(s: &Settings, rng: &mut Rng) -> Option<Island> {
    let main = if s.island_large {
        large_cells()
    } else if s.island_hexagon {
        within(Cell::new(0, 0), HEXAGON_RADIUS)
    } else {
        grow_main(rng, s.island_size as usize)
    };
    let main_set: HashSet<Cell> = main.iter().copied().collect();
    if has_lakes(&main, &main_set) {
        return None;
    }
    let counts = if s.island_hexagon || s.island_large { Some(s.tile_counts()) } else { None };
    let land = if uses_spiral(s) { dress_spiral(rng, &main, s) } else { dress_main(rng, &main, s.island_dunes as usize, counts, tokens_for(s))? };
    let main_average = land.iter().map(|h| pips(h.number)).sum::<u32>() as f64
        / land.iter().filter(|h| h.number != 0).count() as f64;
    let mut land = land;
    if s.box_scenario == 1 {
        fishing_lake(rng, &mut land)?;
    }
    if s.sea && matches!(s.sea_scenario, 1 | 5 | 8) {
        land.extend(small_islands(rng, &main_set, s)?);
    } else if s.sea {
        // The other sea scenarios make all their land themselves (sea_maps.rs)
        land = crate::sea_maps::scenario_land(rng, s)?;
    }
    let (fog_tiles, fog_numbers) = if s.sea && s.sea_scenario == 3 { crate::sea_maps::fog_stacks() } else { (Vec::new(), Vec::new()) };
    // Markets and the squall's start: on the main island, or anywhere where setup is anywhere
    let everywhere = s.sea && crate::sea_maps::setup_anywhere(s.sea_scenario);

    // Sea around the island, then a fixed order (row by row)
    let land_set: HashSet<Cell> = land.iter().map(|h| h.cell).collect();
    let mut seen = HashSet::new();
    let mut sea = Vec::new();
    for h in &land {
        for c in within(h.cell, SEA_REACH) {
            if !land_set.contains(&c) && seen.insert(c) {
                sea.push(Hex { cell: c, terrain: Sea, number: 0, region: SEA });
            }
        }
    }
    let mut hexes = land;
    hexes.extend(sea);
    hexes.sort_by_key(|h| (h.cell.r, h.cell.q));
    let graph = Graph::new(&hexes.iter().map(|h| h.cell).collect::<Vec<_>>());

    let markets = place_markets(rng, &hexes, &graph, s.markets_any, s.markets_each, s.markets_extra_fish, everywhere)?;
    let mut hexes = hexes;
    if s.box_scenario == 1 {
        fishing_grounds(rng, &mut hexes, &graph, &markets)?;
    }
    let squall = *rng.pick(&squall_starts(&hexes, everywhere));
    // (Wonders and the Pirate Islands have no pirate; the Pirate Islands' fleet is its own)
    let pirate = if s.sea && !matches!(s.sea_scenario, 7 | 8) { pirate_start(rng, &hexes, s.sea_scenario == 1)? } else { NONE };
    let markers = if s.sea && s.sea_scenario == 8 { wonder_markers(rng, &hexes, &graph)? } else { Vec::new() };
    let prizes = if s.sea && s.sea_scenario == 5 { tribe_prizes(rng, &hexes, &graph)? } else { Vec::new() };
    let (forts, fleet) = if s.sea && s.sea_scenario == 7 { pirate_layout(&hexes, &graph)? } else { (Vec::new(), Vec::new()) };

    Some(Island {
        seed: String::new(),
        hexes,
        markets,
        squall,
        pirate,
        fog_tiles,
        fog_numbers,
        markers,
        prizes,
        forts,
        fleet,
        info: Info { main_size: main.len(), main_average, attempts: 0 },
    })
}

// Fishing: the Dunes move to the island's middle (the hex or one of its 6 neighbours) as the Lake; the
// hex they swap with takes the Dunes' place, number and all
fn fishing_lake(rng: &mut Rng, land: &mut [Hex]) -> Option<()> {
    let dunes = land.iter().position(|h| h.terrain == Dunes)?;
    let inner: Vec<usize> = (0..land.len()).filter(|&i| land[i].region == 0 && land[i].cell.distance(Cell::new(0, 0)) <= 1).collect();
    let i = *rng.pick(&inner);
    let (t, n) = (land[i].terrain, land[i].number);
    land[dunes].terrain = t;
    land[dunes].number = n;
    // (last, so a Dunes already in the middle simply becomes the Lake)
    land[i].terrain = Lake;
    land[i].number = 0;
    Some(())
}

// Fishing: six fishing grounds (numbered 4, 5, 6, 8, 9, 10) on sea hexes touching the island, spread
// evenly round it from a random start, away from the sea hexes the markets face
fn fishing_grounds(rng: &mut Rng, hexes: &mut [Hex], g: &Graph, markets: &[Market]) -> Option<()> {
    let faced: Vec<usize> = markets.iter().filter_map(|m| {
        let e = g.hex_edges[m.hex as usize][m.side as usize] as usize;
        listed(&g.edge_hexes[e]).find(|&h| hexes[h].terrain == Sea)
    }).collect();
    let land: Vec<Cell> = hexes.iter().filter(|h| h.terrain != Sea).map(|h| h.cell).collect();
    let angle = |c: Cell| ((3 * c.r) as f64 * 0.5).atan2((2 * c.q + c.r) as f64 * 0.866);
    let coast: Vec<usize> = (0..hexes.len())
        .filter(|&h| hexes[h].terrain == Sea && !faced.contains(&h) && land.iter().any(|l| l.distance(hexes[h].cell) == 1))
        .collect();
    let mut numbers = [4u8, 5, 6, 8, 9, 10];
    rng.shuffle(&mut numbers);
    let turn = rng.next_f64() * std::f64::consts::TAU;
    let mut used: Vec<usize> = Vec::new();
    for (k, &n) in numbers.iter().enumerate() {
        let want = turn + k as f64 * std::f64::consts::TAU / 6.0;
        let gap = |h: usize| {
            let d = (angle(hexes[h].cell) - want).rem_euclid(std::f64::consts::TAU);
            d.min(std::f64::consts::TAU - d)
        };
        let h = coast.iter().copied().filter(|h| !used.contains(h)).min_by(|&a, &b| gap(a).total_cmp(&gap(b)))?;
        used.push(h);
        hexes[h].number = n;
    }
    Some(())
}

// Where the squall may start: the island's Dunes, or with none, its weakest hexes (fewest pips)
fn squall_starts(hexes: &[Hex], everywhere: bool) -> Vec<u16> {
    let main = |i: &usize| !matches!(hexes[*i].terrain, Sea | Fog | Village) && (everywhere || hexes[*i].region == 0);
    let dunes: Vec<u16> = (0..hexes.len()).filter(main).filter(|&i| hexes[i].terrain == Dunes).map(|i| i as u16).collect();
    if !dunes.is_empty() {
        return dunes;
    }
    let weakest = (0..hexes.len()).filter(main).map(|i| pips(hexes[i].number)).min().unwrap();
    (0..hexes.len()).filter(main).filter(|&i| pips(hexes[i].number) == weakest).map(|i| i as u16).collect()
}

// The main island grows from one hex. Each new hex is picked from the coast's outside neighbours,
// weighted by how many land neighbours it has: inlets fill up (so no lakes), but not so strongly that
// every island comes out round.
const GROW_WEIGHT: [f64; 7] = [0.0, 1.0, 2.5, 5.0, 8.0, 12.0, 12.0];
fn grow_main(rng: &mut Rng, size: usize) -> Vec<Cell> {
    let start = Cell::new(0, 0);
    let mut land = vec![start];
    let mut set = HashSet::from([start]);
    let mut options: Vec<(Cell, f64)> = Vec::new();
    let mut seen = HashSet::new();
    while land.len() < size {
        options.clear();
        seen.clear();
        for &c in &land {
            for n in c.neighbors() {
                if set.contains(&n) || !seen.insert(n) {
                    continue;
                }
                let touching = n.neighbors().iter().filter(|m| set.contains(m)).count();
                options.push((n, GROW_WEIGHT[touching]));
            }
        }
        let mut roll = rng.next_f64() * options.iter().map(|o| o.1).sum::<f64>();
        let mut chosen = options[options.len() - 1].0;
        for &(c, w) in &options {
            roll -= w;
            if roll < 0.0 {
                chosen = c;
                break;
            }
        }
        land.push(chosen);
        set.insert(chosen);
    }
    land
}

// True if any non-land hex is cut off from the open sea (a lake inside the island)
fn has_lakes(land: &[Cell], set: &HashSet<Cell>) -> bool {
    let o = Cell::new(0, 0);
    let radius = land.iter().map(|c| c.distance(o)).max().unwrap() + 1;
    let start = Cell::new(radius, 0);
    let mut seen = HashSet::from([start]);
    let mut todo = vec![start];
    while let Some(c) = todo.pop() {
        for n in c.neighbors() {
            if n.distance(o) > radius || set.contains(&n) || !seen.insert(n) {
                continue;
            }
            todo.push(n);
        }
    }
    within(o, radius).iter().any(|c| !set.contains(c) && !seen.contains(c))
}

// Main island terrain and numbers, redrawn until the fairness checks pass
// The hexagon island uses the set number of each tile (`counts`, in PRODUCING's order); a random-
// shaped one shares its hexes out as evenly as possible
fn dress_main(rng: &mut Rng, main: &[Cell], dunes: usize, counts: Option<[u8; 5]>, tokens: Vec<u8>) -> Option<Vec<Hex>> {
    let producing = main.len() - dunes;
    let mut terrains: Vec<Terrain> = match counts {
        Some(c) => PRODUCING.iter().zip(c).flat_map(|(&t, n)| std::iter::repeat_n(t, n as usize)).collect(),
        None => {
            let per = producing / PRODUCING.len();
            let mut kinds = PRODUCING;
            rng.shuffle(&mut kinds);
            let mut list: Vec<Terrain> = PRODUCING.iter().flat_map(|&t| std::iter::repeat_n(t, per)).collect();
            list.extend(&kinds[..producing - per * PRODUCING.len()]);
            list
        }
    };
    terrains.extend(std::iter::repeat_n(Dunes, dunes));
    debug_assert_eq!(terrains.len(), main.len());
    let mut tokens = tokens;
    debug_assert_eq!(tokens.len(), producing);
    let index: HashMap<Cell, usize> = main.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let near: Vec<Vec<usize>> = main.iter().map(|c| c.neighbors().iter().filter_map(|n| index.get(n).copied()).collect()).collect();

    // Numbers go down one at a time, 6s and 8s first (they're the hardest to fit), each on a random
    // hex where it breaks no rule. Drawing them all at random and checking afterwards almost never
    // works on big islands, which need many repeats.
    let mut hexes: Vec<Hex> = main.iter().map(|&cell| Hex { cell, terrain: Dunes, number: 0, region: 0 }).collect();
    let mut open: Vec<usize> = Vec::new();
    for _ in 0..NUMBER_TRIES {
        rng.shuffle(&mut terrains);
        rng.shuffle(&mut tokens);
        tokens.sort_by_key(|&n| !strong(n)); // stable: strong first, the rest still shuffled
        for (h, &t) in hexes.iter_mut().zip(&terrains) {
            h.terrain = t;
            h.number = 0;
        }
        let mut placed = true;
        for &n in &tokens {
            open.clear();
            open.extend((0..hexes.len()).filter(|&i| {
                hexes[i].terrain != Dunes && hexes[i].number == 0 && near[i].iter().all(|&j| {
                    let o = hexes[j].number;
                    o != n && !(strong(o) && strong(n))
                })
            }));
            if open.is_empty() {
                placed = false;
                break;
            }
            let i = *rng.pick(&open);
            hexes[i].number = n;
        }
        if placed && numbers_apart(&hexes, &index) && fair_resources(&hexes) {
            return Some(hexes);
        }
    }
    None
}

// The classic way: tiles shuffled, then the numbers laid in SPIRAL_NUMBERS' order from a random
// corner, skipping the Dunes. Nothing is redrawn: the order itself spreads the numbers out.
fn dress_spiral(rng: &mut Rng, main: &[Cell], s: &Settings) -> Vec<Hex> {
    let mut terrains: Vec<Terrain> = PRODUCING.iter().zip(s.tile_counts()).flat_map(|(&t, n)| std::iter::repeat_n(t, n as usize)).collect();
    terrains.extend(std::iter::repeat_n(Dunes, s.island_dunes as usize));
    rng.shuffle(&mut terrains);
    let mut hexes: Vec<Hex> = main.iter().zip(&terrains).map(|(&cell, &terrain)| Hex { cell, terrain, number: 0, region: 0 }).collect();
    let index: HashMap<Cell, usize> = main.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let mut numbers = SPIRAL_NUMBERS.iter();
    for c in spiral_cells(rng.below(6)) {
        let h = &mut hexes[index[&c]];
        if h.terrain != Dunes {
            h.number = *numbers.next().unwrap();
        }
    }
    hexes
}

// No equal numbers next to each other, and no 6 or 8 next to another 6 or 8
fn numbers_apart(hexes: &[Hex], index: &HashMap<Cell, usize>) -> bool {
    hexes.iter().all(|h| {
        h.number == 0 || h.cell.neighbors().iter().all(|n| match index.get(n) {
            Some(&j) => {
                let other = hexes[j].number;
                other == 0 || (other != h.number && !(strong(other) && strong(h.number)))
            }
            None => true,
        })
    })
}

// ---- The sea expansion ----

// Small islands' tiles for New Shores (the rulebook's 4-player list in our names: 2 Mine, 1 Forest,
// 1 Paddy, 1 Fields, 2 Quarry, 2 Gold), their numbers (the 27 discs less the main island's 18), and
// for 5-6 players 2 more Gold with a 3 and an 11 (our own: that map is only a picture)
fn small_island_tiles(s: &Settings) -> (Vec<Terrain>, Vec<u8>, Vec<usize>) {
    let mut tiles = vec![Mine, Mine, Forest, Paddy, Fields, Quarry, Quarry, Gold, Gold];
    let mut numbers = vec![2, 3, 4, 5, 6, 8, 9, 10, 11];
    let mut sizes = vec![3, 3, 3];
    if s.island_large {
        tiles.extend([Gold, Gold]);
        numbers.extend([3, 11]);
        sizes.push(2);
    }
    (tiles, numbers, sizes)
}

// The small islands: each a few hexes, with at least one sea hex between it and any other land, near
// enough to the main island to sail to (2 or 3 steps from its coast). Numbers: no 6 or 8 together.
fn small_islands(rng: &mut Rng, main: &HashSet<Cell>, s: &Settings) -> Option<Vec<Hex>> {
    let (mut tiles, mut numbers, sizes) = small_island_tiles(s);
    let near_main = |c: Cell| main.iter().map(|m| m.distance(c)).min().unwrap();
    let mut taken: Vec<Cell> = main.iter().copied().collect();
    let mut out: Vec<Hex> = Vec::new();
    let far = |c: Cell, taken: &[Cell], own: &[Cell]| taken.iter().all(|t| own.contains(t) || t.distance(c) >= 2);
    let reach = taken.iter().map(|c| c.distance(Cell::new(0, 0))).max().unwrap() + 3;
    let candidates: Vec<Cell> = within(Cell::new(0, 0), reach).into_iter().filter(|&c| (3..=4).contains(&near_main(c))).collect();
    for (k, &size) in sizes.iter().enumerate() {
        let mut placed = None;
        for _ in 0..60 {
            let start = *rng.pick(&candidates);
            if !far(start, &taken, &[]) {
                continue;
            }
            let mut cells = vec![start];
            while cells.len() < size {
                let options: Vec<Cell> = cells.iter().flat_map(|c| c.neighbors()).filter(|&n| {
                    !cells.contains(&n) && (3..=4).contains(&near_main(n)) && far(n, &taken, &cells)
                }).collect();
                if options.is_empty() {
                    break;
                }
                let next = *rng.pick(&options);
                cells.push(next);
            }
            if cells.len() == size {
                placed = Some(cells);
                break;
            }
        }
        let cells = placed?;
        taken.extend(&cells);
        for c in cells {
            out.push(Hex { cell: c, terrain: Sea, number: 0, region: k as u8 + 1 });
        }
    }
    // Tiles and numbers, redrawn until no 6 or 8 is next to another
    let set: HashMap<Cell, usize> = out.iter().enumerate().map(|(i, h)| (h.cell, i)).collect();
    for _ in 0..NUMBER_TRIES {
        rng.shuffle(&mut tiles);
        rng.shuffle(&mut numbers);
        for (h, &t) in out.iter_mut().zip(&tiles) {
            h.terrain = t;
        }
        let mut next = numbers.iter();
        for h in out.iter_mut() {
            h.number = *next.next().unwrap();
        }
        let ok = out.iter().all(|h| !strong(h.number) || h.cell.neighbors().iter().all(|n| set.get(n).is_none_or(|&j| !strong(out[j].number))));
        if ok {
            return Some(out);
        }
    }
    None
}

// Wonders: 5 wall markers and 2 bridge markers on main-island coast corners, no two touching, and an
// "X" (no setup Hut) on each neighbour of a bridge marker
fn wonder_markers(rng: &mut Rng, hexes: &[Hex], g: &Graph) -> Option<Vec<(u16, u8, u8)>> {
    let land = |h: usize| hexes[h].terrain != Sea;
    let main_coast: Vec<usize> = (0..g.corners())
        .filter(|&c| {
            let hs: Vec<usize> = listed(&g.corner_hexes[c]).collect();
            hs.iter().any(|&h| land(h) && hexes[h].region == 0) && hs.iter().any(|&h| !land(h))
        })
        .collect();
    let mut picked: Vec<(usize, u8)> = Vec::new();
    let mut options = main_coast.clone();
    rng.shuffle(&mut options);
    for kind in [2u8, 2, 1, 1, 1, 1, 1] {
        let c = *options.iter().find(|&&c| picked.iter().all(|&(o, _)| o != c && !listed(&g.corner_corners[o]).any(|n| n == c)))?;
        picked.push((c, kind));
    }
    for &(c, kind) in picked.clone().iter() {
        if kind == 2 {
            for n in listed(&g.corner_corners[c]) {
                if picked.iter().all(|&(o, _)| o != n) {
                    picked.push((n, 3));
                }
            }
        }
    }
    // As (hex, corner of it) so the island's JSON can name them without the graph
    Some(picked.into_iter().map(|(c, kind)| {
        let h = listed(&g.corner_hexes[c]).find(|&h| land(h)).unwrap_or(g.corner_hexes[c][0] as usize);
        let k = g.hex_corners[h].iter().position(|&x| x as usize == c).unwrap();
        (h as u16, k as u8, kind)
    }).collect())
}

// The Pirate Islands: each islet's fortress (its westmost coastal corner) and beachhead (its eastmost),
// and the fleet's ring: the sea hexes touching the main island, in order round it
fn pirate_layout(hexes: &[Hex], g: &Graph) -> Option<(Vec<((u16, u8), (u16, u8))>, Vec<u16>)> {
    let regions: Vec<u8> = {
        let mut r: Vec<u8> = hexes.iter().filter(|h| h.terrain != Sea && h.region != 0).map(|h| h.region).collect();
        r.sort();
        r.dedup();
        r
    };
    let as_hex_corner = |c: usize, region: u8| {
        let h = listed(&g.corner_hexes[c]).find(|&h| hexes[h].region == region).unwrap();
        (h as u16, g.hex_corners[h].iter().position(|&x| x as usize == c).unwrap() as u8)
    };
    let mut forts = Vec::new();
    for region in regions {
        let coast: Vec<usize> = (0..g.corners())
            .filter(|&c| {
                let hs: Vec<usize> = listed(&g.corner_hexes[c]).collect();
                hs.iter().any(|&h| hexes[h].region == region) && hs.iter().any(|&h| hexes[h].terrain == Sea)
            })
            .collect();
        // The fortress: the islet's coastal corner farthest from the middle; the beachhead: the nearest
        // one not next to the fortress (a Hut must fit there once the fortress is taken)
        let far = |c: usize| { let (x, y) = g.corner_xy[c]; x * x * 3 + y * y };
        let fort = *coast.iter().max_by_key(|&&c| (far(c), c))?;
        let beach = *coast.iter().filter(|&&c| c != fort && !listed(&g.corner_corners[c]).any(|n| n == fort)).min_by_key(|&&c| (far(c), c))?;
        forts.push((as_hex_corner(fort, region), as_hex_corner(beach, region)));
    }
    let main: Vec<Cell> = hexes.iter().filter(|h| h.terrain != Sea && h.region == 0).map(|h| h.cell).collect();
    let (mx, my) = (main.iter().map(|c| c.center().0).sum::<i32>() as f64 / main.len() as f64, main.iter().map(|c| c.center().1).sum::<i32>() as f64 / main.len() as f64);
    let mut ring: Vec<(f64, u16)> = (0..hexes.len())
        .filter(|&h| hexes[h].terrain == Sea && main.iter().any(|m| m.distance(hexes[h].cell) == 1))
        .map(|h| {
            let (x, y) = hexes[h].cell.center();
            (((y as f64 - my) * 0.5).atan2((x as f64 - mx) * 0.866), h as u16)
        })
        .collect();
    ring.sort_by(|a, b| a.0.total_cmp(&b.0));
    Some((forts, ring.into_iter().map(|(_, h)| h).collect()))
}

// The Forgotten Tribe: 8 Renown tokens, 4 projects and 3 markets (one general, two of random
// resources) on sea edges touching the small islands, no two sharing a corner
fn tribe_prizes(rng: &mut Rng, hexes: &[Hex], g: &Graph) -> Option<Vec<(u16, u8, u8)>> {
    let small = |h: usize| hexes[h].terrain != Sea && hexes[h].region != 0;
    let mut edges: Vec<usize> = (0..g.edges())
        .filter(|&e| {
            let hs: Vec<usize> = listed(&g.edge_hexes[e]).collect();
            hs.len() == 2 && hs.iter().any(|&h| hexes[h].terrain == Sea) && g.edge_corners[e].iter().any(|&c| listed(&g.corner_hexes[c as usize]).any(small))
        })
        .collect();
    rng.shuffle(&mut edges);
    let mut kinds: Vec<u8> = vec![1; 8];
    kinds.extend([2; 4]);
    kinds.extend([10 + ANY, 10 + rng.below(5) as u8, 10 + rng.below(5) as u8]);
    let mut used = vec![false; g.corners()];
    let mut out = Vec::new();
    for e in edges {
        if out.len() == kinds.len() {
            break;
        }
        let [a, b] = g.edge_corners[e];
        if used[a as usize] || used[b as usize] {
            continue;
        }
        used[a as usize] = true;
        used[b as usize] = true;
        let h = listed(&g.edge_hexes[e]).find(|&h| hexes[h].terrain == Sea).unwrap();
        let side = g.hex_edges[h].iter().position(|&x| x as usize == e).unwrap();
        out.push((h as u16, side as u8, kinds[out.len()]));
    }
    if out.len() == kinds.len() { Some(out) } else { None }
}

// The pirate starts on a sea hex two steps from the main island (between it and the small islands); in
// other scenarios, two steps from the nearest land
fn pirate_start(rng: &mut Rng, hexes: &[Hex], from_main: bool) -> Option<u16> {
    let main: Vec<Cell> = hexes.iter().filter(|h| h.terrain != Sea && (!from_main || h.region == 0)).map(|h| h.cell).collect();
    let land: Vec<Cell> = hexes.iter().filter(|h| h.terrain != Sea).map(|h| h.cell).collect();
    let options: Vec<u16> = (0..hexes.len())
        .filter(|&i| hexes[i].terrain == Sea)
        .filter(|&i| main.iter().map(|m| m.distance(hexes[i].cell)).min() == Some(2))
        .filter(|&i| land.iter().all(|l| l.distance(hexes[i].cell) >= 2))
        .map(|i| i as u16)
        .collect();
    if options.is_empty() { None } else { Some(*rng.pick(&options)) }
}

// Each resource's total pips (how often it's produced, over all its hexes)
pub fn resource_pips(hexes: &[Hex]) -> [u32; 5] {
    let mut totals = [0; 5];
    for h in hexes {
        if let Some(r) = h.terrain.resource() {
            totals[r] += pips(h.number);
        }
    }
    totals
}

// Each resource's total within 60%â€“150% of the average resource's
fn fair_resources(hexes: &[Hex]) -> bool {
    let totals = resource_pips(hexes);
    let mean = totals.iter().sum::<u32>() as f64 / 5.0;
    totals.iter().all(|&t| (FAIR_LOW * mean..=FAIR_HIGH * mean).contains(&(t as f64)))
}

// Markets on the island's coastal edges (land on one side, sea on the other), with no two sharing a
// corner. Neighbouring corners are allowed (that's how 9 fit on the hexagon's coast); the spacing
// rule then lets one building use only one of the two.
//
// Like harbours printed on a sea frame, they're spread evenly around the coast: the coastal edges
// in order around the island, one market every (edges / markets) of them from a random start, kinds
// shuffled. If that ever puts two on a shared corner (only possible on a random-shaped coast), the
// markets go on random coastal edges instead.
fn place_markets(rng: &mut Rng, hexes: &[Hex], g: &Graph, any: u8, each: u8, extra_fish: u8, everywhere: bool) -> Option<Vec<Market>> {
    let mut kinds: Vec<u8> = std::iter::repeat_n(ANY, any as usize)
        .chain((0..5u8).flat_map(|r| std::iter::repeat_n(r, each as usize)))
        .chain(std::iter::repeat_n(4, extra_fish as usize))
        .collect();
    rng.shuffle(&mut kinds);
    let mut coast: Vec<(u16, u8, u16)> = Vec::new();
    for (h, hex) in hexes.iter().enumerate() {
        // Markets are on the main island's coast (the sea expansion's small islands have none)
        if matches!(hex.terrain, Sea | Fog | Village) || (hex.region != 0 && !everywhere) {
            continue;
        }
        for side in 0..6 {
            let n = g.hex_neighbors[h][side];
            if n != NONE && hexes[n as usize].terrain == Sea {
                coast.push((h as u16, side as u8, g.hex_edges[h][side]));
            }
        }
    }
    if kinds.is_empty() {
        return Some(Vec::new());
    }
    if let Some(markets) = spread_markets(rng, hexes, g, &coast, &kinds, everywhere) {
        return Some(markets);
    }
    let mut blocked = vec![false; g.corners()];
    for _ in 0..50 {
        rng.shuffle(&mut coast);
        blocked.fill(false);
        let mut chosen: Vec<(u16, u8)> = Vec::new();
        for &(hex, side, edge) in &coast {
            if chosen.len() == kinds.len() {
                break;
            }
            let ends = g.edge_corners[edge as usize];
            if ends.iter().any(|&c| blocked[c as usize]) {
                continue;
            }
            chosen.push((hex, side));
            for &c in &ends {
                blocked[c as usize] = true;
            }
        }
        if chosen.len() == kinds.len() {
            let mut markets: Vec<Market> = chosen.iter().zip(&kinds).map(|(&(hex, side), &kind)| Market { hex, side, kind }).collect();
            markets.sort_by_key(|m| (m.hex, m.side));
            return Some(markets);
        }
    }
    None
}

// Markets evenly spaced around the coast (see place_markets), or None if two would share a corner
fn spread_markets(rng: &mut Rng, hexes: &[Hex], g: &Graph, coast: &[(u16, u8, u16)], kinds: &[u8], everywhere: bool) -> Option<Vec<Market>> {
    // A point on the plane for a hex (the lattice's x is stretched: see hex.rs)
    let point = |c: Cell| {
        let (x, y) = c.center();
        (x as f64 * 3f64.sqrt() / 2.0, y as f64 / 2.0)
    };
    let land: Vec<(f64, f64)> = hexes.iter().filter(|h| !matches!(h.terrain, Sea | Fog) && (everywhere || h.region == 0)).map(|h| point(h.cell)).collect();
    let mid = (land.iter().map(|p| p.0).sum::<f64>() / land.len() as f64, land.iter().map(|p| p.1).sum::<f64>() / land.len() as f64);
    // Each coastal edge's angle around the island's middle (its midpoint: halfway to the sea hex)
    let mut around: Vec<(f64, (u16, u8, u16))> = coast.iter().map(|&(h, side, e)| {
        let (a, b) = (point(hexes[h as usize].cell), point(hexes[g.hex_neighbors[h as usize][side as usize] as usize].cell));
        (((a.1 + b.1) / 2.0 - mid.1).atan2((a.0 + b.0) / 2.0 - mid.0), (h, side, e))
    }).collect();
    around.sort_by(|x, y| x.0.total_cmp(&y.0));
    let n = around.len() as f64;
    let step = n / kinds.len() as f64;
    let start = rng.next_f64() * n;
    let mut used = vec![false; g.corners()];
    let mut markets = Vec::with_capacity(kinds.len());
    for (i, &kind) in kinds.iter().enumerate() {
        let (_, (hex, side, edge)) = around[((start + i as f64 * step) as usize) % around.len()];
        for &c in &g.edge_corners[edge as usize] {
            if std::mem::replace(&mut used[c as usize], true) {
                return None;
            }
        }
        markets.push(Market { hex, side, kind });
    }
    markets.sort_by_key(|m| (m.hex, m.side));
    Some(markets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::listed;

    fn settings(text: &str) -> Settings {
        Settings::parse(text).unwrap()
    }

    // Every guarantee in RULES.md "The island", checked on one island
    fn check(island: &Island, s: &Settings) {
        let g = island.graph();
        let hexes = &island.hexes;
        let at: HashMap<Cell, usize> = hexes.iter().enumerate().map(|(i, h)| (h.cell, i)).collect();
        let main: Vec<&Hex> = hexes.iter().filter(|h| h.terrain != Sea).collect();
        let why = format!("seed {} ({})", island.seed, s.to_text());

        // The main island: its size (or exactly the hexagon), all one piece, no lakes
        assert_eq!(main.len(), main_size(s), "{why}");
        if s.island_hexagon || s.island_large {
            let mut cells: Vec<Cell> = main.iter().map(|h| h.cell).collect();
            cells.sort();
            let mut want = if s.island_large { large_cells() } else { within(Cell::new(0, 0), HEXAGON_RADIUS) };
            want.sort();
            assert_eq!(cells, want, "hexagon: {why}");
        }
        let mut reached = HashSet::from([main[0].cell]);
        let mut todo = vec![main[0].cell];
        while let Some(c) = todo.pop() {
            for n in c.neighbors() {
                if at.get(&n).is_some_and(|&j| hexes[j].terrain != Sea) && reached.insert(n) {
                    todo.push(n);
                }
            }
        }
        assert_eq!(reached.len(), main.len(), "main island in one piece: {why}");
        let main_set: HashSet<Cell> = main.iter().map(|h| h.cell).collect();
        assert!(!has_lakes(&main.iter().map(|h| h.cell).collect::<Vec<_>>(), &main_set), "no lakes: {why}");

        // Terrain: the set number of dunes; on the hexagon exactly the set tiles, otherwise the five
        // others as even as possible, each 3+
        let dunes = main.iter().filter(|h| h.terrain == Dunes).count();
        assert_eq!(dunes, s.island_dunes as usize, "{why}");
        let counts: Vec<usize> = PRODUCING.iter().map(|&t| main.iter().filter(|h| h.terrain == t).count()).collect();
        if s.island_hexagon || s.island_large {
            assert_eq!(counts, s.tile_counts().map(|n| n as usize).to_vec(), "hexagon tiles: {why}");
        } else {
            let (lo, hi) = (*counts.iter().min().unwrap(), *counts.iter().max().unwrap());
            assert!(lo >= 3 && hi - lo <= 1, "terrain {counts:?}: {why}");
        }

        // Numbers: exactly the tokens, never 7, none on dunes or sea
        let mut numbers: Vec<u8> = main.iter().filter(|h| h.number != 0).map(|h| h.number).collect();
        numbers.sort();
        let mut tokens = tokens_for(s);
        tokens.sort();
        assert_eq!(numbers, tokens, "{why}");
        for h in hexes {
            assert_eq!(h.number == 0, matches!(h.terrain, Sea | Dunes), "{why}");
            assert!(h.number != 7 && h.number <= 12 && (h.number == 0 || h.number >= 2), "{why}");
        }

        // Spiral islands: the numbers follow the spiral from one of the 6 corners (checked here), and
        // aren't redrawn for fairness, so the shuffle's checks below don't apply
        if uses_spiral(s) {
            let follows = (0..6).any(|start| {
                let laid: Vec<u8> = spiral_cells(start).iter().map(|c| hexes[at[c]].number).filter(|&n| n != 0).collect();
                laid == SPIRAL_NUMBERS
            });
            assert!(follows, "numbers in spiral order: {why}");
        } else {
        // Neighbours: no equal numbers, no 6/8 pairs
        for h in hexes.iter().filter(|h| h.number != 0) {
            for n in h.cell.neighbors() {
                let Some(&j) = at.get(&n) else { continue };
                let o = &hexes[j];
                if o.number == 0 { continue; }
                assert_ne!(o.number, h.number, "equal numbers side by side: {why}");
                assert!(!(strong(o.number) && strong(h.number)), "6/8 side by side: {why}");
            }
        }

        // Fair shares of each resource
        let main_hexes: Vec<Hex> = main.iter().map(|&&h| h).collect();
        let totals = resource_pips(&main_hexes);
        let mean = totals.iter().sum::<u32>() as f64 / 5.0;
        for t in totals {
            assert!(t as f64 >= FAIR_LOW * mean && t as f64 <= FAIR_HIGH * mean, "resources {totals:?}: {why}");
        }
        }

        // Sea everywhere within 2 of land
        for h in hexes.iter().filter(|h| h.terrain != Sea) {
            for c in within(h.cell, 2) {
                assert!(at.contains_key(&c), "missing sea: {why}");
            }
        }

        // Markets: how many of each, on the coast, no corner shared
        assert_eq!(island.markets.len(), s.markets_any as usize + 5 * s.markets_each as usize + s.markets_extra_fish as usize, "{why}");
        assert_eq!(island.markets.iter().filter(|m| m.kind == 4).count(), (s.markets_each + s.markets_extra_fish) as usize, "{why}");
        assert_eq!(island.markets.iter().filter(|m| m.kind == ANY).count(), s.markets_any as usize, "{why}");
        let mut used: Vec<usize> = Vec::new();
        for m in &island.markets {
            let h = &hexes[m.hex as usize];
            assert!(h.terrain != Sea, "{why}");
            let across = g.hex_neighbors[m.hex as usize][m.side as usize];
            assert_eq!(hexes[across as usize].terrain, Sea, "{why}");
            let ends = g.edge_corners[g.hex_edges[m.hex as usize][m.side as usize] as usize];
            for &c in &ends {
                for &u in &used {
                    assert!(u != c as usize, "markets share a corner: {why}");
                }
            }
            used.extend(ends.iter().map(|&c| c as usize));
        }

        // The squall starts on dunes, or with none, on a weakest hex
        let sq = &hexes[island.squall as usize];
        assert!(sq.terrain != Sea, "{why}");
        if dunes > 0 {
            assert_eq!(sq.terrain, Dunes, "{why}");
        } else {
            assert_eq!(pips(sq.number), main.iter().map(|h| pips(h.number)).min().unwrap(), "{why}");
        }
    }

    #[test]
    fn default_islands_keep_every_rule() {
        let s = Settings::default();
        for i in 0..2000 {
            check(&generate(&s, &format!("seed{i}")).unwrap(), &s);
        }
    }

    #[test]
    fn every_size_setting_works() {
        // The hexagon with its default tiles, and with other mixes (Dunes counts traded for tiles)
        let shapes = [
            "island.hexagon=true".to_string(),
            "island.hexagon=true;island.forest=3;island.dunes=2".to_string(),
            "island.hexagon=true;island.paddy=5;island.dunes=0".to_string(),
            "island.hexagon=true;island.forest=3;island.fields=3;island.quarry=5;island.dunes=1".to_string(),
        ].into_iter()
            .chain([16, 19, 22, 28, 34, 40].map(|size| format!("island.hexagon=false;island.size={size}")));
        let shapes: Vec<String> = shapes
            .flat_map(|shape| (0..=3).map(move |dunes| format!("{shape};island.dunes={dunes}")))
            .filter(|shape| Settings::parse(shape).is_ok()) // a small random island with many dunes is refused (settings test)
            .collect();
        for (n, shape) in shapes.iter().enumerate() {
            let s = settings(shape);
            for k in 0..200 {
                check(&generate(&s, &format!("grid{n}-{k}")).unwrap(), &s);
            }
        }
    }

    #[test]
    fn market_settings_work() {
        // The hexagon's coast (30 edges) has room for 15 markets that share no corner
        for (shape, any, each) in [("true", 0, 0), ("true", 0, 1), ("true", 4, 1), ("true", 5, 2), ("false", 6, 1), ("false", 4, 2)] {
            let s = settings(&format!("island.hexagon={shape};markets.any={any};markets.each={each}"));
            for k in 0..50 {
                check(&generate(&s, &format!("m{k}")).unwrap(), &s);
            }
        }
    }

    #[test]
    fn the_spiral_visits_every_hexagon_cell_once_each_step_to_a_neighbour() {
        for start in 0..6 {
            let cells = spiral_cells(start);
            let mut sorted = cells.clone();
            sorted.sort();
            let mut want = within(Cell::new(0, 0), HEXAGON_RADIUS);
            want.sort();
            assert_eq!(sorted, want);
            assert!(cells.windows(2).all(|w| w[0].distance(w[1]) == 1), "start {start}: {cells:?}");
            // Counterclockwise on screen (y grows downwards): the turn from the first cell to the
            // second, seen from the middle, has a negative cross product
            let (a, b) = (cells[0].center(), cells[1].center());
            assert!((a.0 * b.1 - a.1 * b.0) < 0, "start {start}");
        }
    }

    #[test]
    fn spiral_islands_and_how_often_red_numbers_touch() {
        // The classic order isn't redrawn: report how often 6 and 8 end up side by side (shown with
        // cargo test -- --nocapture), and check the shuffled setting still never allows it
        let s = Settings::default();
        assert!(uses_spiral(&s));
        let mut touching = 0;
        for k in 0..2000 {
            let island = generate(&s, &format!("spiral{k}")).unwrap();
            let at: HashMap<Cell, usize> = island.hexes.iter().enumerate().map(|(i, h)| (h.cell, i)).collect();
            let red = island.hexes.iter().any(|h| strong(h.number) && h.cell.neighbors().iter().any(|n| at.get(n).is_some_and(|&j| strong(island.hexes[j].number))));
            touching += red as u32;
        }
        println!("spiral: 6/8 side by side on {touching} of 2000 islands");
        let off = Settings::parse("island.spiral=false").unwrap();
        assert!(!uses_spiral(&off));
        for k in 0..200 {
            check(&generate(&off, &format!("shuffled{k}")).unwrap(), &off);
        }
    }

    #[test]
    fn markets_are_spread_evenly_round_the_hexagon() {
        let s = Settings::default();
        for k in 0..300 {
            let island = generate(&s, &format!("spread{k}")).unwrap();
            let g = island.graph();
            // The coast's edges in order: walk corner to corner along coastal edges
            let coastal: Vec<usize> = (0..g.edges()).filter(|&e| {
                let hs: Vec<usize> = listed(&g.edge_hexes[e]).collect();
                hs.len() == 2 && (island.hexes[hs[0]].terrain == Sea) != (island.hexes[hs[1]].terrain == Sea)
            }).collect();
            assert_eq!(coastal.len(), 30);
            let mut order = vec![coastal[0]];
            let mut at = g.edge_corners[coastal[0]][1] as usize;
            while order.len() < coastal.len() {
                let next = *coastal.iter().find(|&&e| !order.contains(&e) && g.edge_corners[e].contains(&(at as u16))).unwrap();
                at = g.edge_corners[next].iter().map(|&c| c as usize).find(|&c| c != at).unwrap();
                order.push(next);
            }
            let market_edges: Vec<usize> = island.markets.iter().map(|m| g.hex_edges[m.hex as usize][m.side as usize] as usize).collect();
            let mut places: Vec<usize> = market_edges.iter().map(|e| order.iter().position(|x| x == e).unwrap()).collect();
            places.sort();
            let gaps: Vec<usize> = (0..9).map(|i| (places[(i + 1) % 9] + 30 - places[i]) % 30).collect();
            assert!(gaps.iter().all(|&d| d == 3 || d == 4), "seed spread{k}: gaps {gaps:?}");
        }
    }

    #[test]
    fn the_large_island_keeps_every_rule() {
        let cells = large_cells();
        assert_eq!(cells.len(), 30);
        let rows: Vec<usize> = (-3..=3).map(|r| cells.iter().filter(|c| c.r == r).count()).collect();
        assert_eq!(rows, vec![3, 4, 5, 6, 5, 4, 3]);
        // Each row lines up with the next: every cell but the ends touches 2 cells in the next row
        let set: HashSet<Cell> = cells.iter().copied().collect();
        assert!(cells.iter().all(|c| c.neighbors().iter().filter(|n| set.contains(n)).count() >= 3), "no loose cells");
        let s = Settings::for_players(6);
        assert_eq!(tokens_for(&s).len(), 28);
        assert!(!uses_spiral(&s), "the large island's spiral order isn't known: fair shuffle");
        for k in 0..500 {
            let island = generate(&s, &format!("large{k}")).unwrap();
            check(&island, &s);
            assert_eq!(island.markets.len(), 11);
        }
    }

    #[test]
    fn same_seed_same_island_and_others_differ() {
        let s = Settings::default();
        let a = generate(&s, "betta").unwrap();
        assert_eq!(a, generate(&s, "betta").unwrap());
        assert_eq!(a.to_json(), generate(&s, "betta").unwrap().to_json());
        let others: HashSet<String> = (0..50).map(|i| generate(&s, &format!("x{i}")).unwrap().to_json()).collect();
        assert_eq!(others.len(), 50);
    }

    #[test]
    fn number_tokens_by_size() {
        assert_eq!(number_tokens(18), BASE_NUMBERS.to_vec());
        assert_eq!(number_tokens(15), vec![3, 4, 4, 5, 5, 6, 6, 8, 8, 9, 9, 10, 10, 11, 11]);
        assert_eq!(number_tokens(20).len(), 20);
        assert_eq!(&number_tokens(20)[18..], &[5, 9]);
    }

    #[test]
    fn json_has_everything() {
        let island = generate(&Settings::default(), "betta").unwrap();
        let j = island.to_json();
        assert!(j.starts_with("{\"version\":2,\"seed\":\"betta\",\"hexes\":[{\"q\":"));
        assert_eq!(j.matches("\"terrain\"").count(), island.hexes.len());
        assert_eq!(j.matches("\"kind\"").count(), 9);
        assert!(j.contains("\"terrain\":\"paddy\""));
    }
}
