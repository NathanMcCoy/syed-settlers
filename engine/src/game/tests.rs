// The game rules (RULES.md), one test per rule where possible, plus random play that checks the whole
// state after every action.
use super::*;
use crate::bot::Greedy;
use crate::island::{Island, generate};
use crate::sim::{play, renown_sources};

fn island(seed: &str) -> Island {
    generate(&Settings::default(), seed).unwrap()
}

fn game(players: usize) -> Game {
    Game::new(&island("rules"), &Settings::default(), players, 7)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

// Greedy bots place everything in setup
fn after_setup(players: usize) -> Game {
    setup_done(game(players))
}

// The same, with settings other than the defaults
fn after_setup_with(players: usize, settings: &str) -> Game {
    let s = Settings::parse(settings).unwrap();
    setup_done(Game::new(&generate(&s, "rules").unwrap(), &s, players, 7))
}

fn setup_done(mut g: Game) -> Game {
    let mut bot = Greedy::new(1);
    while g.turn == 0 {
        let list = legal(&g);
        let a = bot.choose(&g, &list);
        g.apply(a).unwrap();
    }
    g
}

// A free corner on the island, with free neighbours, not on a market
fn free_spot(g: &Game) -> usize {
    (0..g.corner_owner.len()).find(|&c| g.spaced(c) && g.board.corner_market[c] == NOBODY).unwrap()
}

// Everything that must always be true
fn check(g: &Game) {
    let s = g.settings();
    let gr = &g.board.graph;
    for p in 0..g.players {
        let huts = (0..gr.corners()).filter(|&c| g.corner_owner[c] == p as u8 && g.corner_kind[c] == HUT).count() as u8;
        let lodges = (0..gr.corners()).filter(|&c| g.corner_owner[c] == p as u8 && g.corner_kind[c] == LODGE).count() as u8;
        let paths = (0..gr.edges()).filter(|&e| g.edge_owner[e] == p as u8 && g.edge_kind[e] == PATH).count() as u8;
        assert_eq!(huts, g.huts(p), "huts on the board match pieces used");
        assert_eq!(lodges, g.lodges(p));
        assert_eq!(paths + g.pieces[p][P_PATH], s.pieces_path);
        assert_eq!(g.renown(p), renown_sources(g, p).iter().sum::<u32>());
        assert_eq!(g.line[p], g.longest_line(p as u8));
        for r in 0..5 {
            assert!(g.hands[p][r] < 200, "a hand ran away (or wrapped below zero)");
        }
    }
    // A limited bank: players never hold more of a resource than the game has
    if s.bank_cards > 0 {
        for r in 0..5 {
            let held: u32 = (0..g.players).map(|p| g.hands[p][r] as u32).sum();
            assert!(held <= s.bank_cards as u32, "{held} of resource {r} held, only {} exist", s.bank_cards);
        }
    }
    // Spacing: no two buildings on neighbouring corners; Paths only on edges touching land
    for c in 0..gr.corners() {
        if g.corner_owner[c] != NOBODY {
            assert!(g.board.corner_land[c]);
            assert!(listed(&gr.corner_corners[c]).all(|n| g.corner_owner[n] == NOBODY), "buildings too close");
        }
    }
    for e in 0..gr.edges() {
        match g.edge_kind[e] {
            PATH => assert!(g.board.edge_land[e]),
            _ => assert_eq!(g.edge_owner[e], NOBODY),
        }
    }
    // Titles go only to someone at the minimum who isn't behind anyone
    if g.longest != NOBODY {
        let v = g.line[g.longest as usize];
        assert!(v >= s.longest_min && g.line[..g.players].iter().all(|&x| x <= v));
    }
    if g.most_pilots != NOBODY {
        let v = g.pilots[g.most_pilots as usize];
        assert!(v >= s.most_pilots_min && g.pilots[..g.players].iter().all(|&x| x <= v));
    }
    if g.over() && !g.by_rounds {
        assert!(g.renown(g.winner as usize) >= s.win as u32);
    }
    assert!(g.board.terrain[g.squall as usize] != Terrain::Sea, "the squall is on land");
    // Paired turns: only with 5-6 players, only for the player 3 seats on, never before a roll
    if g.paired {
        assert!(g.pairs() && g.current == g.partner(g.owner) && g.phase != Phase::PreRoll);
        assert!(g.offer.is_none(), "no trading with players in the extra build phase");
    } else if g.turn > 0 && !g.over() {
        assert_eq!(g.current, g.owner);
    }
    // An offer only exists during a turn, never to its own maker
    if let Some(o) = &g.offer {
        assert!(g.turn > 0 && !g.over());
        assert_eq!(o.to & (1 << g.current), 0);
        assert!(g.offers_made >= 1 && (s.trade_offers == 0 || g.offers_made <= s.trade_offers));
    }
}

#[test]
fn random_play_never_breaks_a_rule() {
    // Completely random legal actions: odd games, but every state must still be valid
    let mut list = Vec::new();
    let mut ended = 0;
    for seed in 0..400u32 {
        // 2 to 6 players, each with that many players' defaults (5-6: the large island, paired
        // turns); every 4th game the other way: projects face up, on a random-shaped island
        let players = 2 + seed as usize % 5;
        let text = if seed % 4 == 3 { "projects.faceUp=true;island.hexagon=false" } else { "" };
        let settings = Settings::parse_for(text, players).unwrap();
        let isl = generate(&settings, &format!("fuzz{seed}")).unwrap();
        let mut g = Game::new(&isl, &settings, players, seed);
        let mut rng = Rng::new(seed + 1000);
        for step in 0..3000 {
            g.legal(&mut list);
            if g.over() {
                assert!(list.is_empty());
                ended += 1;
                break;
            }
            assert!(!list.is_empty(), "nothing to do in {:?}", g.phase);
            // Now and then an offer to other players, and random answers from them
            if g.phase == Phase::Main && rng.below(6) == 0 {
                let me = g.current as usize;
                let mut give = [0u8; 5];
                let mut get = [0u8; 5];
                if let Some(r) = (0..5).find(|&r| g.hands[me][r] > 0) {
                    give[r] = 1 + rng.below(g.hands[me][r] as usize) as u8;
                    get[(r + 1 + rng.below(4)) % 5] = 1 + rng.below(2) as u8;
                    let to = (1 + rng.below((1 << g.players) - 1)) as u8 & !(1 << me);
                    let ok = g.offer_ok(&give, &get, to).is_ok();
                    assert_eq!(g.apply_as(g.current, Action::Offer { give, get, to, open: rng.below(2) == 0 }).is_ok(), ok);
                }
            }
            if g.offer.is_some() {
                for s in 0..g.players as u8 {
                    let mut answers = Vec::new();
                    g.legal_for(s, &mut answers);
                    answers.retain(|a| matches!(a, Action::Answer(..)));
                    if answers.is_empty() {
                        continue;
                    }
                    // Sometimes a counter-offer instead: one card of theirs for one of the offerer's
                    if rng.below(4) == 0 {
                        let (me, them) = (g.current as usize, s as usize);
                        if let (Some(a), Some(b)) = ((0..5).find(|&r| g.hands[me][r] > 0), (0..5).find(|&r| g.hands[them][r] > 0)) {
                            if a != b {
                                let (mut give, mut get) = ([0u8; 5], [0u8; 5]);
                                give[a] = 1;
                                get[b] = 1;
                                let ok = g.counter_ok(s, &give, &get).is_ok();
                                assert_eq!(g.apply_as(s, Action::Counter { seat: s, give, get }).is_ok(), ok);
                                continue;
                            }
                        }
                    }
                    g.apply_as(s, *rng.pick(&answers)).unwrap();
                    if g.offer.is_none() {
                        break; // an open offer was taken
                    }
                }
                g.legal(&mut list);
            }
            let before: u32 = (0..g.players).map(|p| g.cards(p)).sum();
            // Mostly random, but end turns often enough for games to move on
            let a = if list.contains(&Action::EndTurn) && rng.below(3) == 0 { Action::EndTurn } else { *rng.pick(&list) };
            if let Action::Deal(_) | Action::Withdraw | Action::Answer(..) | Action::Accept(_) = a {
                let mut copy = g.clone();
                copy.apply_unchecked(a);
                assert_eq!((0..g.players).map(|p| copy.cards(p)).sum::<u32>(), before, "trades between players keep every card");
            }
            if step % 7 == 0 {
                g.apply(a).unwrap();
            } else {
                g.apply_unchecked(a);
            }
            check(&g);
        }
    }
    assert!(ended > 50, "only {ended} random games ended");
}

#[test]
fn illegal_actions_are_refused_and_change_nothing() {
    let mut g = after_setup(3);
    let before = (g.hands, g.corner_owner.clone(), g.phase);
    assert!(g.apply(Action::EndTurn).is_err(), "can't end the turn before rolling");
    let taken = (0..g.corner_owner.len()).find(|&c| g.corner_owner[c] != NOBODY).unwrap();
    assert!(g.apply(Action::Hut(taken as u16)).is_err());
    assert!(g.apply(Action::Trade(0, 1)).is_err());
    assert_eq!(before, (g.hands, g.corner_owner.clone(), g.phase));
}

#[test]
fn setup_goes_round_and_back_and_the_second_hut_pays() {
    let mut g = game(3);
    let mut bot = Greedy::new(3);
    let mut order = Vec::new();
    while g.turn == 0 {
        if g.phase == Phase::SetupHut {
            order.push(g.to_act());
        }
        let list = legal(&g);
        assert!(list.iter().all(|a| matches!(a, Action::Hut(_) | Action::Path(_))));
        let a = bot.choose(&g, &list);
        let p = g.to_act() as usize;
        let before = g.hands[p];
        g.apply(a).unwrap();
        if let Action::Hut(c) = a {
            let mut want = before;
            if order.len() > 3 {
                for h in listed(&g.board.graph.corner_hexes[c as usize]) {
                    if g.board.resource[h] != NOBODY {
                        want[g.board.resource[h] as usize] += 1;
                    }
                }
            }
            assert_eq!(g.hands[p], want, "second-round Hut collects one of each around it, first round nothing");
        }
    }
    assert_eq!(order, vec![0, 1, 2, 2, 1, 0]);
    assert_eq!((g.phase, g.current, g.turn), (Phase::PreRoll, 0, 1));
    for p in 0..3 {
        assert_eq!(g.huts(p), 2);
        assert_eq!(g.settings().pieces_path - g.pieces[p][P_PATH], 2);
    }
}

#[test]
fn setup_paths_must_touch_the_new_hut_and_huts_must_be_spaced() {
    let mut g = game(2);
    let c = free_spot(&g);
    g.apply(Action::Hut(c as u16)).unwrap();
    for a in legal(&g) {
        let Action::Path(e) = a else { panic!("{a:?}") };
        assert!(g.board.graph.edge_corners[e as usize].contains(&(c as u16)));
    }
    let a = legal(&g)[0];
    g.apply(a).unwrap();
    let huts: Vec<usize> = legal(&g).iter().map(|a| match a { Action::Hut(c) => *c as usize, _ => panic!() }).collect();
    assert!(!huts.contains(&c));
    for n in listed(&g.board.graph.corner_corners[c]) {
        assert!(!huts.contains(&n), "a neighbour of a building isn't allowed");
    }
}

#[test]
fn rolling_pays_huts_one_and_lodges_two_except_under_the_squall() {
    let mut g = after_setup(3);
    let gr = &g.board.graph;
    // A producing hex with a building on it, not under the squall
    let h = (0..gr.hexes())
        .find(|&h| g.board.resource[h] != NOBODY && h != g.squall as usize && gr.hex_corners[h].iter().any(|&c| g.corner_owner[c as usize] != NOBODY))
        .unwrap();
    let c = *gr.hex_corners[h].iter().find(|&&c| g.corner_owner[c as usize] != NOBODY).unwrap() as usize;
    let p = g.corner_owner[c] as usize;
    let r = g.board.resource[h] as usize;
    let n = g.board.number[h];
    let count_here = |g: &Game| -> u8 {
        g.board.graph.hex_corners[h].iter().filter(|&&x| g.corner_owner[x as usize] == p as u8).map(|&x| g.corner_kind[x as usize]).sum()
    };
    let before = g.hands[p][r];
    let others: u8 = g.board.by_number[n as usize].iter().filter(|&&x| x as usize != h && x != g.squall && g.board.resource[x as usize] as usize == r)
        .map(|&x| g.board.graph.hex_corners[x as usize].iter().filter(|&&k| g.corner_owner[k as usize] == p as u8).map(|&k| g.corner_kind[k as usize]).sum::<u8>())
        .sum();
    g.produce(n);
    assert_eq!(g.hands[p][r], before + count_here(&g) + others);

    g.corner_kind[c] = LODGE;
    g.pieces[p][P_LODGE] -= 1;
    g.pieces[p][P_HUT] += 1;
    let before = g.hands[p][r];
    g.produce(n);
    assert_eq!(g.hands[p][r], before + count_here(&g) + others + 0, "a Lodge pays 2");

    g.squall = h as u16;
    let before = g.hands[p][r];
    g.produce(n);
    assert_eq!(g.hands[p][r], before + others, "nothing under the squall");
}

#[test]
fn a_seven_halves_big_hands_then_moves_the_squall_then_steals() {
    let mut g = after_setup(3);
    g.hands = [[3, 3, 3, 0, 0], [2, 2, 2, 1, 0], [5, 0, 0, 0, 3], [0; 5], [0; 5], [0; 5]];
    g.phase = Phase::PreRoll;
    g.rolled(7);
    assert_eq!(g.phase, Phase::Discard);
    assert_eq!(&g.discards[..3], &[4, 0, 4], "9 cards lose 4, 7 lose none, 8 lose 4");
    assert_eq!(g.to_act(), 0);
    for r in [0, 0, 0, 1] {
        g.apply(Action::Discard(r)).unwrap();
    }
    assert_eq!(g.to_act(), 2);
    assert!(g.apply(Action::Discard(1)).is_err(), "player 3 has no Stone");
    for _ in 0..4 {
        let a = legal(&g)[0];
        g.apply(a).unwrap();
    }
    assert_eq!(g.hands[0].iter().sum::<u8>(), 5);
    assert_eq!(g.hands[2].iter().sum::<u8>(), 4);
    assert_eq!(g.phase, Phase::MoveSquall { anywhere: false });

    // The squall moves to any other land hex
    assert_eq!(g.settings().squall_range, 0);
    let moves: Vec<u16> = legal(&g).iter().map(|a| match a { Action::Squall(h) => *h, _ => panic!() }).collect();
    let want: Vec<u16> = g.board.land_hexes.iter().copied().filter(|&h| h != g.squall).collect();
    assert_eq!(moves, want);

    // Onto a hex with another player's building: one card moves to the roller
    let target = moves.iter().copied().find(|&h| g.board.graph.hex_corners[h as usize].iter().any(|&c| {
        let o = g.corner_owner[c as usize];
        o != NOBODY && o != 0
    }));
    if let Some(h) = target {
        let total = |g: &Game| (0..3).map(|p| g.cards(p)).sum::<u32>();
        let (all, mine) = (total(&g), g.cards(0));
        g.apply(Action::Squall(h)).unwrap();
        if g.phase == Phase::Steal {
            let v = legal(&g)[0];
            g.apply(v).unwrap();
        }
        assert_eq!(total(&g), all);
        assert_eq!(g.cards(0), mine + 1);
        assert_eq!(g.phase, Phase::Main);
    }
}

#[test]
fn a_squall_range_limits_its_steps_after_a_seven_but_not_for_a_pilot() {
    let mut g = after_setup_with(2, "squall.range=1");
    g.hands = [[0; 5]; MAX_PLAYERS];
    g.phase = Phase::PreRoll;
    g.rolled(7);
    let moves = |g: &Game| -> Vec<u16> { legal(g).iter().filter_map(|a| match a { Action::Squall(h) => Some(*h), _ => None }).collect() };
    let near: Vec<u16> = g.board.land_hexes.iter().copied().filter(|&h| h != g.squall && g.board.hex_distance(h, g.squall) <= 1).collect();
    assert_eq!(moves(&g), near);
    assert!(near.len() < g.board.land_hexes.len() - 1, "the limit leaves some hexes out");
    g.phase = Phase::MoveSquall { anywhere: true };
    assert_eq!(moves(&g).len(), g.board.land_hexes.len() - 1, "a Pilot goes anywhere");
}

#[test]
fn small_hands_keep_everything_on_a_seven() {
    let mut g = after_setup(2);
    g.hands = [[1, 1, 1, 1, 1], [7, 0, 0, 0, 0], [0; 5], [0; 5], [0; 5], [0; 5]];
    g.phase = Phase::PreRoll;
    g.rolled(7);
    assert_eq!(g.phase, Phase::MoveSquall { anywhere: false });
}

#[test]
fn nobody_to_steal_from_returns_to_the_turn() {
    let mut g = after_setup(2);
    g.hands = [[0; 5]; MAX_PLAYERS];
    g.hands[0] = [1, 0, 0, 0, 0];
    g.phase = Phase::MoveSquall { anywhere: true };
    g.back = Phase::Main;
    let h = legal(&g).iter().find_map(|a| match a { Action::Squall(h) => Some(*h), _ => None }).unwrap();
    g.apply(Action::Squall(h)).unwrap();
    assert_eq!(g.phase, Phase::Main, "others have no cards, so nothing to steal");
    assert_eq!(g.hands[0], [1, 0, 0, 0, 0]);
}

#[test]
fn trading_rates_bank_and_markets() {
    let mut g = after_setup(2);
    // Clear the board's buildings so markets are controlled by the test
    g.corner_owner.fill(NOBODY);
    g.corner_kind.fill(0);
    assert_eq!(g.rate(0, 0), 4);
    let board = g.board.clone();
    let market_corner = |kind: u8| (0..board.graph.corners()).find(|&c| {
        let m = board.corner_market[c];
        m != NOBODY && board.market_kind[m as usize] == kind
    }).unwrap();
    let general = market_corner(ANY);
    g.corner_owner[general] = 0;
    g.corner_kind[general] = HUT;
    assert_eq!(g.rate(0, 0), 3);
    assert_eq!(g.rate(1, 0), 4, "only for the owner");
    let stone = market_corner(1);
    g.corner_owner[stone] = 0;
    g.corner_kind[stone] = HUT;
    assert_eq!((g.rate(0, 1), g.rate(0, 2)), (2, 3));
    g.current = 0;

    g.phase = Phase::Main;
    g.hands[0] = [0, 2, 0, 0, 0];
    g.apply(Action::Trade(1, 3)).unwrap();
    assert_eq!(g.hands[0], [0, 0, 0, 1, 0]);
    assert!(g.apply(Action::Trade(1, 3)).is_err());
}

#[test]
fn paths_join_your_network_but_not_through_another_players_building() {
    let mut g = after_setup(2);
    g.phase = Phase::Main;
    g.current = 0;
    g.hands[0] = [5, 0, 0, 5, 5];
    let board = g.board.clone();
    let gr = &board.graph;
    let edges = |g: &Game| -> Vec<usize> {
        legal(g).iter().filter_map(|a| match a { Action::Path(e) => Some(*e as usize), _ => None }).collect()
    };
    // Never out to sea: every offered Path touches land
    assert!(edges(&g).iter().all(|&e| board.edge_land[e]));
    for e in edges(&g) {
        let ok = gr.edge_corners[e].iter().any(|&c| {
            let c = c as usize;
            g.corner_owner[c] == 0 || (g.corner_owner[c] == NOBODY && listed(&gr.corner_edges[c]).any(|x| g.edge_owner[x] == 0))
        });
        assert!(ok, "every offered edge joins player 1's network");
    }
    // Put player 2's Hut at the far end of one of player 1's edges: edges beyond it are cut off
    let mine = (0..gr.edges()).find(|&e| g.edge_owner[e] == 0 && gr.edge_corners[e].iter().any(|&c| g.corner_owner[c as usize] == NOBODY)).unwrap();
    let end = gr.edge_corners[mine].iter().map(|&c| c as usize).find(|&c| g.corner_owner[c] == NOBODY).unwrap();
    g.corner_owner[end] = 1;
    g.corner_kind[end] = HUT;
    for e in listed(&gr.corner_edges[end]) {
        if e == mine || g.edge_owner[e] != NOBODY {
            continue;
        }
        let other = gr.edge_corners[e].iter().map(|&c| c as usize).find(|&c| c != end).unwrap();
        let reachable_otherwise = g.corner_owner[other] == 0 || listed(&gr.corner_edges[other]).any(|x| g.edge_owner[x] == 0);
        assert_eq!(edges(&g).contains(&e), reachable_otherwise, "edge {e} past another player's Hut");
    }
}

// A chain of `len` edges walking away from a corner, over land edges
fn chain(g: &Game, start: usize, len: usize) -> Vec<usize> {
    let gr = &g.board.graph;
    let mut seen = vec![start];
    let mut out = Vec::new();
    let mut c = start;
    while out.len() < len {
        let (k, n) = listed(&gr.corner_corners[c]).enumerate()
            .find(|&(k, n)| !seen.contains(&n) && g.board.edge_land[gr.corner_edges[c][k] as usize] && g.board.corner_land[n])
            .expect("room for a chain");
        out.push(gr.corner_edges[c][k] as usize);
        seen.push(n);
        c = n;
        let _ = k;
    }
    out
}

#[test]
fn longest_path_takes_the_longest_line_and_other_buildings_break_it() {
    let mut g = game(3);
    g.turn = 1;
    g.phase = Phase::Main;
    let start = (0..g.corner_owner.len()).find(|&c| g.board.corner_land[c] && listed(&g.board.graph.corner_hexes[c]).count() == 3).unwrap();
    let line = chain(&g, start, 6);
    for &e in &line[..4] {
        g.edge_owner[e] = 0;
        g.edge_kind[e] = PATH;
    }
    g.update_titles();
    assert_eq!((g.line[0], g.longest), (4, NOBODY), "4 is under the minimum of 5");
    g.edge_owner[line[4]] = 0;
    g.edge_kind[line[4]] = PATH;
    g.update_titles();
    assert_eq!((g.line[0], g.longest), (5, 0));
    // Another player's Hut on the corner between the 2nd and 3rd edges splits it into 2 and 3
    let gr = &g.board.graph;
    let [a, b] = gr.edge_corners[line[1]];
    let mid = if gr.edge_corners[line[2]].contains(&a) { a } else { b } as usize;
    g.corner_owner[mid] = 1;
    g.corner_kind[mid] = HUT;
    g.update_titles();
    assert_eq!((g.line[0], g.longest), (3, NOBODY));
}

#[test]
fn titles_change_hands_only_to_someone_strictly_ahead() {
    assert_eq!(title(NOBODY, &[5, 4, 3], 5), 0);
    assert_eq!(title(0, &[5, 5, 3], 5), 0, "a tie doesn't take it");
    assert_eq!(title(0, &[5, 6, 3], 5), 1);
    assert_eq!(title(0, &[4, 6, 6], 5), NOBODY, "the holder fell behind a tie: nobody");
    assert_eq!(title(0, &[4, 3, 2], 5), NOBODY, "nobody at the minimum");
    assert_eq!(title(NOBODY, &[6, 6, 2], 5), NOBODY);
}

// Plays one Pilot for the current player, moving the squall to the first hex offered
fn play_pilot(g: &mut Game) {
    let p = g.current as usize;
    g.held[p][Project::Pilot as usize] += 1;
    g.project_used = false;
    g.bought = [0; 5];
    g.apply(Action::Play(Project::Pilot)).unwrap();
    let h = legal(g)[0];
    g.apply(h).unwrap();
    if g.phase == Phase::Steal {
        let v = legal(g)[0];
        g.apply(v).unwrap();
    }
}

#[test]
fn most_pilots_goes_to_three_played_and_passes_only_to_more() {
    let mut g = after_setup(2);
    g.phase = Phase::Main;
    g.current = 0;
    let renown = g.renown(0);
    play_pilot(&mut g);
    play_pilot(&mut g);
    assert_eq!((g.pilots[0], g.most_pilots), (2, NOBODY), "2 isn't enough");
    play_pilot(&mut g);
    assert_eq!((g.pilots[0], g.most_pilots), (3, 0));
    assert_eq!(g.renown(0), renown + 2);
    // Player 2 draws level: player 1 keeps it; one more takes it
    g.current = 1;
    for _ in 0..3 {
        play_pilot(&mut g);
    }
    assert_eq!(g.most_pilots, 0, "a tie doesn't take it");
    play_pilot(&mut g);
    assert_eq!((g.pilots[1], g.most_pilots), (4, 1));
    // Pilots held but not played don't count
    g.held[0][Project::Pilot as usize] = 5;
    assert_eq!(g.most_pilots, 1);
}

#[test]
fn most_pilots_can_win_the_game_before_the_roll() {
    let mut g = after_setup(2);
    g.phase = Phase::PreRoll;
    g.current = 0;
    g.pilots[0] = 2;
    g.held[0][Project::Lighthouse as usize] = g.settings().win - 2 - g.renown(0) as u8;
    g.held[0][Project::Pilot as usize] = 1;
    g.apply(Action::Play(Project::Pilot)).unwrap();
    assert_eq!((g.over(), g.winner), (true, 0));
}

#[test]
fn a_lodge_replaces_a_hut_and_returns_it() {
    let mut g = after_setup(2);
    g.phase = Phase::Main;
    g.current = 0;
    g.hands[0] = LODGE_COST;
    let c = (0..g.corner_owner.len()).find(|&c| g.corner_owner[c] == 0).unwrap();
    let (huts, renown) = (g.pieces[0][P_HUT], g.renown(0));
    g.apply(Action::Lodge(c as u16)).unwrap();
    assert_eq!(g.corner_kind[c], LODGE);
    assert_eq!(g.pieces[0][P_HUT], huts + 1);
    assert_eq!(g.renown(0), renown + 1);
    assert_eq!(g.hands[0], [0; 5]);
}

#[test]
fn face_up_projects_refill_and_cant_be_played_the_turn_theyre_bought() {
    let mut g = after_setup_with(2, "projects.faceUp=true");
    g.phase = Phase::Main;
    g.current = 0;
    // A known row: Pilot, Harvest, Lighthouse
    g.row = vec![Project::Pilot as u8, Project::Harvest as u8, Project::Lighthouse as u8];
    let deck = g.deck.len();
    let next = *g.deck.last().unwrap();
    g.hands[0] = [0, 3, 3, 0, 3];
    let renown = g.renown(0);
    g.apply(Action::Buy(2)).unwrap();
    assert_eq!(g.renown(0), renown + 1, "a Lighthouse counts at once");
    assert_eq!(g.row, vec![Project::Pilot as u8, Project::Harvest as u8, next]);
    assert_eq!(g.deck.len(), deck - 1);
    g.apply(Action::Buy(1)).unwrap();
    assert!(!legal(&g).contains(&Action::Play(Project::Harvest)), "not on the turn it was bought");
    g.apply(Action::EndTurn).unwrap();
    // Player 2's turn, then back to player 1
    g.phase = Phase::Main;
    g.apply(Action::EndTurn).unwrap();
    assert!(legal(&g).contains(&Action::Play(Project::Harvest)), "playable before rolling next turn");
    g.held[0][Project::Pilot as usize] = 1;
    let copper = g.hands[0][3];
    g.apply(Action::Play(Project::Harvest)).unwrap();
    g.apply(Action::Take(3)).unwrap();
    g.apply(Action::Take(3)).unwrap();
    assert_eq!(g.hands[0][3], copper + 2);
    assert_eq!(g.phase, Phase::PreRoll, "back to before the roll");
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::Play(_))), "one project a turn");
}

