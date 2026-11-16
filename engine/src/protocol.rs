// How games talk to the outside (the server, and through it the page): moves as short text, and the
// game as JSON from one seat's point of view. A seat sees its own hand and projects in full, and
// only counts for everyone else's, so a page (or a bot) can't peek.
//
// Moves as text:
//   hut C | lodge C | path E               C a corner number, E an edge number
//   ship E | moveship E F | pirate H       the sea expansion: build a ship, move one, move the pirate
//   roll | end
//   discard R | take R                     R a resource: 0 Timber, 1 Stone, 2 Grain, 3 Copper, 4 Fish
//   squall H | steal P                     H a hex number, P a seat
//   trade G R                              give G (at your best rate) for one R
//   buy S                                  a project: the face-up row's slot S (hidden: 0)
//   play pilot|roadCrew|harvest|marketDay
//   offer G,G,G,G,G R,R,R,R,R T [open]     to other players: give these 5 counts for those 5; T = seats (a bit each)
//   answer S yes|no                        seat S accepts or declines the offer
//   counter S G,G,G,G,G R,R,R,R,R          seat S proposes instead: what the offerer would give and get
//   accept S                               the offerer takes seat S's counter-offer
//   deal S | withdraw                      complete the offer with seat S (who accepted), or take it back
// The city expansion (K a progress card by name, e.g. irrigation; T a track 0 science, 1 trade, 2 politics):
//   wall C | improve T | recruit C | promote C | activate C | knightmove C D | displace C D | chase C
//   card K | fixedroll R Y | dropcard K | choose X | takecard S R | takeprogress S K | swap H H
//   removepath E | merchant H | expel C | removeknight C | pillage C | skip
use crate::game::*;
use crate::island::{ANY, Island, RESOURCES};
use crate::json;

pub const PROJECT_NAMES: [&str; 5] = ["pilot", "lighthouse", "roadCrew", "harvest", "marketDay"];

pub fn action_text(a: Action) -> String {
    match a {
        Action::Hut(c) => format!("hut {c}"),
        Action::Lodge(c) => format!("lodge {c}"),
        Action::Path(e) => format!("path {e}"),
        Action::Ship(e) => format!("ship {e}"),
        Action::MoveShip(a, b) => format!("moveship {a} {b}"),
        Action::Pirate(h) => format!("pirate {h}"),
        Action::Gift(s, r) => format!("gift {s} {r}"),
        Action::Rob(s) => format!("rob {s}"),
        Action::Damage(e) => format!("damage {e}"),
        Action::Repair(e) => format!("repair {e}"),
        Action::ClaimWonder(w) => format!("claimwonder {w}"),
        Action::BuildWonder => "buildwonder".into(),
        Action::PlaceMarket(e) => format!("placemarket {e}"),
        Action::NeutralHut(n, c) => format!("neutralhut {n} {c}"),
        Action::NeutralPath(n, e) => format!("neutralpath {n} {e}"),
        Action::CashPilot => "cashpilot".into(),
        Action::ForceTrade => "forcetrade".into(),
        Action::SquallHome => "squallhome".into(),
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
        Action::Wall(c) => format!("wall {c}"),
        Action::Improve(t) => format!("improve {t}"),
        Action::Recruit(c) => format!("recruit {c}"),
        Action::Promote(c) => format!("promote {c}"),
        Action::Activate(c) => format!("activate {c}"),
        Action::KnightMove(a, b) => format!("knightmove {a} {b}"),
        Action::Displace(a, b) => format!("displace {a} {b}"),
        Action::Chase(c) => format!("chase {c}"),
        Action::PlayCard(k) => format!("card {}", CARDS[k as usize].0),
        Action::FixedRoll(a, b) => format!("fixedroll {a} {b}"),
        Action::DropCard(k) => format!("dropcard {}", CARDS[k as usize].0),
        Action::Choose(x) => format!("choose {x}"),
        Action::TakeCard(s, k) => format!("takecard {s} {k}"),
        Action::TakeProgress(s, k) => format!("takeprogress {s} {}", CARDS[k as usize].0),
        Action::SwapNumbers(a, b) => format!("swap {a} {b}"),
        Action::RemovePath(e) => format!("removepath {e}"),
        Action::Merchant(h) => format!("merchant {h}"),
        Action::Expel(c) => format!("expel {c}"),
        Action::RemoveKnight(c) => format!("removeknight {c}"),
        Action::Pillage(c) => format!("pillage {c}"),
        Action::Skip => "skip".into(),
        Action::Fish(k) => format!("fish {k}"),
        Action::GiveBoot(s) => format!("giveboot {s}"),
        Action::Bridge(e) => format!("bridge {e}"),
        Action::BuyCoin(r) => format!("buycoin {r}"),
        Action::SpendCoins(r) => format!("spendcoins {r}"),
    }
}

