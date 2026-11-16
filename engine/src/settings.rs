// Every balance knob in one place (RULES.md marks them with ⚙). Each one is listed once below with
// its name, default and allowed range; the page builds its controls from Settings::table_json(), so
// nothing else repeats them. Settings::parse() refuses values out of range and misspelt names, so a
// typo can never quietly make a different game.
//
// As text: one `name=value` per line (or separated by `;`), e.g. "squall.range=3;win=8". Names not
// given keep their defaults.
use crate::json;

pub trait Value: Sized + Copy {
    const KIND: &'static str;
    fn parse(text: &str, name: &str, min: f64, max: f64) -> Result<Self, String>;
    fn json(self) -> String;
}

fn number(text: &str, name: &str, min: f64, max: f64) -> Result<f64, String> {
    let v: f64 = text.trim().parse().map_err(|_| format!("{name} must be a number"))?;
    if !v.is_finite() || v < min || v > max {
        return Err(format!("{name} must be between {min} and {max}"));
    }
    Ok(v)
}

impl Value for u8 {
    const KIND: &'static str = "whole";
    fn parse(text: &str, name: &str, min: f64, max: f64) -> Result<u8, String> {
        let v = number(text, name, min, max)?;
        if v.fract() != 0.0 {
            return Err(format!("{name} must be a whole number"));
        }
        Ok(v as u8)
    }
    fn json(self) -> String { self.to_string() }
}

impl Value for f64 {
    const KIND: &'static str = "number";
    fn parse(text: &str, name: &str, min: f64, max: f64) -> Result<f64, String> { number(text, name, min, max) }
    fn json(self) -> String { json::number(self) }
}

impl Value for bool {
    const KIND: &'static str = "switch";
    fn parse(text: &str, name: &str, _: f64, _: f64) -> Result<bool, String> {
        match text.trim() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(format!("{name} must be true or false")),
        }
    }
    fn json(self) -> String { self.to_string() }
}

macro_rules! settings {
    ($($field:ident: $ty:ty = $default:expr, $name:literal, $min:expr, $max:expr, $about:literal;)*) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct Settings { $(pub $field: $ty,)* }

        impl Default for Settings {
            fn default() -> Self { Settings { $($field: $default,)* } }
        }

        impl Settings {
            fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
                match name {
                    $($name => self.$field = <$ty as Value>::parse(value, $name, $min as f64, $max as f64)?,)*
                    _ => return Err(format!("unknown setting {name}")),
                }
                Ok(())
            }

            // Every setting as JSON: [{name, kind ("whole", "number" or "switch"), default, min, max, about}]
            pub fn table_json() -> String {
                let d = Settings::default();
                let rows: Vec<String> = vec![$(format!(
                    "{{\"name\":{},\"kind\":\"{}\",\"default\":{},\"min\":{},\"max\":{},\"about\":{}}}",
                    json::string($name), <$ty as Value>::KIND, Value::json(d.$field),
                    json::number($min as f64), json::number($max as f64), json::string($about),
                )),*];
                format!("[{}]", rows.join(","))
            }

            // Only the settings that differ from the defaults, as text parse() reads back
            pub fn to_text(&self) -> String {
                let d = Settings::default();
                let mut out = Vec::new();
                $(if self.$field != d.$field { out.push(format!("{}={}", $name, Value::json(self.$field))); })*
                out.join(";")
            }
        }
    };
}

