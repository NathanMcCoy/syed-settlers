// The city expansion (city.rs)
use super::*;
use crate::island::generate;

fn city_game(players: usize) -> Game {
    let s = Settings::parse_for("city=true", players).unwrap();
    Game::new(&generate(&s, "city").unwrap(), &s, players, 3)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

// Play setup with the first legal choices
fn after_setup(players: usize) -> Game {
    let mut g = city_game(players);
    while g.turn == 0 {
        let a = legal(&g)[0];
        g.apply(a).unwrap();
    }
    g
}

// A corner of p's (Hut or Lodge)
fn building(g: &Game, p: u8, kind: u8) -> usize {
    (0..g.corner_owner.len()).find(|&c| g.corner_owner[c] == p && g.corner_kind[c] == kind).unwrap()
}

// A free corner by one of p's Paths (for a knight)
fn knight_spot(g: &Game, p: u8) -> usize {
    (0..g.corner_owner.len()).find(|&c| g.corner_owner[c] == NOBODY && !g.has_knight(c) && listed(&g.board.graph.corner_edges[c]).any(|e| g.edge_owner[e] == p)).unwrap()
}

fn main_turn(g: &mut Game, p: u8) {
    g.current = p;
    g.owner = p;
    g.phase = Phase::Main;
    g.pending.clear();
}

#[test]
fn setup_gives_a_lodge_second_13_to_win_no_projects_and_no_squall_yet() {
    let g = after_setup(4);
    let s = g.settings();
    assert_eq!((s.win, g.deck.len(), g.squall), (13, 0, NONE));
    assert!(!Title::MostPilots.active(s) && Title::LongestPath.active(s));
    for p in 0..4 {
        assert_eq!((g.huts(p), g.lodges(p)), (1, 1), "a Hut, then a Lodge");
        assert_eq!(g.renown(p), 3);
    }
    assert_eq!(g.cs.decks.iter().map(|d| d.len()).sum::<usize>(), 54);
    assert!(Settings::parse_for("city=true;sea=true;sea.scenario=2", 4).unwrap_err().contains("combines with sea scenarios"));
    assert_eq!(Settings::parse_for("city=true;sea=true;sea.scenario=1", 4).unwrap().win, 16, "a sea scenario's target + 2");
}

#[test]
fn lodges_make_commodities_on_forest_paddy_and_quarry() {
    // A Lodge of seat 1's by each kind of hex: (terrain, resource, commodity or none)
    for (terrain, r, goods) in [
        (Terrain::Forest, 0, Some(PAPER)),
        (Terrain::Paddy, 4, Some(CLOTH)),
        (Terrain::Quarry, 1, Some(COIN)),
        (Terrain::Fields, 2, None),
        (Terrain::Mine, 3, None),
    ] {
        let mut g = city_game(3);
        let h = (0..g.board.terrain.len()).find(|&h| g.board.terrain[h] == terrain).unwrap();
        let n = g.board.number[h];
        // A corner of that hex whose other hexes don't share its number
        let c = g.board.graph.hex_corners[h].iter().map(|&c| c as usize)
            .find(|&c| listed(&g.board.graph.corner_hexes[c]).all(|o| o == h || g.board.number[o] != n)).unwrap();
        g.corner_owner[c] = 0;
        g.corner_kind[c] = LODGE;
        g.produce(n);
        let mut expect = [0u8; 8];
        match goods {
            Some(k) => (expect[r], expect[k]) = (1, 1),
            None => expect[r] = 2,
        }
        assert_eq!(g.hands[0], expect, "{terrain:?}");
        // A Hut there: 1 resource, no commodity
        g.hands[0] = [0; 8];
        g.corner_kind[c] = HUT;
        g.produce(n);
        let mut hut = [0u8; 8];
        hut[r] = 1;
        assert_eq!(g.hands[0], hut, "{terrain:?} Hut");
    }
}

#[test]
fn improvements_climb_and_the_first_to_4_takes_the_metropolis_and_5_keeps_it() {
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    g.hands[0] = [0, 0, 0, 0, 0, 20, 0, 0];
    for level in 1..=3 {
        assert!(legal(&g).contains(&Action::Improve(0)));
        g.apply(Action::Improve(0)).unwrap();
        assert_eq!(g.cs.levels[0][0], level);
    }
    assert_eq!(g.hands[0][PAPER], 20 - 6, "1 + 2 + 3 Paper");
    let r = g.renown(0);
    g.apply(Action::Improve(0)).unwrap();
    assert_eq!((g.cs.metro_owner[0], g.renown(0)), (0, r + 2), "level 4: the metropolis, +2 Renown");
    let lodge = building(&g, 0, LODGE);
    assert!(g.is_metro(lodge));
    // Seat 2 reaching 4 doesn't take it; reaching 5 first does
    g.cs.levels[1][0] = 4;
    g.cs.metro_owner[0] = 0;
    main_turn(&mut g, 1);
    g.hands[1] = [0, 0, 0, 0, 0, 5, 0, 0];
    g.apply(Action::Improve(0)).unwrap();
    assert_eq!(g.cs.metro_owner[0], 1, "level 5 first takes it");
    assert!(!g.is_metro(lodge));
    // Without a Lodge, no improvements
    let mut h = after_setup(3);
    main_turn(&mut h, 0);
    let c = building(&h, 0, LODGE);
    h.corner_kind[c] = HUT;
    h.pieces[0][P_LODGE] += 1;
    h.hands[0] = [0, 0, 0, 0, 0, 5, 5, 5];
    assert!(!legal(&h).iter().any(|a| matches!(a, Action::Improve(_))));
}

#[test]
fn crane_trade_level_3_and_trade_fleet_change_costs_and_rates() {
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    g.hands[0] = [0, 0, 0, 0, 0, 0, 1, 0];
    g.cs.levels[0][1] = 1;
    assert!(!legal(&g).contains(&Action::Improve(1)), "level 2 costs 2 Cloth");
    g.cs.progress[0] = vec![CRANE];
    g.apply(Action::PlayCard(CRANE)).unwrap();
    g.apply(Action::Improve(1)).unwrap();
    assert_eq!((g.cs.levels[0][1], g.hands[0][CLOTH]), (2, 0));
    assert!(g.rate(0, COIN) >= 3, "4, or 3 with a general market");
    g.cs.levels[0][1] = 3;
    assert_eq!(g.rate(0, COIN), 2, "trade level 3: commodities 2:1");
    assert_eq!(g.rate(0, 2), g.rate(0, 2).min(4));
    g.cs.progress[0] = vec![TRADE_FLEET];
    g.apply(Action::PlayCard(TRADE_FLEET)).unwrap();
    g.apply(Action::Choose(2)).unwrap();
    assert_eq!(g.rate(0, 2), 2, "Trade Fleet: Grain 2:1 this turn");
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.cs.fleet, NOBODY);
}

