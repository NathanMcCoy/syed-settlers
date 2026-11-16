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

// Every bot (F13 in EXPANSIONS.md): it picks one of the legal actions for the seat to act, and answers
// trade offers made to it. A bot must always be able to pick from whatever legal() offers, so a new
// rule only needs its actions in legal() for every bot to keep finishing games; Greedy then gets a
// sensible rule for it.
pub trait Bot {
    fn choose(&mut self, g: &Game, legal: &[Action]) -> Action;
    fn answer(&mut self, g: &Game, seat: u8) -> bool;
}

// Picks any legal action at random and answers offers at random: the "never stuck" check for new
// rules (it reaches odd corners of the rules quickly) and the floor every real bot must beat
pub struct RandomBot {
    pub rng: Rng,
}

impl RandomBot {
    pub fn new(seed: u32) -> RandomBot {
        RandomBot { rng: Rng::new(seed) }
    }
}

impl Bot for RandomBot {
    fn choose(&mut self, _g: &Game, legal: &[Action]) -> Action {
        *self.rng.pick(legal)
    }
    fn answer(&mut self, _g: &Game, _seat: u8) -> bool {
        self.rng.below(2) == 0
    }
}

impl Bot for Greedy {
    fn choose(&mut self, g: &Game, legal: &[Action]) -> Action {
        Greedy::choose(self, g, legal)
    }
    fn answer(&mut self, g: &Game, seat: u8) -> bool {
        Greedy::answer(self, g, seat)
    }
}

pub struct Greedy {
    pub rng: Rng,
}

