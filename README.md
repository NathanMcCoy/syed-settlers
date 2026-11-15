# Syed Settlers

Build on a hex island, trade, and race to 10 Renown, against friends and bots in any mix.
The rules are in [RULES.md](RULES.md). Live at https://settlers.pacificfish.ca.

**Status:** playable against bots and friends (links, no accounts), plain and functional. Games are
kept in the server's memory, so a restart ends them. There's no sailing: it was removed on Nov 15
to keep the game simple.

## How it's built

These are requirements, not just the current state:

- **One engine, written once, in Rust** (`engine/`): the island, the rules, and later the bots.
  It's built natively for simulations and bot training on a PC, and to WebAssembly
  (`site/js/settlers.wasm`) for the server and the page. They all run the same code, so they can't
  disagree. Speed comes first, because bots will play thousands of games: the board is flat arrays
  of small numbers, and the engine has no outside packages.
- **Every seat is a person or a bot, in any mix**, from 2 to 4 seats. Every game runs on the server,
  even one person against bots, so the server holds the dice and the cards, and bots keep playing
  whoever is connected. Each page only gets what its own seat may see. Bots get the same view, with
  no peeking.
- **Every balance knob is a setting** (`engine/src/settings.rs`, marked ⚙ in RULES.md), checked
  against its allowed range. The page builds its controls from the engine's own list.
- **Seeded randomness:** the same seed and settings always make the same island, so games can be
  replayed and tested.

## Working on it

Needs Node 24+ and Rust with the WebAssembly target (`rustup target add wasm32-unknown-unknown`).

| Command | What it does |
|---|---|
| `npm install` | Once: Playwright, for the browser tests (uses Chromium and WebKit) |
| `npm test` | The engine's Rust tests, then the Node tests (WebAssembly, server, game API, browsers) |
| `cargo run --release --bin cli -- sim --games 10000` | Bots play each other: game length, seat fairness, Renown sources (in `engine/`) |
| `npm run build` | Rebuilds `site/js/settlers.wasm` after a change in `engine/src` (a test fails if you forget) |
| `npm run serve` | The site on http://localhost:8091/ |
| `cargo run --release --bin cli -- island --seed betta` | One island as JSON (in `engine/`) |
| `cargo run --release --bin cli -- bench-island` | How fast islands are made |

GitHub Actions runs every test on every pull request and push to `main`, and on `main` publishes the
image `ghcr.io/nathanmccoy/syed-settlers:main` for the server to deploy.

## License

MIT, see [LICENSE](LICENSE).
