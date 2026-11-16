// The exploration expansion (`explore` 1-5; RULES.md "The exploration expansion"): a known start island
// and two hidden regions to sail into. Scenarios, each adding to the one before: 1 Land Ho! (8
// Renown), 2 Pirate lairs (12), 3 Fish (15), 4 Spices (15), 5 the full game (17).
//
// - Pieces: Huts (5), harbour buildings (the Lodge pieces, 4: 2 Grain + 2 Stone, replacing a coastal
//   Hut; 2 Renown; produce 1, like a Hut), Paths, ships (3: Timber + Fish, on a sea edge by your
//   harbour), settlers (2: Timber + Grain + Copper + Fish) and crews (9: Fish + Stone), built straight
//   onto your ship by a harbour (a ship holds 1 settler or fish haul, or 2 crews or spice sacks).
// - A turn: production (nothing for you: 1 coin), the action phase, then the movement phase: each
//   ship may move once, up to 4 sea edges (6 for a Fish; +1 or +2 with Swift Voyage farms), at most
//   2 ships ending on an edge. A ship by a hidden hex explores it: the move ends; land takes a number
//   from its region's stack and pays 1 of its resource, sea 2 coins. Before or after moving, a ship
//   acts where it is: a settler founds a Hut on a free land corner at either end; crews go onto a gold
//   field's lair or a spice farm by it; a fish haul or spice sack is taken on; cargo is delivered at the
//   Council (a corner of its hex).
// - Bank trades are 3:1 (for a resource or a coin); 2 coins buy a resource, twice a turn. No Longest
//   Path, no projects. Build only on explored land, never on corners or edges by a hidden hex, a gold
//   field still held by pirates, or a spice farm without your crew.
// - Missions (tracks of 7 spaces: 1, 1, 2, 2, 2, 3, 3 Renown, and 1 more for the one furthest along):
//   pirate lairs (scenarios 2 and 5: 3 crews capture a lair; each player there 2 coins and a step, the
//   hero (a die + their crews) one more step and a crew home; the field then produces 2 coins a building),
//   fish (3 and 5: once a movement phase roll a die for a haul on that shoal; deliver to the Council),
//   spices (4 and 5: a crew on a farm stays, gives its ability and one sack per player; deliver to the
//   Council).
// - Pirate ships (2 and up): on a 7, after discards, the roller places their pirate ship on a sea hex
//   not by the start island (removing anyone else's) and robs a player with a ship on it (a card, or a
//   coin). Moving a ship from or to an edge of a hex with another player's pirate costs a coin.
//
// Simplified (RULES.md): no storage in harbours (pieces are built onto ships); a ship moves in one go
// to any edge it reaches; tribute only at the start or end of a move; no chasing pirate ships; the map
// and the lair numbers are our own.
use super::*;

pub const HARBOUR_COST: Hand = [0, 2, 2, 0, 0];
pub const XSHIP_COST: Hand = [1, 0, 0, 0, 1];
pub const SETTLER_COST: Hand = [1, 0, 1, 1, 1];
pub const CREW_COST: Hand = [0, 1, 0, 0, 1];
pub const TRACK_RENOWN: [u8; 8] = [0, 1, 1, 2, 2, 2, 3, 3];
// Cargo
pub const SETTLER: u8 = 1;
pub const CREW: u8 = 2;
pub const HAUL: u8 = 3;
pub const SACK: u8 = 4;
// Missions
pub const LAIRS: usize = 0;
pub const FISH: usize = 1;
pub const SPICES: usize = 2;
// Spice farm abilities
pub const SWIFT_VOYAGE: u8 = 0;
pub const PIRATE_BONUS: u8 = 1;
pub const FAST_GOLD: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ship {
    pub owner: u8,
    pub edge: u16,
    pub cargo: [u8; 2], // 0 empty; a settler or haul fills both (the second slot is 0)
    pub moved: bool,
}

