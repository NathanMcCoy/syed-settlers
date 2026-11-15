// Syed Settlers' engine. Everything that decides what happens in a game lives here, written once:
// the server and the page run it as WebAssembly (site/js/settlers.wasm), and simulations and bot
// training run it natively on this PC, so they can never disagree.
//
// Speed matters (bots will play thousands of games), so the board is flat arrays of small numbers,
// with no searching through lists during play, and nothing here allocates while a game is played.
pub mod bot;
pub mod game;
pub mod hex;
pub mod sim;
pub mod island;
pub mod json;
pub mod rng;
pub mod settings;

#[cfg(target_arch = "wasm32")]
mod wasm;

// The fingerprint of this source (see build.rs)
pub const SOURCE_HASH: &str = env!("SOURCE_HASH");
