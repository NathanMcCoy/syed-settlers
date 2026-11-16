// The two-player variant (the variants box)
use super::*;
use crate::island::generate;

fn two_game() -> Game {
    let s = Settings::parse_for("variant.two=true", 2).unwrap();
    Game::new(&generate(&s, "two").unwrap(), &s, 2, 7)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

#[test]
fn only_for_two_players() {
    assert!(Settings::parse_for("variant.two=true", 3).unwrap_err().contains("2 players"));
}

#[test]
fn two_neutral_huts_start_on_the_island() {
    let g = two_game();
    for n in NEUTRAL {
        assert_eq!((0..g.corner_owner.len()).filter(|&c| g.corner_owner[c] == n).count(), 1);
    }
}

#[test]
fn building_a_path_or_a_hut_owes_a_neutral_one() {
    let mut g = two_game();
    g.turn = 1;
    g.phase = Phase::Main;
    // Seat 1's Hut and a Path off it
    let c = (0..g.corner_owner.len()).find(|&c| g.spaced(c) && g.board.corner_region[c] == 0).unwrap();
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    g.hands[0] = cards_of(PATH_COST);
    let e = legal(&g).iter().find_map(|a| if let Action::Path(e) = a { Some(*e) } else { None }).unwrap();
    g.apply(Action::Path(e)).unwrap();
    assert_eq!(g.deciding().unwrap(), Pending { seat: 0, decision: Decision::Neutral { hut: false } });
    let list = legal(&g);
    assert!(!list.is_empty() && list.iter().all(|a| matches!(a, Action::NeutralPath(..))));
    let Action::NeutralPath(n, ne) = list[0] else { unreachable!() };
    g.apply(list[0]).unwrap();
    assert_eq!((g.edge_owner[ne as usize], g.phase), (n, Phase::Main));
    // Neutral Huts get no resources
    g.hands = [[0; 8]; MAX_PLAYERS];
    for total in 2..=12 {
        if total != 7 {
            g.produce(total);
        }
    }
    assert!(g.hands[2..4].iter().all(|h| h.iter().all(|&x| x == 0)));
}

#[test]
fn two_production_rolls_the_second_a_different_number() {
    let mut g = two_game();
    g.turn = 1;
    g.phase = Phase::PreRoll;
    g.hands = [[0; 8]; MAX_PLAYERS];
    for _ in 0..20 {
        g.phase = Phase::PreRoll;
        g.rolled_once = false;
        g.pending.clear();
        g.apply(Action::Roll).unwrap();
        let first = g.last_roll;
        // A 7: discards (none here) and the squall first
        while g.phase != Phase::SecondRoll {
            let a = legal(&g)[0];
            g.apply(a).unwrap();
        }
        assert_eq!(legal(&g), vec![Action::Roll]);
        g.apply(Action::Roll).unwrap();
        assert_ne!(g.last_roll, first, "a different number");
        while g.phase != Phase::Main {
            let a = legal(&g)[0];
            g.apply(a).unwrap();
        }
    }
}

#[test]
fn trade_tokens_buy_a_forced_trade_or_send_the_squall_home() {
    let mut g = two_game();
    g.turn = 1;
    g.phase = Phase::Main;
    g.hands[0] = [2, 0, 0, 0, 0, 0, 0, 0];
    g.hands[1] = [0, 3, 0, 0, 0, 0, 0, 0];
    assert!(!legal(&g).contains(&Action::ForceTrade), "no tokens yet");
    g.pilots[0] = 1;
    g.apply(Action::CashPilot).unwrap();
    assert_eq!((g.tokens_of(0, Token::Trade), g.pilots[0]), (2, 0));
    assert!(!legal(&g).contains(&Action::CashPilot), "once a turn");
    // Seat 1 isn't ahead: a forced trade costs 1
    assert_eq!(g.token_cost(0), 1);
    g.apply(Action::ForceTrade).unwrap();
    assert_eq!((g.hands[0][1], g.hands[1][1]), (2, 1), "two random cards taken (all Stone)");
    for _ in 0..2 {
        assert_eq!(g.deciding().unwrap().decision, Decision::Gift { to: 1 });
        let a = legal(&g)[0];
        g.apply(a).unwrap();
    }
    assert_eq!(g.cards(0), 2, "and two given back");
    assert!(!legal(&g).contains(&Action::SquallHome), "tokens once a turn");
}

#[test]
fn two_player_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for text in ["variant.two=true", "variant.two=true;sea=true", "variant.two=true;variant.events=true;variant.friendly=true;variant.ports=true"] {
        let s = Settings::parse_for(text, 2).unwrap();
        for seed in 0..25 {
            play_with(&s, seed, 2, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, 2, 600, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{text}, game {seed} didn't finish");
        }
    }
}
