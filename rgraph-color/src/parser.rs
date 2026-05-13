//! CSS color string parser.
//!
//! Mirrors d3-color's regex set without pulling in a regex engine. The
//! grammar is restricted enough that a hand-written parser is faster, has
//! zero allocations on the hot path, and avoids dragging in a sizable
//! dependency.
//!
//! Recognised forms (with the same whitespace tolerance as d3):
//!
//! * `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
//! * `rgb(r,g,b)`, `rgb(r%,g%,b%)`, `rgba(r,g,b,a)`, `rgba(r%,g%,b%,a)`
//! * `hsl(h,s%,l%)`, `hsla(h,s%,l%,a)`
//! * Named colors and `transparent`
//!
//! Inside the comma-separated lists, the numeric forms are:
//!
//! * Integer (`reI`): `[+-]?\d+`
//! * Number  (`reN`): `[+-]?(\d*\.)?\d+([eE][+-]?\d+)?`
//! * Percent (`reP`): same as `reN` followed by `%`
//!
//! Each numeric token is surrounded by `\s*` (any amount of whitespace).
//!
//! Returns `None` if the input doesn't match any form — exactly like d3's
//! `color(format)` returns `null`.

use crate::color::Color;
use crate::hsl::Hsl;
use crate::named;
use crate::rgb::Rgb;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parses a CSS color string. Returns `None` for unrecognised input.
///
/// This is the equivalent of d3's `color(format)`. Trims surrounding ASCII
/// whitespace and is case-insensitive (because the input is lowered before
/// matching).
pub fn parse(format: &str) -> Option<Color> {
    let trimmed = format.trim_matches(is_ascii_ws);
    if trimmed.is_empty() {
        return None;
    }

    // Lowercase only when needed: hex / function names / named colors all
    // require it. Allocates at most one short String.
    let lower = trimmed.to_ascii_lowercase();

    if let Some(rest) = lower.strip_prefix('#') {
        return parse_hex(rest);
    }
    if let Some(rest) = strip_func(&lower, "rgba") {
        return parse_rgba(rest);
    }
    if let Some(rest) = strip_func(&lower, "rgb") {
        return parse_rgb(rest);
    }
    if let Some(rest) = strip_func(&lower, "hsla") {
        return parse_hsla(rest);
    }
    if let Some(rest) = strip_func(&lower, "hsl") {
        return parse_hsl(rest);
    }
    if lower == "transparent" {
        return Some(Color::Rgb(Rgb::new(f64::NAN, f64::NAN, f64::NAN, 0.0)));
    }
    if let Some(n) = named::lookup(&lower) {
        return Some(Color::Rgb(rgbn(n)));
    }
    None
}

// ---------------------------------------------------------------------------
// Hex
// ---------------------------------------------------------------------------

fn parse_hex(rest: &str) -> Option<Color> {
    if !rest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(rest, 16).ok()?;
    let len = rest.len();
    Some(Color::Rgb(match len {
        3 => Rgb::new(
            (((n >> 8) & 0xf) | ((n >> 4) & 0xf0)) as f64,
            (((n >> 4) & 0xf) | (n & 0xf0)) as f64,
            (((n & 0xf) << 4) | (n & 0xf)) as f64,
            1.0,
        ),
        6 => rgbn(n),
        4 => {
            let r = (((n >> 12) & 0xf) | ((n >> 8) & 0xf0)) as f64;
            let g = (((n >> 8) & 0xf) | ((n >> 4) & 0xf0)) as f64;
            let b = (((n >> 4) & 0xf) | (n & 0xf0)) as f64;
            let a = (((n & 0xf) << 4) | (n & 0xf)) as f64 / 255.0;
            rgba(r, g, b, a)
        }
        8 => {
            let r = ((n >> 24) & 0xff) as f64;
            let g = ((n >> 16) & 0xff) as f64;
            let b = ((n >> 8) & 0xff) as f64;
            let a = (n & 0xff) as f64 / 255.0;
            rgba(r, g, b, a)
        }
        _ => return None,
    }))
}

fn rgbn(n: u32) -> Rgb {
    Rgb::new(
        ((n >> 16) & 0xff) as f64,
        ((n >> 8) & 0xff) as f64,
        (n & 0xff) as f64,
        1.0,
    )
}

