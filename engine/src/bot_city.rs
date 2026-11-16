// Greedy's rules for the city expansion (city.rs): keep enough active knights when the raiders are
// close, climb the improvement tracks, play progress cards when they help, and answer every new
// decision. No lookahead, like the rest of Greedy.
use crate::bot::Greedy;
use crate::game::*;
use crate::hex::NONE;
use crate::island::pips;

// How much a progress card is worth keeping (dropped lowest first)
fn card_value(k: u8) -> f64 {
    match k {
        MONOPOLY | WEDDING | MERCHANT | MEDICINE | GUILD_DUES => 5.0,
        IRRIGATION | QUARRYING | SMITHING | RALLY | PATH_BUILDERS | SABOTAGE | TAXES => 4.0,
        CRANE | ENGINEER | GOODS_MONOPOLY | SPY | HARBOUR | FIXED_ROLL => 3.0,
        TREASON | INTRIGUE | TRADE_FLEET | DIPLOMACY => 2.0,
        _ => 1.0,
    }
}

// What p wants of each card kind: its plan's resources, and the commodities for its next improvements
fn wants(g: &Game, p: u8, need: &Hand) -> [u8; CARD_KINDS] {
    let pu = p as usize;
    let mut w = [0u8; CARD_KINDS];
    w[..5].copy_from_slice(need);
    for t in 0..3 {
        if g.cs.levels[pu][t] < 5 {
            w[PAPER + t] = g.improve_cost(pu, t);
        }
    }
    w
}

// p's weight on hex h: 1 per Hut, 2 per Lodge around it
fn weight(g: &Game, p: u8, h: usize) -> i32 {
    g.board.graph.hex_corners[h].iter().filter(|&&c| g.corner_owner[c as usize] == p).map(|&c| g.corner_kind[c as usize] as i32).sum()
}

fn leader_score(g: &Game, o: usize) -> f64 {
    g.renown(o) as f64 * 10.0 + g.cards(o) as f64
}

impl Greedy {
    // The city expansion's choices; None leaves it to the usual rules
    pub(crate) fn city_choose(&mut self, g: &Game, p: u8, legal: &[Action], need: Hand) -> Option<Action> {
        let pu = p as usize;
        let hand = g.hands[pu];
        let want = wants(g, p, &need);
        let spare = |k: usize| hand[k] as f64 - want[k] as f64;
        match g.phase {
            Phase::PreRoll => {
                // Fixed Roll: the total that pays this bot most (never 7), red die as low as possible
                let best = self.best_of(legal, |a| match a {
                    Action::FixedRoll(r, y) if r + y != 7 && r <= y => Some(roll_value(g, p, r + y) - *r as f64 * 0.01),
                    _ => None,
                });
                Some(match best {
                    Some(a @ Action::FixedRoll(r, y)) if roll_value(g, p, r + y) >= 2.0 => a,
                    _ => Action::Roll,
                })
            }
            Phase::Decide => Some(self.best_by(legal, |a| match a {
                Action::Discard(k) => spare(*k as usize),
                Action::Take(r) => -spare(*r as usize),
                Action::Gift(to, k) => spare(*k as usize) - g.renown(*to as usize) as f64 / 10.0,
                Action::Rob(v) => leader_score(g, *v as usize),
                Action::DropCard(k) => -card_value(*k),
                Action::Pillage(c) => -(corner_pips(&g.board, *c as usize, g.squall) as f64),
                Action::KnightMove(_, to) => corner_pips(&g.board, *to as usize, NONE) as f64,
                Action::Choose(x) => match g.deciding().map(|d| d.decision) {
                    Some(Decision::DrawCard) => g.cs.decks[*x as usize].len() as f64,
                    Some(Decision::Name { card: TRADE_FLEET }) => spare(*x as usize),
                    Some(Decision::Name { .. }) => -spare(*x as usize),
                    _ => leader_score(g, *x as usize), // a target: the leader
                },
                Action::TakeCard(_, k) => -spare(*k as usize),
                Action::TakeProgress(_, k) => card_value(*k),
                Action::Promote(c) => g.cs.knight_level[*c as usize] as f64 + g.cs.knight_active[*c as usize] as u8 as f64,
                Action::Wall(_) => 1.0,
                Action::SwapNumbers(a, b) => {
                    let (a, b) = (*a as usize, *b as usize);
                    let (pa, pb) = (pips(g.board.number[a]) as i32, pips(g.board.number[b]) as i32);
                    (weight(g, p, a) * (pb - pa) + weight(g, p, b) * (pa - pb)) as f64
                }
                Action::Merchant(h) => {
                    let h = *h as usize;
                    weight(g, p, h) as f64 * pips(g.board.number[h]) as f64
                }
                Action::RemovePath(e) => {
                    let o = g.edge_owner[*e as usize];
                    if o == p { -100.0 } else { leader_score(g, o as usize) }
                }
                Action::Expel(c) => leader_score(g, g.cs.knight_owner[*c as usize] as usize) + g.cs.knight_level[*c as usize] as f64,
                Action::RemoveKnight(c) => -(g.cs.knight_level[*c as usize] as f64) - 2.0 * g.cs.knight_active[*c as usize] as u8 as f64,
                Action::Recruit(c) => corner_pips(&g.board, *c as usize, NONE) as f64,
                Action::Skip => -50.0,
                _ => 0.0,
            })),
            Phase::Main => self.city_main(g, p, legal, &want),
            _ => None,
        }
    }

