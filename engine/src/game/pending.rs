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
        // Nothing to decide after all: an event card's or the fleet's number produces now
        if let Phase::Rolled { total } = self.phase {
            self.rolled(total);
        }
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
            // An event card's number produces once its decisions are made
            if let Phase::Rolled { total } = self.phase {
                self.rolled(total);
            }
        }
    }
}
