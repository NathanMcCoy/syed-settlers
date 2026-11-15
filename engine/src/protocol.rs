// How games talk to the outside (the server, and through it the page): moves as short text, and the
// game as JSON from one seat's point of view. A seat sees its own hand and projects in full, and
// only counts for everyone else's, so a page (or a bot) can't peek.
//
// Moves as text:
//   hut C | lodge C | path E               C a corner number, E an edge number
//   roll | end
//   discard R | take R                     R a resource: 0 Timber, 1 Stone, 2 Grain, 3 Copper, 4 Fish
//   squall H | steal P                     H a hex number, P a seat
//   trade G R                              give G (at your best rate) for one R
//   buy S                                  a project: the face-up row's slot S (hidden: 0)
//   play pilot|roadCrew|harvest|bargain
//   offer G,G,G,G,G R,R,R,R,R T [open]     to other players: give these 5 counts for those 5; T = seats (a bit each)
//   answer S yes|no                        seat S accepts or declines the offer
//   counter S G,G,G,G,G R,R,R,R,R          seat S proposes instead: what the offerer would give and get
//   accept S                               the offerer takes seat S's counter-offer
//   deal S | withdraw                      complete the offer with seat S (who accepted), or take it back
use crate::game::*;
use crate::island::{ANY, Island, RESOURCES};
use crate::json;

pub const PROJECT_NAMES: [&str; 5] = ["pilot", "lighthouse", "roadCrew", "harvest", "bargain"];

pub fn action_text(a: Action) -> String {
    match a {
        Action::Hut(c) => format!("hut {c}"),
        Action::Lodge(c) => format!("lodge {c}"),
        Action::Path(e) => format!("path {e}"),
        Action::Roll => "roll".into(),
        Action::Discard(r) => format!("discard {r}"),
        Action::Squall(h) => format!("squall {h}"),
        Action::Steal(p) => format!("steal {p}"),
        Action::Trade(g, r) => format!("trade {g} {r}"),
        Action::Buy(s) => format!("buy {s}"),
        Action::Play(k) => format!("play {}", PROJECT_NAMES[k as usize]),
        Action::Take(r) => format!("take {r}"),
        Action::EndTurn => "end".into(),
        Action::Offer { give, get, to, open } => format!("offer {} {} {to}{}", counts(&give), counts(&get), if open { " open" } else { "" }),
        Action::Counter { seat, give, get } => format!("counter {seat} {} {}", counts(&give), counts(&get)),
        Action::Accept(s) => format!("accept {s}"),
        Action::Answer(s, yes) => format!("answer {s} {}", if yes { "yes" } else { "no" }),
        Action::Deal(s) => format!("deal {s}"),
        Action::Withdraw => "withdraw".into(),
    }
}

fn counts(h: &Hand) -> String {
    h.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
}

fn parse_counts(word: Option<&&str>, text: &str) -> Result<Hand, String> {
    let bad = || format!("expected 5 counts like 1,0,0,2,0 in \"{text}\"");
    let parts: Vec<&str> = word.ok_or_else(bad)?.split(',').collect();
    if parts.len() != 5 {
        return Err(bad());
    }
    let mut h = [0u8; 5];
    for (i, p) in parts.iter().enumerate() {
        h[i] = p.parse().map_err(|_| bad())?;
        if h[i] > 30 {
            return Err(bad());
        }
    }
    Ok(h)
}

