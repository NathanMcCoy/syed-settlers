// The turn as steps (F8 in EXPANSIONS.md). A turn is: the owner's phase (before the roll -> the
// production roll -> production or a 7 -> building and trading -> the end of the phase), then, with
// paired turns, the partner's extra phase (building and trading -> the end of the phase), then the
// next owner's turn. Rulesets hook in at these named steps instead of inside one long match:
//   production_roll   where the number comes from (two dice; the variants box's event cards)
//   rolled            what a number does (production, or a 7)
//   end_phase         what happens when a phase ends (expansions: end-of-turn votes, moving
//                     knights or wagons, the movement phase), then who acts next
use super::*;

impl Game {
    // Where the production number comes from: two dice
    pub(super) fn production_roll(&mut self) -> u8 {
        let dice = [self.rng.below(6) as u8 + 1, self.rng.below(6) as u8 + 1];
        self.last_dice = dice;
        dice[0] + dice[1]
    }

    // What a production number does (separate from the dice so tests can choose the total)
    pub(crate) fn rolled(&mut self, total: u8) {
        self.last_roll = total;
        self.short = 0;
        // The two-player variant: the first roll leads to a second (a different number)
        let next = if self.two() && !self.rolled_once { Phase::SecondRoll } else { Phase::Main };
        self.back = next;
        if total == 7 {
            // Everyone over the hand limit loses half, in seat order, then the squall moves
            let over = self.settings().discard_over as u32;
            let discards: Vec<Pending> = (0..self.players)
                .filter(|&p| self.cards(p) > over)
                .map(|p| Pending { seat: p as u8, decision: Decision::Discard { left: (self.cards(p) / 2) as u8 } })
                .collect();
            self.decide_then(discards, Phase::MoveSquall { anywhere: false });
            return;
        }
        // Gold fields' owners then choose their resources (pending.rs), starting with this player
        let gold = self.produce(total);
        self.village_production(total);
        self.epidemic = false;
        let decisions = self.gold_decisions(&gold);
        self.decide_then(decisions, next);
    }

    // The player acting ends their phase (their own turn's, or an extra phase)
    pub(super) fn end_phase(&mut self) {
        self.offer = None;
        self.offers_made = 0;
        self.bought = [0; 5];
        self.project_used = false;
        self.ships_new.clear();
        self.ship_moved = false;
        self.rolled_once = false;
        self.token_used = false;
        self.pilot_cashed = false;
        if self.cloth_ending() {
            return;
        }
        if self.pairs() && !self.paired {
            self.start_extra_phase(self.partner(self.owner));
        } else {
            self.start_turn((self.owner as usize + 1) % self.players);
        }
    }

    // Paired turns: `seat` builds after the owner's turn: no roll, straight to building
    fn start_extra_phase(&mut self, seat: u8) {
        self.paired = true;
        self.current = seat;
        self.phase = Phase::Main;
        self.back = Phase::Main;
    }

    // `seat`'s own turn starts (or the game ends at the round limit)
    fn start_turn(&mut self, seat: usize) {
        self.paired = false;
        self.owner = seat as u8;
        self.current = self.owner;
        self.turn += 1;
        self.phase = Phase::PreRoll;
        let limit = self.settings().round_limit as u32;
        if limit > 0 && self.turn > limit * self.players as u32 {
            self.end_by_rounds();
        }
    }
}
