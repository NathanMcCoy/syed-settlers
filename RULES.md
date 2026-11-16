# Syed Settlers: rules

Version 0.9 (draft, Nov 16 2026: the city expansion; every sea scenario. 0.8: sea scenarios Four Islands, Through the Desert and New World. 0.7: the variants box's four variants. 0.6: the sea expansion's first scenario, New Shores. 0.5: 5 and 6 players, from the classic game's 5-6 player rules: the
large island, more cards and projects, and paired turns. 0.4 (Nov 15): as close as we can make it to
the classic game this one is modelled on, checked against its rulebook: other players' buildings block Paths and break
Longest Path again; no limit on trade offers; markets spread evenly round the coast; numbers in the
classic spiral. 0.3: Most Pilots replaces Most Markets; Market Day replaces Good Bargain, with the
deck re-counted; the squall goes to any land hex after a 7; 9 markets; a bank of 19; a short bank
pays its last cards when only one player is owed. Earlier the same day: sailing removed; Stone and
Copper swapped in every recipe). Numbers marked ⚙ are settings in `engine/src/settings.rs`.
They're first guesses: playtesting and bot simulations will tune them.

## Players

2–6 seats ⚙. **Each seat is a person or a bot, in any mix**: you and 3 bots, you and a friend and
a bot, six friends. People play from their own devices.

