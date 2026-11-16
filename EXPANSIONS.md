# Expansions: the plan

Draft, Nov 16 2026. Made by reading the four expansion rulebooks the user supplied (`D:\TRUENAS\CLAUDE\catan rulesets`):
the sea expansion (Seafarers), the city expansion (Cities & Knights), the variants-and-scenarios box (Traders & Barbarians)
and the exploration expansion (Explorers & Pirates, with its Mission Guide). Nothing here is built yet.

**The requirement:** every expansion, scenario and variant is playable by any mix of people and bots, and the bot can play
each one through to a win. The rules follow the rulebooks; the names, theme, look and **maps are our own** (the printed
scenario maps are pictures and part of the original game's identity, so we design maps that meet each scenario's rules and
tile counts instead).

In this file, resources use our names. The rulebooks' resources map to ours as: wood → **Timber**, ore → **Stone**,
wheat → **Grain**, brick → **Copper**, wool → **Fish** (so a Path costs Timber + Copper, a Lodge 3 Stone + 2 Grain, as now).
Rulebook terms are given once in brackets so each rule can be traced; working names for new things are placeholders
until the user picks names.

---

## Part 1: what every expansion needs first (the foundation)

The engine today plays exactly one ruleset on one kind of board. Every expansion breaks at least one of these
assumptions, so they get fixed once, before any expansion, with the base game's behaviour (and its tests and sim numbers)
unchanged.

| # | Change | Why | Needed by |
|---|---|---|---|
| F1 | **Rulesets:** settings choose an expansion, a scenario and variant switches (`ruleset.*`); `Settings::parse` refuses combinations the rulebooks don't allow, and applies the combined-rule changes (e.g. sea + city: +2 Renown to win). New Game screen picks them. | Everything | all |
| F2 | **Maps as data:** a map file lists hexes (terrain, number or hidden stack, region/island id, special markers), frame, port/market spots, setup spots and placement rules; the generator fills random parts ("variable setup") with the same fairness checks per region. Our current hexagon/random generator becomes two built-in maps. | Scenarios have fixed regions, islands, hidden areas, special hexes | sea, exploration, T&B scenarios |
| F3 | **Sea is playable:** sea edges and sea corners exist for pieces (ships, pirates); edges know coast/sea/land. Bring back the useful part of the sailing code removed in PR #6 (commit 9c47b41; the code before it is `git show 9c47b41^:engine/src/game.rs`). | Ships | sea, city+sea, exploration |
| F4 | **Extra board geometry:** points and paths that aren't hex corners/edges: river bridge sites, the 4 interior paths and centre plaza of a commodity hex, fishing grounds at coastal "V"s, anchors on a council hex. The graph gets optional extra nodes/edges from the map file. | T&B scenarios, exploration | T&B, exploration |
| F5 | **Pieces generalised:** a corner can hold a building (Hut, Lodge, harbour building, metropolis flag, wall) *and* a knight; an edge can hold a Path, ship(s) (up to 2 in exploration), bridge, wagon, barbarian or edge-knight. Owner + kind + state (active/inactive, damaged, conquered, warship, cargo). | City knights, sea, T&B, exploration | all but variants |
| F6 | **More kinds of cards and tokens:** hand = 5 resources + optional commodities (3); separate counters for coins, trade tokens, fish tokens, cloth, prisoners, Renown tokens, commodity tokens; each with its own "counts toward the 7 limit / can be stolen / can be traded" flags. | City, T&B, exploration | most |
| F7 | **Decisions by other seats (a queue):** today only discards and trade answers come from seats not to act. Expansions add many: picking a gold-field resource, discarding progress cards off-turn, moving a displaced knight, voting bids, giving cards (Wedding, Good Neighbours), commercial-harbour swaps, choosing a city to lose. One `pending` queue of `(seat, decision)` that `to_act()` reads; legal()/apply_as() already work per seat. | All expansions | all |
| F8 | **Turn structure as steps:** optional steps before/after production (event die, event card, pirate fleet move, barbarians), an end-of-turn/movement step, rolls that repeat (reroll 2/12). | City, T&B, exploration | all |
| F9 | **Hidden information:** hidden hex and number stacks, three progress decks, event deck, fish/commodity token bags. The server draws when revealed; seat views never show hidden order; bots get only their seat's view (the bot already only looks at what its seat may see: keep it that way by giving bots a view-derived state for anything hidden). | Fog/exploration, city, T&B | most |
| F10 | **Win conditions as data:** target Renown per scenario, special wins (wonder finished, cloth exhaustion, fortress + 10), modifiers (old boot +1, Poor -2). | All scenarios | all |
| F11 | **Titles generalised:** a title = "who has strictly most of X, at least N, worth R". Existing: Longest Path, Most Pilots. New: Strongest Ports, Wealthiest/Poor, three mission titles. | Many | many |
| F12 | **Protocol, view and page:** new move words, view fields and board drawing for every new piece; reuse the shop/tabs. The page builds controls from the legal moves, so most new actions need only a label and a place. | All | all |
| F13 | **Bot interface:** every new decision has (a) a legal-move list, so a random-legal bot can always finish, and (b) a Greedy rule. Then the simulator proves it (see Part 3). | All | all |
| F14 | **Saves and replays** (already a goal for the post-game review): seed + settings + move list replays exactly. Expansions with hidden stacks depend on this being exact. | Review, debugging | all |

