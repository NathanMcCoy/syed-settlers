// A simple bot with no lookahead: it builds where the dice pay best, works toward the best free spot
// it can reach, trades toward what it's missing, and sends the squall at the leader. It's the
// simulator's opponent for balance checks, and the first bot to play against. Stronger bots (search,
// learning) come later and will be measured against this one.
//
// It only looks at what its seat may see: its own hand, the board, and public counts.
use crate::game::*;
use crate::hex::listed;
use crate::island::pips;
use crate::rng::Rng;

pub struct Greedy {
    pub rng: Rng,
}

enum Goal {
    Hut,
    Lodge,
    Edge(usize, u8), // the next edge toward the target spot, and PATH or BOAT
    Project,
}

impl Greedy {
    pub fn new(seed: u32) -> Greedy {
        Greedy { rng: Rng::new(seed) }
    }

    pub fn choose(&mut self, g: &Game, legal: &[Action]) -> Action {
        if legal.len() == 1 {
            return legal[0];
        }
        let p = g.to_act();
        match g.phase {
            Phase::SetupHut => self.best_by(legal, |a| match a {
                Action::Hut(c) => spot_value(g, p, *c as usize),
                _ => f64::MIN,
            }),
            Phase::SetupPath { .. } => self.best_by(legal, |a| match a {
                Action::Path(e) | Action::Boat(e) => best_spot_beyond(g, p, *e as usize),
                _ => f64::MIN,
            }),
            Phase::Discard => {
                let need = self.plan(g, p).1;
                let hand = g.hands[p as usize];
                self.best_by(legal, |a| match a {
                    Action::Discard(r) => hand[*r as usize] as f64 - need[*r as usize] as f64,
                    _ => f64::MIN,
                })
            }
            Phase::MoveSquall { .. } => self.best_by(legal, |a| match a {
                Action::Squall(h) => squall_value(g, p, *h as usize),
                _ => f64::MIN,
            }),
            Phase::Steal => self.best_by(legal, |a| match a {
                Action::Steal(v) => g.renown(*v as usize) as f64 * 10.0 + g.cards(*v as usize) as f64,
                _ => f64::MIN,
            }),
            Phase::Harvest { .. } => {
                let (_, need) = self.plan(g, p);
                let hand = g.hands[p as usize];
                self.best_by(legal, |a| match a {
                    Action::Take(r) => need[*r as usize] as f64 - hand[*r as usize] as f64,
                    _ => f64::MIN,
                })
            }
            Phase::Winds => {
                let hand = g.hands[p as usize];
                self.best_by(legal, |a| match a {
                    Action::Take(r) => hand[*r as usize] as f64,
                    _ => f64::MIN,
                })
            }
            Phase::FreeBuild { .. } => match self.plan(g, p).0 {
                Goal::Edge(e, _) if legal.contains(&Action::Path(e as u16)) => Action::Path(e as u16),
                Goal::Edge(e, _) if legal.contains(&Action::Boat(e as u16)) => Action::Boat(e as u16),
                _ => *self.rng.pick(legal),
            },
            Phase::PreRoll => {
                if self.pilot_wanted(g, p) && legal.contains(&Action::Play(Project::Pilot)) {
                    Action::Play(Project::Pilot)
                } else {
                    Action::Roll
                }
            }
            Phase::Main => self.main(g, p, legal),
            Phase::Over => legal[0],
        }
    }