**With 5 or 6 players** the game grows (these are the defaults for 5-6 seats; any can still be
changed):
- **The large island** ⚙ (`island.large`): 30 hexes in rows of 3, 4, 5, 6, 5, 4, 3: 6 Forest,
  5 Quarry, 6 Fields, 5 Mine, 6 Paddy, 2 Dunes. Its 28 numbers are two 2s, two 12s and three of every
  other. (The classic game lays them in a spiral too, but its order for this island isn't in the
  rulebook we have, so they're shuffled with the fairness checks below.)
- **11 markets:** 5 general (3:1) and one 2:1 market for each resource plus a second 2:1 Fish
  market ⚙ (`markets.extraFish`).
- **24 cards of each resource** in the bank, and **34 projects**: 20 Pilot, 5 Lighthouse,
  3 Road Crew, 3 Good Harvest, 3 Market Day.
- **Paired turns** ⚙ (`turns.paired`): see "A turn".

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
| Most Pilots title | 2 ⚙ |

## The island

A new island every game, made from a **seed**: the same seed and settings always make the same
island, so a good (or unfair) island can be shared and replayed.

- **The island:** a hexagon 5 hexes across (rows of 3, 4, 5, 4, 3: 19 land hexes) ⚙
  (`island.hexagon`). With it off, a random shape of 22 ⚙ land hexes (16–40) with an irregular
  coastline and no lakes inside.
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
- **How numbers are placed:** on the hexagon with 1 Dunes (the default), the classic way: the
  numbers 5, 2, 6, 3, 8, 10, 9, 12, 11, 4, 8, 10, 9, 4, 5, 6, 3, 11 go down in that order in a
  spiral, starting at a random corner of the outer ring and going counterclockwise inwards,
  skipping the Dunes ⚙ (`island.spiral`). The order keeps 6s and 8s apart; nothing is redrawn.
- **Fairness checks** for every other island (spiral off, other shapes or Dunes counts): numbers
  are shuffled and the generator redraws until all hold:
  - no 6 or 8 next to another 6 or 8;
  - no two equal numbers next to each other;
  - each resource's total chance of being produced is 60%–150% of the average resource's.
- **Sea** surrounds the island (every hex within 2 of land). Nobody builds on it: there's no
  sailing (removed Nov 15 to keep the game simple).
- **Markets:** 9 ⚙ on the coast, each on one coastal edge: 4 general markets (3:1) and one 2:1
  market for each resource, spread evenly round the coast like harbours on a sea frame (on the
  hexagon, 3 or 4 coastal edges apart), starting at a random place, kinds shuffled. No two markets
  share a corner (they may be on neighbouring corners).
- **The squall** starts on a Dunes hex. With no Dunes, it starts on one of the weakest hexes (the
  fewest dice rolls produce there).

## Pieces

| Piece | Where | Cost | Limit ⚙ |
|---|---|---|---|
| Path | an edge touching land (coastal edges too) | Timber + Copper | 15 |
| Hut | a corner touching land | Timber + Grain + Copper + Fish | 5 |
| Lodge | replaces your Hut (the Hut comes back to you) | 3 Stone + 2 Grain | 4 |

- **Spacing:** no building may be on a corner next to another building, anyone's. Buildings are
  at least 2 edges apart.
- **Connection:** Huts must touch one of your own Paths (except during setup). Paths must share a
  corner with one of your buildings or Paths, and can't continue past a corner holding another
  player's building.

## Setup

Seat order is random. In round 1 each player, in order, places a Hut on any free corner (keeping
the spacing) and a Path touching it. Round 2 goes in reverse order. After your second Hut, take one
resource from each producing hex around it.

## A turn

1. You may use one project before rolling (useful for a Pilot).
2. **Roll two dice.** Unless it's a 7, every hex with that number produces: 1 resource for each
   Hut and 2 for each Lodge touching it. The squall's hex produces nothing.
   **The bank is limited:** the game has 19 ⚙ cards of each resource (`bank.cards`; 0 = unlimited).
   Cards spent, discarded or traded to the bank go back into it. If the bank can't pay everyone owed
   a resource in full, nobody gets that resource this roll (the others pay as usual), unless only
   one player is owed it: they take what's left. Bank trades and Good Harvest only give resources
   the bank still has.
3. Then in any order: trade with the bank and markets, build, buy projects, and use one project (if
   you didn't before rolling).
4. End your turn.

**Paired turns (5 or 6 players):** when a player ends their turn, the player **3 seats after them**
takes an **extra build phase**: no roll, but they may trade with the bank and markets, build, buy
projects and use one project. They may not trade with other players. If they reach the Renown to
win during it, they win. Then the next player's turn starts as usual. (A project bought in one of
your phases can be used in your next one, whichever kind it is.)

## Rolling a 7

In this order:

1. Everyone holding **more than 7** ⚙ cards loses half of them (rounded down), **choosing which**.
2. The roller **moves the squall** to any other land hex. It must move. (⚙ `squall.range`, 0 by
   default: 1–8 limits it to that many steps, counting every hex, sea included. A Pilot always goes
   anywhere.)
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
  As many offers a turn as you like ⚙ (`trade.offers`: a limit, 0 = none, the default;
  `trade.players` off turns trading between players off). Players can't
  trade with each other outside the offerer's turn. Bots answer offers; they don't make them yet.

## Projects

Cost: **Stone + Grain + Fish.** The deck ⚙ has 25 cards:

| Project | Count | Effect |
|---|---|---|
| Pilot | 14 | Move the squall to any land hex, then steal as after a 7. Played Pilots count toward Most Pilots |
| Lighthouse | 5 | 1 Renown, counted the moment you get it |
| Road Crew | 2 | Place 2 Paths for free |
| Good Harvest | 2 | Take any 2 resources from the bank |
| Market Day | 2 | Name a resource: every other player gives you all of theirs |

- **Hidden** (the default): you draw the top card unseen. Others see how many projects you hold,
  not which. Hidden Lighthouses still count: you can win on your turn by revealing them.
- **Face up** ⚙ (`projects.faceUp`, chosen when starting a game): 3 projects lie face up and you
  choose which to buy. A new one fills the gap; when the deck is empty, the row shrinks.
- You can't use a project on the turn you bought it, and you can use at most one per turn.
  (A Lighthouse isn't "used": it counts as soon as you have it.)

## Titles

- **Longest Path** (2 Renown ⚙): the longest unbroken line of your Paths, at least 5 ⚙ long.
  Another player's building on one of its corners breaks it there. If that leaves the holder under
  5, or level with someone, the title goes to whoever is now strictly ahead, or to nobody.
- **Most Pilots** (2 Renown ⚙): you've played the most Pilots, at least 3 ⚙. Played Pilots stay
  on show to everyone.
- To take a title from its holder you need strictly more. If two or more players tie for the lead
  when nobody holds it yet, nobody holds it until one of them is strictly ahead.

## The variants box: variants ⚙ (chosen when starting a game)

- **Friendly squall** (`variant.friendly`): the squall may not go next to a building of a player with 2 Renown or less
  (public Renown: hidden Lighthouses don't count) and never robs them. With nowhere allowed, it goes to the Dunes.
- **Strongest Ports** (`variant.ports`): a title worth 2 Renown for the most Renown in buildings on markets (a Hut 1,
  a Lodge 2), at least 3. 1 more Renown to win.
- **Event cards** (`variant.events`): a deck of 36 cards replaces the dice, one per two-dice combination (so the numbers
  come up as often as with dice, but every number in a deck comes up exactly as often as it should). Each card's event
  is resolved, then its number produces. Every 7 is "the squall attacks" (a 7 as usual). The other events: a beautiful
  day (nothing), the squall flees (to the Dunes, no theft), conflict (Most Pilots' holder, or whoever has played the most,
  robs a player of their choice), trade advantage (Longest Path's holder does the same), calm seas (the most buildings on
  markets: a resource of their choice), tournament (the most Pilots played: the same), a plentiful year (everyone takes a
  resource of their choice), good neighbours (everyone gives a card of their choice to the player on their left), a
  helpful neighbour (the player(s) with the most Renown give a card to someone with less), epidemic (Lodges produce 1),
  earthquake (everyone damages one of their own Paths: no new Paths until it's repaired for Timber + Copper, and no Hut
  next to a damaged Path; damaged Paths still count for Longest Path). The New Year card lies 5 cards from the bottom and
  rebuilds the deck. Which card carries which event is ours (the rulebook shows the cards only as pictures): ten
  beautiful days, two of each other event, six 7s.
- **Two players** (`variant.two`, 2 seats only): two neutral colours start with a Hut each on the main island. After
  building a Path you build one for a neutral (either, joined to its network); after a Hut, a neutral Hut (or a Path if
  no Hut fits). Neutrals get no resources and can hold Longest Path. Each turn has two production rolls, the second a
  different number. Trade tokens: 2 for a Hut next to the Dunes, 1 for a Hut on the coast (setup Huts too), 2 for
  cashing in one played Pilot a turn (it no longer counts for Most Pilots). Once a turn, before or after rolling, spend
  1 token (2 if your public Renown is higher than the other player's) on a forced trade (take 2 random cards from the
  other player, then give back 2 of your choice) or on sending the squall to the Dunes (no theft).

## The sea expansion ⚙ (`sea`, chosen when starting a game)

From the classic game's sea expansion. Version 1 has its first scenario, **New Shores** (`sea.scenario` 1); the rest
are planned (EXPANSIONS.md). Everything above still applies, with these changes.

- **The map:** the main island as usual (the hexagon, or the large island with 5-6 players), plus **small islands**
  with at least one sea hex between any two pieces of land. With 2-4 players: 3 small islands of 3 hexes, holding
  2 Mine, 1 Forest, 1 Paddy, 1 Fields, 2 Quarry and **2 Gold** (the rulebook's 4-player tiles), numbered 2, 3, 4, 5, 6,
  8, 9, 10, 11 (no 6 or 8 together). With 5-6 players, a fourth island of 2 more Gold numbered 3 and 11 (our own: that
  map is only a picture in the rulebook). Markets are on the main island; the squall starts on its Dunes; the
  **pirate** starts on a sea hex two steps from the main island.
- **Winning:** 14 Renown (the scenario's default; `win` can change it).
- **Setup:** Huts on the main island only. A Hut on the coast may take a **ship** instead of a Path.
- **Ships** (Timber + Fish, 15 ⚙ each, `pieces.ship`): on an edge touching the sea; a coastal edge takes a Path or a
  ship, not both. A ship joins one of your buildings or ships (not a Path), never past another player's building, and
  never on an edge of the pirate's hex. A Hut may be built where your ship reaches land.
- **Moving a ship:** once a turn (in an extra build phase too), move one of your ships with an open end (no ship or
  building of yours there) that wasn't built this turn and isn't by the pirate, to any spot a new ship could go.
- **Longest Path** counts Paths and ships; they join only at one of your own buildings.
- **Gold fields** produce a resource of your choice: 1 per Hut, 2 per Lodge (any mix). Players choose in turn order,
  starting with the roller. A setup Hut on gold takes one of your choice. Gold pays nothing when the bank is empty.
- **The pirate:** after a 7, or with a Pilot, move the squall *or* the pirate (to another sea hex), then steal from a
  player with a ship on its edges.
- **Road Crew** places 2 Paths, 2 ships, or one of each.
- **Island bonus** ⚙ (`sea.islandBonus`): the islands of your setup Huts are your home islands; your first Hut on
  each other island earns 2 Renown tokens (in New Shores that's every small island).

### Other sea scenarios (`sea.scenario`)

Each map is made from the seed to the scenario's rules and tile counts; the layouts are our own.

- **2 Four Islands** (13 Renown): four islands of 5 hexes, every resource on 4 of them (six islands with 5-6 players),
  no main island and no Dunes; the squall starts on a 2 or 12. Set up on any island (one or two: your home islands);
  every other island is a bonus (2). 8 markets (3 general), on any coast.
- **3 Fog Islands** (12 Renown): two start islands face up (17 tiles: 3 Mine, 4 Forest, 4 Paddy, 3 Fields, 3 Quarry)
  and two areas of 6 hidden hexes a sea hex away. Under the fog (shuffled, unseen by anyone): 2 sea, 2 Gold, 2 Mine,
  1 Forest, 1 Paddy, 2 Fields, 2 Quarry, with 10 numbers for the land. Placing (or moving) a Path or ship next to a
  hidden hex's corner turns it over: land gets the next number, and the player takes one of what it produces (gold:
  their choice). Set up on either start island (or land already revealed); no island bonus. 6s and 8s may touch here.
- **4 Through the Desert** (14 Renown): the main island with a desert of 2 more Dunes along its east side, a strip of
  4 hexes beyond it (Forest, Paddy, Fields, Mine; numbers 3, 5, 9, 11) and two small islands of 2 hexes holding the
  2 Gold, a Quarry and a Paddy (4, 6, 8, 10). Set up on the main island; the strip and each small island are a bonus (2).
- **5 The Forgotten Tribe** (13 Renown): New Shores' map (no island bonus). 8 Renown tokens, 4 face-down projects and
  3 markets (one general, two of random resources) lie on sea edges touching the small islands, no two sharing a
  corner. The first ship built or moved onto such an edge takes it: a Renown token; a project, as if bought this turn;
  a market, placed (a move in the main phase) on a coastal edge at one of your buildings with no market on it or on an
  edge sharing a corner with it, and kept until there is one. Huts only next to a numbered hex; the squall only on
  numbered hexes.
- **6 Cloth** (14 Renown): two large islands of 12 (2 Gold, 3 Mine, 4 Forest, 4 Paddy, 5 Fields, 4 Quarry, 2 Dunes) and
  four small islands of two **villages** each (numbers 3-11, 5 cloth each; 10 more in a general supply). No Huts on
  the small islands; the squall never goes there. Three setup Huts each (forward, back, forward; only the third
  collects). A line of your Paths and ships from one of your buildings to a village's corner is a trade relation:
  1 cloth from it at once, and whenever its number is rolled each linked player takes 1 (from the village, then the
  general supply; an empty village sends nothing). 2 cloth = 1 Renown. Ships on such a route can't move. The pirate
  can't be moved by a player with no trade relation. No Longest Path. Ends at 14 Renown, or at the end of a turn with
  5 empty villages: the most Renown wins, then the most cloth, else a draw.
- **7 The Pirate Islands** (a fortress and 10 Renown): the main island and, evenly spread round it 3 hexes out, one pirate
  islet of 2 hexes per possible seat (4, or 6 with 5-6 players), each with a fortress (its corner farthest out, held by 3
  lairs; it produces nothing) and a beachhead (its nearest coast corner not touching the fortress). No squall and no
  pirate: a 7 means discards, then the roller robs any player. The pirate fleet circles the sea hexes round the main
  island: after every roll it sails the lower die's number of hexes, then fights each player with a building on its hex
  (its strength the lower die, theirs their warships): stronger fleet, they lose a random card plus one per Lodge;
  stronger player, they take a resource of their choice; a tie, nothing. Each player starts with a Hut on the main
  island's coast nearest their beachhead and a ship from it (the rulebook's pre-placed pieces), then sets up two Huts as
  usual. All of a player's ships form one unbranching line, built from its tip (or, to start, from their building
  nearest the target) one step closer along a shortest free route to their beachhead, then their fortress; a line walled
  in by others may take any free sea edge from its tip. Ships don't move. A Hut may go on the player's own beachhead.
  Pilots only turn the line's first normal ship into a warship (Lighthouses are Pilots with 4+ players, left out with
  fewer). At the end of each turn of a player whose line reached their fortress, they attack it: more warships than a
  die take a lair (the last one makes the fortress their Hut); fewer lose the 2 ships nearest the fortress, a tie 1.
  A captured fortress and 10 Renown win. No Longest Path or Most Pilots. (Ours: the islets' layout; a walled-in line's
  detour; all ships being the one line.)
- **8 Wonders**: New Shores' map (small islands a bonus of 1) with no pirate; 5 wall markers and 2 bridge markers on
  main-island coast corners (none touching), and no setup Hut on a marker or next to a bridge marker. Claim one wonder
  once its requirement is met (one each, never the same one): Great Bridge (a building on a bridge marker), Great Wall
  (a building on a wall marker), Grand Theater (2 Lodges), Grand Monument (a Lodge on a market and a line of 5 Paths or
  ships), Grand Castle (a Lodge and 6 Renown). Then build its 4 levels, each costing the same 5 cards (read off the
  tiles): Great Bridge 3 Copper + Timber + Grain, Great Wall 3 Timber + Fish + Grain, Grand Theater 3 Grain + 2 Stone,
  Grand Monument 3 Fish + Copper + Timber, Grand Castle 3 Stone + Copper + Grain. Win by finishing your wonder, or with
  10 Renown and a higher level than everyone else; Renown alone doesn't win. (The 5-6 player box's two more wonders
  aren't in yet: their costs are pictures we haven't read.)
- **9 New World** (12 Renown): the rulebook's 23 tiles (4 Mine, 5 Forest, 5 Paddy, 5 Fields, 4 Quarry; 2 more of
  each with 5-6 players) on 3 to 5 random islands; 10 markets (5 general) on any coast. Set up anywhere; every island
  beyond your home islands is a bonus of 1.