Biggest risks: F4 (non-hex geometry) and F7 (many off-turn decisions) touch everything; do them carefully and early.

---

## Part 2: each expansion

Sizes: S ≈ a day or two, M ≈ a week, L ≈ 2–3 weeks, XL ≈ a month or more (rough, including bot and tests).

### A. Variants box, part 1: small rule switches (from Traders & Barbarians) — **S each, do first**

These change the base game and combine with everything else, so they're the cheapest proof of F1/F7/F8.

| Variant | Rules | Bot needs |
|---|---|---|
| **Friendly squall** (Friendly Robber) | The squall may not go to a hex touching a building of a player with only 2 Renown; if nowhere is valid, it goes to the Dunes and may steal only from players with more than 2. | Nothing new (legal list shrinks). |
| **Strongest Ports** (Harbors) | New title: 2 Renown for most Renown in buildings on markets, at least 3. Win target +1 (11). | Value market spots a bit more when chasing it. |
| **Event cards** | 36 cards + New Year card replace the dice: each card has a number (same spread as two dice) and an event: Conflict, Beautiful Day, Calm Seas, Epidemic, Good Neighbours, Earthquake (damaged Paths, repair Timber + Copper, no building next to them, no new Paths until repaired), Helpful Neighbour, Plentiful Year, Squall Attacks (a 7), Squall Flees, Tournament, Trade Advantage. New Year: reshuffle with 5 cards under it. Each event has a city-expansion version. | Choices in Good Neighbours / Helpful Neighbour / Plentiful Year (give least useful, take most needed); repair damaged Paths when it blocks its plan. Card counting is optional (strength later). |
| **Two players** (Catan for Two) | Two neutral colours with starting Huts; every Path/Hut you build also builds one for a neutral (your choice of where); two production rolls a turn (second must differ); trade tokens: gained by Huts on Dunes (2) and the coast (1), and by discarding a played Pilot (2); spent once a turn for Forced Trade or sending the squall to the Dunes (cost 1, or 2 if you lead). Neutrals can hold Longest Path. | Where to put neutral pieces (block the opponent), when to spend tokens. |

### B. The sea expansion (Seafarers) — **L, plus maps**