#[test]
fn hidden_projects_are_drawn_unseen() {
    let s = Settings::parse("projects.faceUp=false").unwrap();
    let mut g = Game::new(&generate(&s, "hidden").unwrap(), &s, 2, 3);
    assert!(g.row.is_empty());
    let total = s.deck_pilot + s.deck_lighthouse + s.deck_road_crew + s.deck_harvest + s.deck_market_day;
    assert_eq!(g.deck.len(), total as usize);
    g.turn = 1;
    g.phase = Phase::Main;
    g.hands[0] = PROJECT_COST;
    let top = *g.deck.last().unwrap();
    assert!(legal(&g).contains(&Action::Buy(0)));
    g.apply(Action::Buy(0)).unwrap();
    assert_eq!(g.held[0][top as usize], 1);
}

#[test]
fn road_crew_places_two_free_paths() {
    let mut g = after_setup(2);
    g.phase = Phase::Main;
    g.current = 0;
    g.held[0][Project::RoadCrew as usize] = 1;
    g.hands[0] = [0; 5];
    let paths = g.pieces[0][P_PATH];
    g.apply(Action::Play(Project::RoadCrew)).unwrap();
    for _ in 0..2 {
        assert!(matches!(g.phase, Phase::FreeBuild { .. }));
        let list = legal(&g);
        assert!(list.iter().all(|a| matches!(a, Action::Path(_))), "only Paths");
        g.apply(list[0]).unwrap();
    }
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(paths - g.pieces[0][P_PATH], 2);
    assert_eq!(g.hands[0], [0; 5], "free");
}

