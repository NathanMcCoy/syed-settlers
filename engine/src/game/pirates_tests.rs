// The sea scenario The Pirate Islands
use super::*;
use crate::island::generate;

fn pirate_game(players: usize) -> Game {
    let s = Settings::parse_for("sea=true;sea.scenario=7", players).unwrap();
    Game::new(&generate(&s, "pirates").unwrap(), &s, players, 7)
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

#[test]
fn fortresses_beachheads_the_fleet_and_the_starting_pieces() {
    let g = pirate_game(4);
    assert_eq!((g.settings().win, g.pirate, g.squall), (10, NONE, NONE), "10 to win, no pirate, no squall");
    assert!(!g.fleet.is_empty());
    assert!(!Title::LongestPath.active(g.settings()) && !Title::MostPilots.active(g.settings()));
    assert_eq!(g.deck.iter().filter(|&&k| k == Project::Lighthouse as u8).count(), 0, "Lighthouses are Pilots with 4 players");
    for p in 0..4 {
        let (f, b) = (g.fort_corner[p] as usize, g.beach_corner[p] as usize);
        assert_eq!(g.corner_owner[f], pirates::PIRATES);
        assert!(f != b && !listed(&g.board.graph.corner_corners[f]).any(|n| n == b), "the beachhead doesn't touch the fortress");
        // A Hut and a ship facing the islet already placed
        assert_eq!(g.huts(p), 1);
        assert_eq!(g.ship_line.iter().filter(|&&(o, _)| o as usize == p).count(), 1);
    }
}

#[test]
fn the_line_grows_from_its_tip_one_step_nearer_and_pilots_make_warships() {
    let mut g = pirate_game(3);
    g.turn = 1;
    g.phase = Phase::Main;
    g.current = 0;
    g.hands[0] = [5, 0, 0, 0, 5, 0, 0, 0];
    let tip = g.line_tip[0] as usize;
    let ships: Vec<Action> = legal(&g).into_iter().filter(|a| matches!(a, Action::Ship(_))).collect();
    assert!(!ships.is_empty());
    for a in &ships {
        let Action::Ship(e) = a else { unreachable!() };
        assert!(g.board.graph.edge_corners[*e as usize].contains(&(tip as u16)), "only from the tip");
    }
    g.apply(ships[0]).unwrap();
    assert_ne!(g.line_tip[0] as usize, tip);
    assert!(!legal(&g).iter().any(|a| matches!(a, Action::MoveShip(..))), "ships don't move here");
    // A Pilot turns the first normal ship into a warship (and moves no squall)
    g.held[0][Project::Pilot as usize] = 1;
    g.apply(Action::Play(Project::Pilot)).unwrap();
    assert_eq!((g.warships(0), g.phase), (1, Phase::Main));
}

#[test]
fn attacking_the_fortress_takes_lairs_or_loses_ships() {
    let mut g = pirate_game(3);
    // Pretend seat 1's line reached its fortress
    g.line_tip[0] = g.fort_corner[0];
    // No warships: always beaten (the die is at least 1): 2 ships lost, or 1 on a tie (impossible here)
    let ships = g.pieces[0][P_SHIP];
    g.attack_fortress(0);
    assert_eq!(g.lairs[0], 3);
    assert!(g.pieces[0][P_SHIP] > ships, "ships came back");
    // Seven warships always win: three wins take the fortress (a Hut of seat 1's)
    let mut g = pirate_game(3);
    g.line_tip[0] = g.fort_corner[0];
    for e in 0..7usize {
        g.ship_line.push((0, e as u16));
        g.warship[e] = true;
    }
    for _ in 0..3 {
        g.attack_fortress(0);
    }
    assert!(g.fort_taken[0]);
    assert_eq!(g.corner_owner[g.fort_corner[0] as usize], 0);
    // A fortress and 10 Renown win; Renown alone doesn't
    let mut h = pirate_game(3);
    h.held[0][Project::Lighthouse as usize] = 10;
    assert!(!h.has_won(0));
    g.held[0][Project::Lighthouse as usize] = 10;
    assert!(g.has_won(0));
}

#[test]
fn a_seven_has_no_squall_the_roller_robs_anyone() {
    let mut g = pirate_game(3);
    g.turn = 1;
    g.current = 0;
    g.phase = Phase::PreRoll;
    g.hands = [[0; 8]; MAX_PLAYERS];
    g.hands[2] = [0, 0, 2, 0, 0, 0, 0, 0];
    g.rolled(7);
    assert_eq!(g.deciding().unwrap(), Pending { seat: 0, decision: Decision::Rob });
    g.apply(Action::Rob(2)).unwrap();
    assert_eq!((g.hands[0][2], g.phase), (1, Phase::Main));
}

#[test]
fn the_fleet_strips_the_undefended_and_feeds_the_well_defended() {
    let mut g = pirate_game(3);
    // Put the fleet one step before a hex touching seat 1's Hut
    let hut = (0..g.corner_owner.len()).find(|&c| g.corner_owner[c] == 0).unwrap();
    let k = g.fleet.iter().position(|&h| g.board.graph.hex_corners[h as usize].contains(&(hut as u16))).expect("the fleet passes seat 1's Hut");
    g.fleet_at = ((k + g.fleet.len() - 1) % g.fleet.len()) as u8;
    g.hands[0] = [3, 0, 0, 0, 0, 0, 0, 0];
    let winners = g.fleet_sails(1);
    assert!(winners.is_empty());
    assert_eq!(g.cards(0), 2, "no warships: 1 card lost (no Lodges)");
    // With 2 warships against a 1, seat 1 picks a resource
    g.fleet_at = ((k + g.fleet.len() - 1) % g.fleet.len()) as u8;
    for e in 0..2usize {
        g.ship_line.push((0, 1000 + e as u16));
    }
    g.warship.resize(2000, false);
    g.warship[1000] = true;
    g.warship[1001] = true;
    assert_eq!(g.fleet_sails(1), vec![Pending { seat: 0, decision: Decision::Pick { left: 1 } }]);
}

#[test]
fn pirate_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        let s = Settings::parse_for("sea=true;sea.scenario=7", players).unwrap();
        for seed in 0..15 {
            play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
            let o = play_with(&s, seed, players, 800, &[BotKind::Greedy]).unwrap();
            assert_ne!(o.winner, NOBODY, "{players} players, game {seed} didn't finish");
        }
    }
}
