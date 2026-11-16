// The engine as WebAssembly, for the server and the page (site/js/engine.js wraps this). Text goes in
// and out through the module's memory:
//   alloc(n) -> where to write n bytes;  free(where, n) when done with them
//   out() -> where the last answer is (its length is what the call returned)
//   island(where, n) -> the answer's length. The input is "<seed>\n<settings text>" (settings.rs);
//     the answer is the island as JSON (island.rs to_json), or "error <why>"
//   settings_table() -> the answer's length: every setting as JSON (Settings::table_json)
//   source_hash() -> the answer's length: the source's fingerprint (build.rs)
// Games live in the module's memory, by number:
//   game_new(where, n) -> the answer's length. Input "<seed>\n<players>\n<settings text>"; answer the
//     game's number, or "error <why>". The seed makes the island and the dice.
//   game_board(id) -> length: what never changes (protocol.rs board_json)
//   game_view(id, seat) -> length: the game as that seat sees it (255: a watcher)
//   game_act(id, seat, where, n) -> length: a move as text (protocol.rs) by that seat; "ok" or "error <why>"
//   game_bot(id) -> length: the bot for the seat to act makes its move; answers the move's text
//   game_bot_answer(id, seat) -> length: that seat's bot answers an open offer ("answer S yes|no"), or "none"
//   game_free(id): forget the game
use crate::bot::Greedy;
use crate::game::{Action, Game};
use crate::island::{Island, generate};
use crate::protocol::{action_text, board_json, parse_action, view_json};
use crate::rng::Rng;
use crate::settings::Settings;
use std::cell::RefCell;

struct Live {
    island: Island,
    game: Game,
    bots: Vec<Greedy>,
    // How the game started and every move since: enough to rebuild it exactly (Game::replay), for
    // saved games and the post-game review later
    seed: u32,
    moves: Vec<(u8, Action)>,
}

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static GAMES: RefCell<Vec<Option<Live>>> = const { RefCell::new(Vec::new()) };
}

fn text<'a>(p: *const u8, n: usize) -> Result<&'a str, String> {
    let bytes = unsafe { std::slice::from_raw_parts(p, n) };
    std::str::from_utf8(bytes).map_err(|_| "the input isn't UTF-8".to_string())
}

fn with_game<T>(id: u32, f: impl FnOnce(&mut Live) -> T) -> Result<T, String> {
    GAMES.with(|g| match g.borrow_mut().get_mut(id as usize) {
        Some(Some(live)) => Ok(f(live)),
        _ => Err(format!("no game {id}")),
    })
}

fn answer_result(r: Result<String, String>) -> usize {
    answer(r.unwrap_or_else(|why| format!("error {why}")))
}

/// # Safety
/// `p` must point to `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_new(p: *const u8, n: usize) -> usize {
    answer_result((|| {
        let input = text(p, n)?;
        let mut parts = input.splitn(3, '\n');
        let seed = parts.next().unwrap_or("");
        let players: usize = parts.next().and_then(|x| x.trim().parse().ok()).ok_or("expected the number of players")?;
        // That many players' defaults (5-6: the large island), then the game's own settings
        let settings = Settings::parse_for(parts.next().unwrap_or(""), players)?;
        if players < settings.players_min as usize || players > settings.players_max as usize {
            return Err(format!("{players} players isn't allowed ({}-{})", settings.players_min, settings.players_max));
        }
        let island = generate(&settings, seed)?;
        let mut dice = Rng::from_text(&format!("dice:{seed}"));
        let seed = dice.next_u32();
        let game = Game::new(&island, &settings, players, seed);
        let bots = (0..players).map(|_| Greedy::new(dice.next_u32())).collect();
        GAMES.with(|g| {
            let mut g = g.borrow_mut();
            g.push(Some(Live { island, game, bots, seed, moves: Vec::new() }));
            Ok((g.len() - 1).to_string())
        })
    })())
}

