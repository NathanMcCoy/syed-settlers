// Hex geometry. Hexes are "pointy-top" (a corner at the top) and use axial coordinates (q, r): r is
// the row, growing downwards, and q the position along it, growing to the east. Each row is drawn
// half a hex further right than the one above, so (q, r+1) sits just below-right of (q, r).
//
// Pieces sit on hex CORNERS (Huts, Lodges) and EDGES (Paths), which neighbouring hexes share.
// To give each shared corner one identity, everything is placed on a whole-number lattice: a hex's
// centre is (2q + r, 3r), and its corners are at fixed offsets from that, so two hexes' corners at
// the same lattice point are the same corner. To draw, multiply x by √3/2 and y by 1/2 (times the
// hex size, centre to corner).
//
// Graph is the board for play: flat arrays indexed by hex, corner and edge number, so a rule check is
// a few array reads. Lists that can be shorter (a corner on the edge of the board touches fewer than
// 3 hexes) are padded with NONE.
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Cell {
    pub q: i32,
    pub r: i32,
}

// Neighbour directions, in this order: E, NE, NW, W, SW, SE
pub const DIRS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

// Corner offsets from the centre, clockwise from the top: N, NE, SE, S, SW, NW. Side s of a hex is
// the edge from corner s to corner s+1, so sides are NE, E, SE, SW, W, NW.
pub const CORNERS: [(i32, i32); 6] = [(0, -2), (1, -1), (1, 1), (0, 2), (-1, 1), (-1, -1)];

// Which neighbour direction (index into DIRS) lies across each side
pub const SIDE_DIR: [usize; 6] = [1, 0, 5, 4, 3, 2];

pub const NONE: u16 = u16::MAX;

impl Cell {
    pub const fn new(q: i32, r: i32) -> Cell {
        Cell { q, r }
    }

    pub fn neighbors(self) -> [Cell; 6] {
        DIRS.map(|(dq, dr)| Cell::new(self.q + dq, self.r + dr))
    }

    // Steps between two hexes
    pub fn distance(self, other: Cell) -> i32 {
        let dq = self.q - other.q;
        let dr = self.r - other.r;
        (dq.abs() + dr.abs() + (dq + dr).abs()) / 2
    }

    // The centre on the lattice (see the top of this file)
    pub fn center(self) -> (i32, i32) {
        (2 * self.q + self.r, 3 * self.r)
    }
}

// Every hex within `radius` steps of `c`, including it, in a fixed order
pub fn within(c: Cell, radius: i32) -> Vec<Cell> {
    let mut out = Vec::new();
    for dq in -radius..=radius {
        for dr in (-radius).max(-dq - radius)..=radius.min(-dq + radius) {
            out.push(Cell::new(c.q + dq, c.r + dr));
        }
    }
    out
}

pub struct Graph {
    pub cells: Vec<Cell>,
    pub hex_corners: Vec<[u16; 6]>,   // N first, clockwise
    pub hex_edges: Vec<[u16; 6]>,     // side order
    pub hex_neighbors: Vec<[u16; 6]>, // side order; NONE off the board
    pub corner_xy: Vec<(i32, i32)>,   // lattice position
    pub corner_hexes: Vec<[u16; 3]>,
    pub corner_corners: Vec<[u16; 3]>, // neighbouring corners, each joined by the edge at the same place in corner_edges
    pub corner_edges: Vec<[u16; 3]>,
    pub edge_corners: Vec<[u16; 2]>,
    pub edge_hexes: Vec<[u16; 2]>,
    index: HashMap<Cell, u16>,
}

fn push<const N: usize>(list: &mut [u16; N], v: u16) {
    let slot = list.iter_mut().find(|s| **s == NONE).expect("list is full");
    *slot = v;
}

// The real entries of a NONE-padded list
#[inline]
pub fn listed(list: &[u16]) -> impl Iterator<Item = usize> + '_ {
    list.iter().take_while(|&&v| v != NONE).map(|&v| v as usize)
}

