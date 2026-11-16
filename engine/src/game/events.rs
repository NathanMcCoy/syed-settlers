// Event cards (the variants box, variant.events): a deck of numbered cards replaces the dice. Each card
// has a number (the deck has exactly the 36 two-dice totals) and an event, resolved before production.
// The New Year card is placed with 5 cards under it; drawing it rebuilds the deck.
//
// The rulebook names the events but not which card carries which (the cards are pictures), so the
// mix is ours: every 7 is "the squall attacks" (a 7 as usual), ten cards are a beautiful day, and the
// other ten events appear twice each.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Event {
    BeautifulDay,    // nothing happens
    SquallAttacks,   // a 7: discards and the squall, as usual
    SquallFlees,     // the squall goes to the Dunes, no steal
    Conflict,        // the holder of Most Pilots (or whoever has played the most) robs anyone
    TradeAdvantage,  // the holder of Longest Path robs anyone
    CalmSeas,        // the most buildings on markets: a resource of their choice
    Tournament,      // the most Pilots played: a resource of their choice
    PlentifulYear,   // everyone takes a resource of their choice
    GoodNeighbours,  // everyone gives a card of their choice to the player on their left
    HelpfulNeighbour, // the most Renown gives a card to a player with less
    Epidemic,        // Lodges produce 1, not 2, this time
    Earthquake,      // everyone damages one of their own Paths
    NewYear,         // the deck is rebuilt
}

pub const EVENTS: [Event; 13] = [
    Event::BeautifulDay, Event::SquallAttacks, Event::SquallFlees, Event::Conflict, Event::TradeAdvantage, Event::CalmSeas,
    Event::Tournament, Event::PlentifulYear, Event::GoodNeighbours, Event::HelpfulNeighbour, Event::Epidemic,
    Event::Earthquake, Event::NewYear,
];

impl Event {
    pub fn name(self) -> &'static str {
        [
            "beautifulDay", "squallAttacks", "squallFlees", "conflict", "tradeAdvantage", "calmSeas", "tournament",
            "plentifulYear", "goodNeighbours", "helpfulNeighbour", "epidemic", "earthquake", "newYear",
        ][self as usize]
    }
}

// The 36 cards (number, event), in a fixed order; NEW_YEAR is card 36
pub const NEW_YEAR: u8 = 36;
pub fn event_card(card: u8) -> (u8, Event) {
    use Event::*;
    if card == NEW_YEAR {
        return (0, NewYear);
    }
    // The two-dice totals: one 2, two 3s, ... six 7s, ... one 12
    const TOTALS: [u8; 36] = [2, 3, 3, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 9, 9, 9, 9, 10, 10, 10, 11, 11, 12];
    // The other 30 cards' events, in the order of their totals (7s skipped)
    const EVENTS: [Event; 30] = [
        BeautifulDay, Conflict, CalmSeas, Tournament, PlentifulYear, GoodNeighbours, HelpfulNeighbour, Epidemic,
        Earthquake, SquallFlees, TradeAdvantage, BeautifulDay, BeautifulDay, Conflict, CalmSeas, BeautifulDay,
        Tournament, PlentifulYear, GoodNeighbours, BeautifulDay, HelpfulNeighbour, Epidemic, Earthquake,
        BeautifulDay, SquallFlees, TradeAdvantage, BeautifulDay, BeautifulDay, BeautifulDay, BeautifulDay,
    ];
    let total = TOTALS[card as usize];
    if total == 7 {
        return (7, SquallAttacks);
    }
    let k = (0..card).filter(|&c| TOTALS[c as usize] != 7).count();
    (total, EVENTS[k])
}

impl Game {
    // A fresh event deck: shuffled, 5 cards under the New Year card, the rest on top (top last)
    pub(super) fn new_event_deck(&mut self) {
        let mut cards: Vec<u8> = (0..36).collect();
        self.rng.shuffle(&mut cards);
        let mut deck: Vec<u8> = cards[..5].to_vec();
        deck.push(NEW_YEAR);
        deck.extend_from_slice(&cards[5..]);
        self.event_deck = deck;
    }