pub fn parse_action(text: &str) -> Result<Action, String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let num = |i: usize| -> Result<u16, String> {
        words.get(i).and_then(|w| w.parse().ok()).ok_or_else(|| format!("expected a number in \"{text}\""))
    };
    let small = |i: usize| -> Result<u8, String> { num(i).and_then(|n| u8::try_from(n).map_err(|_| format!("number too big in \"{text}\""))) };
    let a = match words.first().copied() {
        Some("hut") => Action::Hut(num(1)?),
        Some("lodge") => Action::Lodge(num(1)?),
        Some("path") => Action::Path(num(1)?),
        Some("roll") => Action::Roll,
        Some("end") => Action::EndTurn,
        Some("discard") => Action::Discard(small(1)?),
        Some("take") => Action::Take(small(1)?),
        Some("squall") => Action::Squall(num(1)?),
        Some("steal") => Action::Steal(small(1)?),
        Some("trade") => Action::Trade(small(1)?, small(2)?),
        Some("buy") => Action::Buy(small(1)?),
        Some("play") => {
            let k = words.get(1).and_then(|w| PROJECT_NAMES.iter().position(|n| n == w)).ok_or_else(|| format!("unknown project in \"{text}\""))?;
            Action::Play(PROJECTS[k])
        }
        Some("offer") => Action::Offer {
            give: parse_counts(words.get(1), text)?,
            get: parse_counts(words.get(2), text)?,
            to: small(3)?,
            open: match words.get(4).copied() {
                None => false,
                Some("open") => true,
                Some(_) => return Err(format!("expected \"open\" or nothing after the seats in \"{text}\"")),
            },
        },
        Some("counter") => Action::Counter { seat: small(1)?, give: parse_counts(words.get(2), text)?, get: parse_counts(words.get(3), text)? },
        Some("accept") => Action::Accept(small(1)?),
        Some("answer") => Action::Answer(small(1)?, match words.get(2).copied() {
            Some("yes") => true,
            Some("no") => false,
            _ => return Err(format!("expected yes or no in \"{text}\"")),
        }),
        Some("deal") => Action::Deal(small(1)?),
        Some("withdraw") => Action::Withdraw,
        _ => return Err(format!("unknown move \"{text}\"")),
    };
    let expected = match a {
        Action::Roll | Action::EndTurn | Action::Withdraw => 1,
        Action::Trade(..) | Action::Answer(..) => 3,
        Action::Offer { open, .. } => if open { 5 } else { 4 },
        Action::Counter { .. } => 4,
        _ => 2,
    };
    if words.len() != expected {
        return Err(format!("unexpected words in \"{text}\""));
    }
    Ok(a)
}

fn phase_name(p: Phase) -> &'static str {
    match p {
        Phase::SetupHut => "setupHut",
        Phase::SetupPath { .. } => "setupPath",
        Phase::PreRoll => "preRoll",
        Phase::Discard => "discard",
        Phase::MoveSquall { .. } => "moveSquall",
        Phase::Steal => "steal",
        Phase::Main => "main",
        Phase::FreeBuild { .. } => "freeBuild",
        Phase::Harvest { .. } => "harvest",
        Phase::Bargain => "bargain",
        Phase::Over => "over",
    }
}

fn list<T: ToString>(items: impl IntoIterator<Item = T>) -> String {
    let v: Vec<String> = items.into_iter().map(|x| x.to_string()).collect();
    format!("[{}]", v.join(","))
}

fn seat_or_null(s: u8) -> String {
    if s == NOBODY { "null".into() } else { s.to_string() }
}

// What doesn't change during a game: the island, plus every corner's and edge's place, for drawing
pub fn board_json(island: &Island, g: &Game) -> String {
    let gr = &g.board.graph;
    let corners = list((0..gr.corners()).map(|c| format!("[{},{}]", gr.corner_xy[c].0, gr.corner_xy[c].1)));
    let edges = list((0..gr.edges()).map(|e| format!("[{},{}]", gr.edge_corners[e][0], gr.edge_corners[e][1])));
    format!(
        "{{\"island\":{},\"corners\":{corners},\"edges\":{edges},\"players\":{},\"settings\":{}}}",
        island.to_json(),
        g.players,
        json::string(&g.settings().to_text()),
    )
}

