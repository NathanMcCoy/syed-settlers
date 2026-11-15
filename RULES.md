# Syed Settlers: rules

Version 0.2 (draft, Nov 15 2026: sailing removed, so no Boats or islets; Shipwright, Fair Winds,
Navigator and Harbourmaster renamed). Numbers marked ⚙ are settings in `engine/src/settings.rs`.
They're first guesses: playtesting and bot simulations will tune them.

## Players

2–4 seats ⚙. **Each seat is a person or a bot, in any mix**: you and 3 bots, you and a friend and
a bot, four friends. People play from their own devices.

## Goal

The first player to have **10 Renown** ⚙ (`win`, 3–20) during their own turn wins at once.

**Round limit** ⚙ (`rounds`, chosen when starting a game; 0 = no limit, the default): when the last
round ends, the most Renown wins. A tie goes to the player with more Lodges; still level, it's a
draw. **Hand limit** ⚙ and **projects face up or hidden** ⚙ are also chosen when starting a game.

| Renown from | |
|---|---|
| Each Hut | 1 |
| Each Lodge | 2 |
| Each Lighthouse project | 1 |
| Longest Path title | 2 ⚙ |
| Most Markets title | 2 ⚙ |

## The island

A new island every game, made from a **seed**: the same seed and settings always make the same
island, so a good (or unfair) island can be shared and replayed.

- **The island:** a hexagon 5 hexes across (rows of 3, 4, 5, 4, 3: 19 land hexes) ⚙
  (`island.hexagon`). With it off, a random shape of 22 ⚙ land hexes (16–40) with an irregular
  coastline and no lakes inside. The hexagon's coast has room for at most 7 markets.
- **Terrain and resources:**

  | Hex | Produces | On the hexagon island ⚙ |
  |---|---|---|
  | Forest | Timber | 4 |
  | Quarry | Stone | 3 |
  | Fields | Grain | 4 |
  | Mine | Copper | 3 |
  | Paddy (shallow flooded fields, where bettas live) | Fish | 4 |
  | Dunes | nothing | 1 |

  The hexagon always has exactly these tiles (`island.forest`, `.quarry`, `.fields`, `.mine`,
  `.paddy`, `.dunes`; they must add up to 19), shuffled into new places each game. A random-shaped
  island has `island.dunes` Dunes and shares the rest out as evenly as possible, each kind at least 3
  times (so it needs at least 15 producing hexes).
- **Numbers** 2–12 (never 7) on every producing hex. The usual spread is one 2, one 12, and two each
  of 3–6 and 8–11. Smaller islands drop the rarest numbers first; bigger ones add middle numbers.
- **Fairness checks** (the generator redraws until all hold):
  - no 6 or 8 next to another 6 or 8;
  - no two equal numbers next to each other;
  - each resource's total chance of being produced is 60%–150% of the average resource's.
- **Sea** surrounds the island (every hex within 2 of land). Nobody builds on it: there's no
  sailing (removed Nov 15 to keep the game simple).
- **Markets:** 7 ⚙ on the coast, each on one coastal edge: 2 general markets (3:1) and one 2:1
  market for each resource. No two markets share or neighbour a corner.
- **The squall** starts on a Dunes hex. With no Dunes, it starts on one of the weakest hexes (the
  fewest dice rolls produce there).

## Pieces

| Piece | Where | Cost | Limit ⚙ |
|---|---|---|---|
| Path | an edge touching land (coastal edges too) | Timber + Stone | 15 |
| Hut | a corner touching land | Timber + Stone + Grain + Fish | 5 |
| Lodge | replaces your Hut (the Hut comes back to you) | 2 Grain + 3 Copper | 4 |

- **Spacing:** no building may be on a corner next to another building, anyone's. Buildings are
  at least 2 edges apart.
- **Connection:** Huts must touch one of your own Paths (except during setup). Paths must share a
  corner with one of your buildings or Paths, and can't continue through a corner holding another
  player's building.

## Setup

Seat order is random. In round 1 each player, in order, places a Hut on any free corner (keeping
the spacing) and a Path touching it. Round 2 goes in reverse order. After your second Hut, take one
resource from each producing hex around it.

