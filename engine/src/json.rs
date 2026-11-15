// Just enough JSON writing for the engine's answers to the page and server (no outside packages).

// A JSON string, with quotes and control characters escaped
pub fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// A JSON number (null if it isn't finite)
pub fn number(x: f64) -> String {
    if x.is_finite() { format!("{x}") } else { "null".into() }
}

pub fn optional(x: Option<f64>) -> String {
    x.map_or("null".into(), number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped() {
        assert_eq!(string("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(3.0), "3");
        assert_eq!(number(f64::NAN), "null");
    }
}
