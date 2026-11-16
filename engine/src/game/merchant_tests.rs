// The variants box's scenario Merchant trains (merchant.rs)
use super::*;
use crate::island::generate;

fn trains_game(players: usize) -> Game {
    let s = Settings::parse_for("box.scenario=3", players).unwrap();
    Game::new(&generate(&s, "trains").unwrap(), &s, players, 5)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

#[test]
fn the_watering_hole_is_in_the_middle_with_three_train_starts() {
    let g = trains_game(3);
    assert_ne!(g.oasis, NONE);
    assert_eq!(g.board.graph.cells[g.oasis as usize], crate::hex::Cell::new(0, 0));
    assert_eq!(g.board.number[g.oasis as usize], 0);
    assert_eq!((g.train_starts().len(), g.wagon_spots().len(), g.wagons_left), (3, 3, WAGONS));
    assert_eq!((g.settings().win, g.squall), (12, NONE));
}

#[test]
fn trains_grow_nose_to_tail_without_branching_and_end_where_they_meet() {
    let mut g = trains_game(3);
    let (e, train, far) = g.wagon_spots()[0];
    assert_eq!(train, NONE);
    g.place_wagon(e);
    assert_eq!(g.trains.len(), 1);
    // The train now goes on only from its front
    let spots = g.wagon_spots();
    let extend: Vec<_> = spots.iter().filter(|s| s.1 == 0).collect();
    assert!(!extend.is_empty());
    assert!(extend.iter().all(|s| g.board.graph.edge_corners[s.0].contains(&(far as u16))), "from the front only");
    assert_eq!(spots.iter().filter(|s| s.1 == NONE).count(), 2, "two starts left");
    // Grow it until it ends; it never takes an edge twice
    for _ in 0..30 {
        let Some(&(e, _, _)) = g.wagon_spots().iter().find(|s| s.1 == 0) else { break };
        assert!(!g.wagon[e]);
        g.place_wagon(e);
    }
    assert!(g.trains[0].2, "the train ended");
}

#[test]
fn buildings_between_two_wagons_and_wagons_on_paths_count() {
    let mut g = trains_game(3);
    let (e1, _, far) = g.wagon_spots()[0];
    g.place_wagon(e1);
    let (e2, _, _) = *g.wagon_spots().iter().find(|s| s.1 == 0).unwrap();
    g.place_wagon(e2);
    // A Hut of seat 1's on the corner between the two wagons: +1 Renown
    let r = g.renown(0);
    g.corner_owner[far] = 0;
    g.corner_kind[far] = HUT;
    g.pieces[0][P_HUT] -= 1;
    assert_eq!(g.renown(0), r + 2, "1 for the Hut, 1 for the wagons");
    // A Path under a wagon counts twice for Longest Path
    g.edge_owner[e1] = 1;
    g.edge_kind[e1] = PATH;
    assert_eq!(g.longest_line(1), 2);
}

#[test]
fn building_earns_a_wagon_and_the_vote_decides_who_places_it() {
    let mut g = trains_game(3);
    g.turn = 1;
    g.phase = Phase::Main;
    g.current = 0;
    g.owner = 0;
    g.wagon_due = true;
    g.hands[0] = [0, 0, 1, 0, 0, 0, 0, 0];
    g.hands[1] = [0, 0, 0, 0, 3, 0, 0, 0];
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.deciding().unwrap(), Pending { seat: 0, decision: Decision::Bid });
    g.apply(Action::Bid(0, 1)).unwrap();
    assert!(legal(&g).contains(&Action::Bid(3, 0)));
    g.apply(Action::Bid(2, 0)).unwrap();
    g.apply(Action::Bid(0, 0)).unwrap();
    // Seat 2 has 2 of 3 votes: a majority, they place it
    assert_eq!(g.deciding().unwrap(), Pending { seat: 1, decision: Decision::PlaceWagon });
    assert_eq!((g.hands[0][2], g.hands[1][4]), (0, 1), "bids are spent");
    let a = legal(&g)[0];
    g.apply(a).unwrap();
    assert_eq!((g.wagons_left, g.current, g.phase), (WAGONS - 1, 1, Phase::PreRoll), "then the next turn");
    // No votes at all: the player acting places it
    g.bids = [0; MAX_PLAYERS];
    g.current = 2;
    assert_eq!(g.vote_winner(), 2);
    g.bids[..3].copy_from_slice(&[2, 2, 1]);
    assert_eq!(g.vote_winner(), 2, "a tie for the most: the player acting");
}

#[test]
fn merchant_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("box.scenario=3", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 800, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