#[test]
fn market_day_takes_every_card_of_one_resource_from_everyone_else() {
    let mut g = after_setup(3);
    g.phase = Phase::Main;
    g.current = 1;
    g.hands = [[1, 0, 3, 0, 0], [0, 0, 1, 2, 0], [2, 1, 4, 0, 0], [0; 5], [0; 5], [0; 5]];
    g.held[1][Project::MarketDay as usize] = 1;
    g.apply(Action::Play(Project::MarketDay)).unwrap();
    assert_eq!(g.phase, Phase::MarketDay);
    assert_eq!(legal(&g), (0..5).map(Action::Take).collect::<Vec<_>>(), "any of the 5, even one nobody has");
    g.apply(Action::Take(2)).unwrap();
    assert_eq!(g.hands[1], [0, 0, 8, 2, 0], "1 + 3 + 4 Grain");
    assert_eq!((g.hands[0], g.hands[2]), ([1, 0, 0, 0, 0], [2, 1, 0, 0, 0]));
    assert_eq!(&g.last_take[..3], &[3, 0, 4]);
    assert_eq!(g.phase, Phase::Main);
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::Play(_))), "one project a turn");
}

#[test]
fn you_only_win_on_your_own_turn() {
    let mut g = after_setup(2);
    g.phase = Phase::Main;
    g.current = 0;
    g.held[1][Project::Lighthouse as usize] = 10;
    g.hands[0] = [4, 0, 0, 0, 0];
    g.apply(Action::Trade(0, 1)).unwrap();
    assert!(!g.over(), "player 2 has 10+ Renown, but it's player 1's turn");
    g.apply(Action::EndTurn).unwrap();
    assert!(g.over());
    assert_eq!(g.winner, 1);
}

