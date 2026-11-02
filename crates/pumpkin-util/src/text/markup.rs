//! Small `&`-code text format used by the server configuration (join/quit messages,
//! private messages...).
//!
//! * `&0-&9`, `&a-&f`: vanilla colors (a color resets the other styles, like in vanilla)
//! * `&k &l &m &n &o`: obfuscated, bold, strikethrough, underlined, italic
//! * `&r`: reset
//! * `&#RRGGBB`: hexadecimal color
//! * `&&`: a literal `&`
//! * `\n` (backslash + n, or a real line feed): line break, see [`render`]
//!
//! Variables are written `{name}`; their value is escaped (`&` becomes `&&`) so that a
//! player name or a chat message never injects formatting, unless the variable is passed
//! as a *raw* one.

use crate::text::TextComponent;
use crate::text::color::{Color, RGBColor};

/// Escapes `&` so that `value` is displayed literally by [`parse`].
#[must_use]
pub fn escape(value: &str) -> String {
    value.replace('&', "&&")
}

/// Replaces literal backslash-n sequences by line feeds.
fn unescape_newlines(template: &str) -> String {
    template.replace("\\n", "\n").replace("\r\n", "\n")
}

/// Expands the `{name}` variables of `template`.
///
/// * `vars`: plain values, escaped (no formatting is interpreted in them);
/// * `raw`: values inserted as is (their `&` codes are interpreted).
///
/// Unknown variables are left untouched. Names are case-insensitive.
#[must_use]
pub fn expand(template: &str, vars: &[(&str, &str)], raw: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('}') {
            let name = &after[..end];
            if let Some((_, v)) = vars.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
                out.push_str(&escape(v));
                rest = &after[end + 1..];
                continue;
            }
            if let Some((_, v)) = raw.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
                out.push_str(v);
                rest = &after[end + 1..];
                continue;
            }
        }
        out.push('{');
        rest = after;
    }
    out.push_str(rest);
    out
}

#[derive(Clone, Copy, Default)]
struct State {
    color: Option<Color>,
    bold: bool,
    italic: bool,
    underlined: bool,
    strikethrough: bool,
    obfuscated: bool,
}

impl State {
    fn apply(self, text: String) -> TextComponent {
        let mut c = TextComponent::text(text);
        if let Some(color) = self.color {
            c = c.color(color);
        }
        if self.bold {
            c = c.bold();
        }
        if self.italic {
            c = c.italic();
        }
        if self.underlined {
            c = c.underlined();
        }
        if self.strikethrough {
            c = c.strikethrough();
        }
        if self.obfuscated {
            c = c.obfuscated();
        }
        c
    }
}

/// Parses one line of `&`-formatted text. Line feeds are kept as text (use [`render`]
/// to split lines).
#[must_use]
pub fn parse(input: &str) -> TextComponent {
    let chars: Vec<char> = input.chars().collect();
    let mut root = TextComponent::text("");
    let mut state = State::default();
    let mut buf = String::new();
    let mut i = 0;

    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                root = root.add_child(state.apply(std::mem::take(&mut buf)));
            }
        };
    }

    while i < chars.len() {
        let c = chars[i];
        if c != '&' {
            buf.push(c);
            i += 1;
            continue;
        }
        let Some(&code) = chars.get(i + 1) else {
            buf.push('&');
            i += 1;
            continue;
        };
        match code.to_ascii_lowercase() {
            '&' => {
                buf.push('&');
                i += 2;
            }
            '#' => {
                let hex: String = chars.iter().skip(i + 2).take(6).collect();
                if hex.chars().count() == 6 && hex.chars().all(|h| h.is_ascii_hexdigit()) {
                    flush!();
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&hex[0..2], 16),
                        u8::from_str_radix(&hex[2..4], 16),
                        u8::from_str_radix(&hex[4..6], 16),
                    ) {
                        state = State {
                            color: Some(Color::Rgb(RGBColor::new(r, g, b))),
                            ..State::default()
                        };
                    }
                    i += 8;
                } else {
                    buf.push('&');
                    i += 1;
                }
            }
            lc @ ('0'..='9' | 'a'..='f') => {
                flush!();
                state = State {
                    color: Color::from_legacy_code(lc),
                    ..State::default()
                };
                i += 2;
            }
            'k' => {
                flush!();
                state.obfuscated = true;
                i += 2;
            }
            'l' => {
                flush!();
                state.bold = true;
                i += 2;
            }
            'm' => {
                flush!();
                state.strikethrough = true;
                i += 2;
            }
            'n' => {
                flush!();
                state.underlined = true;
                i += 2;
            }
            'o' => {
                flush!();
                state.italic = true;
                i += 2;
            }
            'r' => {
                flush!();
                state = State::default();
                i += 2;
            }
            _ => {
                // Unknown code: keep the ampersand literally.
                buf.push('&');
                i += 1;
            }
        }
    }
    flush!();
    root
}