#[derive(Clone, Default)]
pub struct ExploreState {
    pub scenario: u8,
    pub ships: Vec<Ship>,
    pub hidden: Vec<(Terrain, u8)>, // per hex: what lies under it (Sea when nothing), and its region (0 none)
    pub stacks: [Vec<u8>; 3],       // number discs per region (1 parrot, 2 goose), top last
    pub lair_numbers: Vec<u8>,
    pub lair_crews: Vec<Vec<u8>>,   // per hex: crews on a pirate lair (owners), empty when none
    pub lair_taken: Vec<bool>,
    pub farm_crews: Vec<Vec<u8>>,   // per hex: crews on a spice farm
    pub farm_kind: Vec<u8>,         // per hex: a spice farm's ability
    pub sacks: Vec<Vec<u8>>,        // per hex: players who have taken their sack here
    pub shoal: Vec<u8>,             // per hex: a shoal's die number (0: none)
    pub haul: Vec<bool>,            // per hex: a fish haul waiting
    pub hauls_left: u8,
    pub tracks: [[u8; MAX_PLAYERS]; 3],
    pub leaders: [u8; 3],           // the player holding each track's tile
    pub council: u16,
    pub start_hexes: Vec<bool>,     // per hex: the start island (no pirate beside it)
    pub pirate_of: [u16; MAX_PLAYERS], // each player's pirate ship hex (NONE)
    pub pirate_owner: u8,           // whose pirate ship is out (only one at a time)
    pub settlers_left: [u8; MAX_PLAYERS],
    pub crews_left: [u8; MAX_PLAYERS],
    pub fished: bool,               // the fish die rolled this movement phase
    pub last_fish: u8,
    pub gold_coins: [u8; MAX_PLAYERS], // fast gold used this turn
    pub boosted: Vec<bool>,         // per ship: a Fish paid for 2 more this turn
    pub chased: Vec<bool>,          // per ship: tried to chase a pirate this turn
}

impl Game {
    pub fn explore(&self) -> bool {
        self.settings().explore != 0
    }
    fn has_mission(&self, m: usize) -> bool {
        let s = self.ex.scenario;
        match m {
            LAIRS => s == 2 || s == 5,
            FISH => s == 3 || s == 5,
            _ => s == 4 || s == 5,
        }
    }
    fn pirates_on(&self) -> bool {
        self.ex.scenario >= 2
    }

    pub(super) fn explore_setup(&mut self, island: &Island) {
        let n = self.board.terrain.len();
        let e = &mut self.ex;
        e.scenario = self.board.settings.explore;
        e.hidden = vec![(Terrain::Sea, 0); n];
        for &(h, t, region) in &island.hidden {
            e.hidden[h as usize] = (t, region);
        }
        e.stacks = [Vec::new(), vec![3, 4, 5, 6, 9, 10], vec![4, 5, 8, 9, 10, 11]];
        e.lair_numbers = vec![3, 4, 5, 9, 10, 11];
        e.lair_crews = vec![Vec::new(); n];
        e.lair_taken = vec![false; n];
        e.farm_crews = vec![Vec::new(); n];
        e.farm_kind = vec![0; n];
        e.sacks = vec![Vec::new(); n];
        e.shoal = vec![0; n];
        e.haul = vec![false; n];
        e.hauls_left = 6;
        e.leaders = [NOBODY; 3];
        e.council = island.council;
        e.start_hexes = vec![false; n];
        for h in 0..n {
            e.start_hexes[h] = island.hexes[h].region == 0 && !matches!(island.hexes[h].terrain, Terrain::Sea | Terrain::Fog | Terrain::Council);
        }
        e.pirate_of = [NONE; MAX_PLAYERS];
        e.pirate_owner = NOBODY;
        e.settlers_left = [2; MAX_PLAYERS];
        e.crews_left = [9; MAX_PLAYERS];
        for k in 1..3 {
            let mut s = std::mem::take(&mut self.ex.stacks[k]);
            self.rng.shuffle(&mut s);
            self.ex.stacks[k] = s;
        }
        let mut l = std::mem::take(&mut self.ex.lair_numbers);
        self.rng.shuffle(&mut l);
        self.ex.lair_numbers = l;
        // Shoal numbers and spice abilities per region: 1-3 / 4-6, and one of each ability
        let mut shoal_no = [0u8, 1, 4];
        let mut farm_no = [0u8, 0, 0];
        for h in 0..n {
            let (t, region) = self.ex.hidden[h];
            match t {
                Terrain::Shoal => {
                    self.ex.shoal[h] = shoal_no[region as usize];
                    shoal_no[region as usize] += 1;
                }
                Terrain::Spice => {
                    self.ex.farm_kind[h] = farm_no[region as usize] % 3;
                    farm_no[region as usize] += 1;
                }
                _ => {}
            }
        }
        self.coins = [2; MAX_PLAYERS];
        self.squall = NONE;
    }