fn trading_game() -> Game {
    let mut g = after_setup(3);
    g.phase = Phase::Main;
    g.current = 0;
    g.hands = [[2, 0, 0, 0, 1], [0, 0, 0, 2, 0], [0, 0, 1, 0, 0], [0; 5], [0; 5], [0; 5]];
    g
}

#[test]
fn offers_must_be_real_trades_the_player_can_pay() {
    let g = trading_game();
    let one = |r: usize| { let mut h = [0u8; 5]; h[r] = 1; h };
    for (give, get, to, why) in [
        ([0; 5], one(3), 0b110, "both sides"),
        (one(0), [0; 5], 0b110, "both sides"),
        (one(0), one(0), 0b110, "both sides"),
        ([3, 0, 0, 0, 0], one(3), 0b110, "don't have"),
        (one(0), one(3), 0b001, "other players"),
        (one(0), one(3), 0, "other players"),
        (one(0), one(3), 0b1000, "other players"),
    ] {
        let err = g.offer_ok(&give, &get, to).unwrap_err();
        assert!(err.contains(why), "{give:?} {get:?} {to}: {err}");
    }
    assert!(g.offer_ok(&one(0), &one(3), 0b110).is_ok());
    let mut before = g.clone();
    before.phase = Phase::PreRoll;
    assert!(before.offer_ok(&one(0), &one(3), 0b110).unwrap_err().contains("after rolling"));
    let s = Settings::parse("trade.players=false").unwrap();
    let mut off = Game::new(&generate(&s, "rules").unwrap(), &s, 3, 7);
    off.phase = Phase::Main;
    off.turn = 1;
    off.hands[0] = [1, 0, 0, 0, 0];
    assert!(off.offer_ok(&one(0), &one(3), 0b110).unwrap_err().contains("off"), "trade.players=false turns it off");
    // A limit, when set: trade.offers=2 allows 2 a turn
    let s = Settings::parse("trade.offers=2").unwrap();
    let mut two = Game::new(&generate(&s, "rules").unwrap(), &s, 3, 7);
    two.phase = Phase::Main;
    two.turn = 1;
    two.hands[0] = [1, 0, 0, 0, 0];
    for _ in 0..2 {
        two.apply(Action::Offer { give: one(0), get: one(3), to: 0b110, open: false }).unwrap();
        two.apply(Action::Withdraw).unwrap();
    }
    assert!(two.offer_ok(&one(0), &one(3), 0b110).unwrap_err().contains("2 offers"));
    // No limit (the default): 20 offers in one turn are all fine
    let mut g = trading_game();
    for _ in 0..20 {
        g.apply(Action::Offer { give: one(0), get: one(3), to: 0b110, open: false }).unwrap();
        g.apply(Action::Withdraw).unwrap();
    }
    assert!(g.offer_ok(&one(0), &one(3), 0b110).is_ok());
}