// A progress card by name
fn card_named(word: Option<&&str>, text: &str) -> Result<u8, String> {
    word.and_then(|w| CARDS.iter().position(|c| c.0 == *w)).map(|k| k as u8).ok_or_else(|| format!("unknown progress card in \"{text}\""))
}

// 5 counts (resources), or 8 when commodities are in it (the city expansion)
fn counts(h: &Cards) -> String {
    let n = if h[5..].iter().all(|&x| x == 0) { 5 } else { 8 };
    h[..n].iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
}

fn parse_counts(word: Option<&&str>, text: &str) -> Result<Cards, String> {
    let bad = || format!("expected 5 counts like 1,0,0,2,0 (or 8, with commodities) in \"{text}\"");
    let parts: Vec<&str> = word.ok_or_else(bad)?.split(',').collect();
    if parts.len() != 5 && parts.len() != 8 {
        return Err(bad());
    }
    let mut h = [0u8; 8];
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
        Some("ship") => Action::Ship(num(1)?),
        Some("moveship") => Action::MoveShip(num(1)?, num(2)?),
        Some("pirate") => Action::Pirate(num(1)?),
        Some("gift") => Action::Gift(small(1)?, small(2)?),
        Some("rob") => Action::Rob(small(1)?),
        Some("damage") => Action::Damage(num(1)?),
        Some("repair") => Action::Repair(num(1)?),
        Some("claimwonder") => Action::ClaimWonder(small(1)?),
        Some("buildwonder") => Action::BuildWonder,
        Some("placemarket") => Action::PlaceMarket(num(1)?),
        Some("neutralhut") => Action::NeutralHut(small(1)?, num(2)?),
        Some("neutralpath") => Action::NeutralPath(small(1)?, num(2)?),
        Some("cashpilot") => Action::CashPilot,
        Some("forcetrade") => Action::ForceTrade,
        Some("squallhome") => Action::SquallHome,
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
        Some("wall") => Action::Wall(num(1)?),
        Some("improve") => Action::Improve(small(1)?),
        Some("recruit") => Action::Recruit(num(1)?),
        Some("promote") => Action::Promote(num(1)?),
        Some("activate") => Action::Activate(num(1)?),
        Some("knightmove") => Action::KnightMove(num(1)?, num(2)?),
        Some("displace") => Action::Displace(num(1)?, num(2)?),
        Some("chase") => Action::Chase(num(1)?),
        Some("card") => Action::PlayCard(card_named(words.get(1), text)?),
        Some("fixedroll") => Action::FixedRoll(small(1)?, small(2)?),
        Some("dropcard") => Action::DropCard(card_named(words.get(1), text)?),
        Some("choose") => Action::Choose(small(1)?),
        Some("takecard") => Action::TakeCard(small(1)?, small(2)?),
        Some("takeprogress") => Action::TakeProgress(small(1)?, card_named(words.get(2), text)?),
        Some("swap") => Action::SwapNumbers(num(1)?, num(2)?),
        Some("removepath") => Action::RemovePath(num(1)?),
        Some("merchant") => Action::Merchant(num(1)?),
        Some("expel") => Action::Expel(num(1)?),
        Some("removeknight") => Action::RemoveKnight(num(1)?),
        Some("pillage") => Action::Pillage(num(1)?),
        Some("skip") => Action::Skip,
        Some("fish") => Action::Fish(small(1)?),
        Some("giveboot") => Action::GiveBoot(small(1)?),
        Some("bridge") => Action::Bridge(num(1)?),
        Some("buycoin") => Action::BuyCoin(small(1)?),
        Some("spendcoins") => Action::SpendCoins(small(1)?),
        _ => return Err(format!("unknown move \"{text}\"")),
    };
    let expected = match a {
        Action::Roll | Action::EndTurn | Action::Withdraw | Action::BuildWonder | Action::CashPilot | Action::ForceTrade | Action::SquallHome | Action::Skip => 1,
        Action::Trade(..) | Action::Answer(..) | Action::MoveShip(..) | Action::Gift(..) | Action::NeutralHut(..) | Action::NeutralPath(..) => 3,
        Action::KnightMove(..) | Action::Displace(..) | Action::FixedRoll(..) | Action::TakeCard(..) | Action::TakeProgress(..) | Action::SwapNumbers(..) => 3,
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
        Phase::Decide => "decide",
        Phase::Rolled { .. } => "rolled",
        Phase::SecondRoll => "secondRoll",
        Phase::MoveSquall { .. } => "moveSquall",
        Phase::Steal => "steal",
        Phase::Main => "main",
        Phase::FreeBuild { .. } => "freeBuild",
        Phase::Harvest { .. } => "harvest",
        Phase::MarketDay => "marketDay",
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
                "{{\"cards\":{},\"renown\":{},\"pieces\":{},\"projects\":{projects},\"projectCount\":{held},\"line\":{},\"pilots\":{},\"discard\":{},\"tokens\":{{{}}},\"target\":{}}}",
                g.cards(p), g.renown(p) - hidden, list(g.pieces[p]), g.line[p], g.pilots[p], g.discards_left(p),
                // Tokens by name (tokens.rs), and the Renown this player needs to win
                TOKENS.iter().map(|&t| format!("\"{}\":{}", t.name(), g.tokens_of(p, t))).collect::<Vec<_>>().join(","),
                g.win_target(p),
            )
        })
        .collect();
    let mine = seat != NOBODY && (seat as usize) < g.players;
    let (hand, rates) = if mine {
        (list(g.hands[seat as usize][..5].iter()), list((0..5).map(|r| g.rate(seat, r))))
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
    // The open offer, which everyone sees, with each seat's answer (null: none yet). Counts: 5
    // resources, or 8 with the city expansion's commodities
    let kinds = if g.city() { CARD_KINDS } else { 5 };
    let cards = |h: &Cards| list(h[..kinds].iter());
    let offer = match &g.offer {
        None => "null".into(),
        Some(o) => format!(
            "{{\"by\":{},\"give\":{},\"get\":{},\"to\":{},\"open\":{},\"answers\":{},\"counters\":{}}}",
            g.current, cards(&o.give), cards(&o.get),
            list((0..g.players).filter(|&s| o.to & (1 << s) != 0)),
            o.open,
            list((0..g.players).map(|s| match o.answers[s] {
                ACCEPTED => "true", DECLINED => "false", COUNTERED => "\"counter\"", _ => "null",
            })),
            // Each seat's counter-offer: what the offerer would give and get instead
            list((0..g.players).map(|s| match o.counters[s] {
                Some((give, get)) => format!("{{\"give\":{},\"get\":{}}}", cards(&give), cards(&get)),
                None => "null".into(),
            })),
        ),
    };
    let can_offer = mine && seat == g.current && g.phase == Phase::Main && g.offers_left();
    // Offers left this turn (null: no limit)
    let offers_left = if s.trade_offers == 0 { "null".into() } else { s.trade_offers.saturating_sub(g.offers_made).to_string() };
    let row = list(g.row.iter().map(|&k| json::string(PROJECT_NAMES[k as usize])));
    format!(
        concat!(
            "{{\"phase\":\"{}\",\"current\":{},\"toAct\":{},\"turn\":{},\"lastRoll\":{},\"squall\":{},\"winner\":{},",
            "\"buildings\":[{}],\"roads\":[{}],\"players\":[{}],\"longestPath\":{},\"mostPilots\":{},",
            "\"row\":{},\"deck\":{},\"lastTake\":{},\"win\":{},\"you\":{},\"hand\":{},\"rates\":{},\"legal\":{},",
            "\"offer\":{},\"canOffer\":{},\"offersLeft\":{},\"dice\":{},\"bank\":{},\"short\":{},\"handLimit\":{},\"round\":{},\"roundLimit\":{},\"byRounds\":{},\"bought\":{},\"turnOf\":{},\"paired\":{},\"titles\":{},\"deciding\":{},\"pirate\":{},\"shipMoved\":{},\"event\":{},\"damaged\":{},\"revealed\":{},\"villages\":{},\"clothEnded\":{},\"wonders\":{},\"markers\":{},\"prizes\":{},\"heldMarkets\":{},\"markets\":{},\"fleet\":{},\"forts\":{},\"warships\":{},\"lastAttack\":{},\"city\":{},\"fishing\":{},\"rivers\":{}}}"
        ),
        // While decisions are being made (pending.rs), the phase is named after the first one ("discard")
        g.deciding().map_or(phase_name(g.phase), |d| d.decision.name()), g.current, g.to_act(), g.turn,
        if g.last_roll == 0 { "null".into() } else { g.last_roll.to_string() },
        g.squall, seat_or_null(g.winner),
        buildings.join(","), roads.join(","), players.join(","),
        seat_or_null(g.holder(Title::LongestPath)), seat_or_null(g.holder(Title::MostPilots)),
        row, g.deck.len(),
        // What each seat handed over on the last Market Day (public: everyone saw it happen)
        list(&g.last_take[..g.players]),
        s.win,
        if mine { seat.to_string() } else { "null".into() },
        hand, rates, legal,
        offer, can_offer, offers_left,
        if g.last_dice[0] == 0 { "null".into() } else { list(g.last_dice) },
        // What's left in the bank (null: it never runs out), and what the last roll couldn't pay
        if s.bank_cards == 0 { "null".into() } else { list((0..5).map(|r| g.bank(r))) },
        list((0..5).filter(|r| g.short & (1 << r) != 0)),
        s.discard_over,
        g.round(), s.round_limit, g.by_rounds,
        // Projects this seat bought this turn (they can't be played until its next turn)
        if mine && seat == g.current && g.turn > 0 { list(g.bought) } else { list([0u8; 5]) },
        // Whose turn it is, and whether `current` is the paired player's extra build phase
        g.owner, g.paired,
        // Every title (titles.rs): its name, holder, minimum and Renown, so new titles need no page code
        list(TITLES.iter().filter(|t| t.active(s)).map(|&t| format!(
            "{{\"name\":\"{}\",\"holder\":{},\"min\":{},\"renown\":{}}}",
            t.name(), seat_or_null(g.holder(t)), t.min(s), t.renown(s),
        ))),
        // The decision being made now (pending.rs): who, and what kind
        match g.deciding() {
            Some(d) => format!("{{\"seat\":{},\"decision\":\"{}\"}}", d.seat, d.decision.name()),
            None => "null".into(),
        },
        // The sea expansion: the pirate's hex (null without the sea), and whether a ship moved this phase
        if g.pirate == crate::hex::NONE { "null".into() } else { g.pirate.to_string() },
        g.ship_moved,
        // Event cards: the last card's number and event, and the damaged Paths
        if g.last_event == NOBODY { "null".into() } else {
            format!("{{\"number\":{},\"name\":\"{}\"}}", g.last_roll, EVENTS[g.last_event as usize].name())
        },
        list((0..g.damaged.len()).filter(|&e| g.damaged[e])),
        // Fog Islands: hexes revealed so far, as [hex, terrain, number or null]
        list(g.revealed.iter().map(|&h| {
            let hex = &g.board.hexes[h as usize];
            format!("[{h},\"{}\",{}]", hex.terrain.name(), if hex.number == 0 { "null".into() } else { hex.number.to_string() })
        })),
        // Cloth: each village as [hex, cloth left, [linked seats]]; whether 5 empty villages ended it
        list(g.villages().into_iter().enumerate().map(|(k, h)| format!("[{h},{},{}]", g.village_cloth[k], list((0..g.players).filter(|&p| g.village_links[p] & (1 << k) != 0))))),
        g.cloth_ended,
        // Wonders: each wonder [name, owner or null, its owner's level, cost]; corner markers [corner, kind]
        if !g.wonders_on() { "[]".into() } else {
            list(WONDERS.iter().map(|&w| {
                let o = g.wonder_owner[w as usize];
                let level = if o == NOBODY { 0 } else { g.wonder_level[o as usize] };
                format!("[\"{}\",{},{level},{}]", w.name(), seat_or_null(o), list(w.cost()))
            }))
        },
        list((0..g.board.corner_marker.len()).filter(|&c| g.board.corner_marker[c] != 0).map(|c| format!("[{c},{}]", g.board.corner_marker[c]))),
        // The Forgotten Tribe: prizes left as [edge, kind]; each player's markets taken but not placed;
        // every market now on the board (placed ones included) as [hex, side, kind]
        list((0..g.prizes.len()).filter(|&e| g.prizes[e] != 0).map(|e| format!("[{e},{}]", g.prizes[e]))),
        list((0..g.players).map(|p| list(g.held_markets[p].iter().map(|&k| json::string(market_name(k)))))),
        list(g.board.markets.iter().map(|m| format!("[{},{},{}]", m.hex, m.side, json::string(market_name(m.kind))))),
        // The Pirate Islands: the fleet's hex; per seat [fortress corner, lairs left, taken, beachhead
        // corner]; warship edges; the last fortress attack [seat, die, warships] (seat null: none yet)
        if g.fleet.is_empty() { "null".into() } else { g.fleet[g.fleet_at as usize].to_string() },
        if !g.pirate_islands() { "[]".into() } else {
            list((0..g.players).map(|p| format!("[{},{},{},{}]", g.fort_corner[p], g.lairs[p], g.fort_taken[p], g.beach_corner[p])))
        },
        list((0..g.warship.len()).filter(|&e| g.warship[e])),
        if g.last_attack[0] == NOBODY { "null".into() } else { list(g.last_attack) },
        city_json(g, seat, mine),
        fishing_json(g, seat, mine),
        rivers_json(g),
    )
}