// The game as `seat` may see it (NOBODY: a watcher, who sees no hand)
pub fn view_json(g: &Game, seat: u8) -> String {
    let s = g.settings();
    let face_up = s.projects_face_up;
    let mut buildings = Vec::new();
    for c in 0..g.corner_owner.len() {
        if g.corner_owner[c] != NOBODY {
            buildings.push(format!("[{c},{},{}]", g.corner_owner[c], g.corner_kind[c]));
        }
    }
    let mut roads = Vec::new();
    for e in 0..g.edge_owner.len() {
        if g.edge_owner[e] != NOBODY {
            roads.push(format!("[{e},{},{}]", g.edge_owner[e], g.edge_kind[e]));
        }
    }
    let players: Vec<String> = (0..g.players)
        .map(|p| {
            let me = p as u8 == seat;
            let held: u32 = g.held[p].iter().map(|&n| n as u32).sum();
            // Hidden Lighthouses aren't shown to others (they count when their owner wins)
            let hidden = if face_up || me || g.over() { 0 } else { g.held[p][Project::Lighthouse as usize] as u32 };
            let projects = if face_up || me || g.over() { list(g.held[p]) } else { "null".into() };
            format!(
                "{{\"cards\":{},\"renown\":{},\"pieces\":{},\"projects\":{projects},\"projectCount\":{held},\"line\":{},\"markets\":{},\"discard\":{}}}",
                g.cards(p), g.renown(p) - hidden, list(g.pieces[p]), g.line[p], g.markets[p], g.discards[p],
            )
        })
        .collect();
    let mine = seat != NOBODY && (seat as usize) < g.players;
    let (hand, rates) = if mine {
        (list(g.hands[seat as usize]), list((0..5).map(|r| g.rate(seat, r))))
    } else {
        ("null".into(), "null".into())
    };
    // Every move this seat may make now (answers to an offer come while it's someone else's turn)
    let legal = if mine {
        let mut v = Vec::new();
        g.legal_for(seat, &mut v);
        format!("[{}]", v.iter().map(|&a| json::string(&action_text(a))).collect::<Vec<_>>().join(","))
    } else {
        "[]".into()
    };
    // The open offer, which everyone sees, with each seat's answer (null: none yet)
    let offer = match &g.offer {
        None => "null".into(),
        Some(o) => format!(
            "{{\"by\":{},\"give\":{},\"get\":{},\"to\":{},\"open\":{},\"answers\":{},\"counters\":{}}}",
            g.current, list(o.give), list(o.get),
            list((0..g.players).filter(|&s| o.to & (1 << s) != 0)),
            o.open,
            list((0..g.players).map(|s| match o.answers[s] {
                ACCEPTED => "true", DECLINED => "false", COUNTERED => "\"counter\"", _ => "null",
            })),
            // Each seat's counter-offer: what the offerer would give and get instead
            list((0..g.players).map(|s| match o.counters[s] {
                Some((give, get)) => format!("{{\"give\":{},\"get\":{}}}", list(give), list(get)),
                None => "null".into(),
            })),
        ),
    };
    let can_offer = mine && seat == g.current && g.phase == Phase::Main && g.offers_made < s.trade_offers;
    let row = list(g.row.iter().map(|&k| json::string(PROJECT_NAMES[k as usize])));
    format!(
        concat!(
            "{{\"phase\":\"{}\",\"current\":{},\"toAct\":{},\"turn\":{},\"lastRoll\":{},\"squall\":{},\"winner\":{},",
            "\"buildings\":[{}],\"roads\":[{}],\"players\":[{}],\"longestPath\":{},\"mostMarkets\":{},",
            "\"row\":{},\"deck\":{},\"bargain\":{},\"win\":{},\"you\":{},\"hand\":{},\"rates\":{},\"legal\":{},",
            "\"offer\":{},\"canOffer\":{},\"offersLeft\":{},\"dice\":{},\"bank\":{},\"short\":{},\"handLimit\":{},\"round\":{},\"roundLimit\":{},\"byRounds\":{},\"bought\":{}}}"
        ),
        phase_name(g.phase), g.current, g.to_act(), g.turn,
        if g.last_roll == 0 { "null".into() } else { g.last_roll.to_string() },
        g.squall, seat_or_null(g.winner),
        buildings.join(","), roads.join(","), players.join(","),
        seat_or_null(g.longest), seat_or_null(g.most_markets),
        row, g.deck.len(),
        if g.bargain == NOBODY { "null".into() } else { json::string(RESOURCES[g.bargain as usize]) },
        s.win,
        if mine { seat.to_string() } else { "null".into() },
        hand, rates, legal,
        offer, can_offer, s.trade_offers.saturating_sub(g.offers_made),
        if g.last_dice[0] == 0 { "null".into() } else { list(g.last_dice) },
        // What's left in the bank (null: it never runs out), and what the last roll couldn't pay
        if s.bank_cards == 0 { "null".into() } else { list((0..5).map(|r| g.bank(r))) },
        list((0..5).filter(|r| g.short & (1 << r) != 0)),
        s.discard_over,
        g.round(), s.round_limit, g.by_rounds,
        // Projects this seat bought this turn (they can't be played until its next turn)
        if mine && seat == g.current && g.turn > 0 { list(g.bought) } else { list([0u8; 5]) },
    )
}