fn rgba(r: f64, g: f64, b: f64, a: f64) -> Rgb {
    if a <= 0.0 {
        Rgb::new(f64::NAN, f64::NAN, f64::NAN, a)
    } else {
        Rgb::new(r, g, b, a)
    }
}

// ---------------------------------------------------------------------------
// Functional forms — rgb / rgba / hsl / hsla
// ---------------------------------------------------------------------------

fn parse_rgb(args: &str) -> Option<Color> {
    let mut p = Parser::new(args);
    p.expect('(')?;
    // Either three integers or three percents; no mixing.
    let saved = p.pos;
    if let Some((r, g, b)) = parse_three_ints(&mut p)
        && p.expect(')').is_some()
        && p.eof()
    {
        return Some(Color::Rgb(Rgb::new(r, g, b, 1.0)));
    }
    p.pos = saved;
    let (r, g, b) = parse_three_percents(&mut p)?;
    p.expect(')')?;
    if !p.eof() { return None; }
    Some(Color::Rgb(Rgb::new(r * 255.0 / 100.0, g * 255.0 / 100.0, b * 255.0 / 100.0, 1.0)))
}

fn parse_rgba(args: &str) -> Option<Color> {
    let mut p = Parser::new(args);
    p.expect('(')?;
    let saved = p.pos;
    if let Some((r, g, b)) = parse_three_ints(&mut p) {
        p.expect(',')?;
        let a = p.read_number()?;
        p.expect(')')?;
        if !p.eof() { return None; }
        return Some(Color::Rgb(rgba(r, g, b, a)));
    }
    p.pos = saved;
    let (r, g, b) = parse_three_percents(&mut p)?;
    p.expect(',')?;
    let a = p.read_number()?;
    p.expect(')')?;
    if !p.eof() { return None; }
    Some(Color::Rgb(rgba(r * 255.0 / 100.0, g * 255.0 / 100.0, b * 255.0 / 100.0, a)))
}

fn parse_hsl(args: &str) -> Option<Color> {
    let mut p = Parser::new(args);
    p.expect('(')?;
    let h = p.read_number()?;
    p.expect(',')?;
    let s = p.read_percent()?;
    p.expect(',')?;
    let l = p.read_percent()?;
    p.expect(')')?;
    if !p.eof() { return None; }
    Some(Color::Hsl(hsla(h, s / 100.0, l / 100.0, 1.0)))
}

fn parse_hsla(args: &str) -> Option<Color> {
    let mut p = Parser::new(args);
    p.expect('(')?;
    let h = p.read_number()?;
    p.expect(',')?;
    let s = p.read_percent()?;
    p.expect(',')?;
    let l = p.read_percent()?;
    p.expect(',')?;
    let a = p.read_number()?;
    p.expect(')')?;
    if !p.eof() { return None; }
    Some(Color::Hsl(hsla(h, s / 100.0, l / 100.0, a)))
}

fn hsla(h: f64, s: f64, l: f64, a: f64) -> Hsl {
    let (mut h, mut s) = (h, s);
    if a <= 0.0 {
        return Hsl::new(f64::NAN, f64::NAN, f64::NAN, a);
    } else if l <= 0.0 || l >= 1.0 {
        h = f64::NAN;
        s = f64::NAN;
    } else if s <= 0.0 {
        h = f64::NAN;
    }
    Hsl::new(h, s, l, a)
}

fn parse_three_ints(p: &mut Parser) -> Option<(f64, f64, f64)> {
    let r = p.read_integer()?;
    p.expect(',')?;
    let g = p.read_integer()?;
    p.expect(',')?;
    let b = p.read_integer()?;
    Some((r, g, b))
}