    // ---- Where things may be ----

    fn hidden_hex(&self, h: usize) -> bool {
        self.board.terrain[h] == Terrain::Fog
    }

    // A corner where nothing may be built: by a hidden hex, an untaken lair, or a spice farm without
    // p's crew
    pub fn explore_blocked_corner(&self, p: u8, c: usize) -> bool {
        self.explore() && listed(&self.board.graph.corner_hexes[c]).any(|h| self.blocked_hex(p, h))
    }
    pub fn explore_blocked_edge(&self, p: u8, e: usize) -> bool {
        self.explore() && listed(&self.board.graph.edge_hexes[e]).any(|h| self.blocked_hex(p, h))
    }
    fn blocked_hex(&self, p: u8, h: usize) -> bool {
        match self.board.terrain[h] {
            Terrain::Fog => true,
            Terrain::Gold => !self.ex.lair_taken[h],
            Terrain::Spice => !self.ex.farm_crews[h].contains(&p),
            _ => false,
        }
    }

    // A sea edge for ships: touches a sea (or shoal) hex, and no hidden hex
    fn sea_edge(&self, e: usize) -> bool {
        let hs: Vec<usize> = listed(&self.board.graph.edge_hexes[e]).collect();
        hs.iter().any(|&h| matches!(self.board.terrain[h], Terrain::Sea | Terrain::Shoal))
            && !hs.iter().any(|&h| self.hidden_hex(h))
    }

    // A setup ship's edge: a free sea edge
    pub(super) fn explore_sea_edge(&self, e: usize) -> bool {
        self.sea_edge(e) && self.ships_on(e) < 2
    }

    fn ships_on(&self, e: usize) -> usize {
        self.ex.ships.iter().filter(|s| s.edge as usize == e).count()
    }

    fn harbour_at(&self, p: u8, c: usize) -> bool {
        self.corner_owner[c] == p && self.corner_kind[c] == LODGE
    }

    // Edges next to one of p's harbours where a new ship may go
    fn harbour_edges(&self, p: u8) -> Vec<usize> {
        let g = &self.board.graph;
        (0..g.corners())
            .filter(|&c| self.harbour_at(p, c))
            .flat_map(|c| listed(&g.corner_edges[c]).collect::<Vec<_>>())
            .filter(|&e| self.sea_edge(e) && self.ships_on(e) < 2)
            .collect()
    }

    fn ship_by_harbour(&self, s: &Ship) -> bool {
        self.board.graph.edge_corners[s.edge as usize].iter().any(|&c| self.harbour_at(s.owner, c as usize))
    }

    fn room(s: &Ship, large: bool) -> bool {
        if large { s.cargo == [0, 0] } else { s.cargo[1] == 0 && !matches!(s.cargo[0], SETTLER | HAUL) }
    }

    fn load(s: &mut Ship, kind: u8) {
        if s.cargo[0] == 0 { s.cargo[0] = kind } else { s.cargo[1] = kind }
    }

    fn unload(s: &mut Ship, kind: u8) -> bool {
        if let Some(i) = s.cargo.iter().position(|&k| k == kind) {
            s.cargo[i] = 0;
            if i == 0 {
                s.cargo = [s.cargo[1], 0];
            }
            true
        } else {
            false
        }
    }

    // ---- Legal actions ----

