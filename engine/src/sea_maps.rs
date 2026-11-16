// The sea expansion's scenario maps (EXPANSIONS.md, B), each made from a seed to the scenario's rules
// and tile counts. The rulebook's maps are pictures and part of the original game's identity, so the
// layouts here are our own: islands grown at random with at least one sea hex between any two.
// (Scenario 1, New Shores, is made in island.rs from the usual main island plus small islands.)
//
//   2 Four Islands      4 islands of 5 hexes (6 with 5-6 players), set up on any; 13 to win
//   4 Through the Desert the main island, a strip beyond 2 more Dunes, small islands with gold; 14
//   9 New World         3-5 random islands from the rulebook's tiles; 12
use crate::hex::{Cell, within};
use crate::island::{Hex, NUMBER_TRIES, Terrain, number_tokens, strong};
use crate::rng::Rng;
use crate::settings::Settings;
use std::collections::HashMap;
use Terrain::*;

// Cells already taken by land (any island): a new island keeps at least one sea hex away from them
fn clear_of(c: Cell, taken: &[Cell], own: &[Cell]) -> bool {
    taken.iter().all(|t| own.contains(t) || t.distance(c) >= 2)
}

// Grows an island of `size` cells from a random start among `starts`, keeping clear of `taken` and
// within `limit` steps of the middle
fn grow_island(rng: &mut Rng, size: usize, starts: &[Cell], taken: &[Cell], limit: i32) -> Option<Vec<Cell>> {
    for _ in 0..80 {
        let start = *rng.pick(starts);
        if !clear_of(start, taken, &[]) {
            continue;
        }
        let mut cells = vec![start];
        while cells.len() < size {
            let options: Vec<Cell> = cells
                .iter()
                .flat_map(|c| c.neighbors())
                .filter(|&n| !cells.contains(&n) && n.distance(Cell::new(0, 0)) <= limit && clear_of(n, taken, &cells))
                .collect();
            if options.is_empty() {
                break;
            }
            let next = *rng.pick(&options);
            cells.push(next);
        }
        if cells.len() == size {
            return Some(cells);
        }
    }
    None
}

// Several islands: the first around the middle, the others where they fit (a ring of candidate starts)
fn islands(rng: &mut Rng, sizes: &[usize], limit: i32) -> Option<Vec<Vec<Cell>>> {
    let mut taken: Vec<Cell> = Vec::new();
    let mut out = Vec::new();
    for (k, &size) in sizes.iter().enumerate() {
        let starts: Vec<Cell> = if k == 0 { vec![Cell::new(0, 0)] } else { within(Cell::new(0, 0), limit) };
        let cells = grow_island(rng, size, &starts, &taken, limit)?;
        taken.extend(&cells);
        out.push(cells);
    }
    Some(out)
}

// Tiles and numbers on cells (each with its region): tiles shuffled over the cells, numbers over the
// producing ones, redrawn until no 6 or 8 touches another and no two equal numbers touch
fn dress(rng: &mut Rng, cells: &[(Cell, u8)], tiles: &mut [Terrain], numbers: &mut [u8]) -> Option<Vec<Hex>> {
    let index: HashMap<Cell, usize> = cells.iter().enumerate().map(|(i, &(c, _))| (c, i)).collect();
    for _ in 0..NUMBER_TRIES {
        rng.shuffle(tiles);
        rng.shuffle(numbers);
        let mut hexes: Vec<Hex> = cells.iter().zip(tiles.iter()).map(|(&(cell, region), &terrain)| Hex { cell, terrain, number: 0, region }).collect();
        let mut next = numbers.iter();
        for h in hexes.iter_mut() {
            if h.terrain != Dunes {
                h.number = *next.next()?;
            }
        }
        let ok = hexes.iter().all(|h| {
            h.number == 0
                || h.cell.neighbors().iter().all(|n| index.get(n).is_none_or(|&j| {
                    let o = hexes[j].number;
                    o == 0 || (o != h.number && !(strong(o) && strong(h.number)))
                }))
        });
        if ok {
            return Some(hexes);
        }
    }
    None
}

