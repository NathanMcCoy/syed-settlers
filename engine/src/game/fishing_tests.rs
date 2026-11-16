// The variants box's scenario Fishing (fishing.rs)
use super::*;
use crate::island::generate;

fn fishing_game(players: usize, seed: &str) -> Game {
    let s = Settings::parse_for("box.scenario=1", players).unwrap();
    Game::new(&generate(&s, seed).unwrap(), &s, players, 5)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

fn main_turn(g: &mut Game, p: u8) {
    g.turn = 1;
    g.current = p;
    g.owner = p;
    g.phase = Phase::Main;
}

#[test]
fn the_lake_sits_in_the_middle_and_six_fishing_grounds_ring_the_island() {
    for k in 0..200 {
        let g = fishing_game(3, &format!("fish{k}"));
        let lake = g.lake as usize;
        assert_ne!(g.lake, NONE);
        assert!(g.board.graph.cells[lake].distance(crate::hex::Cell::new(0, 0)) <= 1, "the Lake is in the middle");
        assert_eq!(g.board.terrain.iter().filter(|&&t| t == Terrain::Dunes).count(), 0, "the Dunes became the Lake");
        let mut grounds: Vec<u8> = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Sea && g.board.number[h] != 0).map(|h| g.board.number[h]).collect();
        grounds.sort();
        assert_eq!(grounds, vec![4, 5, 6, 8, 9, 10], "six fishing grounds");
        // The land keeps its 18 numbers
        let land = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] != Terrain::Sea && g.board.number[h] != 0).count();
        assert_eq!(land, 18);
        assert_eq!(g.squall, NONE, "the squall starts off the island");
        assert_eq!(g.fish_bag.len(), 30);
    }
    assert!(Settings::parse_for("box.scenario=1;sea=true", 3).unwrap_err().contains("on its own"));
    assert!(Settings::parse_for("box.scenario=2", 3).unwrap_err().contains("isn't ready"));
}

#[test]
fn the_lake_and_grounds_pay_fish_huts_1_lodges_2_up_to_7_and_the_boot_shows() {
    let mut g = fishing_game(3, "fish");
    let lake = g.lake as usize;
    let c = g.board.graph.hex_corners[lake][0] as usize;
    g.corner_owner[c] = 0;
    g.corner_kind[c] = LODGE;
    g.fish_bag = vec![2, 3, OLD_BOOT, 1]; // drawn from the end: 1, the boot, 3, 2
    g.fish_production(2);
    assert_eq!((g.fish[0].clone(), g.boot), (vec![1], 0), "a Lodge draws 2: a 1 and the old boot (shown, not counted)");
    assert_eq!(g.win_target(0), 11, "the boot's holder needs 1 more");
    g.fish_production(12);
    assert_eq!(g.fish[0], vec![1, 3, 2]);
    // The squall on the Lake stops all four numbers
    g.squall = g.lake;
    g.fish_bag = vec![3; 5];
    g.fish_production(11);
    assert_eq!(g.fish[0].len(), 3);
    // Never more than 7
    g.squall = NONE;
    g.fish_bag = vec![1; 20];
    for _ in 0..5 {
        g.fish_production(3);
    }
    assert_eq!(g.fish[0].len(), MOST_FISH);
    // A fishing ground pays its coast
    let mut g = fishing_game(3, "fish");
    let ground = (0..g.board.terrain.len()).find(|&h| g.board.terrain[h] == Terrain::Sea && g.board.number[h] != 0).unwrap();
    let c = g.board.graph.hex_corners[ground].iter().map(|&c| c as usize).find(|&c| g.board.corner_land[c]).unwrap();
    g.corner_owner[c] = 1;
    g.corner_kind[c] = HUT;
    g.fish_bag = vec![2, 2];
    g.fish_production(g.board.number[ground]);
    assert_eq!(g.fish[1], vec![2]);
}

#[test]
fn spending_fish_wastes_the_least_and_does_what_it_says() {
    let mut g = fishing_game(3, "fish");
    main_turn(&mut g, 0);
    g.fish[0] = vec![3, 1, 2, 2];
    assert!(!legal(&g).contains(&Action::Fish(0)), "no squall to send away");
    // 4 fish for a resource: 2 + 2 (no waste), not 3 + 1 + ... in this order of preference
    g.apply(Action::Fish(2)).unwrap();
    let left: u32 = g.fish[0].iter().map(|&v| v as u32).sum();
    assert_eq!(left, 4, "exactly 4 spent");
    assert_eq!(g.deciding().unwrap().decision, Decision::Pick { left: 1 });
    g.apply(Action::Take(3)).unwrap();
    assert_eq!((g.hands[0][3], g.phase), (1, Phase::Main));
    // 2 fish: the squall leaves; the next 7 brings it back anywhere
    g.squall = g.board.land_hexes[0];
    g.fish[0] = vec![3];
    g.apply(Action::Fish(0)).unwrap();
    assert_eq!((g.squall, g.fish[0].len()), (NONE, 0), "3 paid for 2: the extra fish is lost");
    g.phase = Phase::MoveSquall { anywhere: false };
    assert!(legal(&g).len() >= g.board.land_hexes.len() - 1);
    // 7 fish: a free project (counted as bought this turn)
    main_turn(&mut g, 0);
    g.fish[0] = vec![3, 3, 1];
    let deck = g.deck.len();
    g.apply(Action::Fish(4)).unwrap();
    assert_eq!((g.deck.len(), g.bought.iter().sum::<u8>()), (deck - 1, 1));
    // 3 fish: steal from a player
    g.hands[2] = [0, 0, 4, 0, 0, 0, 0, 0];
    g.fish[0] = vec![3];
    g.apply(Action::Fish(1)).unwrap();
    g.apply(Action::Rob(2)).unwrap();
    assert_eq!(g.hands[2][2], 3);
    // Fish aren't cards
    assert_eq!(g.cards(1), 0);
}

#[test]
fn the_old_boot_goes_only_to_a_player_with_as_much_renown() {
    let mut g = fishing_game(3, "fish");
    main_turn(&mut g, 0);
    g.boot = 0;
    // Seat 1 has 1 Renown, seat 2 has 2, seat 3 none (Huts in hand count as built)
    g.pieces[0][P_HUT] -= 1;
    g.pieces[1][P_HUT] -= 2;
    let l = legal(&g);
    assert!(l.contains(&Action::GiveBoot(1)) && !l.iter().any(|a| matches!(a, Action::GiveBoot(2))), "{l:?}");
    g.apply(Action::GiveBoot(1)).unwrap();
    assert_eq!((g.boot, g.win_target(0), g.win_target(1)), (1, 10, 11));
}

#[test]
fn fishing_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("box.scenario=1", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 800, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
