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
use std::f64::consts::TAU;

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
pub const MAINLAND: u8 = u8::MAX; // `islet` for hexes not on an islet

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
const BEST_PIPS: u32 = 5;

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
const ISLET_SIZES: [usize; 4] = [1, 2, 2, 3];
const FAIR_LOW: f64 = 0.6;
const FAIR_HIGH: f64 = 1.5;
const ATTEMPTS: u32 = 300;
const NUMBER_TRIES: usize = 400;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hex {
    pub cell: Cell,
    pub terrain: Terrain,
    pub number: u8, // 0: none
    pub islet: u8,  // MAINLAND, or which islet
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
    pub islets: Vec<(i32, usize)>, // (gap, size)
    pub average_gap: Option<f64>,
    pub main_average: f64,           // pips per producing hex on the main island
    pub islet_average: Option<f64>,  // pips per islet hex
    pub islet_resources: Option<f64>, // islet_average / main_average
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
            "{{\"q\":{},\"r\":{},\"terrain\":\"{}\",\"number\":{},\"islet\":{}}}",
            h.cell.q, h.cell.r, h.terrain.name(),
            if h.number == 0 { "null".into() } else { h.number.to_string() },
            if h.islet == MAINLAND { "null".into() } else { h.islet.to_string() },
        )).collect();
        let markets: Vec<String> = self.markets.iter().map(|m| format!(
            "{{\"hex\":{},\"side\":{},\"kind\":\"{}\"}}",
            m.hex, m.side, if m.kind == ANY { "any" } else { RESOURCES[m.kind as usize] },
        )).collect();
        let i = &self.info;
        let islets: Vec<String> = i.islets.iter().map(|(gap, size)| format!("{{\"gap\":{gap},\"size\":{size}}}")).collect();
        format!(
            "{{\"version\":1,\"seed\":{},\"hexes\":[{}],\"markets\":[{}],\"squall\":{},\"info\":{{\"mainSize\":{},\"islets\":[{}],\"averageGap\":{},\"mainAverage\":{},\"isletAverage\":{},\"isletResources\":{},\"attempts\":{}}}}}",
            json::string(&self.seed), hexes.join(","), markets.join(","), self.squall,
            i.main_size, islets.join(","), json::optional(i.average_gap), json::number(i.main_average),
            json::optional(i.islet_average), json::optional(i.islet_resources), i.attempts,
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
    Err(format!("no island met the rules after {ATTEMPTS} attempts (seed {seed}); try fewer markets or islets"))
}