settings! {
    players_min: u8 = 2, "players.min", 2, 6, "Fewest seats in a game";
    players_max: u8 = 6, "players.max", 2, 6, "Most seats in a game";
    turns_paired: bool = true, "turns.paired", 0, 1, "With 5 or 6 players, the player 3 seats after the one whose turn it is takes an extra build phase (no trading with players)";
    win: u8 = 10, "win", 3, 20, "Renown needed to win";
    island_large: bool = false, "island.large", 0, 1, "The large island for 5-6 players: 30 hexes in rows of 3-4-5-6-5-4-3 (on by default with 5 or 6 players)";
    island_hexagon: bool = true, "island.hexagon", 0, 1, "Main island is a hexagon 5 hexes across, 19 hexes (off: a random shape of island.size hexes)";
    island_size: u8 = 22, "island.size", 16, 40, "Land hexes on a random-shaped main island";
    island_spiral: bool = true, "island.spiral", 0, 1, "Numbers laid in the classic spiral order on the hexagon with 1 Dunes (off: shuffled, redrawn until fair)";
    island_dunes: u8 = 1, "island.dunes", 0, 3, "Dunes hexes (produce nothing) on the main island";
    island_forest: u8 = 4, "island.forest", 0, 8, "Forest hexes (Timber) on the hexagon island";
    island_quarry: u8 = 3, "island.quarry", 0, 8, "Quarry hexes (Stone) on the hexagon island";
    island_fields: u8 = 4, "island.fields", 0, 8, "Fields hexes (Grain) on the hexagon island";
    island_mine: u8 = 3, "island.mine", 0, 8, "Mine hexes (Copper) on the hexagon island";
    island_paddy: u8 = 4, "island.paddy", 0, 8, "Paddy hexes (Fish) on the hexagon island";
    markets_any: u8 = 4, "markets.any", 0, 6, "General markets (3:1)";
    markets_each: u8 = 1, "markets.each", 0, 2, "2:1 markets for each resource";
    markets_extra_fish: u8 = 0, "markets.extraFish", 0, 2, "Extra 2:1 Fish markets (1 on the large island)";
    squall_range: u8 = 0, "squall.range", 0, 8, "Most steps the squall moves after a 7 (0: to any land hex)";
    discard_over: u8 = 7, "discard.over", 4, 20, "After a 7, holding more than this loses half";
    bank_cards: u8 = 19, "bank.cards", 0, 60, "Cards of each resource in the game (0: the bank never runs out)";
    trade_players: bool = true, "trade.players", 0, 1, "Trading between players (off: only with the bank and markets)";
    trade_offers: u8 = 0, "trade.offers", 0, 10, "Most offers to other players each turn (0: no limit)";
    projects_face_up: bool = false, "projects.faceUp", 0, 1, "Projects lie face up (off, the default: a hidden draw)";
    projects_row: u8 = 3, "projects.row", 1, 6, "Face-up projects showing";
    deck_pilot: u8 = 14, "projects.deck.pilot", 0, 30, "Pilot projects in the deck";
    deck_lighthouse: u8 = 5, "projects.deck.lighthouse", 0, 10, "Lighthouse projects in the deck";
    deck_road_crew: u8 = 2, "projects.deck.roadCrew", 0, 10, "Road Crew projects in the deck";
    deck_harvest: u8 = 2, "projects.deck.harvest", 0, 10, "Good Harvest projects in the deck";
    deck_market_day: u8 = 2, "projects.deck.marketDay", 0, 10, "Market Day projects in the deck";
    longest_min: u8 = 5, "titles.longestPath.min", 2, 15, "Shortest line of Paths that can hold Longest Path";
    round_limit: u8 = 0, "rounds", 0, 100, "Rounds before the most Renown wins (0: no limit)";
    longest_renown: u8 = 2, "titles.longestPath.renown", 0, 4, "Renown for Longest Path";
    most_pilots_min: u8 = 3, "titles.mostPilots.min", 1, 15, "Fewest Pilots played that can hold Most Pilots";
    most_pilots_renown: u8 = 2, "titles.mostPilots.renown", 0, 4, "Renown for Most Pilots";
    pieces_path: u8 = 15, "pieces.path", 1, 40, "Paths each player has";
    pieces_hut: u8 = 5, "pieces.hut", 2, 10, "Huts each player has";
    pieces_lodge: u8 = 4, "pieces.lodge", 0, 10, "Lodges each player has";
    pieces_ship: u8 = 15, "pieces.ship", 0, 40, "Ships each player has (the sea expansion)";
    variant_friendly: bool = false, "variant.friendly", 0, 1, "Friendly squall: it leaves alone players with only 2 Renown (the variants box)";
    variant_events: bool = false, "variant.events", 0, 1, "Event cards instead of dice: each card has a number and an event (the variants box)";
    variant_two: bool = false, "variant.two", 0, 1, "The two-player variant: two neutral colours, two rolls a turn, trade tokens (2 players only)";
    variant_ports: bool = false, "variant.ports", 0, 1, "Strongest Ports: 2 Renown for the most Renown in buildings on markets (at least 3); 1 more Renown to win";
    sea: bool = false, "sea", 0, 1, "The sea expansion: ships, the pirate, gold fields and small islands to sail to (sea.scenario picks the map)";
    sea_scenario: u8 = 1, "sea.scenario", 1, 9, "Sea scenario: 1 New Shores, 2 Four Islands, 3 Fog Islands, 4 Through the Desert, 5 The Forgotten Tribe, 6 Cloth, 7 The Pirate Islands, 8 Wonders, 9 New World";
    explore: u8 = 0, "explore", 0, 5, "The exploration expansion: 1 Land Ho!, 2 Pirate lairs, 3 Fish, 4 Spices, 5 the full game (0: none)";
    box_scenario: u8 = 0, "box.scenario", 0, 5, "A variants box scenario: 1 Fishing, 2 Rivers, 3 Merchant trains, 4 Raider attack, 5 Traders and raiders (0: none)";
    city: bool = false, "city", 0, 1, "The city expansion: commodities, city improvements, walls, knights, raiders and progress cards (13 to win)";
    city_raiders: u8 = 7, "city.raiders", 2, 12, "Steps the raiders take before they land (the city expansion)";
    bank_goods: u8 = 12, "bank.goods", 0, 40, "Cards of each commodity in the game (the city expansion; 0: never runs out)";
    sea_island_bonus: u8 = 2, "sea.islandBonus", 0, 4, "Renown tokens for your first Hut on each small island (the sea expansion)";
}

