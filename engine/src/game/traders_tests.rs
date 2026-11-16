// The variants box's scenario Traders and raiders (traders.rs)
use super::*;
use crate::island::generate;

fn traders_game(players: usize) -> Game {
    let s = Settings::parse_for("box.scenario=5", players).unwrap();
    Game::new(&generate(&s, "traders").unwrap(), &s, players, 5)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

fn after_setup(players: usize) -> Game {
    let mut g = traders_game(players);
    while g.turn == 0 {
        let a = legal(&g)[0];
        g.apply(a).unwrap();
    }
    g
}

#[test]
fn three_works_no_2_or_12_three_raiders_and_wagons_at_the_lodges() {
    let g = after_setup(3);
    assert!(g.works.iter().all(|&h| h != NONE), "the Keep, the Glassworks and the Stoneworks");
    assert!(g.board.number.iter().all(|&n| n != 2 && n != 12));
    assert_eq!(g.barbarian.iter().filter(|&&b| b).count(), 3);
    for p in 0..3 {
        let c = g.wagon_corner[p] as usize;
        assert_eq!((g.corner_owner[c], g.corner_kind[c]), (p as u8, LODGE), "the wagon starts at the Lodge");
        assert_eq!(g.coins[p], 5);
    }
    assert!(!Title::LongestPath.active(g.settings()) && !Title::MostPilots.active(g.settings()));
    assert_eq!(g.settings().win, 13);
}

#[test]
fn wagon_steps_cost_by_the_path_under_them_and_raiders_cost_more() {
    let mut g = after_setup(3);
    let c = g.wagon_corner[0] as usize;
    let gr = g.board.graph.clone();
    let e = listed(&gr.corner_edges[c]).find(|&e| g.board.edge_land[e]).unwrap();
    g.edge_owner[e] = NOBODY;
    g.barbarian[e] = false;
    assert_eq!(g.step_cost(0, e), Some((2, 0)), "no Path: 2");
    g.edge_owner[e] = 0;
    assert_eq!(g.step_cost(0, e), Some((1, 0)), "your Path: 1");
    g.edge_owner[e] = 1;
    assert_eq!(g.step_cost(0, e), Some((1, 1)), "another player's Path: 1 and a coin");
    g.coins[0] = 0;
    assert_eq!(g.step_cost(0, e), None, "no coin for the toll");
    g.coins[0] = 5;
    g.barbarian[e] = true;
    assert_eq!(g.step_cost(0, e), Some((3, 1)), "a raider: 2 more");
}

#[test]
fn the_end_of_the_turn_drives_the_wagon_and_a_works_takes_and_gives_goods() {
    let mut g = after_setup(3);
    g.current = 0;
    g.owner = 0;
    g.phase = Phase::Main;
    // Put seat 1's wagon next to the Glassworks carrying Sand
    let w = 1;
    let h = g.works[w] as usize;
    let gr = g.board.graph.clone();
    // A corner of it with a neighbouring land corner that belongs to no works
    let (corner, next) = gr.hex_corners[h].iter().map(|&c| c as usize)
        .find_map(|c| listed(&gr.corner_corners[c]).find(|&n| g.works_at(n).is_none() && g.board.corner_land[n]).map(|n| (c, n)))
        .unwrap();
    g.wagon_corner[0] = next as u16;
    // No Path on the step (no toll)
    let k = listed(&gr.corner_corners[next]).position(|x| x == corner).unwrap();
    g.edge_owner[gr.corner_edges[next][k] as usize] = NOBODY;
    g.cargo[0] = SAND;
    for e in 0..g.barbarian.len() {
        g.barbarian[e] = false;
    }
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.deciding().unwrap().decision, Decision::Drive);
    let coins = g.coins[0];
    g.apply(Action::WagonStep(corner as u16)).unwrap();
    assert_eq!(g.delivered[0], 0, "reaching the corner isn't entering");
    assert!(legal(&g).contains(&Action::EnterWorks), "4 points - 2 for the step leaves 2 to enter");
    g.apply(Action::EnterWorks).unwrap();
    assert_eq!(g.delivered[0], 1, "Sand delivered");
    assert_eq!(g.coins[0], coins + 1, "1 coin at level 0");
    assert!(WORKS[w].2.contains(&g.cargo[0]), "a good of the Glassworks picked up");
    assert_eq!((g.current, g.phase), (1, Phase::PreRoll), "the move ended at the works, and the turn");
}

#[test]
fn upgrades_raise_the_wagon_and_the_last_is_renown() {
    let mut g = after_setup(3);
    g.current = 0;
    g.phase = Phase::Main;
    let r = g.renown(0);
    for lvl in 0..4 {
        g.hands[0] = cards_of(UPGRADES[lvl]);
        assert!(legal(&g).contains(&Action::UpgradeWagon));
        g.apply(Action::UpgradeWagon).unwrap();
    }
    assert_eq!((g.wagon_level[0], g.renown(0)), (4, r + 1));
    assert!(!legal(&g).contains(&Action::UpgradeWagon));
}

#[test]
fn traders_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("box.scenario=5", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 900, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}

#[test]
#[ignore] // a diagnostic: cargo test -- --ignored --nocapture traders_diagnosis
fn traders_diagnosis() {
    use crate::bot::{Bot, Greedy};
    let s = Settings::parse_for("box.scenario=5", 4).unwrap();
    for seed in 0..5u32 {
        let island = generate(&s, &format!("sim{seed}")).unwrap();
        let mut g = Game::new(&island, &s, 4, seed.wrapping_mul(2654435761));
        let mut bots: Vec<Greedy> = (0..4).map(|i| Greedy::new(seed ^ (i as u32 + 1).wrapping_mul(0x9E3779B9))).collect();
        let mut list = Vec::new();
        while !g.over() && g.turn < 900 {
            g.legal(&mut list);
            let p = g.to_act() as usize;
            let a = Bot::choose(&mut bots[p], &g, &list);
            g.apply_unchecked(a);
        }
        let w = g.winner as usize;
        println!("game {seed}: turn {} winner {w} renown {} huts {} lodges {} delivered {} level {} lighthouses {} | others delivered {:?}",
            g.turn, g.renown(w), g.huts(w), g.lodges(w), g.delivered[w], g.wagon_level[w], g.held[w][Project::Lighthouse as usize],
            (0..4).map(|p| g.delivered[p]).collect::<Vec<_>>());
    }
}