#[test]
fn walls_raise_the_hand_limit_up_to_three() {
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    let limit = g.hand_limit(0);
    g.hands[0] = [0, 0, 0, 4, 0, 0, 0, 0];
    let c = building(&g, 0, LODGE);
    g.apply(Action::Wall(c as u16)).unwrap();
    assert_eq!(g.hand_limit(0), limit + 2);
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::Wall(_))), "one wall per Lodge");
    // A 7: holding 9 with a wall keeps them all
    g.hands[0] = [9, 0, 0, 0, 0, 0, 0, 0];
    g.current = 1;
    g.rolled(7);
    assert_eq!(g.cards(0), 9);
}

#[test]
fn knights_recruit_activate_move_and_block() {
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    g.hands[0] = [0, 2, 2, 0, 2, 0, 0, 0];
    let spot = knight_spot(&g, 0);
    g.apply(Action::Recruit(spot as u16)).unwrap();
    assert_eq!((g.cs.knight_owner[spot], g.cs.knight_level[spot], g.cs.knight_active[spot]), (0, 1, false));
    // A knight on an open spot blocks it for Huts (and for everyone)
    let open = (0..g.corner_owner.len()).find(|&c| g.spaced(c)).unwrap();
    g.place_knight(1, open, 1, false);
    assert!(!g.spaced(open), "a knight blocks a Hut spot");
    g.lift_knight(open);
    g.apply(Action::Activate(spot as u16)).unwrap();
    assert!(g.cs.knight_active[spot]);
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::KnightMove(..) | Action::Chase(_))), "activated this turn: no action yet");
    assert_eq!(g.defence(0), 1);
    // Promoting to mighty needs Politics 3
    g.apply(Action::Promote(spot as u16)).unwrap();
    assert_eq!(g.cs.knight_level[spot], 2);
    assert!(!legal(&g).contains(&Action::Promote(spot as u16)), "once a turn");
    g.apply(Action::EndTurn).unwrap();
    main_turn(&mut g, 0);
    g.hands[0] = [0, 1, 0, 0, 1, 0, 0, 0];
    assert!(!legal(&g).contains(&Action::Promote(spot as u16)), "mighty needs Politics level 3");
    g.cs.levels[0][2] = 3;
    assert!(legal(&g).contains(&Action::Promote(spot as u16)));
    // Another player's knight stops seat 1's Path past it
    let mut h = after_setup(3);
    let spot = knight_spot(&h, 0);
    h.cs.knight_owner[spot] = 1;
    h.cs.knight_level[spot] = 1;
    for e in listed(&h.board.graph.corner_edges[spot]) {
        if h.edge_owner[e] == NOBODY {
            assert!(!h.edge_connected(0, e) || h.board.graph.edge_corners[e].iter().any(|&c| c as usize != spot && h.corner_owner[c as usize] == 0));
        }
    }
}

