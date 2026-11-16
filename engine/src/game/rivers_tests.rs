// The variants box's scenario Rivers (rivers.rs)
use super::*;
use crate::island::generate;

fn rivers_game(players: usize, seed: &str) -> Game {
    let s = Settings::parse_for("box.scenario=2", players).unwrap();
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
fn two_rivers_of_three_hexes_each_with_a_swamp_and_three_bridge_sites() {
    for k in 0..200 {
        let s = Settings::parse_for("box.scenario=2", 3).unwrap();
        let island = generate(&s, &format!("river{k}")).unwrap();
        assert_eq!(island.rivers.len(), 6, "two rivers of 3 hexes");
        assert_eq!(island.bridge_sites.len(), 6, "3 bridge sites each");
        let swamps = island.rivers.iter().filter(|&&h| island.hexes[h as usize].terrain == Terrain::Dunes).count();
        assert_eq!(swamps, 2, "a Swamp on each river");
        let numbered = island.hexes.iter().filter(|h| h.terrain != Terrain::Sea && h.number != 0).count();
        assert_eq!(numbered, 17, "17 numbered hexes (the 2 shares the 12's)");
        assert!(island.hexes.iter().all(|h| h.number != 2), "no hex of its own for the 2");
        let g = Game::new(&island, &s, 3, 1);
        let twelve = (0..g.board.number.len()).find(|&h| g.board.number[h] == 12).unwrap();
        assert!(g.board.by_number[2].contains(&(twelve as u16)), "the 12's hex pays on a 2 too");
    }
}

#[test]
fn bridges_go_only_on_bridge_sites_and_pay_3_coins() {
    let mut g = rivers_game(3, "river");
    let site = (0..g.edge_owner.len()).find(|&e| g.is_bridge_site(e)).unwrap();
    // A Hut of seat 1's at one end of the site
    let c = g.board.graph.edge_corners[site][0] as usize;
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    main_turn(&mut g, 0);
    g.hands[0] = [3, 0, 0, 6, 0, 0, 0, 0];
    let l = legal(&g);
    assert!(!l.contains(&Action::Path(site as u16)), "no Path on a bridge site");
    assert!(l.contains(&Action::Bridge(site as u16)));
    g.apply(Action::Bridge(site as u16)).unwrap();
    assert_eq!((g.coins[0], g.bridges_left[0], g.edge_owner[site]), (3, 2, 0));
    assert_eq!(g.hands[0], [2, 0, 0, 4, 0, 0, 0, 0], "Timber + 2 Copper");
}

#[test]
fn building_on_a_river_pays_a_coin_and_coins_buy_resources_twice_a_turn() {
    let mut g = rivers_game(3, "river");
    let river = (0..g.river_hex.len()).find(|&h| g.river_hex[h]).unwrap();
    let c = g.board.graph.hex_corners[river][0] as usize;
    g.river_coins_corner(1, c);
    assert_eq!(g.coins[1], 1);
    main_turn(&mut g, 0);
    g.coins[0] = 6;
    g.hands[0] = [0; 8];
    for _ in 0..2 {
        g.apply(Action::SpendCoins(2)).unwrap();
    }
    assert_eq!((g.hands[0][2], g.coins[0]), (2, 2));
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::SpendCoins(_))), "twice a turn");
    g.hands[0] = [4, 0, 0, 0, 0, 0, 0, 0];
    let rate = g.rate(0, 0);
    g.apply(Action::BuyCoin(0)).unwrap();
    assert_eq!((g.coins[0], g.hands[0][0]), (3, 4 - rate));
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.coin_buys, 0);
}

#[test]
fn the_richest_gains_1_and_the_poorest_lose_2() {
    let mut g = rivers_game(3, "river");
    let base: Vec<u32> = (0..3).map(|p| g.renown(p)).collect();
    g.coins[..3].copy_from_slice(&[5, 2, 2]);
    assert_eq!(g.wealthiest(), 0);
    assert_eq!(g.renown(0), base[0] + 1);
    assert!(g.poor(1) && g.poor(2));
    // Renown never goes below 0
    assert_eq!(g.renown(1), base[1].saturating_sub(2));
    g.coins[..3].copy_from_slice(&[5, 5, 2]);
    assert_eq!(g.wealthiest(), NOBODY, "a tie: nobody is the Wealthiest");
    assert!(!g.poor(0) && g.poor(2));
}

#[test]
fn rivers_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("box.scenario=2", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 800, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
