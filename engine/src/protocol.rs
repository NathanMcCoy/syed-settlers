// How games talk to the outside (the server, and through it the page): moves as short text, and the
// game as JSON from one seat's point of view. A seat sees its own hand and projects in full, and
// only counts for everyone else's, so a page (or a bot) can't peek.
//
// Moves as text:
//   hut C | lodge C | path E | boat E      C a corner number, E an edge number
//   roll | end
//   discard R | take R                     R a resource: 0 Timber, 1 Stone, 2 Grain, 3 Copper, 4 Fish
//   squall H | steal P                     H a hex number, P a seat
//   trade G R                              give G (at your best rate) for one R
//   buy S                                  a project: the face-up row's slot S (hidden: 0)
//   play pilot|shipwright|harvest|winds
use crate::game::*;
use crate::island::{ANY, Island, RESOURCES};
use crate::json;

pub const PROJECT_NAMES: [&str; 5] = ["pilot", "lighthouse", "shipwright", "harvest", "winds"];

pub fn action_text(a: Action) -> String {
    match a {
        Action::Hut(c) => format!("hut {c}"),
        Action::Lodge(c) => format!("lodge {c}"),
        Action::Path(e) => format!("path {e}"),
        Action::Boat(e) => format!("boat {e}"),
        Action::Roll => "roll".into(),
        Action::Discard(r) => format!("discard {r}"),
        Action::Squall(h) => format!("squall {h}"),
        Action::Steal(p) => format!("steal {p}"),
        Action::Trade(g, r) => format!("trade {g} {r}"),
        Action::Buy(s) => format!("buy {s}"),
        Action::Play(k) => format!("play {}", PROJECT_NAMES[k as usize]),
        Action::Take(r) => format!("take {r}"),
        Action::EndTurn => "end".into(),
    }
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
        Some("boat") => Action::Boat(num(1)?),
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
        _ => return Err(format!("unknown move \"{text}\"")),
    };
    let expected = match a {
        Action::Roll | Action::EndTurn => 1,
        Action::Trade(..) => 3,
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
        Phase::Winds => "winds",
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
    let legal = if mine && g.to_act() == seat {
        let mut v = Vec::new();
        g.legal(&mut v);
        format!("[{}]", v.iter().map(|&a| json::string(&action_text(a))).collect::<Vec<_>>().join(","))
    } else {
        "[]".into()
    };
    let row = list(g.row.iter().map(|&k| json::string(PROJECT_NAMES[k as usize])));
    format!(
        concat!(
            "{{\"phase\":\"{}\",\"current\":{},\"toAct\":{},\"turn\":{},\"lastRoll\":{},\"squall\":{},\"winner\":{},",
            "\"buildings\":[{}],\"roads\":[{}],\"players\":[{}],\"navigator\":{},\"harbourmaster\":{},",
            "\"row\":{},\"deck\":{},\"winds\":{},\"win\":{},\"you\":{},\"hand\":{},\"rates\":{},\"legal\":{}}}"
        ),
        phase_name(g.phase), g.current, g.to_act(), g.turn,
        if g.last_roll == 0 { "null".into() } else { g.last_roll.to_string() },
        g.squall, seat_or_null(g.winner),
        buildings.join(","), roads.join(","), players.join(","),
        seat_or_null(g.navigator), seat_or_null(g.harbourmaster),
        row, g.deck.len(),
        if g.winds == NOBODY { "null".into() } else { json::string(RESOURCES[g.winds as usize]) },
        s.win,
        if mine { seat.to_string() } else { "null".into() },
        hand, rates, legal,
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
        for a in [Action::Play(Project::Winds), Action::Take(4), Action::Steal(2), Action::Discard(0)] {
            assert_eq!(parse_action(&action_text(a)), Ok(a));
        }
    }

    #[test]
    fn bad_text_is_refused() {
        for t in ["", "jump", "hut", "hut x", "hut 1 2", "trade 1", "play lighthouse!", "play dance", "discard 300", "roll now"] {
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