fn parse_three_percents(p: &mut Parser) -> Option<(f64, f64, f64)> {
    let r = p.read_percent()?;
    p.expect(',')?;
    let g = p.read_percent()?;
    p.expect(',')?;
    let b = p.read_percent()?;
    Some((r, g, b))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_ascii_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

/// Returns the rest of `lower` after `name(` if `lower` starts with the
/// function name with no whitespace before the open paren, mirroring d3's
/// regex `^name\(`. The trailing `)` is left to the inner parser to consume.
fn strip_func<'a>(lower: &'a str, name: &str) -> Option<&'a str> {
    if !lower.starts_with(name) {
        return None;
    }
    let rest = &lower[name.len()..];
    // d3's regex *embeds* the parens, so no whitespace is allowed between
    // the name and the open paren. Push back the '(' for the inner parser.
    if rest.starts_with('(') {
        Some(rest)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Number / percent / integer lexer
// ---------------------------------------------------------------------------

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser { src: s.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0c {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn expect(&mut self, c: char) -> Option<()> {
        self.skip_ws();
        if self.peek() == Some(c as u8) {
            self.bump();
            self.skip_ws();
            Some(())
        } else {
            None
        }
    }

    /// Reads an integer (`reI`): optional sign + one-or-more digits.
    fn read_integer(&mut self) -> Option<f64> {
        self.skip_ws();
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.bump();
        }
        let digits_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        if self.pos == digits_start {
            return None;
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).ok()?;
        let v = s.parse::<f64>().ok()?;
        self.skip_ws();
        Some(v)
    }

    /// Reads a number (`reN`): `[+-]?(\d*\.)?\d+([eE][+-]?\d+)?`.
    fn read_number(&mut self) -> Option<f64> {
        self.skip_ws();
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.bump();
        }

        // Optional `\d*\.` followed by `\d+` for the mantissa.
        // Two valid mantissa shapes after sign:
        //  (a) `\d+` (integer part only, no dot)
        //  (b) `\d*\.\d+` (with leading dot allowed, i.e. ".5")
        let int_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        let int_end = self.pos;

        let saw_dot = self.peek() == Some(b'.');
        if saw_dot {
            self.bump();
            let frac_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
            if self.pos == frac_start {
                // `\d*\.` with no fractional digits is invalid (matches d3 regex).
                return None;
            }
        } else if int_end == int_start {
            // Neither integer nor fractional digits — not a number.
            return None;
        }

        // Optional exponent.
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            let exp_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
            if self.pos == exp_start {
                return None;
            }
        }

        let s = std::str::from_utf8(&self.src[start..self.pos]).ok()?;
        let v = s.parse::<f64>().ok()?;
        self.skip_ws();
        Some(v)
    }

    /// Reads a number followed by `%`, with no whitespace allowed between
    /// the digits and the `%` sign (d3's regex captures the digits and the
    /// `%` is anchored immediately after).
    fn read_percent(&mut self) -> Option<f64> {
        // Mirror read_number but stop before its trailing skip_ws so the `%`
        // can be checked at the very next byte.
        self.skip_ws();
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.bump();
        }
        let int_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        let int_end = self.pos;
        let saw_dot = self.peek() == Some(b'.');
        if saw_dot {
            self.bump();
            let frac_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
            if self.pos == frac_start {
                return None;
            }
        } else if int_end == int_start {
            return None;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            let exp_start = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
            if self.pos == exp_start {
                return None;
            }
        }
        // % must immediately follow the numeric token — no whitespace.
        if self.peek() != Some(b'%') {
            return None;
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).ok()?;
        let v = s.parse::<f64>().ok()?;
        self.bump(); // consume '%'
        self.skip_ws();
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_rgb(c: &Color, r: f64, g: f64, b: f64, a: f64) -> bool {
        if let Color::Rgb(rgb) = c {
            (rgb.r - r).abs() < 1e-6 && (rgb.g - g).abs() < 1e-6
                && (rgb.b - b).abs() < 1e-6 && (rgb.opacity - a).abs() < 1e-6
        } else { false }
    }

    #[test]
    fn parses_named() {
        assert!(approx_rgb(&parse("yellow").unwrap(), 255.0, 255.0, 0.0, 1.0));
        assert!(approx_rgb(&parse("rebeccapurple").unwrap(), 102.0, 51.0, 153.0, 1.0));
    }

    #[test]
    fn parses_hex() {
        assert!(approx_rgb(&parse("#abcdef").unwrap(), 171.0, 205.0, 239.0, 1.0));
        assert!(approx_rgb(&parse("#abc").unwrap(), 170.0, 187.0, 204.0, 1.0));
    }

    #[test]
    fn rejects_bad_decimals() {
        assert!(parse("rgb(120.,30,50)").is_none());
        assert!(parse("rgb(120.5,30,50)").is_none()); // integer expected
    }

    #[test]
    fn rejects_invalid() {
        assert!(parse("rgb (120,30,50)").is_none());
        assert!(parse("invalid").is_none());
        assert!(parse("#abcdef3").is_none());
        assert!(parse("#ab").is_none());
    }
}