#[unsafe(no_mangle)]
pub extern "C" fn game_board(id: u32) -> usize {
    answer_result(with_game(id, |l| board_json(&l.island, &l.game)))
}

#[unsafe(no_mangle)]
pub extern "C" fn game_view(id: u32, seat: u32) -> usize {
    answer_result(with_game(id, |l| view_json(&l.game, seat.min(255) as u8)))
}

/// # Safety
/// `p` must point to `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn game_act(id: u32, seat: u32, p: *const u8, n: usize) -> usize {
    answer_result((|| {
        let action = parse_action(text(p, n)?)?;
        with_game(id, |l| {
            let seat = seat.min(255) as u8;
            // Answers and counter-offers come from players offered a trade, on someone else's turn
            let answering = matches!(action, crate::game::Action::Answer(s, _) | crate::game::Action::Counter { seat: s, .. } if s == seat);
            if l.game.to_act() != seat && !answering {
                return Err("it isn't your move".to_string());
            }
            l.game.apply_as(seat, action)?;
            l.moves.push((seat, action));
            Ok("ok".to_string())
        })?
    })())
}

#[unsafe(no_mangle)]
pub extern "C" fn game_bot(id: u32) -> usize {
    answer_result(with_game(id, |l| {
        if l.game.over() {
            return Err("the game is over".to_string());
        }
        let mut list = Vec::new();
        l.game.legal(&mut list);
        let seat = l.game.to_act() as usize;
        let a = l.bots[seat].choose(&l.game, &list);
        l.game.apply_unchecked(a);
        l.moves.push((seat as u8, a));
        Ok(action_text(a))
    }).and_then(|r| r))
}

// The bot in `seat` answers the open offer made to it; answers the move's text, or "none" when it
// has nothing to answer
#[unsafe(no_mangle)]
pub extern "C" fn game_bot_answer(id: u32, seat: u32) -> usize {
    answer_result(with_game(id, |l| {
        let seat = seat.min(255) as u8;
        let mut list = Vec::new();
        l.game.legal_for(seat, &mut list);
        if !list.iter().any(|a| matches!(a, crate::game::Action::Answer(..))) {
            return Ok("none".to_string());
        }
        let yes = l.bots[seat as usize].answer(&l.game, seat) && list.contains(&crate::game::Action::Answer(seat, true));
        let a = Action::Answer(seat, yes);
        l.game.apply_as(seat, a)?;
        l.moves.push((seat, a));
        Ok(action_text(a))
    }).and_then(|r| r))
}

#[unsafe(no_mangle)]
pub extern "C" fn game_free(id: u32) {
    GAMES.with(|g| {
        if let Some(slot) = g.borrow_mut().get_mut(id as usize) {
            *slot = None;
        }
    });
}

fn answer(text: String) -> usize {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        *o = text.into_bytes();
        o.len()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(n: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(n.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `p` and `n` must be what alloc() gave and was asked for.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(p: *mut u8, n: usize) {
    unsafe { drop(Vec::from_raw_parts(p, 0, n.max(1))) }
}

#[unsafe(no_mangle)]
pub extern "C" fn out() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// # Safety
/// `p` must point to `n` bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn island(p: *const u8, n: usize) -> usize {
    let input = unsafe { std::slice::from_raw_parts(p, n) };
    let Ok(text) = std::str::from_utf8(input) else { return answer("error the input isn't UTF-8".into()) };
    let (seed, settings) = text.split_once('\n').unwrap_or((text, ""));
    answer(match Settings::parse(settings).and_then(|s| generate(&s, seed)) {
        Ok(island) => island.to_json(),
        Err(why) => format!("error {why}"),
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn settings_table() -> usize {
    answer(Settings::table_json())
}

#[unsafe(no_mangle)]
pub extern "C" fn source_hash() -> usize {
    answer(crate::SOURCE_HASH.into())
}