// Rivers' part of the view (null without it): everyone's coins (public, like the tiles they decide),
// bridges built and in hand, the Wealthiest and the Poor, coin purchases left this turn
fn rivers_json(g: &Game) -> String {
    if !g.rivers() {
        return "null".into();
    }
    format!(
        "{{\"coins\":{},\"bridges\":{},\"bridgesLeft\":{},\"wealthiest\":{},\"poor\":{},\"coinBuysLeft\":{}}}",
        list(&g.coins[..g.players]),
        list((0..g.bridge.len()).filter(|&e| g.bridge[e])),
        list(&g.bridges_left[..g.players]),
        seat_or_null(g.wealthiest()),
        list((0..g.players).filter(|&p| g.poor(p))),
        2u8.saturating_sub(g.coin_buys),
    )
}

// Fishing's part of the view (null without it): the Lake and its numbers, how many tokens each player
// holds (their values only to their owner), who has the old boot, the bag, and the costs
fn fishing_json(g: &Game, seat: u8, mine: bool) -> String {
    if !g.fishing() {
        return "null".into();
    }
    format!(
        "{{\"lake\":{},\"lakeNumbers\":{},\"tokens\":{},\"mine\":{},\"fish\":{},\"boot\":{},\"bag\":{},\"costs\":{}}}",
        g.lake, list(LAKE_NUMBERS),
        list((0..g.players).map(|p| g.fish[p].len())),
        if mine { list(g.fish[seat as usize].iter()) } else { "null".into() },
        if mine { g.fish_total(seat as usize).to_string() } else { "null".into() },
        seat_or_null(g.boot), g.fish_bag.len(), list(FISH_COSTS),
    )
}

