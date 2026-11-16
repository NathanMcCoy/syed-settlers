// Decisions by other seats (F7 in EXPANSIONS.md): things the rules make one or more players decide
// before play goes on, often players whose turn it isn't. They wait in a queue: `to_act()` is the
// first one's seat, `legal()` lists that decision's choices, and once the queue is empty the game
// goes on to `after`. The base game has one kind, a 7's discards; expansions add kinds (choose a gold
// field's resource, give a card to a neighbour, move a displaced knight, bid in a vote) as rows here.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    Discard { left: u8 }, // lose this many more cards, one Discard action each (after a 7)
    Gold { left: u8 },    // take this many more resources of your choice, one Take each (a gold field)
    Pick { left: u8 },    // the same, from an event card (a plentiful year, calm seas, a tournament)
    Gift { to: u8 },      // give one card of your choice to `to` (NOBODY: to any player with less Renown)
    Rob,                  // take a random card from a player of your choice (an event card)
    Damage,               // an earthquake: damage one of your own Paths
    Neutral { hut: bool }, // the two-player variant: build a neutral Hut (or Path) after building yours
    // The city expansion (city.rs)
    DropProgress,          // over 4 progress cards: drop one (DropCard)
    Pillage,               // the raiders won: one of your Lodges becomes a Hut (Pillage)
    Displaced { at: u16, level: u8, active: bool }, // your knight was pushed off this corner: move it (KnightMove)
    DrawCard,              // tied best defenders: draw a progress card from a deck of your choice (Choose)
    Name { card: u8 },     // name a card kind for this progress card (Choose)
    Target { card: u8 },   // the player this progress card is played against (Choose)
    TakeCards { from: u8, left: u8 }, // Guild Dues: take cards of your choice from this player (TakeCard)
    Spy { from: u8 },      // Spy: take one of this player's progress cards, or none (TakeProgress / Skip)
    Harbour { asked: u8 }, // Harbour Deal: offer a resource to each player with commodities (Gift / Skip)
    PayGoods { to: u8 },   // Harbour Deal: give this player a commodity of your choice (Gift)
    Smith { left: u8 },    // Smithing: promote up to this many knights for free (Promote / Skip)
    FreeWall,              // Engineer: a wall for free (Wall)
    Swap,                  // Number Swap: swap two number discs (SwapNumbers)
    PlaceMerchant,         // Merchant: place the merchant (Merchant)
    RemovePath,            // Diplomacy: remove an open Path (RemovePath)
    Expel,                 // Intrigue: push off a knight touching your routes (Expel)
    GiveUpKnight { to: u8 }, // Treason: remove one of your knights (RemoveKnight)
    TreasonKnight { level: u8, active: bool }, // Treason: place a knight of yours no stronger (Recruit / Skip)
    // Merchant trains (merchant.rs)
    Bid,                   // bid Fish and Grain for where the wagon goes (Bid)
    PlaceWagon,            // place the wagon (Wagon)
    // Raider attack (raiders.rs)
    Capture,               // take a raider prisoner from a coastal hex (Capture)
    PlaceGuard { keep: bool }, // place a knight on an edge of the Keep, or any free edge (PlaceGuard)
    MoveRaider { from: u16 }, // move a raider between coastal hexes, not from `from` (MoveRaider)
    MoveGuards,            // the end of your turn: move your knights (GuardMove / GuardRun / Skip)
    // Traders and raiders (traders.rs)
    Drive,                 // the end of your turn: move your wagon (WagonStep / DriveOff / Boost / Skip)
    MoveBarbarian { from: u16, steal: bool }, // move a raider (from NONE: any) to a free edge (BarbarianTo)
    // The exploration expansion (explore.rs)
    Sail,                  // the movement phase: move and use your ships (Sail ... / Skip)
    PiratePlace,           // a 7: put your pirate ship on a sea hex (PiratePlace)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pending {
    pub seat: u8,
    pub decision: Decision,
}

impl Decision {
    // The name the page and the protocol use for the phase while this decision is being made
    pub fn name(self) -> &'static str {
        match self {
            Decision::Discard { .. } => "discard",
            Decision::Gold { .. } => "gold",
            Decision::Pick { .. } => "pick",
            Decision::Gift { .. } => "gift",
            Decision::Rob => "rob",
            Decision::Damage => "damage",
            Decision::Neutral { .. } => "neutral",
            Decision::DropProgress => "dropProgress",
            Decision::Pillage => "pillage",
            Decision::Displaced { .. } => "displaced",
            Decision::DrawCard => "drawCard",
            Decision::Name { .. } => "name",
            Decision::Target { .. } => "target",
            Decision::TakeCards { .. } => "takeCards",
            Decision::Spy { .. } => "spy",
            Decision::Harbour { .. } => "harbour",
            Decision::PayGoods { .. } => "payGoods",
            Decision::Smith { .. } => "smith",
            Decision::FreeWall => "freeWall",
            Decision::Swap => "swap",
            Decision::PlaceMerchant => "placeMerchant",
            Decision::RemovePath => "removePath",
            Decision::Expel => "expel",
            Decision::GiveUpKnight { .. } => "giveUpKnight",
            Decision::TreasonKnight { .. } => "treasonKnight",
            Decision::Bid => "bid",
            Decision::PlaceWagon => "placeWagon",
            Decision::Capture => "capture",
            Decision::PlaceGuard { .. } => "placeGuard",
            Decision::MoveRaider { .. } => "moveRaider",
            Decision::MoveGuards => "moveGuards",
            Decision::Drive => "drive",
            Decision::MoveBarbarian { .. } => "moveBarbarian",
            Decision::Sail => "sail",
            Decision::PiratePlace => "piratePlace",
        }
    }
}

