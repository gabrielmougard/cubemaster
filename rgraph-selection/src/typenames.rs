//! Dot-namespaced event-name parser. Port of the `parseTypenames` helper
//! that appears in d3-selection's `selection/on.js`.
//!
//! d3 lets users register multiple listeners against the same event type by
//! tagging them with a dot-namespace, e.g. `"click.zoom click.tooltip"`.
//! Removing `".zoom"` then targets only the namespaced subset. This module
//! ports the parser as a standalone utility because the same syntax is
//! reused across d3-selection, d3-dispatch, d3-drag, and d3-zoom.
//!
//! Note: a separate, fuller implementation of dispatch lives in the
//! `rgraph-dispatch` crate. This module is purely the parser, returned as
//! plain data so it can be plugged into a custom Dioxus event handler
//! registry.

/// One element of a parsed typenames string.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Typename {
    /// Event-type prefix, e.g. `"click"`. Empty when the input was a bare
    /// `".name"` token.
    pub type_: String,
    /// Dot-namespace tag, e.g. `"zoom"`. Empty when the input had no dot.
    pub name: String,
}

impl Typename {
    /// Returns whether this has a non-empty type.
    pub fn is_typed(&self) -> bool { !self.type_.is_empty() }
    /// Returns whether this has a non-empty namespace tag.
    pub fn is_named(&self) -> bool { !self.name.is_empty() }
}

/// Parse a whitespace-separated list of `type`, `type.name`, or `.name`
/// tokens into [`Typename`] entries.
///
/// Mirrors d3's parseTypenames except that we use Rust `split_whitespace`
/// (matches `\s+`) and accept any number of leading/trailing spaces. The
/// d3 source uses `t.trim().split(/^|\s+/).map(...)` which produces an
/// initial empty entry for leading whitespace; we drop those for
/// ergonomics.
///
/// # Examples
///
/// ```ignore
/// use rgraph_selection::typenames::parse_typenames;
/// let p = parse_typenames("click.foo mouseover.bar");
/// assert_eq!(p.len(), 2);
/// assert_eq!(p[0].type_, "click");
/// assert_eq!(p[0].name, "foo");
/// ```
pub fn parse_typenames(typenames: &str) -> Vec<Typename> {
    let mut out = Vec::new();
    for t in typenames.split_whitespace() {
        out.push(parse_one(t));
    }
    out
}

/// Parse a single token. Public for callers that already split their input.
pub fn parse_one(token: &str) -> Typename {
    if let Some(i) = token.find('.') {
        Typename { type_: token[..i].to_owned(), name: token[i + 1..].to_owned() }
    } else {
        Typename { type_: token.to_owned(), name: String::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_unnamed_type() {
        let p = parse_typenames("click");
        assert_eq!(p, vec![Typename { type_: "click".into(), name: "".into() }]);
    }

    #[test]
    fn single_namespaced_type() {
        let p = parse_typenames("click.zoom");
        assert_eq!(p, vec![Typename { type_: "click".into(), name: "zoom".into() }]);
    }

    #[test]
    fn bare_dot_name() {
        let p = parse_typenames(".tagged");
        assert_eq!(p, vec![Typename { type_: "".into(), name: "tagged".into() }]);
        assert!(!p[0].is_typed());
        assert!(p[0].is_named());
    }

    #[test]
    fn space_separated_list() {
        let p = parse_typenames("click.zoom mouseover.tooltip drag");
        assert_eq!(
            p,
            vec![
                Typename { type_: "click".into(), name: "zoom".into() },
                Typename { type_: "mouseover".into(), name: "tooltip".into() },
                Typename { type_: "drag".into(), name: "".into() },
            ]
        );
    }

    #[test]
    fn leading_and_trailing_whitespace_ok() {
        assert_eq!(parse_typenames("   click   "), vec![parse_one("click")]);
        assert_eq!(parse_typenames("\tclick\nzoom "),
                   vec![parse_one("click"), parse_one("zoom")]);
    }

    #[test]
    fn empty_returns_empty() {
        assert!(parse_typenames("").is_empty());
        assert!(parse_typenames("   ").is_empty());
    }

    #[test]
    fn multiple_dots_take_first() {
        // d3's indexOf("." ) finds the first dot.
        let p = parse_typenames("click.foo.bar");
        assert_eq!(p, vec![Typename { type_: "click".into(), name: "foo.bar".into() }]);
    }
}