**Rules:**
- **Ships** (Timber + Fish, 15 each): on sea edges; connect to your ships or buildings, not Paths; can't pass an opponent's building; not on the pirate's hex edges. A coastal edge holds a Path or a ship, not both. Setup: a coastal Hut may take a ship instead of a Path.
- **Move one ship a turn:** only an open-ended ship that isn't part of a line connecting two of your buildings, not built this turn, not to or from the pirate's hex.
- **Longest Path** counts Paths and ships, joined only at your own buildings.
- **Gold hexes:** produce 1 resource of your choice per Hut (2, in any mix, per Lodge): an F7 decision for each producing player.
- **The pirate:** on a 7 (or a Pilot) move the squall *or* the pirate (to the frame or a sea hex) and steal from a player with a ship there.
- **Road Crew** may place Paths or ships.
- **Island bonus:** first Hut on each small/unexplored island (per player) gives +2 (or +1) Renown tokens.
- **Scenarios** (each: own map, setup area, win target 12–14):
  1. New Shores (main island + small islands, 14).
  2. Four Islands (home islands chosen by setup, 13).
  3. Fog Islands (hidden hexes and numbers revealed when a ship or Path reaches them, take 1 resource; 12): needs F9.
  4. Through the Desert (regions split by desert strip, 14).
  5. Forgotten Tribe (Huts only on numbered hexes; ship onto marked edges to take Renown tokens, project cards and markets you then place by a coastal Hut; 13).
  6. Cloth for the island (3 starting Huts; villages on small islands produce cloth to everyone with a route to them; 2 cloth = 1 Renown; game also ends when 5 villages run out; no Longest Path; 14).
  7. Pirate Islands (pirate fleet moves round a set path by the lower die and attacks; one unbranching line of ships to your own fortress; Pilots convert ships to warships; attack the fortress at end of turn; no squall, no titles; win with fortress + 10).
  8. Wonders (claim one of five wonders by its requirement, build 4 levels of 5 resources; win by finishing or 10 + highest level; no pirate).
  - New World: random islands (our generator, multi-island), random markets one edge apart, +1 per new island, 12.
- **Not text-readable, to confirm from the pages:** each wonder's requirement is in the text, but its level costs are icons.

**Bot:** where to sail (reuse the route planner over land + sea edges; value islands by their bonus), when to move a ship
(free the open end toward the best spot), gold choices (most needed), pirate vs squall (whichever hurts the leader more),
scenario goals: cloth routes, Forgotten Tribe edge prizes, warships + fortress timing (attack when warships > expected die),
wonder choice and level building. The single-ship move and "open end" rule are exactly the kind of thing to test with random
play.

**Maps:** 9 to design (3- and 4-player versions where the rulebook has both), meeting the tile and number counts listed
per scenario. A map editor/viewer page (extend `viewer.html`) makes this much faster.

### C. The city expansion (Cities & Knights) — **XL**

Replaces the project deck and Most Pilots. Win at 13.

- **Three dice:** two production dice + an event die (3 ship faces, 3 coloured gate faces). Ship faces move the raiders one
  step; gate faces + the red die decide who draws a progress card of that colour.
- **Commodities from Lodges:** a producing Lodge gives 1 resource + 1 commodity on Forest (paper), Paddy (cloth), Quarry
  (coin); 2 Grain on Fields; 2 Copper on Mines; Dunes nothing. Commodities count toward the hand limit and can be stolen and
  traded (bank 4:1 / market 3:1, both ways; not 2:1 markets).
- **City walls** (2 Copper, max 3, only under Lodges): +2 hand limit each; lost when the Lodge is pillaged.
- **City improvements:** three tracks (science/trade/politics), level n costs n of that track's commodity; need a Lodge.
  Level 3 powers: science = a free resource when you get nothing (not on a 7); trade = commodities 2:1 with the bank;
  politics = may promote to level 3 knights. First to level 4 takes that track's metropolis (+2, can't be pillaged; level 5
  makes it permanent). Progress draws on red die ≤ level + 1 (to confirm on the track picture).
- **Knights** (2 of each strength 1–3): recruit (Fish + Stone, starts inactive, on a corner touching your Path, no spacing
  rule), promote (Fish + Stone, once per turn each), activate (Grain). Active knights can move along your routes, displace a
  weaker opponent knight (who then relocates: F7), or chase the squall from an adjacent hex. Knights block Paths and break
  Longest Path like buildings.
- **Raiders (barbarians):** when they land, strength = number of Lodges on the board vs sum of active knights. Raiders win:
  the lowest contributor(s) lose a Lodge (not a metropolis). Defenders win: the top contributor gets a Renown token (ties:
  each draws a progress card). Then all knights become inactive. The squall stays off the board until the first landing.
