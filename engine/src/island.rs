// Makes a new island from settings and a seed (the same pair always makes the same island). RULES.md
// "The island" says what it guarantees; the tests at the bottom check every guarantee on thousands of
// islands. When a random attempt breaks a rule, the generator simply tries again.
//
// This runs once per game, so it favours being clear over being fast, but it never iterates over a
// HashMap/HashSet: their order changes from run to run, and the same seed must give the same island.
use crate::hex::{Cell, Graph, NONE, listed, within};
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
            Sea | Dunes => None,
        }
    }
    pub fn name(self) -> &'static str {
        ["sea", "forest", "quarry", "fields", "mine", "paddy", "dunes"][self as usize]
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

const SEA_REACH: i32 = 2; // sea is every hex within this many steps of land
const FAIR_LOW: f64 = 0.6;
const FAIR_HIGH: f64 = 1.5;
const ATTEMPTS: u32 = 300;
const NUMBER_TRIES: usize = 400;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hex {
    pub cell: Cell,
    pub terrain: Terrain,
    pub number: u8, // 0: none
}

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
        let hexes: Vec<String> = self.hexes.iter().map(|h| format!(
            "{{\"q\":{},\"r\":{},\"terrain\":\"{}\",\"number\":{}}}",
            h.cell.q, h.cell.r, h.terrain.name(),
            if h.number == 0 { "null".into() } else { h.number.to_string() },
        )).collect();
        let markets: Vec<String> = self.markets.iter().map(|m| format!(
            "{{\"hex\":{},\"side\":{},\"kind\":\"{}\"}}",
            m.hex, m.side, if m.kind == ANY { "any" } else { RESOURCES[m.kind as usize] },
        )).collect();
        let i = &self.info;
        // Version 2: no islets (Nov 15)
        format!(
            "{{\"version\":2,\"seed\":{},\"hexes\":[{}],\"markets\":[{}],\"squall\":{},\"info\":{{\"mainSize\":{},\"mainAverage\":{},\"attempts\":{}}}}}",
            json::string(&self.seed), hexes.join(","), markets.join(","), self.squall,
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
    if s.island_hexagon { within(Cell::new(0, 0), HEXAGON_RADIUS).len() } else { s.island_size as usize }
}

fn attempt_island(s: &Settings, rng: &mut Rng) -> Option<Island> {
    let main = if s.island_hexagon { within(Cell::new(0, 0), HEXAGON_RADIUS) } else { grow_main(rng, s.island_size as usize) };
    let main_set: HashSet<Cell> = main.iter().copied().collect();
    if has_lakes(&main, &main_set) {
        return None;
    }
    let counts = if s.island_hexagon { Some(s.tile_counts()) } else { None };
    let land = dress_main(rng, &main, s.island_dunes as usize, counts)?;
    let main_average = land.iter().map(|h| pips(h.number)).sum::<u32>() as f64
        / land.iter().filter(|h| h.number != 0).count() as f64;

    // Sea around the island, then a fixed order (row by row)
    let land_set: HashSet<Cell> = land.iter().map(|h| h.cell).collect();
    let mut seen = HashSet::new();
    let mut sea = Vec::new();
    for h in &land {
        for c in within(h.cell, SEA_REACH) {
            if !land_set.contains(&c) && seen.insert(c) {
                sea.push(Hex { cell: c, terrain: Sea, number: 0 });
            }
        }
    }
    let mut hexes = land;
    hexes.extend(sea);
    hexes.sort_by_key(|h| (h.cell.r, h.cell.q));
    let graph = Graph::new(&hexes.iter().map(|h| h.cell).collect::<Vec<_>>());

    let markets = place_markets(rng, &hexes, &graph, s.markets_any, s.markets_each)?;
    let squall = *rng.pick(&squall_starts(&hexes));

    Some(Island {
        seed: String::new(),
        hexes,
        markets,
        squall,
        info: Info { main_size: main.len(), main_average, attempts: 0 },
    })
}

