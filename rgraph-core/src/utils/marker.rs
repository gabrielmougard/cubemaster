//! Port of `xyflow-core/src/utils/marker.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use std::collections::HashSet;

use crate::types::edges::{Edge, EdgeMarkerType, MarkerProps, MarkerType};

#[cfg(test)]
use crate::types::edges::EdgeMarker;

/// Generate a deterministic marker id for an [`EdgeMarkerType`].
///
/// For `Builtin` (string-keyed) markers, the id is the string itself.
/// For `Custom` markers we sort the populated fields alphabetically and
/// build a `key=value&key=value` string, optionally prefixed with
/// `<id>__`. This matches the JS `getMarkerId` byte-for-byte so SVG
/// `<marker id>` lookups stay stable across SSR / hydration.
///
/// Returns the empty string for an absent marker (mirrors the TS
/// `if (!marker) return ''`).
#[must_use]
pub fn get_marker_id(marker: Option<&EdgeMarkerType>, id: Option<&str>) -> String {
    match marker {
        None => String::new(),
        Some(EdgeMarkerType::Builtin(s)) => s.clone(),
        Some(EdgeMarkerType::Custom(m)) => {
            let prefix = match id {
                Some(p) if !p.is_empty() => format!("{p}__"),
                _ => String::new(),
            };
            let mut parts: Vec<(&'static str, String)> = Vec::with_capacity(7);

            // We collect each Some-valued field as the key/value pair the
            // JS counterpart would have iterated via Object.keys(...).
            parts.push(("type", marker_type_to_js(&m.type_)));
            if let Some(c) = &m.color {
                parts.push(("color", c.clone()));
            }
            if let Some(w) = m.width {
                parts.push(("width", crate::utils::edges::format::js_num(w)));
            }
            if let Some(h) = m.height {
                parts.push(("height", crate::utils::edges::format::js_num(h)));
            }
            if let Some(u) = &m.marker_units {
                parts.push(("markerUnits", u.clone()));
            }
            if let Some(o) = &m.orient {
                parts.push(("orient", o.clone()));
            }
            if let Some(sw) = m.stroke_width {
                parts.push(("strokeWidth", crate::utils::edges::format::js_num(sw)));
            }

            // Sort by JS object key — alphabetical.
            parts.sort_by(|a, b| a.0.cmp(b.0));
            let body = parts
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            format!("{prefix}{body}")
        }
    }
}

#[inline]
fn marker_type_to_js(t: &MarkerType) -> String {
    match t {
        MarkerType::Arrow => "arrow".to_string(),
        MarkerType::ArrowClosed => "arrowclosed".to_string(),
    }
}

/// Options for [`create_marker_ids`].
#[derive(Debug, Clone, Default)]
pub struct CreateMarkerIdsOptions {
    pub id: Option<String>,
    pub default_color: Option<String>,
    pub default_marker_start: Option<EdgeMarkerType>,
    pub default_marker_end: Option<EdgeMarkerType>,
}

/// Walk `edges`, collect every unique custom marker referenced by
/// `markerStart` / `markerEnd` (or the fall-through defaults), and
/// return them as [`MarkerProps`] sorted by id.
///
/// Mirrors the TS `createMarkerIds`. Builtin (string-typed) markers are
/// ignored — only custom markers need a unique id and a `<defs>` entry.
#[must_use]
pub fn create_marker_ids<D: Clone>(
    edges: &[Edge<D>],
    options: &CreateMarkerIdsOptions,
) -> Vec<MarkerProps> {
    let mut ids: HashSet<String> = HashSet::new();
    let mut markers: Vec<MarkerProps> = Vec::new();

    for edge in edges {
        let candidates: [Option<&EdgeMarkerType>; 2] = [
            edge.marker_start.as_ref().or(options.default_marker_start.as_ref()),
            edge.marker_end.as_ref().or(options.default_marker_end.as_ref()),
        ];
        for marker in candidates.into_iter().flatten() {
            if let EdgeMarkerType::Custom(m) = marker {
                let marker_id = get_marker_id(Some(marker), options.id.as_deref());
                if ids.insert(marker_id.clone()) {
                    let final_color = m
                        .color
                        .clone()
                        .or_else(|| options.default_color.clone());
                    let mut resolved = m.clone();
                    if resolved.color.is_none() {
                        resolved.color = final_color;
                    }
                    markers.push(MarkerProps {
                        id: marker_id,
                        marker: resolved,
                    });
                }
            }
        }
    }

    markers.sort_by(|a, b| a.id.cmp(&b.id));
    markers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_marker_id_is_passthrough() {
        let m = EdgeMarkerType::Builtin("custom-arrow".into());
        assert_eq!(get_marker_id(Some(&m), None), "custom-arrow");
        assert_eq!(get_marker_id(Some(&m), Some("rf")), "custom-arrow");
    }

    #[test]
    fn empty_marker_returns_empty_string() {
        assert_eq!(get_marker_id(None, None), "");
        assert_eq!(get_marker_id(None, Some("rf")), "");
    }

    #[test]
    fn custom_marker_id_alphabetises_fields() {
        let m = EdgeMarkerType::Custom(EdgeMarker {
            type_: MarkerType::Arrow,
            color: Some("red".into()),
            width: Some(10.0),
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        });
        // Keys sorted: color, type, width
        assert_eq!(
            get_marker_id(Some(&m), None),
            "color=red&type=arrow&width=10"
        );
    }

    #[test]
    fn custom_marker_id_uses_id_prefix() {
        let m = EdgeMarkerType::Custom(EdgeMarker {
            type_: MarkerType::Arrow,
            color: None,
            width: None,
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        });
        assert_eq!(get_marker_id(Some(&m), Some("rf")), "rf__type=arrow");
    }

    #[test]
    fn create_marker_ids_dedupes_and_sorts() {
        let mut e1 = Edge::<()>::minimal("e1", "a", "b");
        let mut e2 = Edge::<()>::minimal("e2", "a", "c");
        let m_arrow = EdgeMarker {
            type_: MarkerType::Arrow,
            color: None,
            width: None,
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        };
        let m_closed = EdgeMarker {
            type_: MarkerType::ArrowClosed,
            color: None,
            width: None,
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        };
        e1.marker_end = Some(EdgeMarkerType::Custom(m_arrow.clone()));
        e2.marker_end = Some(EdgeMarkerType::Custom(m_closed.clone()));
        // e2 also has the same arrow as start → should be deduped with e1's
        e2.marker_start = Some(EdgeMarkerType::Custom(m_arrow.clone()));

        let result = create_marker_ids(&[e1, e2], &CreateMarkerIdsOptions::default());
        assert_eq!(result.len(), 2);
        // Sorted by id alphabetically — `type=arrow` before `type=arrowclosed`.
        assert_eq!(result[0].id, "type=arrow");
        assert_eq!(result[1].id, "type=arrowclosed");
    }

    #[test]
    fn create_marker_ids_applies_default_color() {
        let mut e1 = Edge::<()>::minimal("e1", "a", "b");
        e1.marker_end = Some(EdgeMarkerType::Custom(EdgeMarker {
            type_: MarkerType::Arrow,
            color: None,
            width: None,
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        }));
        let opts = CreateMarkerIdsOptions {
            default_color: Some("blue".into()),
            ..Default::default()
        };
        let result = create_marker_ids(&[e1], &opts);
        assert_eq!(result[0].marker.color.as_deref(), Some("blue"));
    }

    #[test]
    fn create_marker_ids_skips_builtin_markers() {
        // Builtin markers don't get a `<defs>` entry — they reference
        // pre-registered ids in the consumer's app.
        let mut e1 = Edge::<()>::minimal("e1", "a", "b");
        e1.marker_end = Some(EdgeMarkerType::Builtin("user-arrow".into()));
        let result = create_marker_ids(&[e1], &CreateMarkerIdsOptions::default());
        assert!(result.is_empty());
    }
}
