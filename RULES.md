# Syed Settlers: rules

Version 0.1 (draft, Nov 14 2026). Numbers marked ⚙ are settings in `engine/src/settings.rs`.
They're first guesses: playtesting and bot simulations will tune them.

## Players

2–4 seats ⚙. **Each seat is a person or a bot, in any mix**: you and 3 bots, you and a friend and
a bot, four friends. People play from their own devices.

## Goal

The first player to have **10 Renown** ⚙ (`win`, 3–20) during their own turn wins at once.

| Renown from | |
|---|---|
| Each Hut | 1 |
| Each Lodge | 2 |
| Each Lighthouse project | 1 |
| The first Hut on an islet | +1 ⚙ |
| Navigator title | 2 ⚙ |
| Harbourmaster title | 2 ⚙ |

## The island

A new island every game, made from a **seed**: the same seed and settings always make the same
island, so a good (or unfair) island can be shared and replayed.

- **Main island:** 22 ⚙ land hexes (16–40), an irregular coastline, no lakes inside.
- **Terrain and resources:**

  | Hex | Produces |
  |---|---|
  | Forest | Timber |
  | Quarry | Stone |
  | Fields | Grain |
  | Mine | Copper |
  | Paddy (shallow flooded fields, where bettas live) | Fish |
  | Dunes | nothing |

  About one Dunes hex per 12 (at least one). The rest are split as evenly as possible among the
  five producing kinds, so each appears at least 3 times.
- **Numbers** 2–12 (never 7) on every producing hex. The usual spread is one 2, one 12, and two each
  of 3–6 and 8–11. Smaller islands drop the rarest numbers first; bigger ones add middle numbers.
- **Fairness checks** (the generator redraws until all hold):
  - no 6 or 8 next to another 6 or 8;
  - no two equal numbers next to each other;
  - each resource's total chance of being produced is 60%–150% of the average resource's.
- **Sea** surrounds everything (every hex within 2 of land).
- **Markets:** 7 ⚙ on the main island's coast, each on one coastal edge: 2 general markets (3:1)
  and one 2:1 market for each resource. No two markets share or neighbour a corner.
- **The squall** starts on a Dunes hex of the main island.

### Islets

Small islands of 1–3 hexes offshore, reachable only by Boat.

- **count** ⚙ 0–6 (default 2), spread around the island, at least one sea hex apart.
- **distance** ⚙ 1.0–3.0 (default 1.5): the average number of sea hexes between an islet and the
  main island. 1 is a narrow strait (about 2 Boats). Between whole numbers, some islets are nearer
  and some further: 1.5 with 2 islets means one at 1 and one at 2.
- **resources** ⚙ 0–140% (default 100%): how productive islet hexes are on average, by their
  numbers, compared with the main island's producing hexes. At 0% islets are barren Dunes (worth
  only the Renown). The cap is 140% because no number beats a 6 or 8; on very small islands the
  real cap is a little lower, and the generator gets as close as it can.
- Islets have no Markets. Inside an islet, the 6/8 rule doesn't apply (with 2–3 hexes it would
  often be impossible), but equal numbers are still kept apart.

## Pieces

| Piece | Where | Cost | Limit ⚙ |
|---|---|---|---|
| Path | an edge touching land | Timber + Stone | 15 |
| Boat | an edge touching sea | Timber + Fish | 12 |
| Hut | a corner touching land | Timber + Stone + Grain + Fish | 5 |
| Lodge | replaces your Hut (the Hut comes back to you) | 2 Grain + 3 Copper | 4 |

- **Coastal edges** (land on one side, sea on the other) can take either a Path or a Boat.
- **Spacing:** no building may be on a corner next to another building, anyone's. Buildings are
  at least 2 edges apart.
- **Connection:** Huts must touch one of your own Paths or Boats (except during setup). Paths and
  Boats must share a corner with one of your buildings, Paths or Boats, and can't continue through a
  corner holding another player's building. Paths and Boats link to each other directly.

## Setup

Seat order is random. In round 1 each player, in order, places a Hut on any free corner of the main
island (keeping the spacing) and a Path or Boat touching it. Round 2 goes in reverse order. After
your second Hut, take one resource from each producing hex around it.

## A turn

1. You may use one project before rolling (useful for a Pilot).
2. **Roll two dice.** Unless it's a 7, every hex with that number produces: 1 resource for each
   Hut and 2 for each Lodge touching it. The squall's hex produces nothing. The bank never runs
   out ⚙.
3. Then in any order: trade with the bank and markets, build, buy projects, and use one project (if
   you didn't before rolling).
4. End your turn.

## Rolling a 7

In this order:

1. Everyone holding **more than 7** ⚙ cards loses half of them (rounded down), **choosing which**.
2. The roller **moves the squall 1 to 4** ⚙ steps (counting every hex, sea included) onto a land
   hex. It must move.
3. The roller takes one random card from a player with a building touching the squall's new hex.
   If several have one, the roller chooses which; never from themselves; nothing if nobody has.

## Trading

- **Bank:** 4 of one resource for 1 of any other.
- **General market:** 3 of one resource for 1 of any other.
- **Resource market:** 2 of that resource for 1 of any other.
- You can use a market if you have a Hut or Lodge on either corner of its edge.
- Trades between players: not in version 1. Planned, for people and bots alike.

## Projects

Cost: **Grain + Copper + Fish.** The deck ⚙ has 21 cards:

| Project | Count | Effect |
|---|---|---|
| Pilot | 8 | Move the squall to any land hex, then steal as after a 7 |
| Lighthouse | 4 | 1 Renown, counted the moment you get it |
| Shipwright | 3 | Place 2 Boats for free |
| Good Harvest | 3 | Take any 2 resources from the bank |
| Fair Winds | 3 | Until the end of your turn, trade 2:1 for one resource of your choice |

- **Face up** ⚙ (`projects.faceUp`, on by default): 3 projects lie face up and you choose which
  to buy. A new one fills the gap; when the deck is empty, the row shrinks.
- **Off:** you draw the top card unseen. Others see how many projects you hold, not which.
  Hidden Lighthouses still count: you can win on your turn by revealing them.
- You can't use a project on the turn you bought it, and you can use at most one per turn.
  (A Lighthouse isn't "used": it counts as soon as you have it.)

## Titles

- **Navigator** (2 Renown ⚙): the longest unbroken line of your Paths and Boats, at least 5 ⚙
  long. Another player's building on a corner breaks the line there.
- **Harbourmaster** (2 Renown ⚙): your buildings touch the most different markets, at least 2 ⚙.
- To take a title from its holder you need strictly more. If the holder's line is broken and they
  drop below someone, it goes to whoever is now strictly ahead; if nobody is (a tie, or nobody
  meets the minimum), nobody holds it until someone is.

## Not in version 1 (planned)

- Trades between players.
- A turn timer, and a bot taking over the seat of a person who leaves.
- Illustrations (the board is plain and functional for now).