#[test]
fn an_offer_is_answered_by_others_and_dealt_with_one_who_accepted() {
    let mut g = trading_game();
    let offer = Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 0, 1, 0], to: 0b110, open: false };
    assert!(g.apply_as(1, offer).is_err(), "only the player whose turn it is offers");
    g.apply_as(0, offer).unwrap();
    assert_eq!(g.offers_made, 1);
    let answers = |g: &Game, s: u8| { let mut v = Vec::new(); g.legal_for(s, &mut v); v };
    assert!(answers(&g, 1).contains(&Action::Answer(1, true)) && answers(&g, 1).contains(&Action::Answer(1, false)));
    assert_eq!(answers(&g, 2), vec![Action::Answer(2, false)], "player 3 has no Copper: they can only decline");
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::Deal(_))), "nobody has accepted yet");
    assert!(g.apply_as(2, Action::Answer(1, true)).is_err(), "nobody answers for someone else");
    g.apply_as(1, Action::Answer(1, true)).unwrap();
    assert!(g.apply_as(1, Action::Answer(1, false)).is_err(), "one answer each");
    g.apply_as(2, Action::Answer(2, false)).unwrap();
    assert!(legal(&g).contains(&Action::Deal(1)) && !legal(&g).contains(&Action::Deal(2)));
    g.apply(Action::Deal(1)).unwrap();
    assert_eq!(g.hands[0], [1, 0, 0, 1, 1]);
    assert_eq!(g.hands[1], [1, 0, 0, 1, 0]);
    assert!(g.offer.is_none());
}