#[test]
fn a_stronger_knight_displaces_a_weaker_one_which_moves_or_goes_home() {
    let mut g = after_setup(3);
    // Seat 1's knight and seat 2's weaker knight, both reachable along seat 1's Paths
    let a = knight_spot(&g, 0);
    g.place_knight(0, a, 2, true);
    let b = knight_spot(&g, 0);
    g.place_knight(1, b, 1, true);
    let reach = g.knight_reach(0, a);
    if !reach.1.contains(&b) {
        return; // this island's Paths don't link the two spots
    }
    main_turn(&mut g, 0);
    assert!(legal(&g).contains(&Action::Displace(a as u16, b as u16)));
    g.apply(Action::Displace(a as u16, b as u16)).unwrap();
    assert_eq!((g.cs.knight_owner[b], g.cs.knight_active[b]), (0, false));
    assert!(g.cs.knight_owner.iter().filter(|&&o| o == 1).count() <= 1);
}

#[test]
fn the_raiders_pillage_the_weakest_or_reward_the_best_defender() {
    // No knights at all: every player is lowest and loses a Lodge
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    g.cs.raiders = g.settings().city_raiders - 1;
    let decisions = g.raid();
    assert_eq!(decisions.len(), 3);
    assert!(decisions.iter().all(|d| d.decision == Decision::Pillage));
    assert!(g.cs.landed && g.squall != NONE, "the squall arrives at the first landing");
    assert_eq!(g.cs.last_raid, [3, 0, 1]);
    let c = building(&g, 0, LODGE);
    g.cs.walls[c] = true;
    g.pillage(0, c);
    assert_eq!((g.corner_kind[c], g.cs.walls[c], g.lodges(0)), (HUT, false, 0));
    // Strong enough: the single best defender gets a Renown token; knights go inactive
    let mut g = after_setup(3);
    let spot = knight_spot(&g, 1);
    g.place_knight(1, spot, 3, true);
    let r = g.renown(1);
    assert!(g.raid().is_empty());
    assert_eq!((g.renown(1), g.cs.knight_active[spot]), (r + 1, false));
    // Tied best: each draws a card of their choice
    let mut g = after_setup(3);
    let (a, b) = (knight_spot(&g, 0), knight_spot(&g, 1));
    g.place_knight(0, a, 2, true);
    g.place_knight(1, b, 2, true);
    let d = g.raid();
    assert_eq!(d.iter().map(|d| (d.seat, d.decision)).collect::<Vec<_>>(), vec![(0, Decision::DrawCard), (1, Decision::DrawCard)]);
    // A metropolis can't be pillaged
    let mut g = after_setup(3);
    let c = building(&g, 0, LODGE);
    g.cs.metro[0] = c as u16;
    g.cs.metro_owner[0] = 0;
    assert!(!g.pillageable(0));
}

#[test]
fn before_the_first_landing_a_seven_only_discards() {
    let mut g = after_setup(3);
    g.phase = Phase::PreRoll;
    g.hands[1] = [9, 0, 0, 0, 0, 0, 0, 0];
    g.rolled(7);
    assert_eq!(g.deciding().unwrap(), Pending { seat: 1, decision: Decision::Discard { left: 4 } });
    for _ in 0..4 {
        g.apply(Action::Discard(0)).unwrap();
    }
    assert_eq!(g.phase, Phase::Main, "no squall to move");
}