    fn main(&mut self, g: &Game, p: u8, legal: &[Action]) -> Action {
        // Anything worth Renown that's affordable now: the best Lodge or Hut
        let lodge = self.best_of(legal, |a| match a {
            Action::Lodge(c) => Some(spot_value(g, p, *c as usize)),
            _ => None,
        });
        if let Some(a) = lodge {
            return a;
        }
        let hut = self.best_of(legal, |a| match a {
            Action::Hut(c) => Some(spot_value(g, p, *c as usize)),
            _ => None,
        });
        if let Some(a) = hut {
            return a;
        }
        let (goal, need) = self.plan(g, p);
        if let Goal::Edge(e, kind) = goal {
            let a = if kind == PATH { Action::Path(e as u16) } else { Action::Boat(e as u16) };
            if legal.contains(&a) {
                return a;
            }
        }
        for k in [Project::Harvest, Project::Shipwright] {
            if legal.contains(&Action::Play(k)) && (k != Project::Shipwright || matches!(goal, Goal::Edge(..))) {
                return Action::Play(k);
            }
        }
        if self.pilot_wanted(g, p) && legal.contains(&Action::Play(Project::Pilot)) {
            return Action::Play(Project::Pilot);
        }
        // Trade a surplus toward the goal: give what's most over need, get what's most short
        let hand = g.hands[p as usize];
        let short = (0..5).filter(|&r| hand[r] < need[r]).max_by_key(|&r| need[r] - hand[r]);
        if let Some(get) = short {
            let give = (0..5)
                .filter(|&r| r != get && hand[r] >= need[r] + g.rate(p, r))
                .max_by_key(|&r| hand[r] - need[r]);
            if let Some(give) = give {
                return Action::Trade(give as u8, get as u8);
            }
        }
        // Projects: when that's the goal, or to spend a hand that a 7 would halve
        let buy = matches!(goal, Goal::Project) || g.cards(p as usize) > g.settings().discard_over as u32;
        if buy {
            let pick = self.best_of(legal, |a| match a {
                Action::Buy(slot) => Some(match g.row.get(*slot as usize).copied() {
                    Some(k) if k == Project::Lighthouse as u8 => 3.0,
                    Some(k) if k == Project::Harvest as u8 => 2.0,
                    _ => 1.0,
                }),
                _ => None,
            });
            if let Some(a) = pick {
                return a;
            }
        }
        Action::EndTurn
    }

    // Answer the open offer made to `seat`: yes if it gets cards this bot is short of for its goal
    // without giving away ones it needs, and never help someone within 2 Renown of winning
    pub fn answer(&self, g: &Game, seat: u8) -> bool {
        let Some(o) = g.offer else { return false };
        let s = seat as usize;
        if !afford(&g.hands[s], &o.get) || g.renown(g.current as usize) + 2 >= g.settings().win as u32 {
            return false;
        }
        let need = self.plan(g, seat).1;
        let hand = g.hands[s];
        let mut score = 0.0;
        for r in 0..5 {
            let (short, spare) = (need[r].saturating_sub(hand[r]) as f64, hand[r].saturating_sub(need[r]) as f64);
            let (gain, loss) = (o.give[r] as f64, o.get[r] as f64);
            score += gain.min(short) * 2.0 + (gain - gain.min(short)) * 0.3;
            score -= loss.min(spare) * 0.5 + (loss - loss.min(spare)) * 2.0;
        }
        score > 0.4
    }

    // What to work toward, and what it costs
    fn plan(&self, g: &Game, p: u8) -> (Goal, Hand) {
        let pu = p as usize;
        let pieces = g.pieces[pu];
        let spot = if pieces[P_HUT] > 0 { reachable_spot(g, p) } else { None };
        if let Some((_, 0, _)) = spot {
            return (Goal::Hut, HUT_COST);
        }
        if pieces[P_LODGE] > 0 && g.huts(pu) > 0 {
            return (Goal::Lodge, LODGE_COST);
        }
        if let Some((_, _, Some((e, kind)))) = spot {
            return (Goal::Edge(e, kind), if kind == PATH { PATH_COST } else { BOAT_COST });
        }
        (Goal::Project, PROJECT_COST)
    }

    fn pilot_wanted(&self, g: &Game, p: u8) -> bool {
        g.board.graph.hex_corners[g.squall as usize].iter().any(|&c| g.corner_owner[c as usize] == p)
    }

    // The legal action with the highest score (ties broken at random)
    fn best_by(&mut self, legal: &[Action], score: impl Fn(&Action) -> f64) -> Action {
        self.best_of(legal, |a| Some(score(a))).unwrap_or(legal[0])
    }

    fn best_of(&mut self, legal: &[Action], score: impl Fn(&Action) -> Option<f64>) -> Option<Action> {
        let mut best: Option<(Action, f64)> = None;
        let mut ties = 0;
        for a in legal {
            let Some(s) = score(a) else { continue };
            match best {
                Some((_, b)) if s < b => {}
                Some((_, b)) if s == b => {
                    ties += 1;
                    if self.rng.below(ties + 1) == 0 {
                        best = Some((*a, s));
                    }
                }
                _ => {
                    best = Some((*a, s));
                    ties = 0;
                }
            }
        }
        best.map(|(a, _)| a)
    }
}