#[test]
fn offers_are_limited_withdrawn_and_end_with_the_turn() {
    let mut g = trading_game();
    let offer = Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 1, 0, 0], to: 0b100, open: false };
    for _ in 0..3 {
        g.apply(offer).unwrap();
        g.apply(Action::Withdraw).unwrap();
        assert!(g.offer.is_none());
    }
    assert!(g.apply(offer).is_ok(), "no limit by default");
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.offers_made, 0);

    // An accepted offer can't be dealt once the offerer has spent the cards
    let mut g = trading_game();
    g.hands[0] = [4, 0, 0, 0, 0];
    g.apply(Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 0, 1, 0], to: 0b010, open: false }).unwrap();
    g.apply_as(1, Action::Answer(1, true)).unwrap();
    assert!(legal(&g).contains(&Action::Deal(1)));
    g.apply(Action::Trade(0, 2)).unwrap();
    assert!(!legal(&g).contains(&Action::Deal(1)), "the Timber went to the bank");
    g.apply(Action::EndTurn).unwrap();
    assert!(g.offer.is_none(), "the turn's end clears the offer");
}

#[test]
fn an_open_offer_goes_to_the_first_who_accepts_at_once() {
    let mut g = trading_game();
    g.hands[2] = [0, 0, 0, 1, 0];
    g.apply(Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 0, 1, 0], to: 0b110, open: true }).unwrap();
    g.apply_as(2, Action::Answer(2, true)).unwrap();
    assert!(g.offer.is_none(), "taken at once");
    assert_eq!(g.hands[0], [1, 0, 0, 1, 1]);
    assert_eq!(g.hands[2], [1, 0, 0, 0, 0]);
    assert!(!{ let mut v = Vec::new(); g.legal_for(1, &mut v); v }.iter().any(|a| matches!(a, Action::Answer(..))), "too late for player 2");

    // Accepting an open offer needs the offerer to still hold the cards
    let mut g = trading_game();
    g.hands[0] = [4, 0, 0, 0, 0];
    g.apply(Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 0, 1, 0], to: 0b010, open: true }).unwrap();
    g.apply(Action::Trade(0, 2)).unwrap();   // the Timber goes to the bank
    let mut v = Vec::new();
    g.legal_for(1, &mut v);
    assert_eq!(v, vec![Action::Answer(1, false)], "only declining is left");
}

#[test]
fn a_counter_offer_can_be_taken_by_the_offerer() {
    let mut g = trading_game();   // player 1 [2,0,0,0,1], player 2 [0,0,0,2,0]
    g.apply(Action::Offer { give: [2, 0, 0, 0, 0], get: [0, 0, 0, 2, 0], to: 0b010, open: false }).unwrap();
    // Player 2 wants Fish too, and gives only 1 Copper
    let counter = Action::Counter { seat: 1, give: [1, 0, 0, 0, 1], get: [0, 0, 0, 1, 0] };
    assert!(g.apply_as(2, counter).is_err(), "only the countering seat sends it");
    assert!(g.counter_ok(1, &[1, 0, 0, 0, 1], &[0, 0, 0, 3, 0]).unwrap_err().contains("don't have"));
    assert!(g.counter_ok(1, &[0, 0, 0, 0, 0], &[0, 0, 0, 1, 0]).unwrap_err().contains("both sides"));
    g.apply_as(1, counter).unwrap();
    let o = g.offer.unwrap();
    assert_eq!((o.answers[1], o.counters[1]), (COUNTERED, Some(([1, 0, 0, 0, 1], [0, 0, 0, 1, 0]))));
    assert!(legal(&g).contains(&Action::Accept(1)));
    assert!(!legal(&g).contains(&Action::Deal(1)), "a counter isn't an acceptance");
    let before: u32 = (0..3).map(|p| g.cards(p)).sum();
    g.apply(Action::Accept(1)).unwrap();
    assert_eq!(g.hands[0], [1, 0, 0, 1, 0]);
    assert_eq!(g.hands[1], [1, 0, 0, 1, 1]);
    assert_eq!((0..3).map(|p| g.cards(p)).sum::<u32>(), before);
    assert!(g.offer.is_none());

    // Someone not offered the trade can't counter it; nor after accepting
    let mut g = trading_game();
    g.apply(Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 0, 1, 0], to: 0b010, open: false }).unwrap();
    assert!(g.counter_ok(2, &[1, 0, 0, 0, 0], &[0, 0, 1, 0, 0]).unwrap_err().contains("only a player offered"));
    g.apply_as(1, Action::Answer(1, true)).unwrap();
    assert!(g.counter_ok(1, &[1, 0, 0, 0, 0], &[0, 0, 0, 1, 0]).unwrap_err().contains("already accepted"));
}