impl Settings {
    // The defaults for a game of this many players: with 5 or 6, the large island and everything
    // that grows with it (the 5-6 player rules): 30 hexes (6 Forest, 5 Quarry, 6 Fields, 5 Mine,
    // 6 Paddy, 2 Dunes), 11 markets (5 general, one 2:1 each, a second 2:1 Fish), 24 of each
    // resource, and 9 more projects (6 Pilot, 1 Road Crew, 1 Good Harvest, 1 Market Day)
    pub fn for_players(players: usize) -> Settings {
        let mut s = Settings::default();
        if players >= 5 {
            s.island_large = true;
            (s.island_forest, s.island_quarry, s.island_fields, s.island_mine, s.island_paddy, s.island_dunes) = (6, 5, 6, 5, 6, 2);
            (s.markets_any, s.markets_extra_fish) = (5, 1);
            s.bank_cards = 24;
            s.bank_goods = 18;
            (s.deck_pilot, s.deck_lighthouse, s.deck_road_crew, s.deck_harvest, s.deck_market_day) = (20, 5, 3, 3, 3);
        }
        s
    }

    pub fn parse(text: &str) -> Result<Settings, String> {
        Settings::parse_onto(Settings::default(), text)
    }

    // Settings for a game of `players` seats: that many players' defaults, then the text's changes
    pub fn parse_for(text: &str, players: usize) -> Result<Settings, String> {
        let s = Settings::parse_onto(Settings::for_players(players), text)?;
        if s.variant_two && players != 2 {
            return Err("the two-player variant (variant.two) is for 2 players".into());
        }
        if s.explore != 0 {
            if s.sea || s.city || s.box_scenario != 0 || s.variant_events || s.variant_two {
                return Err("the exploration expansion is played on its own".into());
            }
            if players > 4 {
                return Err("the exploration expansion is for 2 to 4 players (its 5-6 player map isn't in yet)".into());
            }
            // Its own game: the win by scenario, no projects, no markets
            let mut base = Settings::for_players(players);
            base.explore = s.explore;
            base.win = [0, 8, 12, 15, 15, 17][s.explore as usize];
            (base.deck_pilot, base.deck_lighthouse, base.deck_road_crew, base.deck_harvest, base.deck_market_day) = (0, 0, 0, 0, 0);
            (base.markets_any, base.markets_each, base.markets_extra_fish) = (0, 0, 0);
            base.island_large = false;
            return Settings::parse_onto(base, text);
        }
        if s.box_scenario != 0 {
            if !matches!(s.box_scenario, 1..=5) {
                return Err(format!("variants box scenario {} isn't ready yet", s.box_scenario));
            }
            if s.sea || s.city || s.variant_events || s.variant_two {
                return Err("a variants box scenario is played on its own (no sea, city, event cards or two-player variant)".into());
            }
            if matches!(s.box_scenario, 1 | 3) && s.island_dunes == 0 {
                return Err("this scenario needs a Dunes hex (it becomes the Lake or the watering hole)".into());
            }
            if s.box_scenario == 5 {
                // Traders and raiders: three works for a Paddy, a Fields and the Dunes; 13 to win; its
                // own projects (Knight 16 = Pilots, Path Builders 3, Swift Journey 3 = the Good Harvest
                // slot, Renown 3 = Lighthouses)
                let mut base = Settings::for_players(players);
                base.box_scenario = 5;
                base.win = 13;
                if !base.island_large {
                    (base.island_paddy, base.island_fields, base.island_dunes) = (3, 3, 3);
                } else {
                    (base.island_fields, base.island_dunes) = (5, 3);
                }
                (base.deck_pilot, base.deck_lighthouse, base.deck_road_crew, base.deck_harvest, base.deck_market_day) = (16, 3, 3, 3, 0);
                return Settings::parse_onto(base, text);
            }
            if s.box_scenario == 4 {
                // Raider attack: the Keep and the Dunes (2 Dunes, one Forest fewer on the hexagon), 12 to
                // win, the order deck instead of projects
                let mut base = Settings::for_players(players);
                base.box_scenario = 4;
                base.win = 12;
                if !base.island_large {
                    (base.island_forest, base.island_dunes) = (3, 2);
                } else {
                    base.island_dunes = 3;
                    base.island_forest = 5;
                }
                (base.deck_pilot, base.deck_lighthouse, base.deck_road_crew, base.deck_harvest, base.deck_market_day) = (0, 0, 0, 0, 0);
                return Settings::parse_onto(base, text);
            }
            if s.box_scenario == 3 {
                // Merchant trains: 12 to win
                let mut base = Settings::for_players(players);
                base.box_scenario = 3;
                base.win = 12;
                return Settings::parse_onto(base, text);
            }
            if s.box_scenario == 2 {
                // Rivers: two Swamps (Dunes) and one Paddy fewer on the hexagon (17 numbered hexes)
                let mut base = Settings::for_players(players);
                base.box_scenario = 2;
                if !base.island_large {
                    (base.island_paddy, base.island_dunes) = (3, 2);
                }
                let s = Settings::parse_onto(base, text)?;
                if s.island_dunes < 2 {
                    return Err("Rivers needs 2 Dunes hexes (the Swamps)".into());
                }
                return Ok(s);
            }
        }
        if s.city {
            if s.sea && s.sea_scenario == 7 {
                return Err("the Pirate Islands' warships come from Pilot projects, which the city expansion replaces: it can't be won with the city expansion".into());
            }
            if s.variant_events || s.variant_two {
                return Err("the city expansion doesn't combine with event cards or the two-player variant".into());
            }

        }
        if !s.sea {
            if !s.city {
                return Ok(s);
            }
            // The city expansion's own defaults: 13 to win, no projects
            let mut base = Settings::for_players(players);
            base.city = true;
            base.apply_city_defaults(0);
            return Settings::parse_onto(base, text);
        }
        // Expansions bring their own defaults (F1 in EXPANSIONS.md): players' defaults, then the
        // expansion's, then the game's own settings on top
        if !(1..=9).contains(&s.sea_scenario) {
            return Err(format!("sea scenario {} isn't ready yet", s.sea_scenario));
        }
        let mut base = Settings::for_players(players);
        base.sea = true;
        base.sea_scenario = s.sea_scenario;
        base.apply_sea_defaults();
        if s.city {
            base.city = true;
            base.apply_city_defaults(base.win);
        }
        Settings::parse_onto(base, text)
    }