// The city expansion's part of the view (null without it). A seat sees its own progress cards and
// commodities; for everyone else only counts.
fn city_json(g: &Game, seat: u8, mine: bool) -> String {
    if !g.city() {
        return "null".into();
    }
    let cs = &g.cs;
    let names = |cards: &[u8]| list(cards.iter().map(|&k| json::string(CARDS[k as usize].0)));
    let me = seat as usize;
    // While this seat makes a Spy decision, it sees that player's progress cards
    let spied = match g.deciding() {
        Some(Pending { seat: s, decision: Decision::Spy { from } }) if mine && s == seat => names(&cs.progress[from as usize]),
        _ => "null".into(),
    };
    format!(
        concat!(
            "{{\"levels\":{},\"walls\":{},\"metro\":{},\"knights\":{},\"raiders\":{},\"raidersAt\":{},\"landed\":{},",
            "\"lastRaid\":{},\"eventDie\":{},\"decks\":{},\"progress\":{},\"progressCounts\":{},\"cardRenown\":{},",
            "\"merchant\":{},\"goods\":{},\"goodsRates\":{},\"goodsBank\":{},\"improveCost\":{},\"crane\":{},\"medicine\":{},",
            "\"fleet\":{},\"spied\":{},\"drawn\":{},\"handLimits\":{},\"defence\":{},\"strength\":{}}}"
        ),
        list((0..g.players).map(|p| list(cs.levels[p]))),
        list((0..cs.walls.len()).filter(|&c| cs.walls[c])),
        list((0..3).map(|t| if cs.metro_owner[t] == NOBODY { "null".to_string() } else { format!("[{},{}]", cs.metro[t], cs.metro_owner[t]) })),
        // [corner, owner, level, active, activated this turn]
        list((0..cs.knight_owner.len()).filter(|&c| cs.knight_owner[c] != NOBODY).map(|c| {
            format!("[{c},{},{},{},{}]", cs.knight_owner[c], cs.knight_level[c], cs.knight_active[c], cs.knight_fresh[c])
        })),
        cs.raiders, g.settings().city_raiders, cs.landed,
        if cs.last_raid == [0, 0, 0] { "null".into() } else { list(cs.last_raid) },
        if cs.event_die == NOBODY { "null".into() } else { cs.event_die.to_string() },
        list(cs.decks.iter().map(|d| d.len())),
        if mine { names(&cs.progress[me]) } else { "null".into() },
        list((0..g.players).map(|p| cs.progress[p].len())),
        list(&cs.card_renown[..g.players]),
        if cs.merchant_owner == NOBODY { "null".into() } else { format!("[{},{}]", cs.merchant, cs.merchant_owner) },
        if mine { list(g.hands[me][5..].iter()) } else { "null".into() },
        if mine { list((5..8).map(|k| g.rate(seat, k))) } else { "null".into() },
        if g.settings().bank_goods == 0 { "null".into() } else { list((5..8).map(|k| g.bank(k))) },
        if mine { list((0..3).map(|t| g.improve_cost(me, t))) } else { "null".into() },
        cs.crane, cs.medicine,
        if cs.fleet == NOBODY { "null".into() } else { cs.fleet.to_string() },
        spied,
        list(cs.drawn.iter().map(|&(s, d)| format!("[{s},{d}]"))),
        list((0..g.players).map(|p| g.hand_limit(p))),
        list((0..g.players).map(|p| g.defence(p))),
        g.raider_strength(),
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
            Action::Play(Project::MarketDay), Action::Play(Project::RoadCrew), Action::Take(4), Action::Steal(2), Action::Discard(0),
            Action::Offer { give: [2, 0, 0, 0, 1, 0, 0, 0], get: [0, 0, 0, 1, 0, 0, 0, 0], to: 6, open: false },
            Action::Offer { give: [1, 0, 0, 0, 0, 0, 0, 0], get: [0, 0, 0, 0, 2, 0, 0, 0], to: 14, open: true },
            Action::Counter { seat: 2, give: [1, 0, 1, 0, 0, 0, 0, 0], get: [0, 0, 0, 1, 0, 0, 0, 0] }, Action::Accept(3),
            Action::Answer(2, true), Action::Answer(1, false), Action::Deal(3), Action::Withdraw,
        ] {
            assert_eq!(parse_action(&action_text(a)), Ok(a));
        }
        assert_eq!(action_text(Action::Offer { give: [2, 0, 0, 0, 1, 0, 0, 0], get: [0, 0, 0, 1, 0, 0, 0, 0], to: 6, open: false }), "offer 2,0,0,0,1 0,0,0,1,0 6");
        assert_eq!(action_text(Action::Offer { give: [2, 0, 0, 0, 1, 0, 0, 0], get: [0, 0, 0, 1, 0, 0, 0, 0], to: 6, open: true }), "offer 2,0,0,0,1 0,0,0,1,0 6 open");
    }

    #[test]
    fn bad_text_is_refused() {
        for t in [
            "", "jump", "hut", "hut x", "hut 1 2", "trade 1", "play lighthouse!", "play dance", "play winds", "play shipwright", "play bargain", "boat 12", "discard 300", "roll now",
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
        g.hands[0] = [1, 2, 3, 4, 5, 0, 0, 0];
        g.hands[1] = [9, 0, 0, 0, 0, 0, 0, 0];
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
        g.hands[0] = cards_of(PROJECT_COST);
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
        let renown = |v: &str, p: usize| -> String { v.split(",\"renown\":").nth(p + 1).unwrap().split(',').next().unwrap().to_string() };
        assert_eq!(renown(&view_json(&g, 0), 1), "0");
        assert_eq!(renown(&view_json(&g, 1), 1), "2");
        assert!(view_json(&g, 0).contains("\"projects\":null"));
    }
}