// The land for scenarios 2, 4 and 9
pub fn scenario_land(rng: &mut Rng, s: &Settings) -> Option<Vec<Hex>> {
    let big = s.island_large;
    match s.sea_scenario {
        2 => {
            // Four Islands (the Six Islands with 5-6 players): 5 hexes each, every resource equally
            let count = if big { 6 } else { 4 };
            let shapes = islands(rng, &vec![5; count], if big { 7 } else { 6 })?;
            let cells: Vec<(Cell, u8)> = shapes.iter().enumerate().flat_map(|(k, cs)| cs.iter().map(move |&c| (c, k as u8))).collect();
            let mut tiles: Vec<Terrain> = [Forest, Quarry, Fields, Mine, Paddy].iter().flat_map(|&t| std::iter::repeat_n(t, count)).collect();
            let mut numbers = number_tokens(cells.len());
            dress(rng, &cells, &mut tiles, &mut numbers)
        }
        4 => {
            // Through the Desert: the main island (region 0) with 2 more Dunes along one side and a strip
            // of 4 hexes beyond them (region 1); small islands (regions 2-3) with the gold
            let main: Vec<Cell> = if big { crate::island::large_cells() } else { within(Cell::new(0, 0), 2) };
            let reach = main.iter().map(|c| c.distance(Cell::new(0, 0))).max().unwrap();
            // The desert: 2 cells just outside the main island on its east side; the strip beyond
            let outside = |c: Cell| !main.contains(&c);
            let east = |c: Cell| 2 * c.q + c.r;
            let mut ring: Vec<Cell> = within(Cell::new(0, 0), reach + 1).into_iter().filter(|&c| outside(c) && c.neighbors().iter().any(|n| main.contains(n))).collect();
            ring.sort_by_key(|&c| (-east(c), c.r));
            let desert: Vec<Cell> = ring[..2].to_vec();
            let mut beyond: Vec<Cell> = within(Cell::new(0, 0), reach + 3)
                .into_iter()
                .filter(|&c| outside(c) && !desert.contains(&c) && desert.iter().any(|d| d.distance(c) == 1) && main.iter().all(|m| m.distance(c) >= 2))
                .collect();
            beyond.sort_by_key(|&c| (-east(c), c.r));
            let mut strip: Vec<Cell> = beyond.into_iter().take(4).collect();
            while strip.len() < 4 {
                let grow: Vec<Cell> = strip.iter().flat_map(|c| c.neighbors()).filter(|&n| outside(n) && !desert.contains(&n) && !strip.contains(&n) && main.iter().all(|m| m.distance(n) >= 2)).collect();
                strip.push(*grow.first()?);
            }
            let mut taken: Vec<Cell> = main.iter().chain(&desert).chain(&strip).copied().collect();
            let limit = reach + 5;
            let ring2: Vec<Cell> = within(Cell::new(0, 0), limit).into_iter().filter(|&c| taken.iter().map(|t| t.distance(c)).min().unwrap() == 3).collect();
            let mut small = Vec::new();
            for size in [2usize, 2] {
                let cells = grow_island(rng, size, &ring2, &taken, limit)?;
                taken.extend(&cells);
                small.push(cells);
            }
            // Tiles: the main island's own (with its Dunes); the strip and the small islands their own
            // 8 (the gold only on the small islands), numbered from the 3-11 range
            let main_cells: Vec<(Cell, u8)> = main.iter().map(|&c| (c, 0)).collect();
            let mut main_tiles: Vec<Terrain> = [Forest, Quarry, Fields, Mine, Paddy].iter().zip(s.tile_counts()).flat_map(|(&t, n)| std::iter::repeat_n(t, n as usize)).collect();
            main_tiles.extend(std::iter::repeat_n(Dunes, s.island_dunes as usize));
            let mut main_numbers = crate::island::tokens_for(s);
            let mut hexes = dress(rng, &main_cells, &mut main_tiles, &mut main_numbers)?;
            let strip_cells: Vec<(Cell, u8)> = strip.iter().map(|&c| (c, 1)).collect();
            hexes.extend(dress(rng, &strip_cells, &mut [Forest, Paddy, Fields, Mine], &mut [3, 5, 9, 11])?);
            let island_cells: Vec<(Cell, u8)> = small.iter().enumerate().flat_map(|(k, cs)| cs.iter().map(move |&c| (c, 2 + k as u8))).collect();
            hexes.extend(dress(rng, &island_cells, &mut [Gold, Gold, Quarry, Paddy], &mut [4, 6, 8, 10])?);
            hexes.extend(desert.iter().map(|&c| Hex { cell: c, terrain: Dunes, number: 0, region: 0 }));
            Some(hexes)
        }
        9 => {
            // New World: the rulebook's tiles (4 Mine, 5 Forest, 5 Paddy, 5 Fields, 4 Quarry; more with
            // 5-6 players) on 3-5 random islands
            let mut tiles: Vec<Terrain> = Vec::new();
            for (t, n) in [(Mine, 4), (Forest, 5), (Paddy, 5), (Fields, 5), (Quarry, 4)] {
                tiles.extend(std::iter::repeat_n(t, n + if big { 2 } else { 0 }));
            }
            let total = tiles.len();
            let count = 3 + rng.below(3);
            let mut sizes = vec![total / count; count];
            for size in sizes.iter_mut().take(total % count) {
                *size += 1;
            }
            let shapes = islands(rng, &sizes, if big { 8 } else { 7 })?;
            let cells: Vec<(Cell, u8)> = shapes.iter().enumerate().flat_map(|(k, cs)| cs.iter().map(move |&c| (c, k as u8))).collect();
            let mut numbers = number_tokens(total);
            dress(rng, &cells, &mut tiles, &mut numbers)
        }
        _ => None,
    }
}

// Does this scenario let players set up anywhere (their home islands), or only on the main island?
pub fn setup_anywhere(scenario: u8) -> bool {
    matches!(scenario, 2 | 9)
}