- Every sea scenario is in.

## The variants box: scenarios ⚙ (`box.scenario`, chosen when starting a game)

Played on their own: not with the sea, the city expansion, event cards or the two-player variant.

- **1 Fishing** (10 Renown): the Dunes become the **Lake**, moved to the middle (the centre hex or a neighbour; the hex
  it swaps with takes the Dunes' place and number). It pays on 2, 3, 11 and 12. Six **fishing grounds** (sea hexes
  touching the island, spread evenly round it, away from the sea hexes the markets face) are numbered 4, 5, 6, 8, 9,
  10. A rolled number pays every building on the Lake's or a fishing ground's corners in **fish tokens**: Hut 1,
  Lodge 2, drawn one at a time in turn order from the roller from a bag of 11 one-fish, 10 two-fish, 8 three-fish
  tokens and the **old boot**. At most 7 tokens (the boot aside); a player at 7 draws no more. The bag refills from
  spent tokens. A second setup Hut by fishing water draws 1. Fish aren't cards (no hand limit, no stealing, no
  trading). Spend on your turn after rolling (excess fish lost; the tokens that waste the fewest fish are used):
  2 the squall leaves the island (it returns with a 7 or a Pilot, anywhere), 3 steal a card from a player of your
  choice, 4 a resource of your choice, 5 a free Path, 7 a free project (counted as bought this turn). The boot's
  holder needs 1 more Renown to win and may pass it on their turn to a player with as much public Renown or more.
  The squall starts off the island; it may stand on the Lake (no fish on its four numbers). (Ours: fishing grounds
  pay their coast corners; no swap at 7 tokens.)