// Where the squall may start: the island's Dunes, or with none, its weakest hexes (fewest pips)
fn squall_starts(hexes: &[Hex]) -> Vec<u16> {
    let main = |i: &usize| hexes[*i].terrain != Sea;
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
fn dress_main(rng: &mut Rng, main: &[Cell], dunes: usize, counts: Option<[u8; 5]>) -> Option<Vec<Hex>> {
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
    let mut tokens = number_tokens(producing);
    let index: HashMap<Cell, usize> = main.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let near: Vec<Vec<usize>> = main.iter().map(|c| c.neighbors().iter().filter_map(|n| index.get(n).copied()).collect()).collect();

    // Numbers go down one at a time, 6s and 8s first (they're the hardest to fit), each on a random
    // hex where it breaks no rule. Drawing them all at random and checking afterwards almost never
    // works on big islands, which need many repeats.
    let mut hexes: Vec<Hex> = main.iter().map(|&cell| Hex { cell, terrain: Dunes, number: 0 }).collect();
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

// Markets on the island's coastal edges (land on one side, sea on the other), with no two sharing
// or neighbouring a corner, so each market's corners can all be built on
fn place_markets(rng: &mut Rng, hexes: &[Hex], g: &Graph, any: u8, each: u8) -> Option<Vec<Market>> {
    let mut kinds: Vec<u8> = std::iter::repeat_n(ANY, any as usize)
        .chain((0..5u8).flat_map(|r| std::iter::repeat_n(r, each as usize)))
        .collect();
    rng.shuffle(&mut kinds);
    let mut coast: Vec<(u16, u8, u16)> = Vec::new();
    for (h, hex) in hexes.iter().enumerate() {
        if hex.terrain == Sea {
            continue;
        }
        for side in 0..6 {
            let n = g.hex_neighbors[h][side];
            if n != NONE && hexes[n as usize].terrain == Sea {
                coast.push((h as u16, side as u8, g.hex_edges[h][side]));
            }
        }
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
                for n in listed(&g.corner_corners[c as usize]) {
                    blocked[n] = true;
                }
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

#[cfg(test)]
mod tests {
    use super::*;

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
        if s.island_hexagon {
            let mut cells: Vec<Cell> = main.iter().map(|h| h.cell).collect();
            cells.sort();
            let mut want = within(Cell::new(0, 0), HEXAGON_RADIUS);
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
        if s.island_hexagon {
            assert_eq!(counts, s.tile_counts().map(|n| n as usize).to_vec(), "hexagon tiles: {why}");
        } else {
            let (lo, hi) = (*counts.iter().min().unwrap(), *counts.iter().max().unwrap());
            assert!(lo >= 3 && hi - lo <= 1, "terrain {counts:?}: {why}");
        }

        // Numbers: exactly the tokens, never 7, none on dunes or sea
        let mut numbers: Vec<u8> = main.iter().filter(|h| h.number != 0).map(|h| h.number).collect();
        numbers.sort();
        let mut tokens = number_tokens(main.len() - dunes);
        tokens.sort();
        assert_eq!(numbers, tokens, "{why}");
        for h in hexes {
            assert_eq!(h.number == 0, matches!(h.terrain, Sea | Dunes), "{why}");
            assert!(h.number != 7 && h.number <= 12 && (h.number == 0 || h.number >= 2), "{why}");
        }

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

        // Sea everywhere within 2 of land
        for h in hexes.iter().filter(|h| h.terrain != Sea) {
            for c in within(h.cell, 2) {
                assert!(at.contains_key(&c), "missing sea: {why}");
            }
        }

        // Markets: how many of each, on the coast, corners apart
        assert_eq!(island.markets.len(), s.markets_any as usize + 5 * s.markets_each as usize, "{why}");
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
                    assert!(u != c as usize && !listed(&g.corner_corners[u]).any(|n| n == c as usize), "markets too close: {why}");
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
        // The hexagon's coast has room for 7 spaced markets; more need a random shape
        for (shape, any, each) in [("true", 0, 0), ("true", 0, 1), ("true", 2, 1), ("false", 4, 1), ("false", 2, 2)] {
            let s = settings(&format!("island.hexagon={shape};markets.any={any};markets.each={each}"));
            for k in 0..50 {
                check(&generate(&s, &format!("m{k}")).unwrap(), &s);
            }
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
        assert_eq!(j.matches("\"kind\"").count(), 7);
        assert!(j.contains("\"terrain\":\"paddy\""));
    }
}