    pub(super) fn legal_explore_main(&self, p: u8, out: &mut Vec<Action>) {
        if !self.explore() {
            return;
        }
        let pu = p as usize;
        let hand = &self.hands[pu];
        let g = &self.board.graph;
        if afford(hand, &HARBOUR_COST) && self.pieces[pu][P_LODGE] > 0 {
            for c in 0..g.corners() {
                let coastal = listed(&g.corner_edges[c]).any(|e| self.sea_edge(e));
                if self.corner_owner[c] == p && self.corner_kind[c] == HUT && coastal {
                    out.push(Action::Harbour(c as u16));
                }
            }
        }
        if afford(hand, &XSHIP_COST) && self.ex.ships.iter().filter(|s| s.owner == p).count() < 3 {
            for e in self.harbour_edges(p) {
                out.push(Action::BuildShip(e as u16));
            }
        }
        for (i, s) in self.ex.ships.iter().enumerate() {
            if s.owner != p || !self.ship_by_harbour(s) {
                continue;
            }
            if afford(hand, &SETTLER_COST) && self.ex.settlers_left[pu] > 0 && Game::room(s, true) {
                out.push(Action::BuildCargo(i as u8, SETTLER));
            }
            if (self.has_mission(LAIRS) || self.has_mission(SPICES)) && afford(hand, &CREW_COST) && self.ex.crews_left[pu] > 0 && Game::room(s, false) {
                out.push(Action::BuildCargo(i as u8, CREW));
            }
        }
        // Make room: send a ship's cargo home (by a harbour)
        for (i, s) in self.ex.ships.iter().enumerate() {
            if s.owner == p && s.cargo != [0, 0] && self.ship_by_harbour(s) {
                out.push(Action::DropCargo(i as u8));
            }
        }
        // Coins: 3:1 for a coin (the bank's rate here is 3:1 for anything), 2 coins for a resource
        for r in 0..5 {
            if hand[r] >= 3 {
                out.push(Action::BuyCoin(r as u8));
            }
        }
        if self.coins[pu] >= 2 && self.coin_buys < 2 {
            for r in 0..5 {
                if self.bank(r) > 0 {
                    out.push(Action::SpendCoins(r as u8));
                }
            }
        }
        // Fast Gold: a resource for a coin, once (twice with both farms) a turn
        let fast = self.abilities(p, FAST_GOLD);
        if fast > self.ex.gold_coins[pu] {
            for r in 0..5 {
                if hand[r] > 0 {
                    out.push(Action::FastGold(r as u8));
                }
            }
        }
    }

    // Is there a free land corner, open to p, that a ship could reach to settle?
    pub fn settle_spot_left(&self, p: u8) -> bool {
        let g = &self.board.graph;
        (0..g.corners()).any(|c| self.spaced(c) && !self.explore_blocked_corner(p, c) && listed(&g.corner_edges[c]).any(|e| self.sea_edge(e)))
    }

    // How many spice farms of this ability p holds (0-2)
    pub fn abilities(&self, p: u8, kind: u8) -> u8 {
        (0..self.ex.farm_crews.len()).filter(|&h| self.ex.farm_kind[h] == kind && self.board.terrain[h] == Terrain::Spice && self.ex.farm_crews[h].contains(&p)).count() as u8
    }

    // The movement phase: every ship that may move or act
    pub(super) fn legal_sail(&self, p: u8, out: &mut Vec<Action>) {
        let pu = p as usize;
        for (i, s) in self.ex.ships.iter().enumerate() {
            if s.owner != p {
                continue;
            }
            let i8 = i as u8;
            if !s.moved {
                for e in self.sail_reach(i, false) {
                    out.push(Action::Sail(i8, e as u16));
                }
                if self.hands[pu][4] > 0 && !self.ex.boosted[i] {
                    out.push(Action::SailFar(i8));
                }
            }
            for a in self.ship_acts(i) {
                out.push(a);
            }
        }
        if self.has_mission(FISH) && !self.ex.fished && self.ex.hauls_left > 0 {
            out.push(Action::FishDie);
        }
        // Chase another player's pirate ship: an unmoved ship with an end on a corner of its hex
        if let Some(h) = self.foreign_pirate(p) {
            let g = &self.board.graph;
            for (i, s) in self.ex.ships.iter().enumerate() {
                let by = g.edge_corners[s.edge as usize].iter().any(|&c| g.hex_corners[h].contains(&c));
                if s.owner == p && !s.moved && by && !self.ex.chased[i] {
                    out.push(Action::ChasePirate(i as u8));
                }
            }
        }
        out.push(Action::Skip);
    }