- **2 Rivers** (10 Renown): two rivers of 3 land hexes each in a line of neighbours, from a coastal hex inland,
  each holding one Swamp (Dunes; `island.dunes` 2, `island.paddy` 3 on the hexagon: 17 numbered hexes, and the 12's
  hex pays on a 2 too); the rivers don't touch. Bridge sites: each river's mouth (a coast edge of its first hex) and
  the edges between its hexes. No Path on a bridge site; a bridge (Timber + 2 Copper, 3 each) goes there, connected
  like a Path, counting as a Path for Longest Path, not with Road Crew. Coins: 1 per Hut or Path built on a river
  hex (setup included), 3 per bridge; buy a coin from the bank with a resource at your rate; 2 coins buy a resource,
  twice a turn. Coins aren't cards. The single most coins: +1 Renown (Wealthiest); everyone tied fewest: -2 (Poor;
  Renown never below 0). (Ours: the layout; no coins in trades between players.)
- **3 Merchant trains** (12 Renown): the Dunes become the watering hole on the centre hex (no number); the squall
  starts off the island. Building any Hut or Lodge in a phase earns 1 wagon, placed when that phase ends after a vote:
  each player from the one acting bids Fish and Grain (spent; up to 9 of each). More than half the votes places it;
  else the single most; else the player acting. Wagons go on land edges without a wagon, not the watering hole's own
  edges: a new train from an unused start (corners 0, 2, 4 of the watering hole, on the edge leading away) or on the
  front of a train that hasn't ended; a train ends when its front reaches a corner with another wagon, or can't go
  on. 22 wagons. A building with 2+ wagons at its corner: +1 Renown. A wagon on a Path: +1 to its owner's line
  length. (Ours: no vote negotiation.)