#[test]
fn bots_accept_trades_that_help_them_and_refuse_ones_that_dont() {
    let mut g = trading_game();
    let bot = Greedy::new(9);
    // Player 2 has 2 Timber, 2 Copper and no Grain: Grain (which both a Hut and a Lodge need) for one
    // of the Timber helps whatever its goal is
    g.hands[0] = [0, 0, 1, 0, 0];
    g.hands[1] = [2, 0, 0, 2, 0];
    g.offer = Some(Offer { give: [0, 0, 1, 0, 0], get: [1, 0, 0, 0, 0], to: 0b010, open: false, answers: [0; MAX_PLAYERS], counters: [None; MAX_PLAYERS] });
    assert!(bot.answer(&g, 1), "Grain it needs for a spare Timber");
    // Taking both Copper for nothing they need
    g.offer = Some(Offer { give: [0, 0, 0, 0, 0], get: [0, 0, 0, 2, 0], to: 0b010, open: false, answers: [0; MAX_PLAYERS], counters: [None; MAX_PLAYERS] });
    assert!(!bot.answer(&g, 1), "giving cards away for nothing");
    // Never when the offerer is about to win
    g.held[0][Project::Lighthouse as usize] = 10;
    g.offer = Some(Offer { give: [0, 0, 1, 0, 0], get: [1, 0, 0, 0, 0], to: 0b010, open: false, answers: [0; MAX_PLAYERS], counters: [None; MAX_PLAYERS] });
    assert!(!bot.answer(&g, 1), "the same trade, but the offerer is near winning");
}

#[test]
fn the_bank_holds_what_players_dont_and_pays_a_roll_in_full_or_not_at_all() {
    let mut g = after_setup(3);
    let cap = g.settings().bank_cards as u32;
    assert_eq!(cap, 19);
    g.hands = [[0; 5]; MAX_PLAYERS];
    assert!((0..5).all(|r| g.bank(r) == 19));
    g.hands[0][2] = 10;
    g.hands[1][2] = 8;
    assert_eq!(g.bank(2), 1, "19 Grain less the 18 held");

    // One Fields hex and its number, with the board cleared and the squall elsewhere: Huts placed
    // on it are the only ones owed Grain
    let board = g.board.clone();
    let fields = (0..board.resource.len()).find(|&h| board.resource[h] == 2).unwrap();
    let n = board.number[fields];
    g.corner_owner.fill(NOBODY);
    g.corner_kind.fill(0);
    g.squall = *board.land_hexes.iter().find(|&&h| h as usize != fields).unwrap();
    let [a, _, b, ..] = board.graph.hex_corners[fields];
    let hut = |g: &mut Game, c: u16, p: u8| {
        g.corner_owner[c as usize] = p;
        g.corner_kind[c as usize] = HUT;
    };

    // Owed to 2 players, 1 Grain left: nobody gets Grain
    hut(&mut g, a, 0);
    hut(&mut g, b, 1);
    let before = g.hands;
    g.produce(n);
    assert_eq!(g.short & (1 << 2), 1 << 2, "Grain was short");
    assert_eq!(g.hands, before, "nobody got Grain");

    // Owed to one player only (2 Grain for a Lodge, 1 left): they get the last card
    g.corner_owner[b as usize] = NOBODY;
    g.corner_kind[a as usize] = LODGE;
    g.produce(n);
    assert_eq!(g.short & (1 << 2), 1 << 2, "still short: 2 owed, 1 paid");
    assert_eq!(g.hands[0][2], before[0][2] + 1);
    assert_eq!(g.bank(2), 0);

    // Bank trades and Good Harvest only offer what the bank has
    let mut g = after_setup(3);
    g.phase = Phase::Main;
    g.current = 0;
    g.hands = [[4, 0, 0, 0, 0], [0, 0, 0, 0, 10], [0, 0, 0, 0, 10], [0; 5], [0; 5], [0; 5]];
    assert!(!legal(&g).contains(&Action::Trade(0, 4)), "no Fish left in the bank");
    assert!(legal(&g).contains(&Action::Trade(0, 1)));
    g.held[0][Project::Harvest as usize] = 1;
    g.apply(Action::Play(Project::Harvest)).unwrap();
    assert!(!legal(&g).contains(&Action::Take(4)));
    g.apply(Action::Take(1)).unwrap();
    g.apply(Action::Take(1)).unwrap();
    assert_eq!(g.hands[0][1], 2);

    // Unlimited (bank.cards=0): always pays
    let s = Settings::parse("bank.cards=0").unwrap();
    let g = Game::new(&generate(&s, "rules").unwrap(), &s, 2, 1);
    assert_eq!(g.bank(0), u32::MAX);
}

#[test]
fn a_round_limit_ends_the_game_with_the_most_renown_winning() {
    let s = Settings::parse("rounds=2").unwrap();
    let island = generate(&s, "rules").unwrap();
    let mut g = Game::new(&island, &s, 3, 7);
    let mut bot = Greedy::new(1);
    while g.turn == 0 {
        let list = legal(&g);
        let a = bot.choose(&g, &list);
        g.apply(a).unwrap();
    }
    assert_eq!(g.round(), 1);
    let end_turn = |g: &mut Game| {
        g.phase = Phase::Main;
        g.apply(Action::EndTurn).unwrap();
    };
    for _ in 0..5 {
        end_turn(&mut g);
        assert!(!g.over());
    }
    assert_eq!(g.round(), 2);
    g.held[1][Project::Lighthouse as usize] = 1;   // player 2 ahead by one
    end_turn(&mut g);
    assert!(g.over() && g.by_rounds);
    assert_eq!(g.winner, 1, "most Renown wins");

    // Level on Renown and Lodges: a draw
    let mut g = Game::new(&island, &s, 2, 7);
    let mut bot = Greedy::new(1);
    while g.turn == 0 {
        let list = legal(&g);
        let a = bot.choose(&g, &list);
        g.apply(a).unwrap();
    }
    for _ in 0..4 {
        end_turn(&mut g);
    }
    assert!(g.over() && g.by_rounds);
    assert_eq!(g.winner, NOBODY, "2 Huts each: a draw");
}