#[cfg(test)]
mod groundwork_tests {
    use super::*;
    use crate::island::generate;
    use crate::settings::Settings;

    // The view lists titles, tokens, targets and the decision being made generically, so the page
    // shows new ones without new code
    #[test]
    fn the_view_lists_titles_tokens_targets_and_decisions() {
        let s = Settings::default();
        let island = generate(&s, "view").unwrap();
        let mut g = Game::new(&island, &s, 3, 1);
        g.give_tokens(1, Token::Renown, 2);
        g.give_tokens(1, Token::Trade, 1);
        let v = view_json(&g, 0);
        assert!(v.contains("\"titles\":[{\"name\":\"longestPath\",\"holder\":null,\"min\":5,\"renown\":2},{\"name\":\"mostPilots\",\"holder\":null,\"min\":3,\"renown\":2}]"));
        assert!(v.contains("\"tokens\":{\"renown\":2,\"trade\":1,\"cloth\":0},\"target\":10"));
        assert!(v.contains("\"deciding\":null"));
        assert_eq!(g.renown(1), 2, "Renown tokens count");
        assert!(!g.spend_tokens(1, Token::Trade, 2) && g.spend_tokens(1, Token::Trade, 1));
        // A 7 with a big hand: seat 2 decides what to lose
        g.turn = 1;
        g.phase = Phase::PreRoll;
        g.hands[2] = [9, 0, 0, 0, 0, 0, 0, 0];
        g.rolled(7);
        let v = view_json(&g, 2);
        assert!(v.contains("\"phase\":\"discard\"") && v.contains("\"deciding\":{\"seat\":2,\"decision\":\"discard\"}"));
    }
}