enum Goal {
    Hut,
    Lodge,
    Edge(usize),     // the next Path toward the target spot
    ShipEdge(usize), // the sea expansion: the next ship toward a spot reached by sea
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
                Action::Path(e) => best_spot_beyond(g, p, *e as usize),
                _ => f64::MIN,
            }),
            Phase::Decide => {
                let need = self.plan(g, p).1;
                let hand = g.hands[p as usize];
                self.best_by(legal, |a| match a {
                    // Lose what's least needed; take (gold) what's most short
                    Action::Discard(r) => hand[*r as usize] as f64 - need[*r as usize] as f64,
                    Action::Take(r) => need[*r as usize] as f64 - hand[*r as usize] as f64,
                    // Event cards: give what's least needed, to whoever is furthest behind; rob the
                    // leader; damage any Path (they're repaired later)
                    Action::Gift(to, r) => hand[*r as usize] as f64 - need[*r as usize] as f64 - g.renown(*to as usize) as f64 / 10.0,
                    Action::Rob(v) => g.renown(*v as usize) as f64 * 10.0 + g.cards(*v as usize) as f64,
                    Action::Damage(_) => 0.0,
                    // The two-player variant: a neutral Hut where the other player would most like to
                    // build (denying it); neutral Paths anywhere
                    Action::NeutralHut(_, c) => spot_value(g, 1 - p, *c as usize),
                    Action::NeutralPath(..) => 0.0,
                    _ => f64::MIN,
                })
            }
            Phase::Rolled { .. } | Phase::SecondRoll => legal[0],
            Phase::MoveSquall { .. } => self.best_by(legal, |a| match a {
                Action::Squall(h) => squall_value(g, p, *h as usize),
                Action::Pirate(h) => pirate_value(g, p, *h as usize),
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
            // Market Day: the resource it's most short of for its goal
            Phase::MarketDay => {
                let (_, need) = self.plan(g, p);
                let hand = g.hands[p as usize];
                self.best_by(legal, |a| match a {
                    Action::Take(r) => need[*r as usize] as f64 - hand[*r as usize] as f64,
                    _ => f64::MIN,
                })
            }
            Phase::FreeBuild { .. } => match self.plan(g, p).0 {
                Goal::Edge(e) if legal.contains(&Action::Path(e as u16)) => Action::Path(e as u16),
                Goal::ShipEdge(e) if legal.contains(&Action::Ship(e as u16)) => Action::Ship(e as u16),
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
        if let Goal::ShipEdge(e) = goal {
            if legal.contains(&Action::Ship(e as u16)) {
                return Action::Ship(e as u16);
            }
        }
        // Wonders: claim one as soon as possible, then build its levels whenever affordable; the
        // Forgotten Tribe: place a market as soon as there's a spot
        if let Some(&a) = legal.iter().find(|a| matches!(a, Action::ClaimWonder(_) | Action::BuildWonder | Action::PlaceMarket(_))) {
            return a;
        }
        // The two-player variant: trade tokens send the squall off this bot's own hex
        if legal.contains(&Action::SquallHome) && g.board.graph.hex_corners[g.squall as usize].iter().any(|&c| g.corner_owner[c as usize] == p) {
            return Action::SquallHome;
        }
        // Event cards: a damaged Path stops new Paths; repair it as soon as it can be afforded
        if let Some(&a) = legal.iter().find(|a| matches!(a, Action::Repair(_))) {
            return a;
        }
        if let Goal::Edge(e) = goal {
            if legal.contains(&Action::Path(e as u16)) {
                return Action::Path(e as u16);
            }
        }
        let short = (0..5).any(|r| g.hands[p as usize][r] < need[r]);
        for k in [Project::Harvest, Project::MarketDay, Project::RoadCrew] {
            let worth = match k {
                Project::RoadCrew => matches!(goal, Goal::Edge(..) | Goal::ShipEdge(..)),
                Project::MarketDay => short,
                _ => true,
            };
            if worth && legal.contains(&Action::Play(k)) {
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
        // Wonders: once claimed, save for its levels (a finished wonder wins)
        if g.wonders_on() && g.wonder_of[pu] != NOBODY && g.wonder_level[pu] < 4 && (g.wonder_level[pu] > 0 || spot.is_none()) {
            return (Goal::Project, WONDERS[g.wonder_of[pu] as usize].cost());
        }
        // The Pirate Islands: two warships first (the fleet strips the undefended), then the line to the
        // fortress, then more Pilots for warships
        if g.pirate_islands() && !g.fort_taken[pu] && g.warships(pu) < 2 && g.ship_line.iter().any(|&(o, _)| o == p) && !g.deck.is_empty() {
            return (Goal::Project, PROJECT_COST);
        }
        if g.pirate_islands() && !g.fort_taken[pu] {
            if g.line_target(pu).is_some() && pieces[P_SHIP] > 0 {
                let mut next = Vec::new();
                g.legal_line_ships(p, &mut next);
                if let Some(Action::Ship(e)) = next.first() {
                    return (Goal::ShipEdge(*e as usize), SHIP_COST);
                }
            } else if g.line_target(pu).is_none() {
                return (Goal::Project, PROJECT_COST);
            }
        }
        // The Forgotten Tribe: sail for the prizes around the small islands
        if g.tribe() && pieces[P_SHIP] > 0 {
            if let Some((d, e)) = reachable_prize(g, p) {
                if d <= 5 {
                    return (Goal::ShipEdge(e), SHIP_COST);
                }
            }
        }
        // Cloth: link to a village not yet traded with (cloth is Renown), by ship
        if g.cloth() && pieces[P_SHIP] > 0 {
            if let Some((d, e)) = reachable_village(g, p) {
                if d <= 12 {
                    return (Goal::ShipEdge(e), SHIP_COST);
                }
            }
        }
        // The sea expansion: sail when the best spot reached by ships beats the best by Paths
        if g.sea() && pieces[P_HUT] > 0 && pieces[P_SHIP] > 0 {
            let by_land = spot.map_or(f64::MIN, |(c, d, _)| spot_value(g, p, c) - 2.0 * d as f64);
            if let Some((c, d, Some(e))) = reachable_by_sea(g, p) {
                if sea_score(g, p, c, d) > by_land {
                    return (Goal::ShipEdge(e), SHIP_COST);
                }
            }
        }
        if let Some((_, _, Some(e))) = spot {
            return (Goal::Edge(e), PATH_COST);
        }
        (Goal::Project, PROJECT_COST)
    }

    // When the squall sits on one of its buildings, or one more Pilot would take (or keep a lead
    // for) Most Pilots
    fn pilot_wanted(&self, g: &Game, p: u8) -> bool {
        let pu = p as usize;
        if g.pirate_islands() {
            return g.ship_line.iter().any(|&(o, e)| o == p && !g.warship[e as usize]); // a warship to make
        }
        let blocked = g.board.graph.hex_corners[g.squall as usize].iter().any(|&c| g.corner_owner[c as usize] == p);
        let next = g.pilots[pu] + 1;
        let title = g.holder(Title::MostPilots) != p
            && next >= g.settings().most_pilots_min
            && (0..g.players).all(|o| o == pu || g.pilots[o] < next);
        blocked || title
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
        // A gold field gives whatever's needed: worth a little more than its pips
        if r == GOLD {
            v += 1.0;
        } else if have[r as usize] == 0 {
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
            if r != NOBODY && r != GOLD {
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

// The pirate: hurting the players with the most ships on its hex, never p itself
fn pirate_value(g: &Game, p: u8, h: usize) -> f64 {
    let mut v = -2.0; // the squall usually hurts more: only worth it with ships to rob
    for &e in &g.board.graph.hex_edges[h] {
        let o = g.edge_owner[e as usize];
        if o == NOBODY || g.edge_kind[e as usize] != SHIP {
            continue;
        }
        if o == p {
            return -100.0;
        }
        v += 3.0 * (1.0 + g.renown(o as usize) as f64 / 5.0);
    }
    v
}

// A spot reached by sea: its value, the island bonus if p has no Hut on that island yet, less distance
fn sea_score(g: &Game, p: u8, c: usize, d: u32) -> f64 {
    let region = g.board.corner_region[c];
    let new_island = region < 16 && g.islands_reached[p as usize] & (1 << region) == 0;
    let bonus = if new_island { 3.0 * g.settings().sea_island_bonus as f64 } else { 0.0 };
    spot_value(g, p, c) + bonus - 2.0 * d as f64
}

// The best free spot p can reach by building ships along free sea edges from its coastal buildings
// and ships (never past another player's building, never by the pirate): (corner, ships needed, the
// first ship to build)
fn reachable_by_sea(g: &Game, p: u8) -> Option<(usize, u32, Option<usize>)> {
    let gr = &g.board.graph;
    let corners = gr.corners();
    let mut dist = vec![u32::MAX; corners];
    let mut first: Vec<Option<usize>> = vec![None; corners];
    let mut queue = std::collections::VecDeque::new();
    for c in 0..corners {
        let own_ship = listed(&gr.corner_edges[c]).any(|e| g.edge_owner[e] == p && g.edge_kind[e] == SHIP);
        let mine = g.corner_owner[c] == p || (g.corner_owner[c] == NOBODY && own_ship);
        if mine && listed(&gr.corner_edges[c]).any(|e| g.board.edge_sea[e]) {
            dist[c] = 0;
            queue.push_back(c);
        }
    }
    while let Some(c) = queue.pop_front() {
        if g.corner_owner[c] != NOBODY && g.corner_owner[c] != p {
            continue; // no ship past another player's building
        }
        for (k, n) in listed(&gr.corner_corners[c]).enumerate() {
            let e = gr.corner_edges[c][k] as usize;
            if dist[n] != u32::MAX || !g.board.edge_sea[e] || g.edge_owner[e] != NOBODY || g.by_pirate(e) {
                continue;
            }
            dist[n] = dist[c] + 1;
            first[n] = if dist[c] == 0 { Some(e) } else { first[c] };
            if dist[n] < 8 {
                queue.push_back(n);
            }
        }
    }
    let mut best: Option<(usize, u32, Option<usize>, f64)> = None;
    for c in 0..corners {
        if dist[c] == u32::MAX || dist[c] == 0 || !g.spaced(c) || !g.hut_allowed(p, c) || g.board.corner_region[c] == crate::island::SEA {
            continue;
        }
        let score = sea_score(g, p, c, dist[c]);
        if best.as_ref().is_none_or(|b| score > b.3) {
            best = Some((c, dist[c], first[c], score));
        }
    }
    best.map(|(c, d, f, _)| (c, d, f))
}

// The Forgotten Tribe: the nearest prize edge by ships (ships needed, the first ship to build)
fn reachable_prize(g: &Game, p: u8) -> Option<(u32, usize)> {
    let gr = &g.board.graph;
    let corners = gr.corners();
    let mut dist = vec![u32::MAX; corners];
    let mut first: Vec<Option<usize>> = vec![None; corners];
    let mut queue = std::collections::VecDeque::new();
    for c in 0..corners {
        let own_ship = listed(&gr.corner_edges[c]).any(|e| g.edge_owner[e] == p && g.edge_kind[e] == SHIP);
        if g.corner_owner[c] == p || (g.corner_owner[c] == NOBODY && own_ship) {
            dist[c] = 0;
            queue.push_back(c);
        }
    }
    while let Some(c) = queue.pop_front() {
        if g.corner_owner[c] != NOBODY && g.corner_owner[c] != p {
            continue;
        }
        for (k, n) in listed(&gr.corner_corners[c]).enumerate() {
            let e = gr.corner_edges[c][k] as usize;
            if !g.board.edge_sea[e] || g.edge_owner[e] != NOBODY || g.by_pirate(e) {
                continue;
            }
            let via = if dist[c] == 0 { e } else { first[c].unwrap() };
            if g.prizes[e] != 0 {
                return Some((dist[c] + 1, via));
            }
            if dist[n] == u32::MAX {
                dist[n] = dist[c] + 1;
                first[n] = Some(via);
                queue.push_back(n);
            }
        }
    }
    None
}

// Cloth: the nearest corner of a village p isn't linked to, by ships from p's coastal buildings and
// ships: (ships needed, the first ship to build)
fn reachable_village(g: &Game, p: u8) -> Option<(u32, usize)> {
    let gr = &g.board.graph;
    let villages = g.villages();
    let unlinked: Vec<usize> = villages.iter().enumerate().filter(|(k, _)| g.village_links[p as usize] & (1 << k) == 0).map(|(_, &h)| h).collect();
    if unlinked.is_empty() {
        return None;
    }
    let target = |c: usize| listed(&gr.corner_hexes[c]).any(|h| unlinked.contains(&h));
    let corners = gr.corners();
    let mut dist = vec![u32::MAX; corners];
    let mut first: Vec<Option<usize>> = vec![None; corners];
    let mut queue = std::collections::VecDeque::new();
    for c in 0..corners {
        let own_ship = listed(&gr.corner_edges[c]).any(|e| g.edge_owner[e] == p && g.edge_kind[e] == SHIP);
        if g.corner_owner[c] == p || (g.corner_owner[c] == NOBODY && own_ship) {
            dist[c] = 0;
            queue.push_back(c);
        }
    }
    while let Some(c) = queue.pop_front() {
        if dist[c] > 0 && target(c) {
            return first[c].map(|e| (dist[c], e));
        }
        if g.corner_owner[c] != NOBODY && g.corner_owner[c] != p {
            continue;
        }
        for (k, n) in listed(&gr.corner_corners[c]).enumerate() {
            let e = gr.corner_edges[c][k] as usize;
            if dist[n] != u32::MAX || !g.board.edge_sea[e] || g.edge_owner[e] != NOBODY || g.by_pirate(e) {
                continue;
            }
            dist[n] = dist[c] + 1;
            first[n] = if dist[c] == 0 { Some(e) } else { first[c] };
            queue.push_back(n);
        }
    }
    None
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
// first edge to build). Distance 0: buildable now.
fn reachable_spot(g: &Game, p: u8) -> Option<(usize, u32, Option<usize>)> {
    let gr = &g.board.graph;
    let corners = gr.corners();
    let mut dist = vec![u32::MAX; corners];
    let mut first: Vec<Option<usize>> = vec![None; corners];
    let mut queue = std::collections::VecDeque::new();
    for c in 0..corners {
        let mine = g.corner_owner[c] == p || (g.corner_owner[c] == NOBODY && listed(&gr.corner_edges[c]).any(|e| g.edge_owner[e] == p));
        if mine {
            dist[c] = 0;
            queue.push_back(c);
        }
    }
    if g.pieces[p as usize][P_PATH] == 0 {
        queue.clear(); // no Paths left: only spots already connected
    }
    while let Some(c) = queue.pop_front() {
        if g.corner_owner[c] != NOBODY && g.corner_owner[c] != p {
            continue; // can't build through another player's building
        }
        for (k, n) in listed(&gr.corner_corners[c]).enumerate() {
            if dist[n] != u32::MAX {
                continue;
            }
            let e = gr.corner_edges[c][k] as usize;
            if g.edge_owner[e] != NOBODY || !g.board.edge_land[e] {
                continue;
            }
            dist[n] = dist[c] + 1;
            first[n] = if dist[c] == 0 { Some(e) } else { first[c] };
            if dist[n] < 6 {
                queue.push_back(n);
            }
        }
    }
    let mut best: Option<(usize, u32, Option<usize>, f64)> = None;
    for c in 0..corners {
        // (A corner by a damaged Path takes no Hut until it's repaired: event cards)
        if dist[c] == u32::MAX || !g.spaced(c) || !g.hut_allowed(p, c) {
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