#[test]
fn progress_cards_come_with_the_gate_and_the_red_die_and_the_hand_holds_4() {
    let mut g = after_setup(3);
    g.cs.levels[1][2] = 1; // politics level 1: red die 1 or 2
    g.cs.decks[2].push(CHARTER);
    assert!(g.draw_progress(1, 2).is_none());
    assert_eq!((g.cs.card_renown[1], g.renown(1)), (1, 4), "a Renown card is shown at once");
    g.cs.progress[1] = vec![RALLY, SPY, WEDDING, SABOTAGE];
    g.cs.decks[2].push(TAXES);
    assert_eq!(g.draw_progress(1, 2), Some(Pending { seat: 1, decision: Decision::DropProgress }), "off their turn: drop to 4");
    // On your own turn: by the end of it
    main_turn(&mut g, 1);
    let l = legal(&g);
    assert!(!l.contains(&Action::EndTurn) && l.contains(&Action::DropCard(TAXES)));
    g.apply(Action::DropCard(TAXES)).unwrap();
    assert!(legal(&g).contains(&Action::EndTurn));
    assert_eq!(*g.cs.decks[2].first().unwrap(), TAXES, "dropped cards go under their deck");
}

#[test]
fn fixed_roll_sets_the_production_dice() {
    let mut g = after_setup(3);
    g.phase = Phase::PreRoll;
    g.cs.progress[0] = vec![FIXED_ROLL];
    assert!(legal(&g).contains(&Action::FixedRoll(2, 4)));
    g.apply(Action::FixedRoll(2, 4)).unwrap();
    assert_eq!((g.last_dice, g.last_roll), ([2, 4], 6));
    assert!(g.cs.progress[0].is_empty());
}

#[test]
fn trade_and_politics_cards_take_from_the_others() {
    // Monopoly: 2 of a resource from each (or their 1); Goods Monopoly: 1 commodity each
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    g.hands = [[0; 8]; MAX_PLAYERS];
    g.hands[1] = [3, 0, 0, 0, 0, 0, 2, 0];
    g.hands[2] = [1, 0, 0, 0, 0, 0, 0, 0];
    g.cs.progress[0] = vec![MONOPOLY, GOODS_MONOPOLY];
    g.apply(Action::PlayCard(MONOPOLY)).unwrap();
    g.apply(Action::Choose(0)).unwrap();
    assert_eq!((g.hands[0][0], g.hands[1][0], g.hands[2][0]), (3, 1, 0));
    g.apply(Action::PlayCard(GOODS_MONOPOLY)).unwrap();
    g.apply(Action::Choose(CLOTH as u8)).unwrap();
    assert_eq!((g.hands[0][CLOTH], g.hands[1][CLOTH]), (1, 1));
    // Wedding: everyone with more Renown gives 2 cards of their choice
    g.cs.card_renown[2] = 5;
    g.hands[2] = [0, 4, 0, 0, 0, 0, 0, 0];
    g.cs.progress[0] = vec![WEDDING];
    g.apply(Action::PlayCard(WEDDING)).unwrap();
    assert_eq!(g.to_act(), 2);
    g.apply(Action::Gift(0, 1)).unwrap();
    g.apply(Action::Gift(0, 1)).unwrap();
    assert_eq!((g.hands[0][1], g.phase), (2, Phase::Main));
    // Sabotage: everyone with as much Renown or more discards half
    g.hands[2] = [6, 0, 0, 0, 0, 0, 0, 0];
    g.cs.progress[0] = vec![SABOTAGE];
    g.apply(Action::PlayCard(SABOTAGE)).unwrap();
    while g.phase == Phase::Decide {
        let a = legal(&g)[0];
        g.apply(a).unwrap();
    }
    assert_eq!(g.hands[2][0], 3);
}

