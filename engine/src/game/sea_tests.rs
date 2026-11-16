// The sea expansion's rules (EXPANSIONS.md, B), one test per rule where possible
use super::*;
use crate::island::generate;

fn sea_game(players: usize, seed: &str) -> Game {
    let s = Settings::parse_for("sea=true", players).unwrap();
    Game::new(&generate(&s, seed).unwrap(), &s, players, 7)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

// A main-island corner on the coast with a sea edge, free of the pirate, and spaced
fn coastal_corner(g: &Game) -> usize {
    let gr = &g.board.graph;
    (0..gr.corners())
        .find(|&c| {
            g.board.corner_region[c] == 0
                && g.spaced(c)
                && listed(&gr.corner_edges[c]).any(|e| g.board.edge_sea[e] && !g.board.edge_land[e] && !g.by_pirate(e))
        })
        .unwrap()
}

// The open sea edge leaving a coastal corner (sea on both sides)
fn open_sea_edge(g: &Game, c: usize) -> usize {
    listed(&g.board.graph.corner_edges[c]).find(|&e| g.board.edge_sea[e] && !g.board.edge_land[e] && !g.by_pirate(e)).unwrap()
}

fn other_end(g: &Game, e: usize, c: usize) -> usize {
    let [a, b] = g.board.graph.edge_corners[e];
    if a as usize == c { b as usize } else { a as usize }
}

// Seat 0 in its main phase with a Hut on `c`
fn main_phase_with_hut(g: &mut Game, c: usize) {
    g.turn = 1;
    g.phase = Phase::Main;
    g.current = 0;
    g.owner = 0;
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    g.pieces[0][P_HUT] -= 1;
}

#[test]
fn sea_islands_keep_every_rule() {
    for players in [3, 4, 6] {
        let s = Settings::parse_for("sea=true", players).unwrap();
        assert_eq!(s.win, 14, "New Shores is played to 14");
        for k in 0..300 {
            let island = generate(&s, &format!("sea{players}-{k}")).unwrap();
            let hexes = &island.hexes;
            let land: Vec<&crate::island::Hex> = hexes.iter().filter(|h| h.terrain != Terrain::Sea).collect();
            let small: Vec<&&crate::island::Hex> = land.iter().filter(|h| h.region != 0).collect();
            assert_eq!(small.len(), if players >= 5 { 11 } else { 9 });
            let gold = small.iter().filter(|h| h.terrain == Terrain::Gold).count();
            assert_eq!(gold, if players >= 5 { 4 } else { 2 });
            assert!(land.iter().all(|h| h.terrain != Terrain::Gold || h.region != 0), "gold only on small islands");
            // Different islands never touch: at least one sea hex between them
            for a in &land {
                for b in &land {
                    if a.region != b.region {
                        assert!(a.cell.distance(b.cell) >= 2, "islands {} and {} touch", a.region, b.region);
                    }
                }
            }
            // Markets on the main island, the squall on its Dunes, the pirate on the sea
            for m in &island.markets {
                assert_eq!(hexes[m.hex as usize].region, 0);
            }
            assert_eq!(hexes[island.squall as usize].terrain, Terrain::Dunes);
            assert_eq!(hexes[island.squall as usize].region, 0);
            assert_eq!(hexes[island.pirate as usize].terrain, Terrain::Sea);
        }
    }
    // Without the sea: no islands, no pirate
    let island = generate(&Settings::default(), "plain").unwrap();
    assert_eq!(island.pirate, NONE);
    assert!(!island.to_json().contains("\"pirate\""));
}

#[test]
fn setup_is_on_the_main_island_and_a_coastal_hut_may_take_a_ship() {
    let mut g = sea_game(3, "setup");
    let list = legal(&g);
    assert!(!list.is_empty());
    for a in &list {
        let Action::Hut(c) = a else { panic!("{a:?}") };
        assert_eq!(g.board.corner_region[*c as usize], 0, "setup Huts on the main island only");
    }
    let c = coastal_corner(&g);
    g.apply(Action::Hut(c as u16)).unwrap();
    let list = legal(&g);
    let e = open_sea_edge(&g, c);
    assert!(list.contains(&Action::Ship(e as u16)), "a ship instead of a Path");
    g.apply(Action::Ship(e as u16)).unwrap();
    assert_eq!((g.edge_owner[e], g.edge_kind[e]), (0, SHIP));
    assert_eq!(g.pieces[0][P_SHIP], g.settings().pieces_ship - 1);
    assert!(g.ships_new.is_empty(), "setup ships aren't 'built this turn'");
}

#[test]
fn ships_join_ships_and_buildings_not_paths_and_never_pass_others_or_the_pirate() {
    let mut g = sea_game(3, "ships");
    let c = coastal_corner(&g);
    main_phase_with_hut(&mut g, c);
    g.hands[0] = [5, 0, 0, 0, 5];
    let e = open_sea_edge(&g, c);
    assert!(legal(&g).contains(&Action::Ship(e as u16)), "next to the Hut");
    g.apply(Action::Ship(e as u16)).unwrap();
    assert_eq!(g.hands[0], [4, 0, 0, 0, 4], "Timber + Fish");
    // Carrying on from the ship's far end
    let far = other_end(&g, e, c);
    let next = listed(&g.board.graph.corner_edges[far]).find(|&f| f != e && g.board.edge_sea[f] && g.edge_owner[f] == NOBODY && !g.by_pirate(f));
    if let Some(f) = next {
        assert!(legal(&g).contains(&Action::Ship(f as u16)));
        // Another player's building at the far end blocks it
        g.corner_owner[far] = 1;
        g.corner_kind[far] = HUT;
        assert!(!legal(&g).contains(&Action::Ship(f as u16)), "not past another player's building");
        g.corner_owner[far] = NOBODY;
        g.corner_kind[far] = 0;
        // The pirate's hex blocks every edge round it
        let h = listed(&g.board.graph.edge_hexes[f]).find(|&h| g.board.terrain[h] == Terrain::Sea).unwrap();
        g.pirate = h as u16;
        assert!(!legal(&g).contains(&Action::Ship(f as u16)), "not by the pirate");
    }
    // A Path doesn't carry ships on: a lone Path's end takes no ship
    let mut g = sea_game(3, "ships");
    let c = coastal_corner(&g);
    main_phase_with_hut(&mut g, c);
    let path = listed(&g.board.graph.corner_edges[c]).find(|&e| g.board.edge_land[e]).unwrap();
    g.corner_owner[c] = NOBODY;
    g.corner_kind[c] = 0;
    g.edge_owner[path] = 0;
    g.edge_kind[path] = PATH;
    g.hands[0] = [5, 0, 0, 0, 5];
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::Ship(_))), "ships join ships or buildings only");
    // A coastal edge takes a Path or a ship, not both
    assert!(g.board.edge_land[path] && g.board.edge_sea[path] || !g.board.edge_sea[path]);
}