impl Graph {
    // The board for these cells, in this order (their order gives the hex numbers; corner and edge
    // numbers follow from it, so a saved game can refer to them by number)
    pub fn new(cells: &[Cell]) -> Graph {
        assert!(cells.len() < NONE as usize / 3, "board too big");
        let index: HashMap<Cell, u16> = cells.iter().enumerate().map(|(i, &c)| (c, i as u16)).collect();
        let mut g = Graph {
            cells: cells.to_vec(),
            hex_corners: Vec::with_capacity(cells.len()),
            hex_edges: Vec::with_capacity(cells.len()),
            hex_neighbors: Vec::with_capacity(cells.len()),
            corner_xy: Vec::new(),
            corner_hexes: Vec::new(),
            corner_corners: Vec::new(),
            corner_edges: Vec::new(),
            edge_corners: Vec::new(),
            edge_hexes: Vec::new(),
            index,
        };
        let mut corner_ids: HashMap<(i32, i32), u16> = HashMap::new();
        for (i, &c) in cells.iter().enumerate() {
            let (x, y) = c.center();
            let mut ids = [0u16; 6];
            for (k, &(dx, dy)) in CORNERS.iter().enumerate() {
                let at = (x + dx, y + dy);
                let id = *corner_ids.entry(at).or_insert_with(|| {
                    g.corner_xy.push(at);
                    g.corner_hexes.push([NONE; 3]);
                    g.corner_corners.push([NONE; 3]);
                    g.corner_edges.push([NONE; 3]);
                    (g.corner_xy.len() - 1) as u16
                });
                push(&mut g.corner_hexes[id as usize], i as u16);
                ids[k] = id;
            }
            g.hex_corners.push(ids);
        }
        let mut edge_ids: HashMap<(u16, u16), u16> = HashMap::new();
        for i in 0..cells.len() {
            let mut edges = [0u16; 6];
            let mut neighbors = [NONE; 6];
            for s in 0..6 {
                let a = g.hex_corners[i][s];
                let b = g.hex_corners[i][(s + 1) % 6];
                let key = (a.min(b), a.max(b));
                let id = match edge_ids.get(&key) {
                    Some(&id) => id,
                    None => {
                        let id = g.edge_corners.len() as u16;
                        edge_ids.insert(key, id);
                        g.edge_corners.push([key.0, key.1]);
                        g.edge_hexes.push([NONE; 2]);
                        push(&mut g.corner_edges[a as usize], id);
                        push(&mut g.corner_edges[b as usize], id);
                        push(&mut g.corner_corners[a as usize], b);
                        push(&mut g.corner_corners[b as usize], a);
                        id
                    }
                };
                push(&mut g.edge_hexes[id as usize], i as u16);
                edges[s] = id;
                let (dq, dr) = DIRS[SIDE_DIR[s]];
                neighbors[s] = g.hex_at(Cell::new(cells[i].q + dq, cells[i].r + dr)).unwrap_or(NONE);
            }
            g.hex_edges.push(edges);
            g.hex_neighbors.push(neighbors);
        }
        g
    }

    pub fn hex_at(&self, c: Cell) -> Option<u16> {
        self.index.get(&c).copied()
    }

    pub fn hexes(&self) -> usize { self.cells.len() }
    pub fn corners(&self) -> usize { self.corner_xy.len() }
    pub fn edges(&self) -> usize { self.edge_corners.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_hex() {
        let g = Graph::new(&[Cell::new(0, 0)]);
        assert_eq!((g.corners(), g.edges()), (6, 6));
        assert_eq!(g.hex_neighbors[0], [NONE; 6]);
    }

    #[test]
    fn a_flower_of_seven_shares_corners_and_edges() {
        let g = Graph::new(&within(Cell::new(0, 0), 1));
        assert_eq!((g.hexes(), g.corners(), g.edges()), (7, 24, 30));
        let mid = g.hex_at(Cell::new(0, 0)).unwrap() as usize;
        for &c in &g.hex_corners[mid] {
            assert_eq!(listed(&g.corner_hexes[c as usize]).count(), 3);
        }
        for &e in &g.hex_edges[mid] {
            assert_eq!(listed(&g.edge_hexes[e as usize]).count(), 2);
        }
    }

    #[test]
    fn corners_are_joined_by_their_edges() {
        let g = Graph::new(&within(Cell::new(0, 0), 3));
        for c in 0..g.corners() {
            let n = listed(&g.corner_corners[c]).count();
            assert!((2..=3).contains(&n));
            assert_eq!(n, listed(&g.corner_edges[c]).count());
            for (k, other) in listed(&g.corner_corners[c]).enumerate() {
                let e = g.corner_edges[c][k] as usize;
                let mut ends = g.edge_corners[e].map(|x| x as usize);
                ends.sort();
                assert_eq!(ends, { let mut v = [c, other]; v.sort(); v });
            }
        }
    }

    #[test]
    fn each_side_is_shared_with_the_hex_across_it() {
        let g = Graph::new(&within(Cell::new(0, 0), 2));
        let h = g.hex_at(Cell::new(0, 0)).unwrap() as usize;
        for s in 0..6 {
            let (dq, dr) = DIRS[SIDE_DIR[s]];
            let n = g.hex_at(Cell::new(dq, dr)).unwrap();
            assert_eq!(g.hex_neighbors[h][s], n);
            assert!(g.hex_edges[n as usize].contains(&g.hex_edges[h][s]), "side {s}");
        }
    }

    #[test]
    fn distances() {
        assert_eq!(Cell::new(0, 0).distance(Cell::new(2, -1)), 2);
        assert_eq!(Cell::new(-3, 3).distance(Cell::new(3, -3)), 6);
        assert_eq!(within(Cell::new(0, 0), 2).len(), 19);
        for n in Cell::new(4, -2).neighbors() {
            assert_eq!(n.distance(Cell::new(4, -2)), 1);
        }
    }
}