- Still to come: 4 Raider attack, 5 Traders and raiders.

## The city expansion ⚙ (`city`, chosen when starting a game)

From the classic game's cities-and-knights expansion, with our own names. 13 Renown to win (with the sea: only
scenarios 1, 4 and 9, at their target + 2). Not with event cards or the two-player variant. The page's City tab holds
improvements and progress cards; walls and knights are in the Build shop.

- **Setup:** the second setup building is a Lodge (it collects the starting resources). No projects, no Most Pilots.
- **Three dice:** red and yellow for production; the **event die** (3 ship faces, a Science, a Trade and a Politics
  gate) is settled first. A ship moves the **raiders** 1 step (`city.raiders`, 7, to land). A gate gives a progress
  card of its colour, in turn order from the roller, to everyone with a level of at least 1 on that track and the
  red die at most their level + 1.
- **The squall** (and with the sea, the pirate) stays off the island until the raiders first land; before that a 7
  only makes big hands discard.
- **Commodities** (`bank.goods`, 12 of each; 18 with 5-6 players): a Lodge on Forest makes 1 Timber + 1 Paper, on
  Paddy 1 Fish + 1 Cloth, on Quarry 1 Stone + 1 Coin; on Fields and Mines 2 of the resource; gold no commodities.
  They're cards: in the hand limit, stolen, discarded and traded like resources (bank 4:1, 3:1 at a general market;
  2:1 markets are for their resource only).