// How good a corner is for p: its dice pips, plus a bonus for resources p doesn't produce yet and
// for a market
fn spot_value(g: &Game, p: u8, c: usize) -> f64 {
    let b = &g.board;
    let have = production(g, p);
    let mut v = 0.0;
    for h in listed(&b.graph.corner_hexes[c]) {
        let r = b.resource[h];
        if r == NOBODY {
            continue;
        }
        let n = pips(b.number[h]) as f64;
        v += n;
        if have[r as usize] == 0 {
            v += 1.5;
        }
    }
    if b.corner_market[c] != NOBODY {
        v += 1.0;
    }
    v
}

// Pips p collects for each resource
fn production(g: &Game, p: u8) -> [u32; 5] {
    let b = &g.board;
    let mut out = [0; 5];
    for c in 0..g.corner_owner.len() {
        if g.corner_owner[c] != p {
            continue;
        }
        for h in listed(&b.graph.corner_hexes[c]) {
            let r = b.resource[h];
            if r != NOBODY {
                out[r as usize] += pips(b.number[h]) * g.corner_kind[c] as u32;
            }
        }
    }
    out
}

// Hurting the leader most, never p itself
fn squall_value(g: &Game, p: u8, h: usize) -> f64 {
    if g.board.resource[h] == NOBODY {
        return -1.0;
    }
    let n = pips(g.board.number[h]) as f64;
    let mut v = 0.0;
    for &c in &g.board.graph.hex_corners[h] {
        let o = g.corner_owner[c as usize];
        if o == p {
            return -100.0;
        }
        if o != NOBODY {
            v += n * g.corner_kind[c as usize] as f64 * (1.0 + g.renown(o as usize) as f64 / 5.0);
        }
    }
    v
}

// In setup: the best free spot one step past this edge
fn best_spot_beyond(g: &Game, p: u8, e: usize) -> f64 {
    let gr = &g.board.graph;
    let mut best = 0.0f64;
    for &c in &gr.edge_corners[e] {
        for n in listed(&gr.corner_corners[c as usize]) {
            if g.spaced(n) {
                best = best.max(spot_value(g, p, n));
            }
        }
    }
    best
}

// The best free spot p can reach by building along free edges (a shortest route that doesn't pass
// through other players' buildings), weighing value against distance: (corner, edges needed, the
// first edge to build and its kind). Distance 0: buildable now.
fn reachable_spot(g: &Game, p: u8) -> Option<(usize, u32, Option<(usize, u8)>)> {
    let gr = &g.board.graph;
    let corners = gr.corners();
    let mut dist = vec![u32::MAX; corners];
    let mut first: Vec<Option<(usize, u8)>> = vec![None; corners];
    let mut queue = std::collections::VecDeque::new();
    for c in 0..corners {
        let mine = g.corner_owner[c] == p || (g.corner_owner[c] == NOBODY && listed(&gr.corner_edges[c]).any(|e| g.edge_owner[e] == p));
        if mine {
            dist[c] = 0;
            queue.push_back(c);
        }
    }
    let pieces = g.pieces[p as usize];
    while let Some(c) = queue.pop_front() {
        if g.corner_owner[c] != NOBODY && g.corner_owner[c] != p {
            continue; // can't build through another player's building
        }
        for (k, n) in listed(&gr.corner_corners[c]).enumerate() {
            if dist[n] != u32::MAX {
                continue;
            }
            let e = gr.corner_edges[c][k] as usize;
            if g.edge_owner[e] != NOBODY {
                continue;
            }
            let kind = if g.board.edge_land[e] && pieces[P_PATH] > 0 {
                PATH
            } else if g.board.edge_sea[e] && pieces[P_BOAT] > 0 {
                BOAT
            } else {
                continue;
            };
            dist[n] = dist[c] + 1;
            first[n] = if dist[c] == 0 { Some((e, kind)) } else { first[c] };
            if dist[n] < 6 {
                queue.push_back(n);
            }
        }
    }
    let mut best: Option<(usize, u32, Option<(usize, u8)>, f64)> = None;
    for c in 0..corners {
        if dist[c] == u32::MAX || !g.spaced(c) {
            continue;
        }
        let connected = listed(&gr.corner_edges[c]).any(|e| g.edge_owner[e] == p);
        let d = if connected { 0 } else { dist[c] };
        if d == 0 && !connected {
            continue;
        }
        let score = spot_value(g, p, c) - 2.0 * d as f64;
        if best.as_ref().is_none_or(|b| score > b.3) {
            best = Some((c, d, first[c], score));
        }
    }
    best.map(|(c, d, f, _)| (c, d, f))
}
