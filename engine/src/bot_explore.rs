// Greedy's rules for the exploration expansion (explore.rs): harbours, then ships and their cargo; in
// the movement phase each ship first does what it can where it is, then sails toward its purpose (a
// spot to settle, a lair or farm for crews, the Council for cargo, otherwise the nearest hidden hex).
use crate::bot::Greedy;
use crate::game::*;
use crate::hex::listed;
use crate::island::{Terrain, pips};

// How good corner c is to settle: its dice
fn settle_value(g: &Game, c: usize) -> f64 {
    listed(&g.board.graph.corner_hexes[c]).filter(|&h| g.board.resource[h] < 5).map(|h| pips(g.board.number[h]) as f64).sum::<f64>()
}

// Can a ship reach hex h: a sea edge with an end on one of its corners?
fn by_sea(g: &Game, h: usize) -> bool {
    let gr = &g.board.graph;
    gr.hex_corners[h].iter().any(|&c| listed(&gr.corner_edges[c as usize]).any(|e| listed(&gr.edge_hexes[e]).any(|x| matches!(g.board.terrain[x], Terrain::Sea | Terrain::Shoal))))
}

// A lair or farm p's crews could still work, that a ship can reach
fn crew_target(g: &Game, p: u8, h: usize) -> bool {
    let open = match g.board.terrain[h] {
        Terrain::Gold => !g.ex.lair_taken[h],
        Terrain::Spice => !g.ex.farm_crews[h].contains(&p),
        _ => false,
    };
    open && by_sea(g, h)
}

// Sailing distance (edges) from every edge to the nearest edge satisfying `goal`, over sailable edges
fn sea_dist(g: &Game, goal: &dyn Fn(usize) -> bool) -> Vec<u32> {
    let gr = &g.board.graph;
    let n = gr.edges();
    let mut dist = vec![u32::MAX; n];
    let mut queue = std::collections::VecDeque::new();
    for e in 0..n {
        if g.sailable(e) && goal(e) {
            dist[e] = 0;
            queue.push_back(e);
        }
    }
    while let Some(x) = queue.pop_front() {
        for &c in &gr.edge_corners[x] {
            for y in listed(&gr.corner_edges[c as usize]) {
                if dist[y] == u32::MAX && g.sailable(y) {
                    dist[y] = dist[x] + 1;
                    queue.push_back(y);
                }
            }
        }
    }
    dist
}

impl Greedy {
    pub(crate) fn explore_choose(&mut self, g: &Game, p: u8, legal: &[Action]) -> Option<Action> {
        let pu = p as usize;
        match g.phase {
            Phase::SetupHut => self.best_of(legal, |a| match a {
                Action::Hut(c) => Some(settle_value(g, *c as usize)),
                _ => None,
            }),
            Phase::SetupPath { .. } => legal.first().copied(),
            Phase::Decide => match g.deciding().map(|d| d.decision) {
                Some(Decision::Sail) => Some(self.sail(g, p, legal)),
                Some(Decision::PiratePlace) => self.best_of(legal, |a| match a {
                    Action::PiratePlace(h) => {
                        let gr = &g.board.graph;
                        Some(g.ex.ships.iter().filter(|s| s.owner != p && listed(&gr.edge_hexes[s.edge as usize]).any(|x| x == *h as usize)).map(|s| 1.0 + g.renown(s.owner as usize) as f64 / 10.0).sum())
                    }
                    _ => None,
                }),
                _ => None,
            },
            Phase::Main => {
                // A harbour (2 Renown), ships (up to 2), cargo for them
                if let Some(a) = legal.iter().find(|a| matches!(a, Action::Harbour(_))) {
                    return Some(*a);
                }
                if legal.iter().any(|a| matches!(a, Action::Hut(_))) {
                    return None; // Greedy builds Huts first
                }
                let ships = g.ex.ships.iter().filter(|s| s.owner == p).count();
                if ships < 2 {
                    if let Some(a) = legal.iter().find(|a| matches!(a, Action::BuildShip(_))) {
                        return Some(*a);
                    }
                }
                let crews_wanted = (0..g.board.terrain.len()).any(|h| crew_target(g, p, h));
                let settlers_useful = g.pieces[pu][P_HUT] > 0 && g.settle_spot_left(p);
                if let Some(a) = legal.iter().find(|a| match a {
                    Action::BuildCargo(_, k) => (*k == SETTLER && settlers_useful) || (*k == CREW && crews_wanted),
                    _ => false,
                }) {
                    return Some(*a);
                }
                // A settler with nowhere to go makes room for crews
                if crews_wanted && !settlers_useful {
                    if let Some(a) = legal.iter().find(|a| matches!(a, Action::DropCargo(i) if g.ex.ships[*i as usize].cargo.contains(&SETTLER))) {
                        return Some(*a);
                    }
                }
                let _ = pu;
                None
            }
            _ => None,
        }
    }