    // What ship i can do where it is: found a Hut, put crews ashore, take a haul or sack, deliver
    pub fn ship_acts(&self, i: usize) -> Vec<Action> {
        let s = self.ex.ships[i];
        let g = &self.board.graph;
        let i8 = i as u8;
        let mut out = Vec::new();
        let ends: Vec<usize> = g.edge_corners[s.edge as usize].iter().map(|&c| c as usize).collect();
        let near: Vec<usize> = {
            let mut v: Vec<usize> = ends.iter().flat_map(|&c| listed(&g.corner_hexes[c]).collect::<Vec<_>>()).collect();
            v.sort();
            v.dedup();
            v
        };
        if s.cargo.contains(&SETTLER) && self.pieces[s.owner as usize][P_HUT] > 0 {
            for &c in &ends {
                if self.spaced(c) && !self.explore_blocked_corner(s.owner, c) {
                    out.push(Action::Settle(i8, c as u16));
                }
            }
        }
        for &h in &near {
            let t = self.board.terrain[h];
            if s.cargo.contains(&CREW) {
                if t == Terrain::Gold && !self.ex.lair_taken[h] && self.has_mission(LAIRS) {
                    out.push(Action::CrewAshore(i8, h as u16));
                }
                if t == Terrain::Spice && self.has_mission(SPICES) && !self.ex.farm_crews[h].contains(&s.owner) {
                    out.push(Action::CrewAshore(i8, h as u16));
                }
            }
            if t == Terrain::Shoal && self.ex.haul[h] && Game::room(&s, true) {
                out.push(Action::TakeHaul(i8, h as u16));
            }
            if t == Terrain::Spice && self.ex.farm_crews[h].contains(&s.owner) && !self.ex.sacks[h].contains(&s.owner) && Game::room(&s, false) {
                out.push(Action::TakeSack(i8, h as u16));
            }
        }
        if near.contains(&(self.ex.council as usize)) && s.cargo.iter().any(|&k| k == HAUL || k == SACK) {
            out.push(Action::Deliver(i8));
        }
        out
    }

    // Edges ship i may end a move on: along sea edges up to its move points, never through a hidden
    // hex; an edge by a hidden hex stops it there (it explores); at most 2 ships at the end
    pub fn sail_reach(&self, i: usize, far: bool) -> Vec<usize> {
        let s = self.ex.ships[i];
        let g = &self.board.graph;
        let swift = self.abilities(s.owner, SWIFT_VOYAGE) as u32;
        let mp = if far || self.ex.boosted[i] { 6 } else { 4 } + swift;
        let mut dist = vec![u32::MAX; g.edges()];
        dist[s.edge as usize] = 0;
        let mut queue = std::collections::VecDeque::from([s.edge as usize]);
        let mut out = Vec::new();
        while let Some(x) = queue.pop_front() {
            if x != s.edge as usize && self.ships_on(x) < 2 {
                out.push(x);
            }
            if dist[x] >= mp || (x != s.edge as usize && self.by_hidden(x)) {
                continue;
            }
            for &c in &g.edge_corners[x] {
                for y in listed(&g.corner_edges[c as usize]) {
                    if dist[y] == u32::MAX && (self.sea_edge(y) || self.by_hidden(y) && self.touches_sea(y)) {
                        dist[y] = dist[x] + 1;
                        queue.push_back(y);
                    }
                }
            }
        }
        out.retain(|&e| self.sea_edge(e));
        out
    }

    // An edge a ship may be on or pass (for bots' route planning)
    pub fn sailable(&self, e: usize) -> bool {
        self.sea_edge(e) || (self.by_hidden(e) && self.touches_sea(e))
    }

    fn touches_sea(&self, e: usize) -> bool {
        listed(&self.board.graph.edge_hexes[e]).any(|h| matches!(self.board.terrain[h], Terrain::Sea | Terrain::Shoal))
    }

    // Does an edge's corner touch a hidden hex (a ship there explores it)?
    pub fn by_hidden(&self, e: usize) -> bool {
        let g = &self.board.graph;
        g.edge_corners[e].iter().any(|&c| listed(&g.corner_hexes[c as usize]).any(|h| self.hidden_hex(h)))
    }

    // ---- Doing things ----

