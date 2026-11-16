// Event cards (the variants box)
use super::*;
use crate::island::generate;

fn game_with(players: usize, text: &str) -> Game {
    let s = Settings::parse_for(text, players).unwrap();
    let mut g = Game::new(&generate(&s, "events").unwrap(), &s, players, 7);
    g.turn = 1;
    g.phase = Phase::PreRoll;
    g
}

fn legal(g: &Game) -> Vec<Action> {
    let mut v = Vec::new();
    g.legal(&mut v);
    v
}

// The card in the deck with this event (and not a 7)
fn card_of(e: Event) -> u8 {
    (0..36).find(|&c| event_card(c).1 == e).unwrap()
}

#[test]
fn the_deck_has_the_two_dice_numbers_and_a_new_year_after_five_from_the_end() {
    let mut counts = [0; 13];
    for c in 0..36 {
        let (n, e) = event_card(c);
        counts[n as usize] += 1;
        assert_eq!(n == 7, e == Event::SquallAttacks, "every 7 is the squall attacking");
    }
    assert_eq!(&counts[2..], &[1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1]);
    let g = game_with(3, "variant.events=true");
    assert_eq!(g.event_deck.len(), 37);
    assert_eq!(g.event_deck[5], NEW_YEAR, "5 cards under the New Year card");
    // After 31 draws the New Year comes up and the deck is rebuilt
    let mut g = g;
    for _ in 0..31 {
        g.event_deck.pop();
    }
    g.phase = Phase::PreRoll;
    g.apply(Action::Roll).unwrap();
    assert_eq!(g.event_deck.len(), 36, "a fresh 37-card deck, one drawn");
    assert!(game_with(3, "").event_deck.is_empty(), "no deck without the variant");
}

#[test]
fn a_card_replaces_the_dice_and_its_number_produces() {
    let mut g = game_with(3, "variant.events=true");
    g.event_deck.push(card_of(Event::BeautifulDay));
    let (n, _) = event_card(card_of(Event::BeautifulDay));
    g.apply(Action::Roll).unwrap();
    assert_eq!((g.last_roll, g.last_dice, g.phase), (n, [0, 0], Phase::Main));
    assert_eq!(g.last_event, Event::BeautifulDay as u8);
}

#[test]
fn an_epidemic_pays_lodges_only_one() {
    let mut g = game_with(3, "variant.events=true");
    let card = card_of(Event::Epidemic);
    let (n, _) = event_card(card);
    let h = g.board.by_number[n as usize][0] as usize;
    let r = g.board.resource[h] as usize;
    let c = g.board.graph.hex_corners[h][0] as usize;
    g.corner_owner[c] = 0;
    g.corner_kind[c] = LODGE;
    g.squall = g.board.land_hexes.iter().copied().find(|&x| x as usize != h && g.board.number[x as usize] != n).unwrap();
    g.hands = [[0; 5]; MAX_PLAYERS];
    g.event_deck.push(card);
    g.apply(Action::Roll).unwrap();
    assert!(g.hands[0][r] >= 1 && g.hands[0][r] <= (g.board.by_number[n as usize].len()) as u8, "1 per Lodge per hex");
    assert!(!g.epidemic, "only for that production");
}

#[test]
fn good_neighbours_pass_a_card_to_the_left() {
    let mut g = game_with(3, "variant.events=true");
    g.hands = [[1, 0, 0, 0, 0], [0, 2, 0, 0, 0], [0; 5], [0; 5], [0; 5], [0; 5]];
    g.event_deck.push(card_of(Event::GoodNeighbours));
    g.apply(Action::Roll).unwrap();
    // Seat 3 has nothing to give: only seats 1 and 2 decide
    assert_eq!(g.deciding().unwrap(), Pending { seat: 0, decision: Decision::Gift { to: 1 } });
    assert_eq!(legal(&g), vec![Action::Gift(1, 0)]);
    g.apply(Action::Gift(1, 0)).unwrap();
    assert_eq!(g.deciding().unwrap().seat, 1);
    g.apply(Action::Gift(2, 1)).unwrap();
    assert_eq!(g.phase, Phase::Main, "then the number produced");
    assert_eq!((g.hands[0][0], g.hands[1][0], g.hands[1][1], g.hands[2][1]), (0, 1, 1, 1));
}

#[test]
fn an_earthquake_damages_a_path_which_blocks_paths_and_huts_until_repaired() {
    let mut g = game_with(3, "variant.events=true");
    let c = (0..g.corner_owner.len()).find(|&c| g.spaced(c)).unwrap();
    let e = listed(&g.board.graph.corner_edges[c]).find(|&e| g.board.edge_land[e]).unwrap();
    g.corner_owner[c] = 0;
    g.corner_kind[c] = HUT;
    g.edge_owner[e] = 0;
    g.edge_kind[e] = PATH;
    g.event_deck.push(card_of(Event::Earthquake));
    g.apply(Action::Roll).unwrap();
    assert_eq!(legal(&g), vec![Action::Damage(e as u16)]);
    g.apply(Action::Damage(e as u16)).unwrap();
    assert!(g.damaged[e]);
    g.phase = Phase::Main;
    g.current = 0;
    g.hands[0] = [3, 0, 0, 3, 0];
    let list = legal(&g);
    assert!(!list.iter().any(|a| matches!(a, Action::Path(_))), "no new Paths until it's repaired");
    assert!(list.contains(&Action::Repair(e as u16)));
    g.apply(Action::Repair(e as u16)).unwrap();
    assert!(!g.damaged[e] && g.hands[0] == [2, 0, 0, 2, 0]);
    assert!(legal(&g).iter().any(|a| matches!(a, Action::Path(_))));
}

#[test]
fn trade_advantage_lets_the_longest_path_rob_anyone_and_a_plentiful_year_feeds_everyone() {
    let mut g = game_with(3, "variant.events=true");
    g.holders[Title::LongestPath as usize] = 2;
    g.hands = [[0, 0, 3, 0, 0], [0; 5], [0; 5], [0; 5], [0; 5], [0; 5]];
    g.event_deck.push(card_of(Event::TradeAdvantage));
    g.apply(Action::Roll).unwrap();
    assert_eq!(g.deciding().unwrap(), Pending { seat: 2, decision: Decision::Rob });
    g.apply(Action::Rob(0)).unwrap();
    assert_eq!((g.hands[0][2], g.hands[2][2]), (2, 1));
    let mut g = game_with(3, "variant.events=true");
    g.event_deck.push(card_of(Event::PlentifulYear));
    g.hands = [[0; 5]; MAX_PLAYERS];
    g.apply(Action::Roll).unwrap();
    for seat in [0u8, 1, 2] {
        assert_eq!(g.deciding().unwrap().seat, seat);
        g.apply(Action::Take(seat)).unwrap();
    }
    assert_eq!((g.hands[0][0], g.hands[1][1], g.hands[2][2]), (1, 1, 1));
}

#[test]
fn event_games_never_get_stuck_and_greedy_finishes_them() {
    use crate::sim::{BotKind, play_with};
    for players in [2, 3, 4, 6] {
        for text in ["variant.events=true", "variant.events=true;variant.friendly=true;variant.ports=true", "variant.events=true;sea=true"] {
            let s = Settings::parse_for(text, players).unwrap();
            for seed in 0..15 {
                play_with(&s, seed, players, 300, &[BotKind::Random]).unwrap();
                let o = play_with(&s, seed, players, 400, &[BotKind::Greedy]).unwrap();
                assert_ne!(o.winner, NOBODY, "{players} players, {text}, game {seed} didn't finish");
            }
        }
    }
}
