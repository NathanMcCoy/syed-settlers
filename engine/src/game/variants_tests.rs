// The variants box's variants
use super::*;
use crate::island::generate;

fn game_with(players: usize, text: &str) -> Game {
    let s = Settings::parse_for(text, players).unwrap();
    let mut g = Game::new(&generate(&s, "variants").unwrap(), &s, players, 7);
    g.turn = 1;
    g.phase = Phase::Main;
    g
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

fn put(g: &mut Game, c: usize, p: u8, kind: u8) {
    g.corner_owner[c] = p;
    g.corner_kind[c] = kind;
    if kind == HUT {
        g.pieces[p as usize][P_HUT] -= 1;
    } else {
        g.pieces[p as usize][P_LODGE] -= 1;
    }
}

#[test]
fn the_friendly_squall_leaves_players_with_2_renown_alone() {
    let mut g = game_with(3, "variant.friendly=true");
    // Seat 2 has 2 Renown (two Huts), seat 3 has 3 (a Lodge and a Hut)
    let land = (0..g.board.resource.len()).filter(|&h| g.board.resource[h] < 5).collect::<Vec<_>>();
    let (a, b) = (land[0], land[land.len() - 1]);
    let ca = g.board.graph.hex_corners[a][0] as usize;
    let cb = g.board.graph.hex_corners[b][3] as usize;
    put(&mut g, ca, 1, HUT);
    put(&mut g, cb, 2, LODGE);
    let far = (0..g.corner_owner.len()).find(|&c| g.spaced(c)).unwrap();
    put(&mut g, far, 1, HUT);
    let far2 = (0..g.corner_owner.len()).find(|&c| g.spaced(c)).unwrap();
    put(&mut g, far2, 2, HUT);
    assert_eq!((g.public_renown(1), g.public_renown(2)), (2, 3));
    g.phase = Phase::MoveSquall { anywhere: false };
    g.back = Phase::Main;
    let squalls: Vec<u16> = legal(&g).iter().filter_map(|a| if let Action::Squall(h) = a { Some(*h) } else { None }).collect();
    assert!(!squalls.contains(&(a as u16)), "not next to seat 2's Hut");
    assert!(squalls.contains(&(b as u16)) || b as u16 == g.squall, "seat 3 can be robbed");
    // Without the variant, seat 2's hex is fair game
    let mut plain = game_with(3, "");
    put(&mut plain, ca, 1, HUT);
    put(&mut plain, far, 1, HUT);
    assert!(plain.squall_allowed(a) && !g.squall_allowed(a));
}

#[test]
fn strongest_ports_goes_to_three_renown_on_markets_and_adds_one_to_win() {
    let mut g = game_with(3, "variant.ports=true");
    assert_eq!(g.win_target(0), 11, "10 + 1");
    let markets: Vec<usize> = (0..g.corner_owner.len()).filter(|&c| g.board.corner_market[c] != NOBODY).collect();
    // A Lodge on one market and a Hut on another: 3 Renown on markets
    let a = markets[0];
    let b = *markets.iter().find(|&&c| g.spaced(c) && c != a && !listed(&g.board.graph.corner_corners[a]).any(|n| n == c)).unwrap();
    g.current = 0;
    put(&mut g, a, 0, HUT);
    g.hands[0] = cards_of(LODGE_COST);
    g.refresh_ports();
    assert_eq!(g.holder(Title::StrongestPorts), NOBODY, "1 isn't enough");
    g.apply(Action::Lodge(a as u16)).unwrap();
    assert_eq!(g.holder(Title::StrongestPorts), NOBODY, "2 isn't enough");
    // A Hut on a second market, joined by a Path
    let e = listed(&g.board.graph.corner_edges[b]).find(|&e| g.board.edge_land[e]).unwrap();
    g.edge_owner[e] = 0;
    g.edge_kind[e] = PATH;
    g.hands[0] = cards_of(HUT_COST);
    g.apply(Action::Hut(b as u16)).unwrap();
    assert_eq!((g.port_renown(0), g.holder(Title::StrongestPorts)), (3, 0));
    let without = game_with(3, "");
    assert_eq!(without.win_target(0), 10);
}