// The market kinds by name, for the page (index = market number)
pub fn market_name(kind: u8) -> &'static str {
    if kind == ANY { "any" } else { RESOURCES[kind as usize] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::generate;
    use crate::settings::Settings;

    #[test]
    fn every_action_survives_text_and_back() {
        let s = Settings::default();
        let island = generate(&s, "proto").unwrap();
        let mut g = Game::new(&island, &s, 3, 1);
        let mut bot = crate::bot::Greedy::new(2);
        let mut list = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..3000 {
            if g.over() {
                break;
            }
            g.legal(&mut list);
            for &a in &list {
                assert_eq!(parse_action(&action_text(a)), Ok(a));
                seen.insert(std::mem::discriminant(&a));
            }
            let a = bot.choose(&g, &list);
            g.apply(a).unwrap();
        }
        assert!(seen.len() >= 10, "most kinds of move came up ({})", seen.len());
        for a in [
            Action::Play(Project::Bargain), Action::Play(Project::RoadCrew), Action::Take(4), Action::Steal(2), Action::Discard(0),
            Action::Offer { give: [2, 0, 0, 0, 1], get: [0, 0, 0, 1, 0], to: 6, open: false },
            Action::Offer { give: [1, 0, 0, 0, 0], get: [0, 0, 0, 0, 2], to: 14, open: true },
            Action::Counter { seat: 2, give: [1, 0, 1, 0, 0], get: [0, 0, 0, 1, 0] }, Action::Accept(3),
            Action::Answer(2, true), Action::Answer(1, false), Action::Deal(3), Action::Withdraw,
        ] {
            assert_eq!(parse_action(&action_text(a)), Ok(a));
        }
        assert_eq!(action_text(Action::Offer { give: [2, 0, 0, 0, 1], get: [0, 0, 0, 1, 0], to: 6, open: false }), "offer 2,0,0,0,1 0,0,0,1,0 6");
        assert_eq!(action_text(Action::Offer { give: [2, 0, 0, 0, 1], get: [0, 0, 0, 1, 0], to: 6, open: true }), "offer 2,0,0,0,1 0,0,0,1,0 6 open");
    }

    #[test]
    fn bad_text_is_refused() {
        for t in [
            "", "jump", "hut", "hut x", "hut 1 2", "trade 1", "play lighthouse!", "play dance", "play winds", "play shipwright", "boat 12", "discard 300", "roll now",
            "offer 1,0,0,0 0,0,0,1,0 2", "offer 1,0,0,0,0 0,0,0,1,0", "offer 99,0,0,0,0 0,0,0,1,0 2", "offer a,0,0,0,0 0,0,0,1,0 2",
            "answer 1 maybe", "answer yes", "deal", "withdraw now", "offer 1,0,0,0,0 0,0,0,1,0 2 closed", "counter 1 1,0,0,0,0", "accept",
        ] {
            assert!(parse_action(t).is_err(), "{t}");
        }
    }

    #[test]
    fn a_seat_sees_only_its_own_hand() {
        let s = Settings::default();
        let island = generate(&s, "view").unwrap();
        let mut g = Game::new(&island, &s, 3, 1);
        g.hands[0] = [1, 2, 3, 4, 5];
        g.hands[1] = [9, 0, 0, 0, 0];
        let v0 = view_json(&g, 0);
        assert!(v0.contains("\"you\":0,\"hand\":[1,2,3,4,5]"));
        assert!(!v0.contains("[9,0,0,0,0]"), "not player 2's hand");
        assert!(v0.contains("\"legal\":[\"hut "), "seat 1 places first");
        let v1 = view_json(&g, 1);
        assert!(v1.contains("\"hand\":[9,0,0,0,0]") && v1.contains("\"legal\":[]"));
        let watcher = view_json(&g, NOBODY);
        assert!(watcher.contains("\"you\":null,\"hand\":null") && !watcher.contains("[1,2,3,4,5]"));
    }

    #[test]
    fn a_buyer_sees_what_they_drew_and_others_only_a_count() {
        let s = Settings::default();   // projects hidden
        let island = generate(&s, "view").unwrap();
        let mut g = Game::new(&island, &s, 2, 1);
        g.turn = 1;
        g.phase = Phase::Main;
        g.hands[0] = PROJECT_COST;
        let top = *g.deck.last().unwrap() as usize;
        g.apply(Action::Buy(0)).unwrap();
        let mut want = [0u8; 5];
        want[top] = 1;
        let mine = view_json(&g, 0);
        assert!(mine.contains(&format!("\"bought\":{}", list(want))), "the buyer's view says what was bought this turn");
        assert!(mine.contains(&format!("\"projects\":{}", list(want))));
        let theirs = view_json(&g, 1);
        assert!(theirs.contains("\"bought\":[0,0,0,0,0]") && theirs.contains("\"projectCount\":1"));
        if top != Project::Lighthouse as usize {
            assert!(theirs.contains("\"projects\":null"), "others don't see which");
        }
    }

    #[test]
    fn hidden_lighthouses_stay_hidden_from_others() {
        let s = Settings::parse("projects.faceUp=false").unwrap();
        let island = generate(&s, "view").unwrap();
        let mut g = Game::new(&island, &s, 2, 1);
        g.held[1][Project::Lighthouse as usize] = 2;
        let renown = |v: &str, p: usize| -> String { v.split("\"renown\":").nth(p + 1).unwrap().split(',').next().unwrap().to_string() };
        assert_eq!(renown(&view_json(&g, 0), 1), "0");
        assert_eq!(renown(&view_json(&g, 1), 1), "2");
        assert!(view_json(&g, 0).contains("\"projects\":null"));
    }
}