    // Draw the next event card (rebuilding the deck at the New Year) and resolve it; the number then
    // produces (after any decisions the event asks for)
    pub(super) fn draw_event(&mut self) {
        let mut card = self.event_deck.pop().expect("the event deck is never empty");
        if card == NEW_YEAR {
            self.new_event_deck();
            card = self.event_deck.pop().unwrap();
        }
        let (number, event) = event_card(card);
        self.last_event = event as u8;
        self.last_dice = [0, 0];
        self.epidemic = event == Event::Epidemic;
        let me = self.current as usize;
        let order: Vec<usize> = (0..self.players).map(|k| (me + k) % self.players).collect();
        let mut decisions: Vec<Pending> = Vec::new();
        let pick = |p: usize| Pending { seat: p as u8, decision: Decision::Pick { left: 1 } };
        match event {
            Event::BeautifulDay | Event::SquallAttacks | Event::Epidemic | Event::NewYear => {}
            Event::SquallFlees => {
                if let Some(&h) = self.board.land_hexes.iter().find(|&&h| self.board.terrain[h as usize] == Terrain::Dunes) {
                    self.squall = h;
                }
            }
            Event::Conflict => {
                let robber = match self.holder(Title::MostPilots) {
                    NOBODY => self.strictly_most(|g, p| g.pilots[p] as u32),
                    h => Some(h as usize),
                };
                decisions.extend(robber.filter(|&r| self.rob_targets(r) > 0).map(|r| Pending { seat: r as u8, decision: Decision::Rob }));
            }
            Event::TradeAdvantage => {
                let h = self.holder(Title::LongestPath);
                if h != NOBODY && self.rob_targets(h as usize) > 0 {
                    decisions.push(Pending { seat: h, decision: Decision::Rob });
                }
            }
            Event::CalmSeas => {
                let most = (0..self.players).map(|p| self.market_buildings(p)).max().unwrap_or(0);
                if most > 0 {
                    decisions.extend(order.iter().filter(|&&p| self.market_buildings(p) == most).map(|&p| pick(p)));
                }
            }
            Event::Tournament => {
                let most = (0..self.players).map(|p| self.pilots[p]).max().unwrap_or(0);
                if most > 0 {
                    decisions.extend(order.iter().filter(|&&p| self.pilots[p] == most).map(|&p| pick(p)));
                }
            }
            Event::PlentifulYear => decisions.extend(order.iter().map(|&p| pick(p))),
            Event::GoodNeighbours => {
                for &p in &order {
                    if self.cards(p) > 0 {
                        decisions.push(Pending { seat: p as u8, decision: Decision::Gift { to: ((p + 1) % self.players) as u8 } });
                    }
                }
            }
            Event::HelpfulNeighbour => {
                let most = (0..self.players).map(|p| self.public_renown(p)).max().unwrap();
                let fewer = (0..self.players).any(|p| self.public_renown(p) < most);
                if fewer {
                    for &p in &order {
                        if self.public_renown(p) == most && self.cards(p) > 0 {
                            decisions.push(Pending { seat: p as u8, decision: Decision::Gift { to: NOBODY } });
                        }
                    }
                }
            }
            Event::Earthquake => {
                for &p in &order {
                    if (0..self.edge_owner.len()).any(|e| self.edge_owner[e] == p as u8 && self.edge_kind[e] == PATH && !self.damaged[e]) {
                        decisions.push(Pending { seat: p as u8, decision: Decision::Damage });
                    }
                }
            }
        }
        self.decide_then(decisions, Phase::Rolled { total: number });
        if let Phase::Rolled { total } = self.phase {
            self.rolled(total);
        }
    }

    // The one player strictly ahead in `value`, if any (and above 0)
    fn strictly_most(&self, value: impl Fn(&Game, usize) -> u32) -> Option<usize> {
        let best = (0..self.players).map(|p| value(self, p)).max()?;
        let at: Vec<usize> = (0..self.players).filter(|&p| value(self, p) == best).collect();
        if best > 0 && at.len() == 1 { Some(at[0]) } else { None }
    }

    // How many players p could rob (others with a card)
    fn rob_targets(&self, p: usize) -> usize {
        (0..self.players).filter(|&o| o != p && self.cards(o) > 0).count()
    }

    // p's buildings on markets
    fn market_buildings(&self, p: usize) -> u32 {
        (0..self.corner_owner.len()).filter(|&c| self.corner_owner[c] as usize == p && self.board.corner_market[c] != NOBODY).count() as u32
    }

    // A player who robs (an event): one random card from `victim`
    pub(super) fn rob(&mut self, thief: usize, victim: usize) {
        let n = self.cards(victim);
        if n == 0 {
            return;
        }
        let mut pick = self.rng.below(n as usize) as u32;
        for r in 0..5 {
            let have = self.hands[victim][r] as u32;
            if pick < have {
                self.hands[victim][r] -= 1;
                self.hands[thief][r] += 1;
                return;
            }
            pick -= have;
        }
    }

    // Earthquake: may p build Paths (none of theirs damaged)? May a Hut go on corner c (no damaged
    // Path next to it)?
    pub(super) fn paths_blocked(&self, p: u8) -> bool {
        self.settings().variant_events && (0..self.edge_owner.len()).any(|e| self.damaged[e] && self.edge_owner[e] == p)
    }
    pub fn by_damage(&self, c: usize) -> bool {
        self.settings().variant_events && listed(&self.board.graph.corner_edges[c]).any(|e| self.damaged[e])
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