#[test]
fn one_ship_moves_a_turn_from_an_open_end_but_not_a_new_one_or_a_closed_line() {
    let mut g = sea_game(3, "move");
    let c = coastal_corner(&g);
    main_phase_with_hut(&mut g, c);
    let e = open_sea_edge(&g, c);
    g.hands[0] = [5, 0, 0, 0, 5];
    g.apply(Action::Ship(e as u16)).unwrap();
    assert!(!g.ship_movable(0, e), "not on the turn it was built");
    g.apply(Action::EndTurn).unwrap();
    // Back to seat 0's main phase on a later turn
    g.current = 0;
    g.owner = 0;
    g.phase = Phase::Main;
    assert!(g.ship_movable(0, e), "its far end is open");
    let moves: Vec<Action> = legal(&g).into_iter().filter(|a| matches!(a, Action::MoveShip(..))).collect();
    assert!(!moves.is_empty());
    let Action::MoveShip(from, to) = moves[0] else { unreachable!() };
    g.apply(moves[0]).unwrap();
    assert_eq!((g.edge_owner[from as usize], g.edge_owner[to as usize], g.edge_kind[to as usize]), (NOBODY, 0, SHIP));
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::MoveShip(..))), "one move a turn");
    // A line of ships between two of p's buildings can't move, even with another's building on it
    let mut g = sea_game(3, "move");
    let c = coastal_corner(&g);
    main_phase_with_hut(&mut g, c);
    let e = open_sea_edge(&g, c);
    g.edge_owner[e] = 0;
    g.edge_kind[e] = SHIP;
    let far = other_end(&g, e, c);
    g.corner_owner[far] = 0;
    g.corner_kind[far] = HUT;
    assert!(!g.ship_movable(0, e), "joins two of the player's buildings");
    g.corner_owner[far] = NOBODY;
    g.corner_kind[far] = 0;
    assert!(g.ship_movable(0, e));
}

#[test]
fn longest_path_joins_paths_and_ships_only_at_the_players_building() {
    let mut g = sea_game(3, "line");
    let c = coastal_corner(&g);
    let path = listed(&g.board.graph.corner_edges[c]).find(|&e| g.board.edge_land[e]).unwrap();
    let ship = open_sea_edge(&g, c);
    for (e, k) in [(path, PATH), (ship, SHIP)] {
        g.edge_owner[e] = 0;
        g.edge_kind[e] = k;
    }
    assert_eq!(g.longest_line(0), 1, "a Path and a ship meeting at an empty corner don't join");
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    assert_eq!(g.longest_line(0), 2, "they join at the player's own Hut");
}