- **Improvements:** Science (Paper), Trade (Cloth), Politics (Coin); level n costs n of the track's commodity, with a
  Lodge on the island. Level 3: Science gives a resource of your choice after any roll but a 7 that gives you nothing;
  Trade trades commodities 2:1; Politics allows mighty knights. The first to level 4 places the track's
  **metropolis** on a Lodge (+2 Renown, can't be pillaged); the first to level 5 takes it for good. A level that would
  bring a metropolis needs a Lodge free for it.
- **Walls** (2 Copper): under a Lodge without one, 3 at most, +2 hand limit each; lost when the Lodge is pillaged.
- **Knights** (2 each of levels 1-3): recruit (Fish + Stone) a basic knight, inactive, on a free corner touching your
  Path or ship; promote (Fish + Stone) once per knight a turn, to mighty only at Politics 3; activate (Grain). A knight
  active since before the turn may move along your Paths and ships (past your pieces, not others') to a free corner,
  displace a weaker knight of another player (who moves it along their routes, or loses it if it can't move), or
  chase the squall or pirate from a corner of its hex (then move it and steal). Each action makes it inactive.
  Knights block Hut spots and other players' Paths and Longest Path.
- **The raiders land:** strength = Lodges on the island, defence = active knights' levels. Stronger raiders: the
  players with the lowest defence who have a Lodge that can be pillaged each turn one into a Hut (their choice).
  Otherwise the single best defender takes a Renown token, or tied best each draw a progress card from a deck of their
  choice. Then all knights become inactive and the raiders go back.
- **Progress cards** (54): at most 4 in hand (off your turn: drop at once; your turn: by its end). Play any number on
  your turn after the roll; Fixed Roll before it. Renown cards (Printing Press, Charter) count at once. Used and
  dropped cards go under their deck. Science: Fixed Roll 2, Crane 2, Engineer 1, Number Swap 2, Irrigation 2,
  Medicine 2, Quarrying 2, Printing Press 1, Path Builders 2, Smithing 2. Trade: Harbour Deal 2, Guild Dues 2,
  Merchant 6, Trade Fleet 2, Monopoly 4, Goods Monopoly 2. Politics: Charter 1, Diplomacy 2, Rally 2, Spy 3, Intrigue 2,
  Taxes 2, Treason 2, Wedding 2, Sabotage 2. (What each does: the site's rules page.)
- **Ours:** a metropolis goes on the owner's Lodge with the best dice; Treason places the strongest knight allowed; a
  pillaged Lodge becomes a Hut even with no Hut piece left (the next Lodge built from it returns the debt).

## Not in version 1 (planned)

- Bots making trade offers and counter-offers (they answer offers, including open ones).
- A turn timer, and a bot taking over the seat of a person who leaves.
- Illustrations (the board is plain and functional for now).
