// The exploration expansion (explore.rs)
use super::*;
use crate::island::generate;

fn explore_game(scenario: u8, players: usize) -> Game {
    let s = Settings::parse_for(&format!("explore={scenario}"), players).unwrap();
    Game::new(&generate(&s, "explore").unwrap(), &s, players, 5)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

fn after_setup(scenario: u8, players: usize) -> Game {
    let mut g = explore_game(scenario, players);
    while g.turn == 0 {
        let a = legal(&g)[0];
        g.apply(a).unwrap();
    }
    g
}

#[test]
fn the_map_a_start_island_two_hidden_regions_and_the_council() {
    for (scenario, size) in [(1u8, 8usize), (2, 11), (3, 11), (4, 11), (5, 16)] {
        let g = explore_game(scenario, 3);
        let hidden = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).count();
        assert_eq!(hidden, 2 * size, "scenario {scenario}: two regions of {size}");
        assert_eq!(g.board.terrain[g.ex.council as usize], Terrain::Council);
        let start = g.ex.start_hexes.iter().filter(|&&s| s).count();
        assert_eq!(start, 15, "the start island");
    }
    // 5-6 players: 21 start hexes, 4 more hidden hexes a region (and a gold field with lairs)
    let g = explore_game(5, 6);
    assert_eq!(g.ex.start_hexes.iter().filter(|&&s| s).count(), 21);
    assert_eq!((0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).count(), 2 * 21);
    let win: Vec<u8> = (1..=5).map(|k| Settings::parse_for(&format!("explore={k}"), 3).unwrap().win).collect();
    assert_eq!(win, vec![8, 12, 15, 15, 17]);
}

#[test]
fn setup_a_harbour_and_a_ship_with_a_settler_then_a_hut() {
    let g = after_setup(1, 3);
    for p in 0..3u8 {
        let harbours = (0..g.corner_owner.len()).filter(|&c| g.corner_owner[c] == p && g.corner_kind[c] == LODGE).count();
        assert_eq!((harbours, g.huts(p as usize)), (1, 1));
        let ships: Vec<&Ship> = g.ex.ships.iter().filter(|s| s.owner == p).collect();
        assert_eq!(ships.len(), 1);
        assert_eq!(ships[0].cargo, [SETTLER, 0]);
        assert_eq!(g.renown(p as usize), 3, "a harbour is 2, a Hut 1");
        assert_eq!(g.coins[p as usize], 2);
    }
}

#[test]
fn sailing_explores_and_a_settler_founds_a_hut() {
    let mut g = after_setup(1, 3);
    g.current = 0;
    g.owner = 0;
    g.phase = Phase::Main;
    g.apply(Action::EndTurn).unwrap();
    assert_eq!(g.deciding().unwrap().decision, Decision::Sail);
    let hidden_before = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).count();
    // Sail toward the hidden hexes until one is revealed (several turns if need be)
    let mut revealed = false;
    for _ in 0..12 {
        let i = g.ex.ships.iter().position(|s| s.owner == 0).unwrap();
        let reach = g.sail_reach(i, false);
        if reach.is_empty() {
            break;
        }
        // the edge closest to a hidden hex
        let gr = &g.board.graph;
        let fog: Vec<usize> = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).collect();
        let near = |e: usize| {
            let c = gr.edge_corners[e][0] as usize;
            let hx = listed(&gr.corner_hexes[c]).next().unwrap();
            fog.iter().map(|&f| gr.cells[f].distance(gr.cells[hx])).min().unwrap_or(99)
        };
        let e = *reach.iter().min_by_key(|&&e| near(e)).unwrap();
        g.apply(Action::Sail(i as u8, e as u16)).unwrap();
        let now = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).count();
        if now < hidden_before {
            revealed = true;
            break;
        }
        g.ex.ships[i].moved = false;
    }
    assert!(revealed, "a hidden hex was explored");
    // A settler on a ship by a free land corner founds a Hut
    let huts = g.huts(0);
    if let Some(&a) = legal(&g).iter().find(|a| matches!(a, Action::Settle(..))) {
        g.apply(a).unwrap();
        assert_eq!(g.huts(0), huts + 1);
        assert!(g.ex.ships.iter().all(|s| s.owner != 0), "the ship and settler go home");
    }
}

#[test]
fn three_crews_capture_a_lair_for_coins_and_track_steps() {
    let mut g = explore_game(2, 3);
    let lair = (0..g.board.terrain.len()).find(|&h| g.ex.hidden[h].0 == Terrain::Gold).unwrap();
    // Reveal it by hand
    let mut hexes = g.board.hexes.clone();
    hexes[lair].terrain = Terrain::Gold;
    hexes[lair].number = 4;
    let b = Board::from_parts(g.board.graph.clone(), hexes, g.board.markets.clone(), g.board.settings.clone());
    g.board = Arc::new(b);
    g.ex.lair_crews[lair] = vec![0, 1];
    g.capture_lair(lair);
    assert!(g.ex.lair_taken[lair]);
    assert_eq!((g.coins[0], g.coins[1]), (4, 4), "2 coins each");
    let steps = g.ex.tracks[LAIRS][0] + g.ex.tracks[LAIRS][1];
    assert_eq!(steps, 3, "a step each, and one more for the hero");
    let r = g.renown(0) + g.renown(1);
    assert!(r >= 3, "track Renown and the leader's tile");
}

