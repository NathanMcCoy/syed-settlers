// Tokens (F6 in EXPANSIONS.md): per-player counters that aren't resource cards. Each kind says what
// it does: counts as Renown, counts toward the 7's hand limit, can be stolen, can be traded. The base
// game uses none yet; expansions add rows (coins, cloth, fish tokens, prisoners, commodities have
// their own place in the hand) instead of new fields.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum Token {
    Renown, // 1 Renown each (island bonuses, defended raids, captured prizes)
    Trade,  // trade tokens (the two-player variant: forced trades, sending the squall away)
    Cloth,  // the sea scenario Cloth: 2 cloth = 1 Renown
}
pub const TOKENS: [Token; 3] = [Token::Renown, Token::Trade, Token::Cloth];
pub const TOKEN_COUNT: usize = TOKENS.len();

impl Token {
    pub fn name(self) -> &'static str {
        match self {
            Token::Renown => "renown",
            Token::Trade => "trade",
            Token::Cloth => "cloth",
        }
    }
    // Renown each token is worth
    pub fn renown(self) -> u32 {
        match self {
            Token::Renown => 1,
            Token::Trade | Token::Cloth => 0, // (cloth counts in pairs: token_renown)
        }
    }
    // Counts toward the hand limit when a 7 is rolled
    pub fn in_hand_limit(self) -> bool {
        false
    }
    // Can be stolen by the squall or a Pilot
    pub fn stealable(self) -> bool {
        false
    }
    // Can be part of a trade between players
    pub fn tradeable(self) -> bool {
        false
    }
}

impl Game {
    pub fn tokens_of(&self, p: usize, t: Token) -> u16 {
        self.tokens[p][t as usize]
    }

    pub fn give_tokens(&mut self, p: usize, t: Token, n: u16) {
        self.tokens[p][t as usize] += n;
    }

    // Spends n tokens; false (and nothing changes) if p has fewer
    pub fn spend_tokens(&mut self, p: usize, t: Token, n: u16) -> bool {
        if self.tokens[p][t as usize] < n {
            return false;
        }
        self.tokens[p][t as usize] -= n;
        true
    }

    // Renown from tokens for player p
    pub fn token_renown(&self, p: usize) -> u32 {
        // (Cloth counts in pairs: 2 cloth = 1 Renown)
        TOKENS.iter().map(|&t| t.renown() * self.tokens_of(p, t) as u32).sum::<u32>() + self.tokens_of(p, Token::Cloth) as u32 / 2
    }
}
