// The variants box's scenario Raider attack (raiders.rs)
use super::*;
use crate::island::generate;

fn raid_game(players: usize) -> Game {
    let s = Settings::parse_for("box.scenario=4", players).unwrap();
    Game::new(&generate(&s, "raid").unwrap(), &s, players, 5)
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
fn the_keep_in_the_middle_twelve_coastal_hexes_and_raiders_on_the_2_and_12() {
    let g = raid_game(3);
    assert_ne!(g.keep, NONE);
    assert_eq!(g.board.graph.cells[g.keep as usize], crate::hex::Cell::new(0, 0));
    assert_eq!(g.coast.len(), 12);
    let on: u32 = g.raiders_on.iter().map(|&n| n as u32).sum();
    assert_eq!(on + g.raider_supply as u32, RAIDERS as u32);
    for &h in &g.coast {
        let n = g.board.number[h as usize];
        assert_eq!(g.raiders_on[h as usize] > 0, n == 2 || n == 12);
    }
    let numbered = (0..g.board.number.len()).filter(|&h| g.board.number[h] != 0 && g.board.terrain[h] != Terrain::Sea).count();
    assert_eq!(numbered, 17, "one Forest and one 11 fewer");
    assert_eq!((g.squall, g.settings().win), (NONE, 12));
}

#[test]
fn a_building_lands_raiders_and_three_conquer_a_hex() {
    let mut g = raid_game(3);
    let before = g.raider_supply;
    g.raider_landing();
    assert_eq!(g.last_landing.len(), 3);
    assert!(g.last_landing.iter().all(|&n| n != 7));
    let mut u = g.last_landing.clone();
    u.dedup();
    assert_eq!(u.len(), 3, "three different numbers");
    assert!(g.raider_supply <= before);
    // Conquest
    let h = g.coast[0] as usize;
    g.raiders_on[h] = 3;
    assert!(g.conquered(h));
    let c = g.board.graph.hex_corners[h][0] as usize;
    assert!(g.by_conquered_corner(c));
    let n = g.board.number[h];
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    let cards = g.cards(0);
    g.produce(n);
    let other = listed(&g.board.graph.corner_hexes[c]).filter(|&x| x != h && g.board.number[x] == n).count();
    assert_eq!(g.cards(0) as usize, cards as usize + other, "a conquered hex produces nothing");
}

#[test]
fn orders_place_knights_and_capture_raiders() {
    let mut g = raid_game(3);
    main_turn(&mut g, 0);
    g.hands[0] = [0, 1, 1, 0, 1, 0, 0, 0];
    g.orders = vec![KNIGHTHOOD];
    assert!(legal(&g).contains(&Action::Buy(0)));
    g.apply(Action::Buy(0)).unwrap();
    let l = legal(&g);
    assert_eq!(l.len(), 6, "the Keep's six edges");
    g.apply(l[0]).unwrap();
    assert_eq!((g.edge_knights(0), g.phase), (1, Phase::Main));
    g.hands[0] = [0, 1, 1, 0, 1, 0, 0, 0];
    g.orders = vec![CAPTURE];
    g.apply(Action::Buy(0)).unwrap();
    let Action::Capture(h) = legal(&g)[0] else { panic!() };
    let r = g.raiders_on[h as usize];
    g.apply(Action::Capture(h)).unwrap();
    assert_eq!((g.raiders_on[h as usize], g.prisoners[0]), (r - 1, 1));
}

#[test]
fn knights_on_the_keep_must_move_and_more_knights_than_raiders_free_a_hex() {
    let mut g = raid_game(3);
    main_turn(&mut g, 0);
    let keep_edge = g.board.graph.hex_edges[g.keep as usize][0] as usize;
    g.edge_knight[keep_edge] = 0;
    g.apply(Action::EndTurn).unwrap();
    let l = legal(&g);
    assert!(!l.contains(&Action::Skip) && l.iter().all(|a| matches!(a, Action::GuardMove(..) | Action::GuardRun(..))), "it must move");
    // Two knights of seat 1's on a coastal hex with 1 raider: freed, a prisoner, maybe a knight lost
    let mut g = raid_game(3);
    main_turn(&mut g, 0);
    let h = g.coast[0] as usize;
    g.raiders_on[h] = 1;
    let edges = g.board.graph.hex_edges[h];
    g.edge_knight[edges[0] as usize] = 0;
    g.edge_knight[edges[1] as usize] = 0;
    g.expel();
    assert_eq!((g.raiders_on[h], g.prisoners[0]), (0, 1));
    let left = edges.iter().filter(|&&e| g.edge_knight[e as usize] == 0).count();
    assert_eq!(g.coins[0] as usize, 3 * (2 - left), "3 coins for each knight lost");
    // Two prisoners are worth a Renown
    let r = g.renown(0);
    g.prisoners[0] = 2;
    assert_eq!(g.renown(0), r + 1);
}

#[test]
fn raider_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("box.scenario=4", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 900, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}

#[test]
#[ignore] // a diagnostic: cargo test -- --ignored --nocapture raid_diagnosis
fn raid_diagnosis() {
    use crate::bot::{Bot, Greedy};
    let s = Settings::parse_for("box.scenario=4", 3).unwrap();
    for seed in 0..400u32 {
        let island = generate(&s, &format!("sim{seed}")).unwrap();
        let mut g = Game::new(&island, &s, 3, seed.wrapping_mul(2654435761));
        let mut bots: Vec<Greedy> = (0..3).map(|i| Greedy::new(seed ^ (i as u32 + 1).wrapping_mul(0x9E3779B9))).collect();
        let mut list = Vec::new();
        while !g.over() && g.turn < 900 {
            g.legal(&mut list);
            let p = g.to_act() as usize;
            let a = Bot::choose(&mut bots[p], &g, &list);
            g.apply_unchecked(a);
        }
        if g.over() {
            continue;
        }
        println!("game {seed} stuck");
        for p in 0..3 {
            println!("seat {p}: renown {} huts {} lodges {} paths left {} cards {} knights {} prisoners {} coins {} hand {:?}",
                g.renown(p), g.huts(p), g.lodges(p), g.pieces[p][P_PATH], g.cards(p), g.edge_knights(p as u8), g.prisoners[p], g.coins[p], &g.hands[p][..5]);
        }
        println!("conquered {:?}", g.coast.iter().map(|&h| g.raiders_on[h as usize]).collect::<Vec<_>>());
        break;
    }
}