#[test]
fn explore_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for scenario in 1..=5u8 {
        for players in [2, 3, 4, 6] {
            let s = Settings::parse_for(&format!("explore={scenario}"), players).unwrap();
            for seed in 0..8 {
                play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
                let o = play_with(&s, seed, players, 1200, &[BotKind::Greedy]).unwrap();
                assert_ne!(o.winner, NOBODY, "scenario {scenario}, {players} players, game {seed} didn't finish");
            }
        }
    }
}

#[test]
#[ignore] // a diagnostic: cargo test -- --ignored --nocapture explore_diagnosis
fn explore_diagnosis() {
    use crate::bot::{Bot, Greedy};
    let sc = std::env::var("SC").unwrap_or("1".into());
    let s = Settings::parse_for(&format!("explore={sc}"), 3).unwrap();
    let mut g = Game::new(&generate(&s, "sim1").unwrap(), &s, 3, 7);
    let mut counts = std::collections::BTreeMap::new();
    for seed in 0..60u32 {
        g = Game::new(&generate(&s, &format!("d{seed}")).unwrap(), &s, 3, seed);
        let mut bots: Vec<Greedy> = (0..3).map(|i| Greedy::new(seed * 3 + i + 1)).collect();
        let mut list = Vec::new();
        counts.clear();
        while !g.over() && g.turn < 1200 {
            g.legal(&mut list);
            let p = g.to_act() as usize;
            let a = Bot::choose(&mut bots[p], &g, &list);
            if g.turn > 1000 {
                *counts.entry(format!("{:?}", a).split('(').next().unwrap().to_string()).or_insert(0) += 1;
            }
            g.apply_unchecked(a);
        }
        if !g.over() {
            println!("seed {seed} stalled");
            break;
        }
    }
    for p in 0..3 {
        let ships: Vec<_> = g.ex.ships.iter().filter(|s| s.owner as usize == p).map(|s| (s.edge, s.cargo)).collect();
        println!("seat {p}: renown {} huts {} harbours {} hand {:?} coins {} ships {:?} settlers left {} crews left {} tracks {:?}", g.renown(p), g.huts(p), g.lodges(p), &g.hands[p][..5], g.coins[p], ships, g.ex.settlers_left[p], g.ex.crews_left[p], [g.ex.tracks[0][p], g.ex.tracks[1][p], g.ex.tracks[2][p]]);
    }
    let fog = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).count();
    println!("fog left {fog}; turn {}; actions {counts:?}", g.turn);
    for h in 0..g.board.terrain.len() {
        if g.board.terrain[h] == Terrain::Spice {
            println!("farm {h}: crews {:?} sacks {:?}", g.ex.farm_crews[h], g.ex.sacks[h]);
        }
    }
    let gr = g.board.graph.clone();
    for (i, sh) in g.ex.ships.iter().enumerate() {
        if !sh.cargo.contains(&CREW) {
            continue;
        }
        let farms: Vec<usize> = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Spice && !g.ex.farm_crews[h].contains(&sh.owner)).collect();
        let dist = |e: usize| listed(&gr.edge_hexes[e]).map(|x| farms.iter().map(|&f| gr.cells[x].distance(gr.cells[f])).min().unwrap_or(99)).min().unwrap_or(99);
        let reach = g.sail_reach(i, false);
        println!("crew ship {i} (seat {}) at {} dist {} moved {}: reach {} edges, best {:?}; acts {:?}", sh.owner, sh.edge, dist(sh.edge as usize), sh.moved, reach.len(), reach.iter().map(|&e| dist(e)).min(), g.ship_acts(i));
    }
    if let Some(i) = g.ex.ships.iter().position(|s| s.owner == 0) {
        let reach = g.sail_reach(i, false);
        let gr = &g.board.graph;
        let fogs: Vec<usize> = (0..g.board.terrain.len()).filter(|&h| g.board.terrain[h] == Terrain::Fog).collect();
        let d = |e: usize| listed(&gr.edge_hexes[e]).map(|x| fogs.iter().map(|&f| gr.cells[x].distance(gr.cells[f])).min().unwrap_or(99)).min().unwrap_or(99);
        println!("ship {i} at {} (fog dist {}): reach {} edges, best fog dist {:?}", g.ex.ships[i].edge, d(g.ex.ships[i].edge as usize), reach.len(), reach.iter().map(|&e| d(e)).min());
        println!("edge hexes of ship edge: {:?}", listed(&gr.edge_hexes[g.ex.ships[i].edge as usize]).map(|h| g.board.terrain[h]).collect::<Vec<_>>());
    }
}
