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
    }

    // Decisions that can't be made any more go: gold with nothing left in the bank
    fn drop_impossible(&mut self) {
        while let Some(d) = self.pending.first() {
            match d.decision {
                Decision::Gold { .. } if (0..5).all(|r| self.bank(r) == 0) => {
                    self.pending.remove(0);
                }
                _ => break,
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
                for r in 0..5 {
                    if hand[r] > 0 {
                        out.push(Action::Discard(r as u8));
                    }
                }
            }
            Decision::Gold { .. } => {
                for r in 0..5 {
                    if self.bank(r) > 0 {
                        out.push(Action::Take(r as u8));
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
        }
    }
}
