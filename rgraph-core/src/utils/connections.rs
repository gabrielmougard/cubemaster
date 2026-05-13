//! Port of `xyflow-core/src/utils/connections.ts`.
//!
//! Status: implemented (phase 2).
//!
//! Note: TS `addEdge` and `reconnectEdge` actually live in
//! `utils/edges/general.ts` upstream and are ported in
//! [`crate::utils::edges::general`]. This module covers the
//! lower-level connection-lookup helpers.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use crate::types::connection::HandleConnection;

/// Connection-status string returned to consumers as a CSS class
/// modifier (`"valid"`, `"invalid"`, or `None` while undecided).
///
/// Mirrors the TS `getConnectionStatus`.
#[must_use]
#[inline]
pub fn get_connection_status(is_valid: Option<bool>) -> Option<&'static str> {
    is_valid.map(|v| if v { "valid" } else { "invalid" })
}

/// Compares two `handle_id -> HandleConnection` maps for *key-set*
/// equality.
///
/// Mirrors the TS `areConnectionMapsEqual(a?, b?)`. Both `None` is
/// treated as equal, exactly one `None` is unequal. Note: the TS
/// implementation checks key-set equality, not value equality — this
/// matches.
#[must_use]
pub fn are_connection_maps_equal(
    a: Option<&HashMap<String, HandleConnection>>,
    b: Option<&HashMap<String, HandleConnection>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            if x.len() != y.len() {
                return false;
            }
            x.keys().all(|k| y.contains_key(k))
        }
        _ => false,
    }
}

/// Call `cb` with every connection that exists in `a` but not in `b`.
///
/// Mirrors the TS `handleConnectionChange(a, b, cb?)`. The callback is
/// only invoked if the diff is non-empty, matching the JS short-circuit.
pub fn handle_connection_change<F: FnOnce(Vec<HandleConnection>)>(
    a: &HashMap<String, HandleConnection>,
    b: &HashMap<String, HandleConnection>,
    cb: Option<F>,
) {
    let Some(cb) = cb else {
        return;
    };
    let diff: Vec<HandleConnection> = a
        .iter()
        .filter(|(k, _)| !b.contains_key(*k))
        .map(|(_, v)| v.clone())
        .collect();
    if !diff.is_empty() {
        cb(diff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::connection::Connection;

    fn make_handle_conn(edge_id: &str) -> HandleConnection {
        HandleConnection {
            connection: Connection {
                source: "a".into(),
                target: "b".into(),
                source_handle: None,
                target_handle: None,
            },
            edge_id: edge_id.into(),
        }
    }

    #[test]
    fn connection_status_strings() {
        assert_eq!(get_connection_status(Some(true)), Some("valid"));
        assert_eq!(get_connection_status(Some(false)), Some("invalid"));
        assert_eq!(get_connection_status(None), None);
    }

    #[test]
    fn maps_equal_both_none() {
        assert!(are_connection_maps_equal(None, None));
    }

    #[test]
    fn maps_unequal_when_one_is_none() {
        let m: HashMap<String, HandleConnection> = HashMap::new();
        assert!(!are_connection_maps_equal(Some(&m), None));
        assert!(!are_connection_maps_equal(None, Some(&m)));
    }

    #[test]
    fn maps_equal_when_same_keys() {
        let mut a = HashMap::new();
        a.insert("h1".to_string(), make_handle_conn("e1"));
        let mut b = HashMap::new();
        b.insert("h1".to_string(), make_handle_conn("e2")); // different value, same key
        assert!(are_connection_maps_equal(Some(&a), Some(&b)));
    }

    #[test]
    fn maps_unequal_when_different_keys() {
        let mut a = HashMap::new();
        a.insert("h1".to_string(), make_handle_conn("e1"));
        let mut b = HashMap::new();
        b.insert("h2".to_string(), make_handle_conn("e1"));
        assert!(!are_connection_maps_equal(Some(&a), Some(&b)));
    }

    #[test]
    fn handle_connection_change_calls_cb_with_diff() {
        let mut a = HashMap::new();
        a.insert("h1".to_string(), make_handle_conn("e1"));
        a.insert("h2".to_string(), make_handle_conn("e2"));
        let mut b = HashMap::new();
        b.insert("h1".to_string(), make_handle_conn("e1"));
        let mut captured: Vec<HandleConnection> = Vec::new();
        handle_connection_change(
            &a,
            &b,
            Some(|diff: Vec<HandleConnection>| {
                captured = diff;
            }),
        );
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].edge_id, "e2");
    }

    #[test]
    fn handle_connection_change_no_op_when_empty_diff() {
        let mut a = HashMap::new();
        a.insert("h1".to_string(), make_handle_conn("e1"));
        let b = a.clone();
        let mut called = false;
        handle_connection_change(
            &a,
            &b,
            Some(|_: Vec<HandleConnection>| {
                called = true;
            }),
        );
        assert!(!called);
    }
}