## A turn

1. You may use one project before rolling (useful for a Pilot).
2. **Roll two dice.** Unless it's a 7, every hex with that number produces: 1 resource for each
   Hut and 2 for each Lodge touching it. The squall's hex produces nothing.
   **The bank is limited:** the game has 20 ⚙ cards of each resource (`bank.cards`; 0 = unlimited).
   Cards spent, discarded or traded to the bank go back into it. If the bank can't pay everyone owed
   a resource in full, nobody gets that resource this roll (the others pay as usual). Bank trades
   and Good Harvest only give resources the bank still has.
3. Then in any order: trade with the bank and markets, build, buy projects, and use one project (if
   you didn't before rolling).
4. End your turn.

## Rolling a 7

In this order:

1. Everyone holding **more than 7** ⚙ cards loses half of them (rounded down), **choosing which**.
2. The roller **moves the squall 1 to 4** ⚙ steps (counting every hex, sea included) onto a land
   hex. It must move.
3. The roller takes one random card from a player with a building touching the squall's new hex
   who has at least one card. If several qualify, the roller chooses which; never from themselves;
   nothing if nobody qualifies.

## Trading

- **Bank:** 4 of one resource for 1 of any other.
- **General market:** 3 of one resource for 1 of any other.
- **Resource market:** 2 of that resource for 1 of any other.
- You can use a market if you have a Hut or Lodge on either corner of its edge.
- **With other players** (people and bots alike): after rolling, the player whose turn it is may
  offer cards for cards to one or more of the others, e.g. 2 Timber + 1 Fish for 1 Copper. Both
  sides need at least one card, a resource can't be on both sides, and you must hold what you
  offer. Everyone sees the offer and every answer. Each player offered to accepts or declines (only
  someone holding the cards asked for can accept). The offerer then trades with one player who
  accepted, or withdraws the offer. An offer ends when it's traded, withdrawn, or the turn ends.
  - **Open offers:** the offerer can make an offer open: the first player to accept trades at once,
    without the offerer choosing.
  - **Counter-offers:** instead of accepting or declining, a player offered the trade can send back
    a changed one (any cards they hold, for any of the offerer's). The offerer may take a
    counter-offer, which trades at once, or ignore it. A player can't counter after accepting.
  At most 3 ⚙ offers a turn (`trade.offers`; 0 turns trading between players off). Players can't
  trade with each other outside the offerer's turn. Bots answer offers; they don't make them yet.

## Projects

Cost: **Grain + Copper + Fish.** The deck ⚙ has 25 cards:

| Project | Count | Effect |
|---|---|---|
| Pilot | 12 | Move the squall to any land hex, then steal as after a 7 |
| Lighthouse | 4 | 1 Renown, counted the moment you get it |
| Road Crew | 3 | Place 2 Paths for free |
| Good Harvest | 3 | Take any 2 resources from the bank |
| Good Bargain | 3 | Until the end of your turn, trade 2:1 for one resource of your choice |

- **Hidden** (the default): you draw the top card unseen. Others see how many projects you hold,
  not which. Hidden Lighthouses still count: you can win on your turn by revealing them.
- **Face up** ⚙ (`projects.faceUp`, chosen when starting a game): 3 projects lie face up and you
  choose which to buy. A new one fills the gap; when the deck is empty, the row shrinks.
- You can't use a project on the turn you bought it, and you can use at most one per turn.
  (A Lighthouse isn't "used": it counts as soon as you have it.)

## Titles

- **Longest Path** (2 Renown ⚙): the longest unbroken line of your Paths, at least 5 ⚙ long.
  Another player's building on a corner breaks the line there.
- **Most Markets** (2 Renown ⚙): your buildings touch the most different markets, at least 2 ⚙.
- To take a title from its holder you need strictly more. If the holder's line is broken and they
  drop below someone, it goes to whoever is now strictly ahead; if nobody is (a tie, or nobody
  meets the minimum), nobody holds it until someone is.

## Not in version 1 (planned)

- Bots making trade offers and counter-offers (they answer offers, including open ones).
- A turn timer, and a bot taking over the seat of a person who leaves.
- Illustrations (the board is plain and functional for now).
