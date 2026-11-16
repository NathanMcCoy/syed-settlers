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
    players_min: u8 = 2, "players.min", 2, 4, "Fewest seats in a game";
    players_max: u8 = 4, "players.max", 2, 4, "Most seats in a game";
    win: u8 = 10, "win", 3, 20, "Renown needed to win";
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
}

impl Settings {
    pub fn parse(text: &str) -> Result<Settings, String> {
        let mut s = Settings::default();
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
        // The hexagon's tiles are set exactly: they must fill its 19 hexes
        if s.island_hexagon {
            let total: u32 = s.tile_counts().iter().map(|&n| n as u32).sum::<u32>() + s.island_dunes as u32;
            if total != 19 {
                return Err(format!(
                    "the hexagon island has 19 hexes: island.forest, .quarry, .fields, .mine, .paddy and .dunes add up to {total}"
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
    fn the_table_lists_every_setting() {
        let t = Settings::table_json();
        assert!(t.starts_with("[{\"name\":\"players.min\""));
        assert!(t.contains("{\"name\":\"squall.range\",\"kind\":\"whole\",\"default\":0,\"min\":0,\"max\":8,"));
        assert_eq!(t.matches("\"name\"").count(), 34);
        assert!(Settings::parse("island.dunes=2;island.forest=3").is_ok(), "a Forest traded for a second Dunes");
    }
}