// A game after setup with this many players and that many players' defaults
fn after_setup_for(players: usize) -> Game {
    let s = Settings::for_players(players);
    setup_done(Game::new(&generate(&s, "rules").unwrap(), &s, players, 7))
}

#[test]
fn with_five_or_six_the_player_three_seats_on_takes_an_extra_build_phase() {
    for players in [5, 6] {
        let mut g = after_setup_for(players);
        assert!(g.pairs());
        assert_eq!((g.current, g.owner, g.turn), (0, 0, 1));
        g.rolled(8);
        g.apply(Action::EndTurn).unwrap();
        // Seat 4 (3 on from seat 1) builds, without rolling
        assert_eq!((g.current, g.owner, g.paired, g.phase, g.turn), (3, 0, true, Phase::Main, 1), "{players} players");
        let list = legal(&g);
        assert!(!list.contains(&Action::Roll) && list.contains(&Action::EndTurn));
        // Bank trades yes, offers to players no
        g.hands[3] = [4, 0, 0, 0, 0];
        assert!(legal(&g).contains(&Action::Trade(0, 1)));
        let offer = Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 1, 0, 0, 0], to: 1, open: false };
        assert!(g.apply(offer).unwrap_err().contains("extra build phase"));
        assert!(!g.offers_left());
        // A project may be played (one per phase); a Pilot moves the squall and steals for seat 4
        g.held[3][Project::Pilot as usize] = 1;
        assert!(legal(&g).contains(&Action::Play(Project::Pilot)));
        g.apply(Action::Play(Project::Pilot)).unwrap();
        let h = legal(&g)[0];
        g.apply(h).unwrap();
        if g.phase == Phase::Steal {
            let v = legal(&g)[0];
            g.apply(v).unwrap();
        }
        assert_eq!((g.phase, g.current), (Phase::Main, 3), "back to the extra build phase");
        // Then the next player's own turn
        g.apply(Action::EndTurn).unwrap();
        assert_eq!((g.current, g.owner, g.paired, g.phase, g.turn), (1, 1, false, Phase::PreRoll, 2));
        // The last seat's partner wraps round: 3 on from seat 6 (or 5) is seat 3 (or 2)
        g.owner = players as u8 - 1;
        g.current = g.owner;
        g.rolled(8);
        g.apply(Action::EndTurn).unwrap();
        assert_eq!(g.current, ((players - 1 + 3) % players) as u8);
    }
}

#[test]
fn no_extra_build_phase_with_four_players_or_when_turned_off() {
    let mut g = after_setup(4);
    assert!(!g.pairs());
    g.rolled(8);
    g.apply(Action::EndTurn).unwrap();
    assert_eq!((g.current, g.paired, g.phase), (1, false, Phase::PreRoll));
    let s = Settings::parse_for("turns.paired=false", 6).unwrap();
    let mut g = setup_done(Game::new(&generate(&s, "rules").unwrap(), &s, 6, 7));
    g.rolled(8);
    g.apply(Action::EndTurn).unwrap();
    assert_eq!((g.current, g.paired), (1, false));
}

#[test]
fn the_paired_player_can_win_in_the_extra_build_phase() {
    let mut g = after_setup_for(6);
    g.rolled(8);
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.current, 3);
    // Seat 4 holds hidden Lighthouses one short of winning, and builds a Lodge
    let c = (0..g.corner_owner.len()).find(|&c| g.corner_owner[c] == 3).unwrap();
    g.held[3][Project::Lighthouse as usize] = g.settings().win - 1 - g.renown(3) as u8;
    g.hands[3] = LODGE_COST;
    g.apply(Action::Lodge(c as u16)).unwrap();
    assert_eq!((g.over(), g.winner), (true, 3));
}

#[test]
fn six_player_bot_games_finish() {
    for players in [5, 6] {
        let s = Settings::for_players(players);
        let mut finished = 0;
        for seed in 0..60 {
            let out = play(&s, seed, players, 600).unwrap();
            finished += (out.winner != NOBODY) as u32;
        }
        assert!(finished >= 58, "{players} players: only {finished} of 60 bot games finished");
    }
}

#[test]
fn bot_games_finish_and_are_repeatable() {
    let s = Settings::default();
    for seed in 0..200 {
        let o = play(&s, seed, 2 + seed as usize % 3, 400).unwrap();
        assert_ne!(o.winner, NOBODY, "game {seed} didn't finish");
    }
    let a = play(&s, 5, 4, 400).unwrap();
    let b = play(&s, 5, 4, 400).unwrap();
    assert_eq!((a.winner, a.turns, a.actions), (b.winner, b.turns, b.actions));
}

#[test]
fn building_a_hut_in_the_middle_of_a_line_takes_longest_path_away() {
    let mut g = game(3);
    g.turn = 1;
    g.phase = Phase::Main;
    let start = (0..g.corner_owner.len()).find(|&c| g.board.corner_land[c] && listed(&g.board.graph.corner_hexes[c]).count() == 3).unwrap();
    let line = chain(&g, start, 6);
    for &e in &line[..5] {
        g.edge_owner[e] = 0;
        g.edge_kind[e] = PATH;
    }
    g.update_titles();
    assert_eq!(g.longest, 0);
    // Player 2 reaches the corner between the 2nd and 3rd edges with a Path of their own, then builds there
    let board = g.board.clone();
    let gr = &board.graph;
    let [a, b] = gr.edge_corners[line[1]];
    let mid = if gr.edge_corners[line[2]].contains(&a) { a } else { b } as usize;
    let side = listed(&gr.corner_edges[mid]).find(|&e| g.edge_owner[e] == NOBODY && board.edge_land[e]).expect("a free edge at the corner");
    g.edge_owner[side] = 1;
    g.edge_kind[side] = PATH;
    g.current = 1;
    g.hands[1] = HUT_COST;
    g.apply(Action::Hut(mid as u16)).unwrap();
    assert_eq!((g.line[0], g.longest), (3, NOBODY), "split into 2 and 3: under 5, so nobody holds it");
    // Player 1 can't build on past player 2's Hut either
    g.current = 0;
    g.hands[0] = PATH_COST;
    let beyond = listed(&gr.corner_edges[mid]).filter(|&e| e != line[1] && e != line[2] && e != side).collect::<Vec<_>>();
    let mut list = Vec::new();
    g.legal(&mut list);
    for e in beyond {
        let other = gr.edge_corners[e].iter().map(|&c| c as usize).find(|&c| c != mid).unwrap();
        if g.corner_owner[other] == NOBODY && !listed(&gr.corner_edges[other]).any(|x| g.edge_owner[x] == 0) {
            assert!(!list.contains(&Action::Path(e as u16)), "no Path past another player's Hut");
        }
    }
}