    pub(super) fn explore_act(&mut self, a: Action) {
        let p = self.to_act();
        let pu = p as usize;
        match a {
            Action::Harbour(c) => {
                pay(&mut self.hands[pu], &HARBOUR_COST);
                self.corner_kind[c as usize] = LODGE;
                self.pieces[pu][P_LODGE] -= 1;
                self.pieces[pu][P_HUT] += 1;
            }
            Action::BuildShip(e) => {
                pay(&mut self.hands[pu], &XSHIP_COST);
                self.ex.ships.push(Ship { owner: p, edge: e, cargo: [0, 0], moved: false });
                self.ex.boosted.push(false);
                self.ex.chased.push(false);
            }
            Action::BuildCargo(i, kind) => {
                let s = &mut self.ex.ships[i as usize];
                if kind == SETTLER {
                    pay(&mut self.hands[pu], &SETTLER_COST);
                    self.ex.settlers_left[pu] -= 1;
                } else {
                    pay(&mut self.hands[pu], &CREW_COST);
                    self.ex.crews_left[pu] -= 1;
                }
                Game::load(s, kind);
            }
            Action::DropCargo(i) => {
                let s = &mut self.ex.ships[i as usize];
                for k in std::mem::take(&mut s.cargo) {
                    match k {
                        SETTLER => self.ex.settlers_left[pu] += 1,
                        CREW => self.ex.crews_left[pu] += 1,
                        HAUL => self.ex.hauls_left += 1,
                        _ => {}
                    }
                }
            }
            Action::FastGold(r) => {
                self.hands[pu][r as usize] -= 1;
                self.coins[pu] += 1;
                self.ex.gold_coins[pu] += 1;
            }
            Action::Sail(i, e) => {
                let from = self.ex.ships[i as usize].edge as usize;
                self.pay_tribute(pu, from, e as usize);
                let s = &mut self.ex.ships[i as usize];
                s.edge = e;
                s.moved = true;
                self.explore_by(pu, e as usize);
            }
            Action::SailFar(i) => {
                self.hands[pu][4] -= 1;
                self.ex.boosted[i as usize] = true;
            }
            Action::Settle(i, c) => {
                let c = c as usize;
                let ship = self.ex.ships.remove(i as usize);
                self.ex.boosted.remove(i as usize);
                self.ex.chased.remove(i as usize);
                self.ex.settlers_left[pu] += 1; // the settler goes home
                let _ = ship;
                self.corner_owner[c] = p;
                self.corner_kind[c] = HUT;
                self.pieces[pu][P_HUT] -= 1;
                self.refresh_ports();
            }
            Action::CrewAshore(i, h) => {
                let h = h as usize;
                Game::unload(&mut self.ex.ships[i as usize], CREW);
                if self.board.terrain[h] == Terrain::Gold {
                    self.ex.lair_crews[h].push(p);
                    if self.ex.lair_crews[h].len() >= 3 {
                        self.capture_lair(h);
                    }
                } else {
                    self.ex.farm_crews[h].push(p);
                }
            }
            Action::TakeHaul(i, h) => {
                self.ex.haul[h as usize] = false;
                Game::load(&mut self.ex.ships[i as usize], HAUL);
            }
            Action::TakeSack(i, h) => {
                self.ex.sacks[h as usize].push(p);
                Game::load(&mut self.ex.ships[i as usize], SACK);
            }
            Action::Deliver(i) => {
                let s = &mut self.ex.ships[i as usize];
                let mut steps = [0usize; 3];
                for k in s.cargo.iter_mut() {
                    match *k {
                        HAUL => {
                            steps[FISH] += 1;
                            *k = 0;
                        }
                        SACK => {
                            steps[SPICES] += 1;
                            *k = 0;
                        }
                        _ => {}
                    }
                }
                if s.cargo[0] == 0 {
                    s.cargo = [s.cargo[1], 0];
                }
                if steps[FISH] > 0 {
                    self.ex.hauls_left += steps[FISH] as u8;
                }
                for m in [FISH, SPICES] {
                    for _ in 0..steps[m] {
                        self.track_step(pu, m);
                    }
                }
            }
            Action::ChasePirate(i) => {
                // A 6 chases it off (5-6 with one Pirate Bonus farm, 4-6 with both); then your own pirate
                self.ex.chased[i as usize] = true;
                let need = 6 - self.abilities(p, PIRATE_BONUS);
                if self.rng.below(6) as u8 + 1 >= need {
                    self.pending.insert(0, Pending { seat: p, decision: Decision::PiratePlace });
                }
            }
            Action::FishDie => {
                self.ex.fished = true;
                let die = self.rng.below(6) as u8 + 1;
                self.ex.last_fish = die;
                if let Some(h) = (0..self.ex.shoal.len()).find(|&h| self.ex.shoal[h] == die && self.board.terrain[h] == Terrain::Shoal) {
                    let pirate = self.ex.pirate_owner != NOBODY && self.ex.pirate_of[self.ex.pirate_owner as usize] == h as u16;
                    if !self.ex.haul[h] && !pirate && self.ex.hauls_left > 0 {
                        self.ex.haul[h] = true;
                        self.ex.hauls_left -= 1;
                    }
                }
            }
            Action::Skip => self.decided(None), // the movement phase ends
            Action::PiratePlace(h) => {
                if self.ex.pirate_owner != NOBODY {
                    let o = self.ex.pirate_owner as usize;
                    self.ex.pirate_of[o] = NONE;
                }
                self.ex.pirate_of[pu] = h;
                self.ex.pirate_owner = p;
                if self.board.terrain[h as usize] == Terrain::Shoal && self.ex.haul[h as usize] {
                    self.ex.haul[h as usize] = false;
                    self.ex.hauls_left += 1;
                }
                // Rob a player with a ship on that hex
                let g = &self.board.graph;
                let victims: Vec<u8> = self.ex.ships.iter().filter(|s| s.owner != p && listed(&g.edge_hexes[s.edge as usize]).any(|x| x == h as usize)).map(|s| s.owner).collect();
                self.decided(None);
                if let Some(&v) = victims.iter().max_by_key(|&&v| self.renown(v as usize)) {
                    if self.cards(v as usize) > 0 {
                        self.rob(pu, v as usize);
                    } else if self.coins[v as usize] > 0 {
                        self.coins[v as usize] -= 1;
                        self.coins[pu] += 1;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    // A ship by a hidden hex turns it over: land takes its region's next number and pays 1 resource
    // (none: 2 coins); a gold field takes a lair; sea pays 2 coins
    fn explore_by(&mut self, p: usize, e: usize) {
        let g = self.board.graph.clone();
        let mut hexes = self.board.hexes.clone();
        let mut any = false;
        for &c in &g.edge_corners[e] {
            for h in listed(&g.corner_hexes[c as usize]) {
                if hexes[h].terrain != Terrain::Fog {
                    continue;
                }
                let (t, region) = self.ex.hidden[h];
                hexes[h].terrain = t;
                any = true;
                self.revealed.push(h as u16);
                match t {
                    Terrain::Gold => {
                        hexes[h].number = self.ex.lair_numbers.pop().unwrap_or(0);
                    }
                    _ if t.resource().is_some() => {
                        hexes[h].number = self.ex.stacks[region as usize].pop().unwrap_or(0);
                    }
                    _ => {}
                }
                match t.resource() {
                    Some(r) if self.bank(r) > 0 => self.hands[p][r] += 1,
                    _ => self.coins[p] += 2,
                }
            }
        }
        if any {
            let markets = self.board.markets.clone();
            let settings = self.board.settings.clone();
            let markers = self.board.corner_marker.clone();
            let mut b = Board::from_parts(g, hexes, markets, settings);
            b.corner_marker = markers;
            self.board = Arc::new(b);
        }
    }

    // The hex of another player's pirate ship, if one is out
    fn foreign_pirate(&self, p: u8) -> Option<usize> {
        let o = self.ex.pirate_owner;
        (self.pirates_on() && o != NOBODY && o != p).then(|| self.ex.pirate_of[o as usize] as usize)
    }

    fn pay_tribute(&mut self, p: usize, from: usize, to: usize) {
        let o = self.ex.pirate_owner;
        if o == NOBODY || o as usize == p || !self.pirates_on() {
            return;
        }
        let h = self.ex.pirate_of[o as usize] as usize;
        let g = &self.board.graph;
        let by = |e: usize| listed(&g.edge_hexes[e]).any(|x| x == h);
        if (by(from) || by(to)) && self.coins[p] > 0 {
            self.coins[p] -= 1;
        }
    }

    fn capture_lair(&mut self, h: usize) {
        self.ex.lair_taken[h] = true;
        let crews = std::mem::take(&mut self.ex.lair_crews[h]);
        let mut involved: Vec<u8> = Vec::new();
        for k in 0..self.players {
            let o = ((self.current as usize + k) % self.players) as u8;
            if crews.contains(&o) {
                involved.push(o);
            }
        }
        for &o in &involved {
            self.coins[o as usize] += 2;
            self.track_step(o as usize, LAIRS);
        }
        // The hero: a die + their crews there; ties to more crews, then dice again
        let count = |o: u8| crews.iter().filter(|&&x| x == o).count() as u32;
        let mut best = (0u32, 0u32, 0u32, NOBODY);
        for &o in &involved {
            let key = (self.rng.below(6) as u32 + 1 + count(o), count(o), self.rng.below(1000) as u32, o);
            if (key.0, key.1, key.2) > (best.0, best.1, best.2) {
                best = key;
            }
        }
        let hero = best.3;
        self.track_step(hero as usize, LAIRS);
        // The crews go home (the hero's one first); the field now produces
        for &o in &crews {
            self.ex.crews_left[o as usize] += 1;
        }
    }

    fn track_step(&mut self, p: usize, m: usize) {
        if self.ex.tracks[m][p] < 7 {
            self.ex.tracks[m][p] += 1;
        }
        // The tile: the furthest along (a tie keeps it where it is: the first to get there)
        let best = *self.ex.tracks[m][..self.players].iter().max().unwrap();
        let holder = self.ex.leaders[m];
        if holder == NOBODY || self.ex.tracks[m][holder as usize] < best {
            self.ex.leaders[m] = (0..self.players).find(|&o| self.ex.tracks[m][o] == best).map_or(NOBODY, |o| o as u8);
        }
    }

    pub(super) fn explore_renown(&self, p: usize) -> u32 {
        if !self.explore() {
            return 0;
        }
        (0..3).map(|m| TRACK_RENOWN[self.ex.tracks[m][p] as usize] as u32 + (self.ex.leaders[m] as usize == p && self.ex.tracks[m][p] > 0) as u32).sum()
    }

    // ---- The turn ----

    // The movement phase at the end of the turn
    pub(super) fn start_sailing(&mut self) -> bool {
        if !self.explore() {
            return false;
        }
        for s in self.ex.ships.iter_mut() {
            s.moved = false;
        }
        self.ex.fished = false;
        self.decide_then([Pending { seat: self.current, decision: Decision::Sail }], Phase::TurnEnd);
        true
    }

    // After the turn: this turn's markers end
    pub(super) fn explore_end_phase(&mut self) {
        if !self.explore() {
            return;
        }
        for b in self.ex.boosted.iter_mut() {
            *b = false;
        }
        self.ex.chased = vec![false; self.ex.ships.len()];
        self.ex.gold_coins = [0; MAX_PLAYERS];
    }

    // Production: a gold field with its lair taken pays 2 coins a building; nothing at all pays 1 coin
    pub(super) fn explore_production(&mut self, total: u8, before: &[u32; MAX_PLAYERS]) {
        if !self.explore() {
            return;
        }
        let g = &self.board.graph;
        let mut coins = [0u8; MAX_PLAYERS];
        for h in 0..self.board.terrain.len() {
            if self.board.terrain[h] == Terrain::Gold && self.ex.lair_taken[h] && self.board.number[h] == total {
                for &c in &g.hex_corners[h] {
                    let o = self.corner_owner[c as usize];
                    if (o as usize) < self.players {
                        coins[o as usize] += 2;
                    }
                }
            }
        }
        for p in 0..self.players {
            self.coins[p] += coins[p] as u16;
            if coins[p] == 0 && self.cards(p) == before[p] {
                self.coins[p] += 1;
            }
        }
    }

    // A 7's pirate ship: any sea hex not by the start island, and not the one it's on
    pub(super) fn legal_pirate_place(&self, out: &mut Vec<Action>) {
        let g = &self.board.graph;
        let current = if self.ex.pirate_owner == NOBODY { NONE } else { self.ex.pirate_of[self.ex.pirate_owner as usize] };
        for h in 0..self.board.terrain.len() {
            if !matches!(self.board.terrain[h], Terrain::Sea | Terrain::Shoal) || h as u16 == current {
                continue;
            }
            let cell = g.cells[h];
            let by_start = (0..self.board.terrain.len()).any(|x| self.ex.start_hexes[x] && g.cells[x].distance(cell) <= 1);
            // (only hexes the drawn board shows: within reach of land)
            let near_land = (0..self.board.terrain.len()).any(|x| !matches!(self.board.terrain[x], Terrain::Sea) && g.cells[x].distance(cell) <= 1);
            if !by_start && near_land {
                out.push(Action::PiratePlace(h as u16));
            }
        }
    }
}

#[cfg(test)]
#[path = "explore_tests.rs"]
mod tests;