    // One step of the movement phase
    fn sail(&mut self, g: &Game, p: u8, legal: &[Action]) -> Action {
        // Act where the ship is first
        let act = self.best_of(legal, |a| match a {
            Action::Settle(_, c) => Some(10.0 + settle_value(g, *c as usize)),
            Action::Deliver(_) => Some(20.0),
            Action::CrewAshore(..) => Some(15.0),
            Action::TakeHaul(..) | Action::TakeSack(..) => Some(12.0),
            Action::FishDie => Some(5.0),
            Action::ChasePirate(_) => Some(4.0),
            _ => None,
        });
        if let Some(a) = act {
            return a;
        }
        // Then move one unmoved ship toward its purpose, by real sailing distance (or not at all)
        let gr = &g.board.graph;
        let pu = p as usize;
        let touches = |e: usize, want: &dyn Fn(usize) -> bool| gr.edge_corners[e].iter().any(|&c| listed(&gr.corner_hexes[c as usize]).any(|h| want(h)));
        let settle_edge = |e: usize| gr.edge_corners[e].iter().any(|&c| g.spaced(c as usize) && !g.explore_blocked_corner(p, c as usize));
        let council = |h: usize| h as u16 == g.ex.council;
        let haul = |h: usize| g.ex.haul[h];
        let crew = |h: usize| crew_target(g, p, h);
        let home = |e: usize| gr.edge_corners[e].iter().any(|&c| g.corner_owner[c as usize] == p && g.corner_kind[c as usize] == LODGE);
        let settlers_useful = g.ex.settlers_left[pu] > 0 && g.pieces[pu][P_HUT] > 0 && g.settle_spot_left(p);
        let crews_useful = g.ex.crews_left[pu] > 0 && (0..g.board.terrain.len()).any(|h| crew_target(g, p, h));
        let d_settle = sea_dist(g, &settle_edge);
        let d_council = sea_dist(g, &|e| touches(e, &council));
        let d_crew = sea_dist(g, &|e| touches(e, &crew));
        let d_haul = sea_dist(g, &|e| touches(e, &haul));
        let d_fog = sea_dist(g, &|e| g.by_hidden(e));
        let d_home = sea_dist(g, &home);
        let score = |d: u32, base: f64| if d == u32::MAX { f64::MIN } else { base - d as f64 };
        let purpose = |s: &Ship, e: usize| -> f64 {
            if s.cargo.contains(&SETTLER) && d_settle[e] != u32::MAX {
                return score(d_settle[e], 60.0);
            }
            if s.cargo.contains(&HAUL) || s.cargo.contains(&SACK) {
                return score(d_council[e], 60.0);
            }
            if s.cargo.contains(&CREW) && d_crew[e] != u32::MAX {
                return score(d_crew[e], 50.0);
            }
            if s.cargo == [0, 0] && d_haul[e] != u32::MAX {
                return score(d_haul[e], 40.0);
            }
            if s.cargo == [0, 0] && (settlers_useful || crews_useful) && d_home[e] != u32::MAX {
                return score(d_home[e], 30.0);
            }
            score(d_fog[e], 20.0)
        };
        let mut best: Option<(f64, Action)> = None;
        for a in legal {
            if let Action::Sail(i, e) = a {
                let s = &g.ex.ships[*i as usize];
                let gain = purpose(s, *e as usize) - purpose(s, s.edge as usize);
                if gain > 0.0 && best.is_none_or(|b| gain > b.0) {
                    best = Some((gain, *a));
                }
            }
        }
        best.map_or(Action::Skip, |b| b.1)
    }
}