- **54 progress cards** (18 per colour, hand limit 4, can play the turn drawn, any number on your turn except Alchemy before
  rolling; 2 Renown cards are played at once): science: Alchemy (2), Crane (2), Engineering (1), Invention (2, swap number
  discs except 2/6/8/12), Irrigation (2), Medicine (2), Mining (2), Printing (1, Renown), Road Building (2), Smithing (2).
  Trade: Commercial Harbor (2), Guild Dues (2), Merchant (6, the merchant piece: +1 Renown, 2:1 on its hex), Merchant Fleet
  (2), Resource Monopoly (4, 2 each), Trade Monopoly (2). Politics: Constitution (1, Renown), Diplomacy (2, remove an open
  Path), Encouragement (2), Espionage (3), Intrigue (2), Taxation (2), Treason (2), Wedding (2), Sabotage (2).
- **Setup:** second building is a Lodge; start resources from it.
- **With the sea expansion:** ships count as Paths for knights; knights can stand at sea corners; ship moves can't strand a
  knight; gold gives no commodities; pirate enters after the first landing; Taxation is squall-only; win target +2.

**Bot:** this is where heuristics get hard. Needed: commodity/improvement planning (which track, when), knight policy (keep
active strength ≥ the raiders' expected need, especially when it holds the weakest contribution), progress-card play rules
for all 24 kinds, and many F7 answers (progress discards, displaced-knight moves, gifts). Plan: Greedy rules for every card
first (so games finish), then a search bot (Part 3) for strength.

### D. Variants box, part 2: scenarios (Traders & Barbarians) — **M to L each**

Each is its own scenario with its own map, often a new token type (F6) and geometry (F4).

| Scenario | Rules | Size |
|---|---|---|
| **Fishing** | Lake hex (4 numbers) in the middle, 6 fishing grounds on the coast; fish tokens (1/2/3 fish, one old boot = +1 to win, can be passed to someone with ≥ your Renown); spend 2/3/4/5/7 fish to: remove the squall, steal, take a resource, free Path, free project. Max 7 tokens. Squall may sit on the lake (blocks all 4 numbers). Win 10. | M |
| **Rivers** | Two river tiles, bridge sites (bridges: 2 Copper + Timber, count as Paths, Road Crew can't build them); coins (1 per Hut/Path on a river hex, 3 per bridge; buy at bank rates; 2 coins → 1 resource, twice a turn); Wealthiest (+1) and Poor (−2) tiles; swamps instead of dunes; the 2 shares the 12's hex. Win 10. | M |
| **Merchant trains** | Watering hole centre hex; building anything on your turn earns a wagon; end-of-turn **vote**: every player bids Fish/Grain, majority places it; wagons form unbranching trains from 3 start points; a wagon on a Path counts +1 for Longest Path; buildings between two wagons +1 Renown. Win 12. | L (the vote is a multi-seat auction: F7) |
| **Raider attack** | Castle hex; no squall; each building placed triggers 3 raider landings on coastal numbered hexes (3 on a hex conquers it: no production, no building, conquered buildings worth nothing); knights on **edges** (6 each, by project cards: Knighthood/Swift Knight), move 3 (5 for Grain) at end of turn; a hex is freed when knights outnumber raiders; prisoners (2 = 1 Renown), knight losses by die, coins. Project deck: Capture 4, Knighthood 14, Swift Knight 4, Treason 4; bought cards are played at once. Win 12. | L |
| **Traders & raiders** (the box's final scenario) | 3 commodity hexes with interior paths (quarry, glassworks, castle: each wants one good and gives others), a wagon per player that moves at end of turn (MP 4–7 by upgrade level; costs 2 MP off-Path, 1 on yours, 1 + a coin on others', +2 with a raider), deliver goods for 1 Renown + coins (1–5 by level), drive raiders off by die range, wagon upgrades (4 levels, the last +1 Renown), no 2/12 (reroll), 7 moves a raider onto a Path and steals. Project deck: Knight 16, Road Building 3, Swift Journey 3, Renown 3. Win 13. | L |

- **Two-player versions** of each scenario (rulebook page 19 / 24 notes) come almost free once A's two-player variant exists.
- **Not text-readable, to confirm from the pages:** wagon upgrade costs after the first (Timber + Fish + Stone), the
  drive-off die ranges per level, the knight/project costs in the raider scenarios, the fish token action table is readable.

**Bot:** fish spending thresholds; coins as a resource; voting (bid if the best spot for it is worth more than the cards);
raider-scenario knight routing (a small assignment problem: put knights where raiders are about to conquer your hexes);
wagon routing (shortest-path over the extended graph with MP costs and coin costs, pick deliveries by value/distance).

### E. The exploration expansion (Explorers & Pirates) — **XL, best last**

A different game on the same engine: no Lodges, no squall, no projects, no titles from the base game. Win 17 (full game).

- **Turn:** production → action → **movement phase**.
- **Coins:** 2 to start; 1 coin when you produce nothing; bank trades are 3:1 (for a resource or a coin); 2 coins → any
  resource, twice a turn.
- **Harbour buildings** (2 Grain + 2 Stone, 2 Renown, replace a coastal Hut, max 4): hold 1 large or 2 small pieces.
- **Ships** (Timber + Fish, 3 each): built next to a harbour building; carry 1 large (settler, fish haul) or 2 small (crew,
  spice sack); move 4 edges (6 for a Fish), up to 2 ships per edge; load/unload at harbours and adjacent hexes.
- **Settlers** (Copper + Timber + Fish + Grain): a ship with a settler next to a land corner founds a Hut (spacing rule).
- **Crews** (Fish + Stone).
- **Exploring:** a ship next to an unexplored hex reveals it (ends that ship's move): land gets a number from its region's
  stack and gives 1 resource; sea gives 2 coins; gold hexes get a pirate lair; spice farms get 1 sack per player; shoals.
  Building only on explored hexes.
- **Pirate ships (one per player):** on a 7 the roller places or moves their own pirate ship (removing anyone else's), not
  next to the starting island, and steals from a ship owner there. Moving ships past another's pirate costs 1 coin tribute.
  Chase it with unmoved adjacent ships: roll 6 (better with spice abilities).
- **Missions** (tracks with Renown per step, plus a 1-Renown tile for the leader): pirate lairs (3 crews capture; involved
  players get 2 coins + a step, the hero an extra step; the gold hex then produces 2 coins per building), fish (roll for a
  haul at an explored shoal once a movement phase, deliver to the council hex), spices (leave a crew on a spice farm for
  its ability: Swift Voyage +1 MP, Pirate Bonus, Fast Gold; take one sack per farm, deliver to the council).
- **Mission Guide scenarios** as milestones: Land Ho! (8), Pirate Lairs (12), Fish (15), Spices (15), then the full game (17).
  2-player variants use neutral pieces as obstacles.

**Bot:** the hardest planning in the set: multi-turn ship logistics (pick up crew/settlers, explore, deliver), exploring
under hidden information (expected value of an unexplored region, from the known remaining tiles), pirate placement, and
mission racing. Plan: a task planner (goal → route over sea edges with MP budget → load/unload steps) on top of Greedy, then
search for strength.

---

## Part 3: the bot across all of this

- **Never stuck:** every decision has a legal list (F13). A random-legal bot must finish 10,000 games of every scenario in
  the simulator; any stall or turn-cap hit is a bug. This is the first gate for each expansion.
- **Sensible:** Greedy gets rules per new decision (listed per expansion above). Second gate: Greedy beats random-legal
  ≥ 95%, every win route actually happens in the sim (e.g. wonder wins, cloth wins, fortress wins), and seat balance is
  within noise.
- **Strong (later):** a search bot that works for every ruleset without per-rule code: sample the hidden information
  consistent with its seat's view, then play out with Greedy (determinised Monte-Carlo). The engine's speed (~13,700 games/s
  on 24 threads) makes this affordable; on the server it must fit the 128 MB container and a time budget per move.
  Measure it against Greedy per expansion. This also feeds the post-game review goal (decisions rated by win chance).
- **Off-turn decisions** (F7) need bot answers too: gold choices, gifts, discards, displaced knights, votes.
- **Tests per expansion:** a rules test per rule (as now), random play checking every state, the sim gates above, and the
  browser test plays one scenario game by clicking.

## Part 4: suggested order

1. **Foundation F1, F6, F7, F8, F10, F11, F12, F13** (base game unchanged, sim numbers unchanged). Then the **small
   variants (A)**: they prove the foundation cheaply and are fun on their own.
2. **F2–F5, F9 + the sea expansion (B)**: scenarios 1–4, then New World, then 5–8. Maps and a map editor along the way.
3. **The city expansion (C)**, then **city + sea** combined.
4. **The variants box scenarios (D)**: Fishing, Rivers, Raider attack, Merchant trains, Traders & raiders.
5. **The exploration expansion (E)**: Mission Guide 1 → 4, then the full game.
6. The **search bot** can start any time after step 1 and benefits every later step.

## Questions for the user

1. **Names:** our own names for each expansion, the new resources/tokens (commodities, coins, gold), pieces (ships,
   knights, wagons, settlers, crews) and cards. (The rulebooks' names stay out of the game.)
2. **Sailing comes back** for the sea and exploration expansions (it was removed from the base game on Nov 15 for
   simplicity). The base game stays without it. OK?
3. **Maps:** we design our own for every scenario. Fixed maps, random maps from scenario rules, or both?
4. **Player counts:** 2–4 only (the rulebooks' 5–6 extensions aren't in these PDFs)?
5. **Order:** as above, or a favourite expansion first?
6. **Values only printed as pictures** (wonder level costs, wagon upgrade costs, improvement-track die ranges, raider-scenario
   knight costs): fine to read them off rendered pages as each part is built.

## Decisions (user, Nov 16)

- **Names:** for now, the working names in this file: "the sea expansion", "the city expansion", "the variants box",
  "the exploration expansion", and plain piece names (ship, knight, city wall, coins, commodities, wagon, bridge, settler,
  crew). The rulebooks' product titles stay out of the game.
- **Sailing** returns only in the expansions that call for it; the base game stays without it.
- **Maps:** not decided by the user; default = generated from each scenario's rules, reproducible from a seed (a good seed
  works as a fixed map). Hand-designed fixed maps can be added later if a scenario plays better with one.
- **Player counts:** the user will provide the 5–6 player rulebooks. Read them and fold them into this plan **before** the
  foundation work starts, since `MAX_PLAYERS = 4`, the 4-bit `to` field of trade offers, setup order, map sizes and the bank
  are all touched by F1–F14 anyway.
- **Order:** Part 4 as written.

## 5 and 6 players (read Nov 16 from the five 5–6 player rulebooks)

**Base game: built first (branch `six-players`, RULES.md v0.5).** The large island (30 hexes, rows 3-4-5-6-5-4-3; 6 Forest,
5 Quarry, 6 Fields, 5 Mine, 6 Paddy, 2 Dunes; 28 numbers: two 2s, two 12s, three of each other), 11 markets (one more
general, a second 2:1 Fish), 24 of each resource, 34 projects (20/5/3/3/3), and **paired turns**: after each turn, the
player 3 seats on takes an extra build phase (build, buy, one project, bank and market trades, no trading with players;
can win in it). `Settings::for_players(n)` gives 5–6 seats these defaults; `turns.paired` switches the pairing.
The classic spiral's letter order for the 28 numbers isn't printed in the rulebook (only the number sides), so the large
island uses the fair shuffle until we have it.

**Every expansion's 5–6 rules use the same paired turn**, with one difference that matters for F8: the second player's
phase is "Action + that ruleset's end-of-turn step" (city: knight actions and progress cards in it; variants box: the
end-of-turn phase, e.g. voting with player 2 as the active player, moving knights, moving the wagon; exploration: Action
+ Movement; sea: Action incl. moving 1 ship). So the paired phase is a **turn step list without the production steps**,
not a special case of Main. Per expansion, the 5–6 additions are:

| Ruleset | 5–6 additions |
|---|---|
| Sea | 10 more hexes (7 sea, 2 gold, 1 desert), 2 more wonders (one needs a building at a lighthouse marker, one needs 2 Lodges), a sixth island ("The Six Islands"), more Renown/cloth/lair tokens, 11 markets on New Shores; Pirate Islands: a "!" hex where the fleet doesn't attack; 5-player games leave one colour's locations unused. Main islands use the large island's setup. |
| City | 18 more commodities (6 of each), the large island, otherwise as 3–4 (knights, walls, improvements per player). |
| Variants box | Fishing: 2 lakes, 2 more fishing grounds, 14 more fish tokens. Rivers: 3 rivers, all 28 numbers (no "2 shares the 12"). Merchant trains: 2 watering holes (6 train starts), 11 more wagons; the paired player holds their own vote. Raider attack: 2 castles, two coastal 5s and 9s (3–5 raiders per landing), 12 more raiders. Traders & raiders: 5 commodity hexes (2 quarries, 2 glassworks, a second castle; some with 6 interior paths), 2/12 not rerolled. |
| Exploration | 9 more hexes (incl. 2 gold), 3 more number discs per region, more crews/harbour buildings/ships/spice sacks/fish hauls/coins; 21 face-up start hexes; mission tracks replace frame pieces. |

**Effect on the plan:** none on the order. F7 (decisions by other seats) and F8 (turn steps) must treat "acting player"
and "whose turn" separately from day one (`owner` vs `current` as built here), and F2 map files need a 5–6 version of
every scenario map. All seat arrays are `MAX_PLAYERS = 6` already.

## Progress

- **Nov 16, the groundwork (branch `groundwork`):** built, base game unchanged (pinned by `base_game_fingerprint`: 500 bot
  games, 2–6 players, hashed; any change to how the base game plays fails it).
  - F11 titles: `engine/src/game/titles.rs` (a `Title` row each: value, min, Renown; `holder()`, `refresh_title()`).
  - F6 tokens: `game/tokens.rs` (`Token::Renown`, `Token::Trade`; per kind: Renown, hand limit, stealable, tradeable).
  - F10 winning: `Game::win_target(p)`, `Game::has_won(p)` (the one place rulesets change).
  - F7 decisions queue: `game/pending.rs` (`Phase::Decide`, `pending`, `after`, `decide_then()`, `decided()`); a 7's
    discards are its first kind. The view names the phase after the decision ("discard").
  - F8 turn steps: `game/turns.rs` (`production_roll`, `rolled`, `end_phase`, `start_extra_phase`, `start_turn`).
  - F13 bots: `bot::Bot` trait, `RandomBot`, `sim::play_with` + `BotKind`, `cli sim --bots greedy,random`; a game that
    isn't over must always have a legal action (checked every step). Gate test: random bots never stuck (2–6 players),
    Greedy beats random ≥ 95%.
  - F14 replays: `Game::replay(island, settings, players, seed, moves)`; the wasm wrapper records every move.
  - F12 view: `titles` [{name, holder, min, renown}], players' `tokens` and `target`, `deciding` {seat, decision}; the
    page draws title badges and token counts from these.
  - Not yet: F1 (rulesets: lands with the first variant, as there's nothing to choose yet); F2–F5, F9 (with the sea
    expansion, as planned).
- **Nov 16, the sea expansion, milestone 1 (branch `groundwork`, after the groundwork commit):** the sea rules and
  scenario 1, New Shores (RULES.md v0.6). `sea=true` (+ `sea.scenario`, `sea.islandBonus`, `pieces.ship`); its own
  defaults via `Settings::parse_for` (win 14) = the start of F1. Map: main island + small islands (`island.rs`
  `small_islands`, `pirate_start`; hexes carry a `region`; `Terrain::Gold`). Rules in `engine/src/game/sea.rs`: ships
  (`Action::Ship`, connect to ships/buildings only, never past others' buildings or by the pirate), one ship move a turn
  (`Action::MoveShip`, open end, not built this phase; the closed-line rule is implied by the open-end rule), Longest
  Path joins Paths and ships only at own buildings, gold as a queued decision (`Decision::Gold`), the pirate
  (`Action::Pirate`, robs ship owners), Road Crew ships, island bonus (Renown tokens). Bot: gold choices, pirate vs
  squall, sailing toward island spots (`reachable_by_sea`). Page: striped gold, dashed ships, the pirate, Ship and
  Move a ship in the shop. Sim (20,000 games): 3p 36 rounds, 4p 28, 6p 17; seats fair; winners get ~11% of their
  Renown from island bonuses. Tests: 10 sea rules tests (5 of 6 deliberate breaks caught; the 6th was the redundant
  rule, removed), a browser test (setup ship by clicking, both browsers) and an API game. Next milestones: Four Islands,
  Fog Islands (F9 hidden hexes), Through the Desert, then 5–8 and New World.