impl Game {
    // Queue decisions and go to Phase::Decide; with nothing queued, straight to `after`
    pub(super) fn decide_then(&mut self, decisions: impl IntoIterator<Item = Pending>, after: Phase) {
        self.pending.extend(decisions);
        self.after = after;
        self.drop_impossible();
        self.phase = if self.pending.is_empty() { after } else { Phase::Decide };
        // Nothing to decide after all: go on (an event card's or the fleet's number produces now)
        self.settle();
    }

    // Decisions that can't be made any more go: gold or a pick with nothing left in the bank, a gift
    // with an empty hand, a robbery with nobody holding a card
    fn drop_impossible(&mut self) {
        while let Some(d) = self.pending.first().copied() {
            let mut list = Vec::new();
            self.legal_decision(d, &mut list);
            if list.is_empty() {
                self.pending.remove(0);
            } else {
                break;
            }
        }
    }

    // The decision being made now
    pub fn deciding(&self) -> Option<Pending> {
        if self.phase == Phase::Decide { self.pending.first().copied() } else { None }
    }

    // Cards p still has to discard (for the page)
    pub fn discards_left(&self, p: usize) -> u8 {
        self.pending
            .iter()
            .filter(|d| d.seat as usize == p)
            .map(|d| match d.decision {
                Decision::Discard { left } => left,
                _ => 0,
            })
            .sum()
    }

    pub(super) fn legal_decision(&self, d: Pending, out: &mut Vec<Action>) {
        let hand = &self.hands[d.seat as usize];
        match d.decision {
            Decision::Discard { .. } => {
                for r in 0..CARD_KINDS {
                    if hand[r] > 0 {
                        out.push(Action::Discard(r as u8));
                    }
                }
            }
            Decision::Gold { .. } | Decision::Pick { .. } => {
                for r in 0..5 {
                    if self.bank(r) > 0 {
                        out.push(Action::Take(r as u8));
                    }
                }
            }
            Decision::Gift { to } => {
                let me = d.seat as usize;
                let mine = self.public_renown(me);
                for o in 0..self.players {
                    let ok = if to == NOBODY { o != me && self.public_renown(o) < mine } else { o == to as usize };
                    if ok {
                        for r in 0..CARD_KINDS {
                            if hand[r] > 0 {
                                out.push(Action::Gift(o as u8, r as u8));
                            }
                        }
                    }
                }
            }
            Decision::Rob => {
                for o in 0..self.players {
                    if o != d.seat as usize && self.cards(o) > 0 {
                        out.push(Action::Rob(o as u8));
                    }
                }
            }
            Decision::Neutral { hut } => self.legal_neutral(hut, out),
            Decision::Damage => {
                for e in 0..self.edge_owner.len() {
                    if self.edge_owner[e] == d.seat && self.edge_kind[e] == PATH && !self.damaged[e] {
                        out.push(Action::Damage(e as u16));
                    }
                }
            }
            Decision::Bid => {
                let (fish, grain) = (hand[4].min(MOST_BID), hand[2].min(MOST_BID));
                for f in 0..=fish {
                    for g in 0..=grain {
                        out.push(Action::Bid(f, g));
                    }
                }
            }
            Decision::PlaceWagon => out.extend(self.wagon_spots().into_iter().map(|(e, _, _)| Action::Wagon(e as u16))),
            Decision::Capture | Decision::PlaceGuard { .. } | Decision::MoveRaider { .. } | Decision::MoveGuards => self.legal_raider_decision(d, out),
            Decision::Drive => self.legal_drive(d.seat, out),
            Decision::MoveBarbarian { .. } => self.legal_barbarian(d, out),
            Decision::Sail => self.legal_sail(d.seat, out),
            Decision::PiratePlace => self.legal_pirate_place(out),
            _ => self.legal_city_decision(d, out),
        }
    }

    // Phases that only mark what happens once the decisions are made: an event card's number produces;
    // Merchant trains' vote is counted; the turn ends
    fn settle(&mut self) {
        match self.phase {
            Phase::Rolled { total } => self.rolled(total),
            Phase::VoteDone => self.resolve_vote(),
            Phase::TurnEnd => self.end_phase(),
            // (a Road Crew Path that owed a neutral Path: the free build may have nothing left)
            Phase::FreeBuild { .. } => self.finish_free_build(),
            Phase::ExpelDone => {
                self.expel();
                self.end_phase();
            }
            _ => {}
        }
    }

    // The first decision in the queue has had one more answer: finish it, and go on when all are done
    pub(super) fn decided(&mut self, next: Option<Decision>) {
        match next {
            Some(d) => self.pending[0].decision = d,
            None => {
                self.pending.remove(0);
            }
        }
        self.drop_impossible();
        if self.pending.is_empty() {
            self.phase = self.after;
            self.settle();
        }
    }
}
