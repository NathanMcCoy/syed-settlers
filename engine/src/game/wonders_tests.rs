// The sea scenario Wonders
use super::*;
use crate::island::generate;

fn wonder_game() -> Game {
    let s = Settings::parse_for("sea=true;sea.scenario=8", 3).unwrap();
    Game::new(&generate(&s, "wonders").unwrap(), &s, 3, 7)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

#[test]
fn markers_no_pirate_and_no_setup_on_marked_corners() {
    let g = wonder_game();
    let count = |k: u8| g.board.corner_marker.iter().filter(|&&m| m == k).count();
    assert_eq!((count(WALL), count(BRIDGE)), (5, 2));
    assert!(count(NO_SETUP) >= 2);
    assert_eq!(g.pirate, NONE, "no pirate in Wonders");
    assert!(legal(&g).iter().all(|a| matches!(a, Action::Hut(c) if g.board.corner_marker[*c as usize] == 0)));
}

#[test]
fn claim_a_wonder_whose_requirement_you_meet_then_build_its_levels() {
    let mut g = wonder_game();
    g.turn = 1;
    g.phase = Phase::Main;
    g.current = 0;
    let wall = (0..g.corner_owner.len()).find(|&c| g.board.corner_marker[c] == WALL).unwrap();
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::ClaimWonder(_))), "nothing met yet");
    g.corner_owner[wall] = 0;
    g.corner_kind[wall] = HUT;
    g.pieces[0][P_HUT] -= 1;
    assert!(g.meets(0, Wonder::GreatWall) && !g.meets(0, Wonder::GrandTheater));
    g.apply(Action::ClaimWonder(Wonder::GreatWall as u8)).unwrap();
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::ClaimWonder(_))), "one wonder each");
    // Another player can't claim it now
    g.corner_owner[wall] = 1;
    g.current = 1;
    assert!(!legal(&g).contains(&Action::ClaimWonder(Wonder::GreatWall as u8)));
    g.corner_owner[wall] = 0;
    g.current = 0;
    // Each level costs 3 Timber + Fish + Grain; four levels win
    let cost = Wonder::GreatWall.cost();
    assert_eq!(cost, [3, 0, 1, 0, 1]);
    for level in 1..=4 {
        g.hands[0] = cards_of(cost);
        g.apply(Action::BuildWonder).unwrap();
        assert_eq!(g.wonder_level[0], level);
        assert_eq!(g.over(), level == 4, "a finished wonder wins");
    }
}

#[test]
fn ten_renown_and_the_highest_level_wins_but_renown_alone_does_not() {
    let mut g = wonder_game();
    g.turn = 1;
    g.held[0][Project::Lighthouse as usize] = 12;
    assert!(!g.has_won(0), "Renown alone doesn't win in Wonders");
    g.wonder_of[0] = Wonder::GrandTheater as u8;
    g.wonder_level[0] = 1;
    g.wonder_level[1] = 1;
    assert!(!g.has_won(0), "level with someone else");
    g.wonder_level[0] = 2;
    assert!(g.has_won(0));
}

#[test]
fn wonder_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 4, 6] {
        let s = Settings::parse_for("sea=true;sea.scenario=8", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 600, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