/// Expands the variables of `template` and parses it into one component per line.
///
/// Returns an empty list when `template` is empty or only made of blanks: by convention
/// an empty configuration field means "send nothing".
#[must_use]
pub fn render(template: &str, vars: &[(&str, &str)], raw: &[(&str, &str)]) -> Vec<TextComponent> {
    if template.trim().is_empty() {
        return Vec::new();
    }
    let template = unescape_newlines(template);
    let expanded = expand(&template, vars, raw);
    let mut lines: Vec<&str> = expanded.split('\n').collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines.into_iter().map(parse).collect()
}

/// Removes the formatting codes of `input` (`&` codes, `&#RRGGBB`, `&&` becomes `&`).
#[must_use]
pub fn strip(input: &str) -> String {
    parse(input).get_text()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::color::NamedColor;

    fn kids(c: &TextComponent) -> Vec<(String, Option<Color>, bool)> {
        c.0.extra
            .iter()
            .map(|e| {
                let text = match &*e.content {
                    crate::text::TextContent::Text { text } => text.to_string(),
                    _ => String::new(),
                };
                (text, e.style.color, e.style.bold.unwrap_or(false))
            })
            .collect()
    }

    #[test]
    fn default_join_colors() {
        let c = parse("&8[&a+&8] &7Steve");
        let k = kids(&c);
        assert_eq!(k.len(), 4);
        assert_eq!(k[0], ("[".into(), Some(Color::Named(NamedColor::DarkGray)), false));
        assert_eq!(k[1], ("+".into(), Some(Color::Named(NamedColor::Green)), false));
        assert_eq!(k[2], ("] ".into(), Some(Color::Named(NamedColor::DarkGray)), false));
        assert_eq!(k[3], ("Steve".into(), Some(Color::Named(NamedColor::Gray)), false));
    }

    #[test]
    fn hex_bold_reset_and_literal() {
        let c = parse("&#FF8800&lHi &&co &rok &z &");
        let k = kids(&c);
        assert_eq!(k[0].0, "Hi &co ");
        assert_eq!(k[0].1, Some(Color::Rgb(RGBColor::new(255, 136, 0))));
        assert!(k[0].2);
        assert_eq!(strip("&#FF8800&lHi &&co &rok &z &"), "Hi &co ok &z &");
    }

    #[test]
    fn color_resets_styles() {
        let k = kids(&parse("&lA&cB"));
        assert!(k[0].2);
        assert!(!k[1].2);
    }

    #[test]
    fn bad_hex_is_literal() {
        assert_eq!(strip("&#12345 x"), "&#12345 x");
    }

    #[test]
    fn variables_are_escaped() {
        let out = expand("{player}: {message} {unknown}", &[("player", "A&b")], &[("message", "&aok")]);
        assert_eq!(out, "A&&b: &aok {unknown}");
        let lines = render("&7{player}\\n&fL2", &[("player", "&cX")], &[]);
        assert_eq!(lines.len(), 2);
        assert_eq!(strip(&expand("&7{player}", &[("player", "&cX")], &[])), "&cX");
    }

    #[test]
    fn empty_means_nothing() {
        assert!(render("", &[], &[]).is_empty());
        assert!(render("   ", &[], &[]).is_empty());
        assert_eq!(render("a\nb\n\n", &[], &[]).len(), 2);
    }
}