#[test]
fn merchant_spy_harbour_and_diplomacy() {
    let mut g = after_setup(3);
    main_turn(&mut g, 0);
    // Merchant: 1 Renown, its hex's resource 2:1
    g.cs.progress[0] = vec![MERCHANT];
    let r = g.renown(0);
    g.apply(Action::PlayCard(MERCHANT)).unwrap();
    let a = legal(&g)[0];
    let Action::Merchant(h) = a else { panic!("{a:?}") };
    g.apply(a).unwrap();
    assert_eq!(g.renown(0), r + 1);
    assert_eq!(g.rate(0, g.board.resource[h as usize] as usize), 2);
    // Spy: take one of a player's progress cards (seat 1 sees them while deciding)
    g.cs.progress[0] = vec![SPY];
    g.cs.progress[2] = vec![RALLY, CHARTER];
    g.apply(Action::PlayCard(SPY)).unwrap();
    g.apply(Action::Choose(2)).unwrap();
    assert!(crate::protocol::view_json(&g, 0).contains("\"spied\":[\"rally\",\"charter\"]"));
    assert_eq!(legal(&g), vec![Action::TakeProgress(2, RALLY), Action::Skip], "never a Renown card");
    g.apply(Action::TakeProgress(2, RALLY)).unwrap();
    assert_eq!(g.cs.progress[0], vec![RALLY]);
    // Harbour Deal: a resource to each player with commodities, a commodity back
    g.hands = [[0; 8]; MAX_PLAYERS];
    g.hands[0] = [2, 0, 0, 0, 0, 0, 0, 0];
    g.hands[1] = [0, 0, 0, 0, 0, 0, 0, 1];
    g.cs.progress[0] = vec![HARBOUR];
    g.apply(Action::PlayCard(HARBOUR)).unwrap();
    assert_eq!(legal(&g), vec![Action::Gift(1, 0), Action::Skip], "only players with commodities");
    g.apply(Action::Gift(1, 0)).unwrap();
    assert_eq!(g.to_act(), 1);
    g.apply(Action::Gift(0, COIN as u8)).unwrap();
    assert_eq!((g.hands[0], g.hands[1], g.phase), ([1, 0, 0, 0, 0, 0, 0, 1], [1, 0, 0, 0, 0, 0, 0, 0], Phase::Main));
    // Diplomacy on your own open Path: one free Path after
    let paths = g.pieces[0][P_PATH];
    g.cs.progress[0] = vec![DIPLOMACY];
    g.apply(Action::PlayCard(DIPLOMACY)).unwrap();
    let own = legal(&g).into_iter().find(|a| matches!(a, Action::RemovePath(e) if g.edge_owner[*e as usize] == 0));
    if let Some(a) = own {
        g.apply(a).unwrap();
        assert_eq!(g.pieces[0][P_PATH], paths + 1);
        assert!(matches!(g.phase, Phase::FreeBuild { .. } | Phase::Main));
    }
}

#[test]
fn treason_and_intrigue_move_other_players_knights() {
    let mut g = after_setup(3);
    let spot = knight_spot(&g, 1);
    g.place_knight(1, spot, 2, true);
    main_turn(&mut g, 0);
    g.cs.progress[0] = vec![TREASON];
    g.apply(Action::PlayCard(TREASON)).unwrap();
    g.apply(Action::Choose(1)).unwrap();
    assert_eq!(g.to_act(), 1);
    g.apply(Action::RemoveKnight(spot as u16)).unwrap();
    assert!(!g.has_knight(spot));
    // Seat 1 may place a knight no stronger, with the same status
    let l = legal(&g);
    assert!(l.contains(&Action::Skip));
    let Some(&Action::Recruit(c)) = l.iter().find(|a| matches!(a, Action::Recruit(_))) else { panic!("{l:?}") };
    g.apply(Action::Recruit(c)).unwrap();
    assert_eq!((g.cs.knight_owner[c as usize], g.cs.knight_level[c as usize], g.cs.knight_active[c as usize]), (0, 2, true));
    assert_eq!(g.phase, Phase::Main);
}

#[test]
fn city_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for (text, counts) in [("city=true", &[2usize, 3, 4, 6][..]), ("city=true;sea=true;sea.scenario=1", &[3, 4]), ("city=true;sea=true;sea.scenario=4", &[3]), ("city=true;sea=true;sea.scenario=9", &[4])] {
        for &players in counts {
            let s = Settings::parse_for(text, players).unwrap();
            for seed in 0..12 {
                play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
                let o = play_with(&s, seed, players, 900, &[BotKind::Greedy]).unwrap();
                assert_ne!(o.winner, NOBODY, "{text}, {players} players, game {seed} didn't finish");
            }
        }
    }
}