    // The city expansion: 13 to win (a sea scenario's target + 2), no project deck
    fn apply_city_defaults(&mut self, sea_win: u8) {
        self.win = if sea_win == 0 { 13 } else { sea_win + 2 };
        (self.deck_pilot, self.deck_lighthouse, self.deck_road_crew, self.deck_harvest, self.deck_market_day) = (0, 0, 0, 0, 0);
    }

    // The sea expansion's defaults, by scenario: the Renown to win, the island bonus, the markets
    fn apply_sea_defaults(&mut self) {
        let (win, bonus) = match self.sea_scenario {
            2 => (13, 2),
            3 => (12, 0),
            4 => (14, 2),
            5 => (13, 0),
            6 => (14, 0),
            7 => (10, 0),
            8 => (10, 1),
            9 => (12, 1),
            _ => (14, 2),
        };
        self.win = win;
        self.sea_island_bonus = bonus;
        if self.sea_scenario == 2 {
            self.markets_any = 3;
        }
        if self.sea_scenario == 9 {
            self.markets_any = 5;
        }
    }

    fn parse_onto(mut s: Settings, text: &str) -> Result<Settings, String> {
        for part in text.split(['\n', ';']) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (name, value) = part.split_once('=').ok_or_else(|| format!("expected name=value, got {part}"))?;
            s.set(name.trim(), value)?;
        }
        if s.players_min > s.players_max {
            return Err("players.min is more than players.max".into());
        }
        // Every resource on at least 3 hexes needs 15 producing hexes (the hexagon has 19)
        if !s.island_hexagon && (s.island_size as i32 - s.island_dunes as i32) < 15 {
            return Err("island.size minus island.dunes must be at least 15, so each resource gets 3 hexes".into());
        }
        // The hexagon's (or the large island's) tiles are set exactly: they must fill it
        if s.island_hexagon || s.island_large {
            let (shape, size) = if s.island_large { ("large", 30) } else { ("hexagon", 19) };
            let total: u32 = s.tile_counts().iter().map(|&n| n as u32).sum::<u32>() + s.island_dunes as u32;
            if total != size {
                return Err(format!(
                    "the {shape} island has {size} hexes: island.forest, .quarry, .fields, .mine, .paddy and .dunes add up to {total}"
                ));
            }
        }
        Ok(s)
    }

    // Forest, Quarry, Fields, Mine, Paddy counts for the hexagon island (the order of RESOURCES)
    pub fn tile_counts(&self) -> [u8; 5] {
        [self.island_forest, self.island_quarry, self.island_fields, self.island_mine, self.island_paddy]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_overrides() {
        assert_eq!(Settings::parse("").unwrap(), Settings::default());
        let s = Settings::parse("squall.range=3; win=8\nbank.cards=25\nprojects.faceUp=true").unwrap();
        assert_eq!((s.squall_range, s.win, s.bank_cards, s.projects_face_up), (3, 8, 25, true));
        assert_eq!(Settings::parse(&s.to_text()).unwrap(), s);
    }

    #[test]
    fn mistakes_are_refused() {
        for (text, why) in [
            ("squall.range=9", "between 0 and 8"),
            ("projects.deck.bargain=2", "unknown setting"),
            ("titles.mostMarkets.min=2", "unknown setting"),
            ("win=9.5", "whole number"),
            ("islets.count=2", "unknown setting"),
            ("pieces.boat=5", "unknown setting"),
            ("win", "name=value"),
            ("projects.faceUp=yes", "true or false"),
            ("players.min=4;players.max=3", "more than"),
            ("island.dunes=4", "between 0 and 3"),
            ("island.dunes=2", "add up to 20"),
            ("island.forest=5", "add up to 20"),
            ("island.hexagon=false;island.size=16;island.dunes=2", "at least 15"),
        ] {
            let err = Settings::parse(text).unwrap_err();
            assert!(err.contains(why), "{text}: {err}");
        }
    }

    #[test]
    fn five_or_six_players_get_the_large_island_and_everything_that_grows_with_it() {
        assert_eq!(Settings::for_players(4), Settings::default());
        let s = Settings::parse_for("", 6).unwrap();
        assert!(s.island_large);
        assert_eq!((s.tile_counts(), s.island_dunes), ([6, 5, 6, 5, 6], 2), "30 hexes");
        assert_eq!((s.markets_any, s.markets_each, s.markets_extra_fish, s.bank_cards), (5, 1, 1, 24));
        assert_eq!(s.deck_pilot + s.deck_lighthouse + s.deck_road_crew + s.deck_harvest + s.deck_market_day, 34);
        // Changes still apply on top, and a round trip through text keeps them
        let t = Settings::parse_for("win=12;bank.cards=30", 5).unwrap();
        assert_eq!((t.win, t.bank_cards, t.island_large), (12, 30, true));
        assert_eq!(Settings::parse_for(&t.to_text(), 5).unwrap(), t);
        assert!(Settings::parse_for("island.forest=7", 6).unwrap_err().contains("large island has 30 hexes"));
    }

    #[test]
    fn the_table_lists_every_setting() {
        let t = Settings::table_json();
        assert!(t.starts_with("[{\"name\":\"players.min\""));
        assert!(t.contains("{\"name\":\"squall.range\",\"kind\":\"whole\",\"default\":0,\"min\":0,\"max\":8,"));
        assert_eq!(t.matches("\"name\"").count(), 50);
        assert!(Settings::parse("island.dunes=2;island.forest=3").is_ok(), "a Forest traded for a second Dunes");
    }
}