    fn city_main(&mut self, g: &Game, p: u8, legal: &[Action], want: &[u8; CARD_KINDS]) -> Option<Action> {
        let pu = p as usize;
        let has = |a: &Action| legal.contains(a);
        // Over 4 progress cards: drop the least useful
        if g.must_drop(pu) {
            return self.best_of(legal, |a| match a {
                Action::DropCard(k) => Some(-card_value(*k)),
                _ => None,
            });
        }
        // Lodges and Huts come first (Greedy's own rules)
        if legal.iter().any(|a| matches!(a, Action::Lodge(_) | Action::Hut(_))) {
            return None;
        }
        // Defence: with a Lodge to lose and the raiders on their way, an active knight (or more, to
        // stay off the bottom); always recruit a first knight
        let raiders_close = g.cs.raiders + 3 >= g.settings().city_raiders;
        let mine = g.defence(pu);
        let lowest_other = (0..g.players).filter(|&o| o != pu).map(|o| g.defence(o)).min().unwrap_or(0);
        let exposed = g.lodges(pu) > 0 && (mine == 0 || (raiders_close && mine <= lowest_other));
        if exposed {
            if let Some(a) = legal.iter().find(|a| matches!(a, Action::Activate(_))) {
                return Some(*a);
            }
            if raiders_close && has(&Action::PlayCard(RALLY)) {
                return Some(Action::PlayCard(RALLY));
            }
        }
        let knights = g.cs.knight_owner.iter().filter(|&&o| o == p).count();
        if knights < 1 + g.lodges(pu) as usize / 2 {
            if let Some(a) = self.best_of(legal, |a| match a {
                Action::Recruit(c) => Some(corner_pips(&g.board, *c as usize, NONE) as f64),
                _ => None,
            }) {
                return Some(a);
            }
        }
        // Improvements: the highest track first (metropolises), Crane when it makes one affordable
        if let Some(a) = self.best_of(legal, |a| match a {
            Action::Improve(t) => Some(g.cs.levels[pu][*t as usize] as f64 + if *t == 2 { 0.5 } else { 0.0 }),
            _ => None,
        }) {
            return Some(a);
        }
        if has(&Action::PlayCard(CRANE)) && (0..3).any(|t| g.cs.levels[pu][t] < 5 && g.hands[pu][PAPER + t] + 1 >= g.improve_cost(pu, t)) {
            return Some(Action::PlayCard(CRANE));
        }
        // Progress cards that help now
        for k in [MONOPOLY, GOODS_MONOPOLY, IRRIGATION, QUARRYING, MERCHANT, WEDDING, GUILD_DUES, TAXES, SPY, SABOTAGE, HARBOUR, INTRIGUE, TREASON, DIPLOMACY, SMITHING, ENGINEER] {
            if !has(&Action::PlayCard(k)) {
                continue;
            }
            let worth = match k {
                MERCHANT => g.cs.merchant_owner != p,
                SMITHING => (0..g.cs.knight_owner.len()).any(|c| legal.contains(&Action::Promote(c as u16)) || g.cs.knight_owner[c] == p),
                ENGINEER => g.walls(pu) < 3 && g.lodges(pu) > 0,
                _ => true,
            };
            if worth {
                return Some(Action::PlayCard(k));
            }
        }
        if has(&Action::PlayCard(MEDICINE)) && g.huts(pu) > 0 && g.pieces[pu][P_LODGE] > 0 && afford(&g.hands[pu], &MEDICINE_COST) {
            return Some(Action::PlayCard(MEDICINE));
        }
        // Chase the squall off one of its own buildings
        if g.squall != NONE && g.board.graph.hex_corners[g.squall as usize].iter().any(|&c| g.corner_owner[c as usize] == p) {
            if let Some(a) = legal.iter().find(|a| matches!(a, Action::Chase(_))) {
                return Some(*a);
            }
        }
        // Promote and activate knights with spare Fish + Stone and Grain once the build goals are met
        if let Some(a) = self.best_of(legal, |a| match a {
            Action::Promote(c) => Some(g.cs.knight_level[*c as usize] as f64),
            _ => None,
        }) {
            if raiders_close || g.cards(pu) > g.hand_limit(pu) {
                return Some(a);
            }
        }
        if raiders_close {
            if let Some(a) = legal.iter().find(|a| matches!(a, Action::Activate(_))) {
                return Some(*a);
            }
        }
        // A wall when the hand is near the limit
        if g.cards(pu) + 2 > g.hand_limit(pu) {
            if let Some(a) = legal.iter().find(|a| matches!(a, Action::Wall(_))) {
                return Some(*a);
            }
        }
        // Trade spare commodities toward an improvement, or spare resources for commodities
        let hand = g.hands[pu];
        let short = (PAPER..CARD_KINDS).filter(|&k| hand[k] < want[k]).max_by_key(|&k| g.cs.levels[pu][k - PAPER]);
        if let Some(get) = short {
            let give = (0..CARD_KINDS)
                .filter(|&k| k != get && hand[k] >= want[k] + g.rate(p, k))
                .max_by_key(|&k| hand[k] - want[k]);
            if let Some(give) = give {
                let a = Action::Trade(give as u8, get as u8);
                if has(&a) {
                    return Some(a);
                }
            }
        }
        // Knights wander toward the best corner they can reach (blocking others' spots)
        None
    }
}

// What this total pays p (dice pips aside): resources, Lodges counting double
fn roll_value(g: &Game, p: u8, total: u8) -> f64 {
    let mut v = 0.0;
    for &h in &g.board.by_number[total as usize] {
        if h == g.squall {
            continue;
        }
        v += weight(g, p, h as usize) as f64;
    }
    v
}