fn attempt_island(s: &Settings, rng: &mut Rng) -> Option<Island> {
    let main = grow_main(rng, s.island_size as usize);
    let main_set: HashSet<Cell> = main.iter().copied().collect();
    if has_lakes(&main, &main_set) {
        return None;
    }
    let islets = place_islets(rng, &main, s.islet_count as usize, s.islet_distance)?;
    let mut land = dress_main(rng, &main)?;
    let main_average = land.iter().map(|h| pips(h.number)).sum::<u32>() as f64
        / land.iter().filter(|h| h.number != 0).count() as f64;
    let (islet_hexes, islet_average) = dress_islets(rng, &islets, main_average, s.islet_resources);
    land.extend(islet_hexes);

    // Sea around everything, then a fixed order (row by row)
    let land_set: HashSet<Cell> = land.iter().map(|h| h.cell).collect();
    let mut seen = HashSet::new();
    let mut sea = Vec::new();
    for h in &land {
        for c in within(h.cell, SEA_REACH) {
            if !land_set.contains(&c) && seen.insert(c) {
                sea.push(Hex { cell: c, terrain: Sea, number: 0, islet: MAINLAND });
            }
        }
    }
    let mut hexes = land;
    hexes.extend(sea);
    hexes.sort_by_key(|h| (h.cell.r, h.cell.q));
    let graph = Graph::new(&hexes.iter().map(|h| h.cell).collect::<Vec<_>>());

    let markets = place_markets(rng, &hexes, &graph, s.markets_any, s.markets_each)?;
    let dunes: Vec<u16> = (0..hexes.len())
        .filter(|&i| hexes[i].terrain == Dunes && hexes[i].islet == MAINLAND)
        .map(|i| i as u16)
        .collect();
    let squall = *rng.pick(&dunes);

    let gaps: Vec<i32> = islets.iter().map(|i| i.gap).collect();
    Some(Island {
        seed: String::new(),
        hexes,
        markets,
        squall,
        info: Info {
            main_size: main.len(),
            islets: islets.iter().map(|i| (i.gap, i.cells.len())).collect(),
            average_gap: if gaps.is_empty() { None } else { Some(gaps.iter().sum::<i32>() as f64 / gaps.len() as f64) },
            main_average,
            islet_average,
            islet_resources: islet_average.map(|a| a / main_average),
            attempts: 0,
        },
    })
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

fn nearest(c: Cell, cells: &[Cell]) -> i32 {
    cells.iter().map(|&x| c.distance(x)).min().unwrap()
}

struct Islet {
    gap: i32,
    cells: Vec<Cell>,
}

// Islets: whole-number gaps that average the setting (1.5 over 2 islets: 1 and 2), each placed at
// its own angle around the island so they're spread out, then grown to 1â€“3 hexes without coming
// nearer the main island than their gap, and with at least one sea hex between islets.
fn place_islets(rng: &mut Rng, main: &[Cell], count: usize, average: f64) -> Option<Vec<Islet>> {
    if count == 0 {
        return Some(Vec::new());
    }
    let low = average.floor() as i32;
    let high = average.ceil() as i32;
    let far = ((average - low as f64) * count as f64).round() as usize;
    let mut gaps: Vec<i32> = (0..count).map(|i| if i < far { high } else { low }).collect();
    rng.shuffle(&mut gaps);

    let o = Cell::new(0, 0);
    let reach = main.iter().map(|c| c.distance(o)).max().unwrap() + high + 4;
    let around: Vec<(Cell, i32)> = within(o, reach).into_iter().map(|c| (c, nearest(c, main))).collect();
    let to_main: HashMap<Cell, i32> = around.iter().copied().collect();
    let real = |c: Cell| {
        let (x, y) = c.center();
        (x as f64 * 3f64.sqrt() / 2.0, y as f64 * 0.5)
    };
    let (mx, my) = main.iter().fold((0.0, 0.0), |(sx, sy), &c| {
        let (x, y) = real(c);
        (sx + x / main.len() as f64, sy + y / main.len() as f64)
    });
    let angle_of = |c: Cell| {
        let (x, y) = real(c);
        (y - my).atan2(x - mx)
    };
    let turn = |a: f64, b: f64| {
        let d = (a - b).abs() % TAU;
        d.min(TAU - d)
    };
    let clear = |c: Cell, taken: &[Cell]| taken.iter().all(|&t| c.distance(t) >= 2);

    let mut placed = Vec::new();
    let mut taken: Vec<Cell> = Vec::new();
    let start = rng.next_f64() * TAU;
    for (i, &gap) in gaps.iter().enumerate() {
        let want = start + TAU * i as f64 / count as f64;
        let mut ring: Vec<Cell> = around.iter().filter(|&&(c, d)| d == gap + 1 && clear(c, &taken)).map(|&(c, _)| c).collect();
        if ring.is_empty() {
            return None;
        }
        ring.sort_by(|&a, &b| turn(angle_of(a), want).total_cmp(&turn(angle_of(b), want)));
        let first = ring[rng.below(ring.len().min(3))];
        let mut cells = vec![first];
        let size = *rng.pick(&ISLET_SIZES);
        while cells.len() < size {
            let mut grow = Vec::new();
            for c in &cells {
                for n in c.neighbors() {
                    if to_main.get(&n).is_some_and(|&d| d >= gap + 1) && clear(n, &taken) && !cells.contains(&n) && !grow.contains(&n) {
                        grow.push(n);
                    }
                }
            }
            if grow.is_empty() {
                break;
            }
            cells.push(*rng.pick(&grow));
        }
        taken.extend(&cells);
        placed.push(Islet { gap, cells });
    }
    Some(placed)
}

// Main island terrain and numbers, redrawn until the fairness checks pass
fn dress_main(rng: &mut Rng, main: &[Cell]) -> Option<Vec<Hex>> {
    let dunes = ((main.len() as f64 / 12.0).round() as usize).max(1);
    let producing = main.len() - dunes;
    let per = producing / PRODUCING.len();
    let mut kinds = PRODUCING;
    rng.shuffle(&mut kinds);
    let mut terrains: Vec<Terrain> = PRODUCING.iter().flat_map(|&t| std::iter::repeat_n(t, per)).collect();
    terrains.extend(&kinds[..producing - per * PRODUCING.len()]);
    terrains.extend(std::iter::repeat_n(Dunes, dunes));
    let mut tokens = number_tokens(producing);
    let index: HashMap<Cell, usize> = main.iter().enumerate().map(|(i, &c)| (c, i)).collect();
    let near: Vec<Vec<usize>> = main.iter().map(|c| c.neighbors().iter().filter_map(|n| index.get(n).copied()).collect()).collect();

    // Numbers go down one at a time, 6s and 8s first (they're the hardest to fit), each on a random
    // hex where it breaks no rule. Drawing them all at random and checking afterwards almost never
    // works on big islands, which need many repeats.
    let mut hexes: Vec<Hex> = main.iter().map(|&cell| Hex { cell, terrain: Dunes, number: 0, islet: MAINLAND }).collect();
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

// Islet hexes share out the pips that make their average `ratio` Ã— the main island's, as evenly as
// whole numbers allow (never above a 6 or 8). A hex left with no pips is Dunes. Equal numbers are
// kept apart inside an islet (a hex drops to the next weaker number if both its numbers are taken
// next to it); 6 next to 8 is allowed there.
fn dress_islets(rng: &mut Rng, islets: &[Islet], average: f64, ratio: f64) -> (Vec<Hex>, Option<f64>) {
    let total_hexes: usize = islets.iter().map(|i| i.cells.len()).sum();
    if total_hexes == 0 {
        return (Vec::new(), None);
    }
    let total = ((ratio * average * total_hexes as f64).round() as u32).min(BEST_PIPS * total_hexes as u32);
    let mut share: Vec<u32> = (0..total_hexes as u32)
        .map(|i| total / total_hexes as u32 + u32::from(i < total % total_hexes as u32))
        .collect();
    rng.shuffle(&mut share);

    let mut out = Vec::with_capacity(total_hexes);
    let mut k = 0;
    for (index, islet) in islets.iter().enumerate() {
        let mut kinds = PRODUCING;
        rng.shuffle(&mut kinds);
        let start = out.len();
        for (n, &cell) in islet.cells.iter().enumerate() {
            let mut p = share[k];
            k += 1;
            let mut number = 0;
            while p > 0 {
                let pair = [7 - (6 - p as u8), 7 + (6 - p as u8)]; // the two numbers with p pips: 5 -> 6, 8
                let beside = |x: u8| out[start..].iter().any(|h: &Hex| h.number == x && h.cell.distance(cell) == 1);
                if let Some(&x) = pair.iter().find(|&&x| !beside(x)) {
                    number = x;
                    break;
                }
                p -= 1;
            }
            let terrain = if number == 0 { Dunes } else { kinds[n % kinds.len()] };
            out.push(Hex { cell, terrain, number, islet: index as u8 });
        }
    }
    let avg = out.iter().map(|h| pips(h.number)).sum::<u32>() as f64 / total_hexes as f64;
    (out, Some(avg))
}

// Markets on the main island's coastal edges (main land on one side, sea on the other), with no two
// sharing or neighbouring a corner, so each market's corners can all be built on
fn place_markets(rng: &mut Rng, hexes: &[Hex], g: &Graph, any: u8, each: u8) -> Option<Vec<Market>> {
    let mut kinds: Vec<u8> = std::iter::repeat_n(ANY, any as usize)
        .chain((0..5u8).flat_map(|r| std::iter::repeat_n(r, each as usize)))
        .collect();
    rng.shuffle(&mut kinds);
    let mut coast: Vec<(u16, u8, u16)> = Vec::new();
    for (h, hex) in hexes.iter().enumerate() {
        if hex.terrain == Sea || hex.islet != MAINLAND {
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
        let main: Vec<&Hex> = hexes.iter().filter(|h| h.terrain != Sea && h.islet == MAINLAND).collect();
        let why = format!("seed {} ({})", island.seed, s.to_text());

        // The main island: its size, all one piece, no lakes
        assert_eq!(main.len(), s.island_size as usize, "{why}");
        let mut reached = HashSet::from([main[0].cell]);
        let mut todo = vec![main[0].cell];
        while let Some(c) = todo.pop() {
            for n in c.neighbors() {
                if at.get(&n).is_some_and(|&j| hexes[j].terrain != Sea && hexes[j].islet == MAINLAND) && reached.insert(n) {
                    todo.push(n);
                }
            }
        }
        assert_eq!(reached.len(), main.len(), "main island in one piece: {why}");
        let main_set: HashSet<Cell> = main.iter().map(|h| h.cell).collect();
        assert!(!has_lakes(&main.iter().map(|h| h.cell).collect::<Vec<_>>(), &main_set), "no lakes: {why}");

        // Terrain: dunes about 1 in 12, at least 1; the five others as even as possible, each 3+
        let dunes = main.iter().filter(|h| h.terrain == Dunes).count();
        assert_eq!(dunes, ((main.len() as f64 / 12.0).round() as usize).max(1), "{why}");
        let counts: Vec<usize> = PRODUCING.iter().map(|&t| main.iter().filter(|h| h.terrain == t).count()).collect();
        let (lo, hi) = (*counts.iter().min().unwrap(), *counts.iter().max().unwrap());
        assert!(lo >= 3 && hi - lo <= 1, "terrain {counts:?}: {why}");

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

        // Neighbours: no equal numbers anywhere; no 6/8 pairs on the main island
        for h in hexes.iter().filter(|h| h.number != 0) {
            for n in h.cell.neighbors() {
                let Some(&j) = at.get(&n) else { continue };
                let o = &hexes[j];
                if o.number == 0 { continue; }
                assert_ne!(o.number, h.number, "equal numbers side by side: {why}");
                if h.islet == MAINLAND && o.islet == MAINLAND {
                    assert!(!(strong(o.number) && strong(h.number)), "6/8 side by side: {why}");
                }
            }
        }

        // Fair shares of each resource on the main island
        let main_hexes: Vec<Hex> = main.iter().map(|&&h| h).collect();
        let totals = resource_pips(&main_hexes);
        let mean = totals.iter().sum::<u32>() as f64 / 5.0;
        for t in totals {
            assert!(t as f64 >= FAIR_LOW * mean && t as f64 <= FAIR_HIGH * mean, "resources {totals:?}: {why}");
        }

        // Islets: how many, sizes, gaps (measured afresh), their average, apart from each other
        let count = s.islet_count as usize;
        assert_eq!(island.info.islets.len(), count, "{why}");
        let mut gaps = Vec::new();
        for i in 0..count {
            let cells: Vec<Cell> = hexes.iter().filter(|h| h.islet == i as u8).map(|h| h.cell).collect();
            assert!((1..=3).contains(&cells.len()), "islet size: {why}");
            let gap = cells.iter().map(|&c| nearest(c, &main.iter().map(|h| h.cell).collect::<Vec<_>>())).min().unwrap() - 1;
            assert_eq!(gap, island.info.islets[i].0, "{why}");
            assert!(gap >= 1, "{why}");
            gaps.push(gap);
            for j in 0..count {
                if j == i { continue; }
                for h in hexes.iter().filter(|h| h.islet == j as u8) {
                    assert!(cells.iter().all(|&c| c.distance(h.cell) >= 2), "islets touching: {why}");
                }
            }
        }
        if count > 0 {
            let avg = gaps.iter().sum::<i32>() as f64 / count as f64;
            assert!((avg - s.islet_distance).abs() <= 0.5 / count as f64 + 1e-9, "average gap {avg}: {why}");
            // Productivity: as near the target as whole numbers allow, never above a 6/8's worth
            let want = s.islet_resources.min(BEST_PIPS as f64 / island.info.main_average);
            // Off by at most half a pip from rounding, plus one pip per islet: in an islet of 3 hexes
            // that all touch, only two can have the strongest numbers (6 and 8), so the third drops
            let got = island.info.islet_resources.unwrap();
            let islet_hexes: usize = island.info.islets.iter().map(|i| i.1).sum();
            let tolerance = (0.5 + count as f64) / (island.info.main_average * islet_hexes as f64) + 1e-9;
            assert!(got <= want + tolerance && got >= want - tolerance, "islet resources {got} for {want}: {why}");
        }

        // Sea everywhere within 2 of land
        for h in hexes.iter().filter(|h| h.terrain != Sea) {
            for c in within(h.cell, 2) {
                assert!(at.contains_key(&c), "missing sea: {why}");
            }
        }

        // Markets: how many of each, on the main coast, corners apart
        assert_eq!(island.markets.len(), s.markets_any as usize + 5 * s.markets_each as usize, "{why}");
        assert_eq!(island.markets.iter().filter(|m| m.kind == ANY).count(), s.markets_any as usize, "{why}");
        let mut used: Vec<usize> = Vec::new();
        for m in &island.markets {
            let h = &hexes[m.hex as usize];
            assert!(h.terrain != Sea && h.islet == MAINLAND, "{why}");
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

        // The squall starts on main-island dunes
        let sq = &hexes[island.squall as usize];
        assert!(sq.terrain == Dunes && sq.islet == MAINLAND, "{why}");
    }

    #[test]
    fn default_islands_keep_every_rule() {
        let s = Settings::default();
        for i in 0..2000 {
            check(&generate(&s, &format!("seed{i}")).unwrap(), &s);
        }
    }

    #[test]
    fn every_size_and_islet_setting_works() {
        let mut n = 0;
        for size in [16, 19, 22, 28, 34, 40] {
            for count in 0..=6 {
                for distance in [1.0, 1.3, 1.5, 2.0, 2.7, 3.0] {
                    for resources in [0.0, 0.5, 1.0, 1.4] {
                        let s = settings(&format!("island.size={size};islets.count={count};islets.distance={distance};islets.resources={resources}"));
                        for k in 0..3 {
                            check(&generate(&s, &format!("grid{n}-{k}")).unwrap(), &s);
                        }
                        n += 1;
                    }
                }
            }
        }
    }

    #[test]
    fn market_settings_work() {
        for (any, each) in [(0, 0), (0, 1), (2, 1), (4, 1), (2, 2)] {
            let s = settings(&format!("markets.any={any};markets.each={each}"));
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
        assert!(j.starts_with("{\"version\":1,\"seed\":\"betta\",\"hexes\":[{\"q\":"));
        assert_eq!(j.matches("\"terrain\"").count(), island.hexes.len());
        assert_eq!(j.matches("\"kind\"").count(), 7);
        assert!(j.contains("\"terrain\":\"paddy\""));
    }
}
