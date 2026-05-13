//! XML namespace parsing — port of d3-selection's `namespace.js` and
//! `namespaces.js`.
//!
//! Useful when an application generates SVG markup at runtime: SVG / XLink /
//! XML attributes are correctly identified as qualified names with their
//! namespace URI rather than plain attribute strings.
//!
//! In a Dioxus codebase you typically don't need to set XML attributes
//! through this API directly — you write `rsx!{ svg::circle { /* … */ } }` —
//! but [`Name::parse`] is still useful for any code that consumes user-typed
//! attribute names (configuration, scripting, persisted plugins) and needs to
//! distinguish `xlink:href` from a generic `href`.

use std::collections::HashMap;

/// XHTML namespace URI. Equivalent to d3's `xhtml` constant.
pub const XHTML: &str = "http://www.w3.org/1999/xhtml";
/// SVG namespace URI.
pub const SVG: &str = "http://www.w3.org/2000/svg";
/// XLink namespace URI.
pub const XLINK: &str = "http://www.w3.org/1999/xlink";
/// XML namespace URI.
pub const XML: &str = "http://www.w3.org/XML/1998/namespace";
/// XMLNS namespace URI.
pub const XMLNS: &str = "http://www.w3.org/2000/xmlns/";

/// Built-in namespace prefix → URI table. Mirrors d3-selection's
/// `namespaces.js`. Returns a fresh allocation per call so callers can
/// extend it locally without affecting the defaults.
pub fn defaults() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::with_capacity(5);
    m.insert("svg", SVG);
    m.insert("xhtml", XHTML);
    m.insert("xlink", XLINK);
    m.insert("xml", XML);
    m.insert("xmlns", XMLNS);
    m
}

/// Parsed attribute or element name. d3's `namespace(name)` returns a string
/// for unqualified names and an object `{space, local}` for qualified ones;
/// we mirror that with a single enum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Name {
    /// An unqualified name. Equivalent to d3 returning the raw string.
    Local(String),
    /// A qualified name carrying the namespace URI.
    Qualified {
        /// Namespace URI (the `space` field in d3).
        space: String,
        /// Local part of the name (after the colon).
        local: String,
    },
}

impl Name {
    /// Parse a name using the [default namespace table](defaults).
    ///
    /// Faithful to d3's behavior:
    /// * `"foo"` → `Name::Local("foo")`
    /// * `"svg:circle"` → `Name::Qualified { space: SVG, local: "circle" }`
    /// * `"xmlns:foo"` → `Name::Qualified { space: XMLNS, local:
    ///   "xmlns:foo" }`. d3 keeps the `xmlns:` prefix on the local part for
    ///   exactly this case (see d3-selection #14), so we do too.
    /// * `"unknown:foo"` → `Name::Local("unknown:foo")` (unrecognized prefix
    ///   passes through).
    pub fn parse(name: &str) -> Self {
        Self::parse_with(name, &defaults())
    }

    /// Parse a name against a user-supplied prefix table.
    pub fn parse_with(name: &str, prefixes: &HashMap<&str, &str>) -> Self {
        if let Some(i) = name.find(':') {
            let (prefix, local) = (&name[..i], &name[i + 1..]);
            if let Some(&space) = prefixes.get(prefix) {
                let local_part = if prefix == "xmlns" {
                    // xmlns:foo - retain full string as local per d3.
                    name.to_owned()
                } else {
                    local.to_owned()
                };
                return Name::Qualified { space: space.to_owned(), local: local_part };
            }
        }
        Name::Local(name.to_owned())
    }

    /// Returns `true` iff this is a qualified name.
    pub fn is_qualified(&self) -> bool {
        matches!(self, Name::Qualified { .. })
    }

    /// Borrowed view of the local name part (handy when serializing).
    pub fn local(&self) -> &str {
        match self {
            Name::Local(s) => s,
            Name::Qualified { local, .. } => local,
        }
    }

    /// Borrowed namespace URI, or `None` for unqualified names.
    pub fn space(&self) -> Option<&str> {
        match self {
            Name::Local(_) => None,
            Name::Qualified { space, .. } => Some(space),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unqualified_passes_through() {
        assert_eq!(Name::parse("foo"), Name::Local("foo".into()));
        assert_eq!(Name::parse("class"), Name::Local("class".into()));
    }

    #[test]
    fn svg_prefix_is_recognized() {
        assert_eq!(
            Name::parse("svg:circle"),
            Name::Qualified { space: SVG.into(), local: "circle".into() }
        );
    }

    #[test]
    fn xlink_prefix_is_recognized() {
        assert_eq!(
            Name::parse("xlink:href"),
            Name::Qualified { space: XLINK.into(), local: "href".into() }
        );
    }

    #[test]
    fn xmlns_prefix_keeps_full_local() {
        // d3 behavior: `xmlns:` prefix is retained in the local name.
        assert_eq!(
            Name::parse("xmlns:foo"),
            Name::Qualified { space: XMLNS.into(), local: "xmlns:foo".into() }
        );
    }

    #[test]
    fn unknown_prefix_passes_through() {
        assert_eq!(
            Name::parse("custom:thing"),
            Name::Local("custom:thing".into())
        );
    }

    #[test]
    fn defaults_contains_all_d3_prefixes() {
        let d = defaults();
        assert_eq!(d.get("svg").copied(), Some(SVG));
        assert_eq!(d.get("xhtml").copied(), Some(XHTML));
        assert_eq!(d.get("xlink").copied(), Some(XLINK));
        assert_eq!(d.get("xml").copied(), Some(XML));
        assert_eq!(d.get("xmlns").copied(), Some(XMLNS));
    }

    #[test]
    fn parse_with_custom_prefix() {
        let mut m = defaults();
        m.insert("my", "urn:example:my");
        assert_eq!(
            Name::parse_with("my:thing", &m),
            Name::Qualified { space: "urn:example:my".into(), local: "thing".into() }
        );
    }

    #[test]
    fn is_qualified() {
        assert!(!Name::parse("foo").is_qualified());
        assert!(Name::parse("svg:circle").is_qualified());
    }

    #[test]
    fn space_and_local_accessors() {
        let n = Name::parse("xlink:href");
        assert_eq!(n.space(), Some(XLINK));
        assert_eq!(n.local(), "href");
        let n = Name::parse("foo");
        assert_eq!(n.space(), None);
        assert_eq!(n.local(), "foo");
    }
}