#[test]
fn gold_fields_pay_resources_of_the_players_choice() {
    let mut g = sea_game(3, "gold");
    let gold = (0..g.board.resource.len()).find(|&h| g.board.resource[h] == GOLD).unwrap();
    let n = g.board.number[gold];
    // Seat 2 with a Lodge on the gold field, seat 1 with a Hut; the squall elsewhere
    let corners = g.board.graph.hex_corners[gold];
    for (c, p, k) in [(corners[0] as usize, 2u8, LODGE), (corners[2] as usize, 1u8, HUT)] {
        g.corner_owner[c] = p;
        g.corner_kind[c] = k;
    }
    g.turn = 1;
    g.current = 0;
    g.phase = Phase::PreRoll;
    g.hands = [[0; 5]; MAX_PLAYERS];
    g.rolled(n);
    // Seat 2 (next after the roller) chooses 2, then seat 3... in seat order from the roller: 1 then 2
    assert_eq!(g.phase, Phase::Decide);
    assert_eq!(g.deciding().unwrap(), Pending { seat: 1, decision: Decision::Gold { left: 1 } });
    assert_eq!(legal(&g), (0..5).map(Action::Take).collect::<Vec<_>>());
    g.apply_as(1, Action::Take(3)).unwrap();
    assert_eq!(g.deciding().unwrap().seat, 2);
    g.apply_as(2, Action::Take(0)).unwrap();
    g.apply_as(2, Action::Take(4)).unwrap();
    assert_eq!(g.phase, Phase::Main);
    assert_eq!((g.hands[1], g.hands[2]), ([0, 0, 0, 1, 0], [1, 0, 0, 0, 1]));
    // With the bank empty, gold pays nothing and play goes on
    let mut g2 = g.clone();
    g2.phase = Phase::PreRoll;
    g2.hands = [[0; 5]; MAX_PLAYERS];
    g2.hands[0] = [19, 19, 19, 19, 19]; // the whole bank
    g2.rolled(n);
    assert_eq!(g2.phase, Phase::Main, "nothing to take: no decision");
}

#[test]
fn the_pirate_robs_ship_owners_and_the_squall_or_pirate_is_the_rollers_choice() {
    let mut g = sea_game(3, "pirate");
    let c = coastal_corner(&g);
    main_phase_with_hut(&mut g, c);
    let e = open_sea_edge(&g, c);
    g.edge_owner[e] = 1;
    g.edge_kind[e] = SHIP;
    g.hands[1] = [0, 0, 2, 0, 0];
    g.hands[0] = [0; 5];
    g.phase = Phase::MoveSquall { anywhere: false };
    g.back = Phase::Main;
    let list = legal(&g);
    assert!(list.iter().any(|a| matches!(a, Action::Squall(_))) && list.iter().any(|a| matches!(a, Action::Pirate(_))));
    let h = listed(&g.board.graph.edge_hexes[e]).find(|&h| g.board.terrain[h] == Terrain::Sea && h != g.pirate as usize).unwrap();
    g.apply(Action::Pirate(h as u16)).unwrap();
    assert_eq!(g.pirate, h as u16);
    assert_eq!((g.hands[0], g.hands[1]), ([0, 0, 1, 0, 0], [0, 0, 1, 0, 0]), "one card from the ship's owner");
    assert_eq!(g.phase, Phase::Main);
    // A Pilot can send the pirate too
    g.held[0][Project::Pilot as usize] = 1;
    g.apply(Action::Play(Project::Pilot)).unwrap();
    assert!(legal(&g).iter().any(|a| matches!(a, Action::Pirate(_))));
}

#[test]
fn the_first_hut_on_each_small_island_earns_the_bonus_once() {
    let mut g = sea_game(3, "bonus");
    g.turn = 1;
    g.phase = Phase::Main;
    let gr = &g.board.graph;
    let spots: Vec<usize> = (0..gr.corners()).filter(|&c| g.board.corner_region[c] == 1 && g.spaced(c)).collect();
    let a = spots[0];
    let b = *spots.iter().find(|&&c| !listed(&gr.corner_corners[a]).any(|n| n == c) && c != a).unwrap();
    let ship_at = |g: &mut Game, c: usize| {
        let e = listed(&g.board.graph.corner_edges[c]).find(|&e| g.board.edge_sea[e]).unwrap();
        g.edge_owner[e] = 0;
        g.edge_kind[e] = SHIP;
    };
    let before = g.renown(0);
    for c in [a, b] {
        ship_at(&mut g, c);
        g.hands[0] = HUT_COST;
        g.apply(Action::Hut(c as u16)).unwrap();
    }
    assert_eq!(g.tokens_of(0, Token::Renown), 2, "2 Renown tokens, once");
    assert_eq!(g.renown(0), before + 2 + 2, "two Huts and the bonus");
}

#[test]
fn road_crew_places_paths_or_ships() {
    let mut g = sea_game(3, "crew");
    let c = coastal_corner(&g);
    main_phase_with_hut(&mut g, c);
    g.held[0][Project::RoadCrew as usize] = 1;
    g.apply(Action::Play(Project::RoadCrew)).unwrap();
    let list = legal(&g);
    assert!(list.iter().any(|a| matches!(a, Action::Path(_))) && list.iter().any(|a| matches!(a, Action::Ship(_))));
    let ship = *list.iter().find(|a| matches!(a, Action::Ship(_))).unwrap();
    g.apply(ship).unwrap();
    let path = *legal(&g).iter().find(|a| matches!(a, Action::Path(_))).unwrap();
    g.apply(path).unwrap();
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(g.hands[0], [0; 5], "free");
}

#[test]
fn sea_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("sea=true", players).unwrap();
        for seed in 0..30 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 400, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
