// The sea scenario The Forgotten Tribe
use super::*;
use crate::island::generate;

fn tribe_game() -> Game {
    let s = Settings::parse_for("sea=true;sea.scenario=5", 3).unwrap();
    Game::new(&generate(&s, "tribe").unwrap(), &s, 3, 7)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

#[test]
fn prizes_wait_on_sea_edges_and_ships_take_them() {
    let mut g = tribe_game();
    assert_eq!(g.settings().win, 13);
    let count = |g: &Game, k: u8| g.prizes.iter().filter(|&&x| x == k).count();
    assert_eq!((count(&g, PRIZE_RENOWN), count(&g, PRIZE_PROJECT)), (8, 4));
    assert_eq!(g.prizes.iter().filter(|&&x| x >= PRIZE_MARKET).count(), 3);
    g.turn = 1;
    g.phase = Phase::Main;
    g.current = 0;
    // A ship onto a Renown prize: a Renown token
    let e = (0..g.prizes.len()).find(|&e| g.prizes[e] == PRIZE_RENOWN).unwrap();
    g.take_prize(0, e);
    assert_eq!((g.tokens_of(0, Token::Renown), g.prizes[e]), (1, 0));
    // A project prize: counts as bought this turn
    let e = (0..g.prizes.len()).find(|&e| g.prizes[e] == PRIZE_PROJECT).unwrap();
    let top = *g.deck.last().unwrap() as usize;
    g.take_prize(0, e);
    assert_eq!((g.held[0][top], g.bought[top]), (1, 1));
    // A market prize: held until placed by one of the player's coastal buildings
    let e = (0..g.prizes.len()).find(|&e| g.prizes[e] >= PRIZE_MARKET).unwrap();
    let kind = g.prizes[e] - PRIZE_MARKET;
    g.take_prize(0, e);
    assert_eq!(g.held_markets[0], vec![kind]);
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::PlaceMarket(_))), "no coastal building yet");
    let gr = &g.board.graph;
    let c = (0..gr.corners()).find(|&c| {
        g.spaced(c) && g.board.corner_market[c] == NOBODY && listed(&gr.corner_corners[c]).all(|n| g.board.corner_market[n] == NOBODY)
            && listed(&gr.corner_hexes[c]).any(|h| g.board.terrain[h] == Terrain::Sea)
    }).unwrap();
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    let spot = legal(&g).into_iter().find(|a| matches!(a, Action::PlaceMarket(_))).expect("a spot by the new Hut");
    let markets = g.board.markets.len();
    g.apply(spot).unwrap();
    assert_eq!(g.board.markets.len(), markets + 1);
    assert_eq!(g.board.corner_market[c] != NOBODY, true, "the Hut is on the new market");
    assert!(g.held_markets[0].is_empty());
}

#[test]
fn huts_and_the_squall_only_by_numbered_hexes() {
    let g = tribe_game();
    for a in legal(&g) {
        let Action::Hut(c) = a else { panic!() };
        assert!(listed(&g.board.graph.corner_hexes[c as usize]).any(|h| g.board.number[h] > 0));
    }
    let mut g = g;
    g.turn = 1;
    g.phase = Phase::MoveSquall { anywhere: true };
    g.back = Phase::Main;
    assert!(legal(&g).iter().all(|a| !matches!(a, Action::Squall(h) if g.board.number[*h as usize] == 0)));
}

#[test]
fn tribe_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 4, 6] {
        let s = Settings::parse_for("sea=true;sea.scenario=5", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 400, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
