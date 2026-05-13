//! Per-node generic data store — port of d3-selection's `local.js`.
//!
//! d3's `local()` returns an object that attaches a unique property name to
//! arbitrary DOM nodes, with `get`/`set`/`remove` operating on the raw
//! object property. The walk-up semantics of `Local#get` (climb the
//! parentNode chain looking for the first ancestor that has the data) lets
//! you scope a value to a subtree.
//!
//! In the absence of a real DOM we keep the same name and walk semantics
//! but key by a user-supplied node id `K: Hash + Eq + Clone`. The caller
//! supplies a parent-chain accessor (a closure or an iterator) so the same
//! "scoped lookup" pattern works against Dioxus elements, ECS entities, or
//! any other tree.
//!
//! Each call to [`Local::new`] creates a fresh, monotonically-numbered
//! `Local<T>` whose entries are independent of every other `Local`'s.
//! Internally we store one `HashMap<K, T>` per `Local` instance — exactly
//! mirroring d3 where each `Local` lives in its own object property
//! namespace.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique id for a `Local<T>`. Useful for debugging and for the
/// `Display`/`to_string` parity with d3's `Local#toString`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct LocalId(pub u64);

impl std::fmt::Display for LocalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Match d3's `"@" + id.toString(36)` shape.
        write!(f, "@{}", radix36(self.0))
    }
}

fn radix36(mut n: u64) -> String {
    if n == 0 { return "0".into(); }
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = Vec::with_capacity(13);
    while n > 0 {
        buf.push(ALPHABET[(n % 36) as usize]);
        n /= 36;
    }
    buf.reverse();
    // SAFETY: ALPHABET is ASCII, so the collected bytes form valid UTF-8.
    unsafe { String::from_utf8_unchecked(buf) }
}

/// Per-process counter for the next `LocalId`. d3's `nextId` is a JS
/// module-level integer; an [`AtomicU64`] is the closest faithful
/// equivalent and lets `Local::new` be `&self`-safe on multiple threads
/// even though [`Local`] itself uses interior mutability and is `!Send`.
static NEXT_LOCAL_ID: AtomicU64 = AtomicU64::new(0);

/// Per-node generic data store.
///
/// `K` — the node-id type (often `u64`, `usize`, or a wrapped `dioxus`
/// element id). `T` — the value type to store.
///
/// All mutating operations take `&self` thanks to interior mutability,
/// matching d3's `local.set(node, v)` ergonomics.
pub struct Local<K: Hash + Eq + Clone, T> {
    id: LocalId,
    /// `RefCell` so callers do not need a `&mut Local` — d3's `set` looks
    /// like a pure call site.
    data: RefCell<HashMap<K, T>>,
}

impl<K: Hash + Eq + Clone, T> Local<K, T> {
    /// Construct a new `Local`. Each call yields a fresh, unique `LocalId`.
    pub fn new() -> Self {
        let id = NEXT_LOCAL_ID.fetch_add(1, Ordering::Relaxed) + 1;
        Local {
            id: LocalId(id),
            data: RefCell::new(HashMap::new()),
        }
    }

    /// The unique id of this `Local`. Mirrors d3's `local._`.
    pub fn id(&self) -> LocalId { self.id }

    /// Number of stored entries. Useful for tests; d3 has no equivalent.
    pub fn len(&self) -> usize { self.data.borrow().len() }
    pub fn is_empty(&self) -> bool { self.data.borrow().is_empty() }

    /// Stores `value` under `node`, returning the previous value, if any.
    /// Mirrors d3's `local.set(node, value)`.
    pub fn set(&self, node: K, value: T) -> Option<T> {
        self.data.borrow_mut().insert(node, value)
    }

    /// Removes the entry for `node`, returning it. Mirrors `local.remove`.
    pub fn remove(&self, node: &K) -> Option<T> {
        self.data.borrow_mut().remove(node)
    }

    /// Returns whether `node` has a value stored.
    pub fn contains(&self, node: &K) -> bool {
        self.data.borrow().contains_key(node)
    }

    /// Returns a clone of the value at `node`, if any.
    ///
    /// Cloning is intentional: d3's `local.get(node)` returns the live
    /// reference, but Rust's borrow rules don't let us hand out a long-lived
    /// reference while mutation may happen elsewhere. For non-Clone `T`,
    /// use [`Local::with`].
    pub fn get(&self, node: &K) -> Option<T>
    where
        T: Clone,
    {
        self.data.borrow().get(node).cloned()
    }

    /// Apply `f` to the (optional) value stored at `node` without cloning.
    pub fn with<R, F: FnOnce(Option<&T>) -> R>(&self, node: &K, f: F) -> R {
        let map = self.data.borrow();
        f(map.get(node))
    }

    /// Walks the parent chain produced by `parents` looking for the first
    /// ancestor (including `node` itself) that has a stored value. Returns
    /// a clone, mirroring [`Local::get`].
    ///
    /// Equivalent to d3's full `Local#get` semantics: `while (!(id in node))
    /// node = node.parentNode;`. The caller-supplied `parents` iterator
    /// must yield ancestors in walking order from `node` up to the root.
    pub fn get_scoped<I, F>(&self, node: K, parents: F) -> Option<T>
    where
        T: Clone,
        I: IntoIterator<Item = K>,
        F: FnOnce(K) -> I,
    {
        let map = self.data.borrow();
        // Check the node itself first.
        if let Some(v) = map.get(&node) {
            return Some(v.clone());
        }
        for ancestor in parents(node) {
            if let Some(v) = map.get(&ancestor) {
                return Some(v.clone());
            }
        }
        None
    }
}

impl<K: Hash + Eq + Clone, T> Default for Local<K, T> {
    fn default() -> Self { Self::new() }
}

impl<K: Hash + Eq + Clone, T> std::fmt::Display for Local<K, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Same as d3's `local.toString()`: `"@" + id.toString(36)`.
        std::fmt::Display::fmt(&self.id, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_monotonic() {
        let a: Local<u64, u32> = Local::new();
        let b: Local<u64, u32> = Local::new();
        assert!(b.id().0 > a.id().0);
    }

    #[test]
    fn radix36_matches_js_tostring36() {
        assert_eq!(radix36(0), "0");
        assert_eq!(radix36(35), "z");
        assert_eq!(radix36(36), "10");
        assert_eq!(radix36(1296), "100"); // 36^2
        assert_eq!(radix36(123_456_789), "21i3v9");
    }

    #[test]
    fn display_matches_d3_to_string() {
        // Reset is impossible without unsafe; verify shape only.
        let l: Local<u64, u32> = Local::new();
        let s = format!("{l}");
        assert!(s.starts_with('@'));
        assert!(s.len() > 1);
    }

    #[test]
    fn set_and_get() {
        let l: Local<u64, &'static str> = Local::new();
        assert_eq!(l.get(&1), None);
        assert_eq!(l.set(1, "foo"), None);
        assert_eq!(l.get(&1), Some("foo"));
        assert_eq!(l.set(1, "bar"), Some("foo"));
        assert_eq!(l.get(&1), Some("bar"));
    }

    #[test]
    fn remove_returns_previous() {
        let l: Local<u64, u32> = Local::new();
        l.set(1, 42);
        assert_eq!(l.remove(&1), Some(42));
        assert_eq!(l.remove(&1), None);
        assert!(!l.contains(&1));
    }

    #[test]
    fn instances_are_independent() {
        let a: Local<u64, u32> = Local::new();
        let b: Local<u64, u32> = Local::new();
        a.set(1, 100);
        assert!(!b.contains(&1));
        b.set(1, 200);
        assert_eq!(a.get(&1), Some(100));
        assert_eq!(b.get(&1), Some(200));
    }

    #[test]
    fn with_does_not_clone() {
        // Type that is intentionally not Clone.
        struct NoClone(u32);
        let l: Local<u64, NoClone> = Local::new();
        l.set(1, NoClone(42));
        let v = l.with(&1, |opt| opt.map(|n| n.0));
        assert_eq!(v, Some(42));
    }

    #[test]
    fn get_scoped_walks_parents() {
        // Tree:
        //   1 (root)
        //   └── 2
        //       └── 3
        let parents = |id: u64| -> Vec<u64> {
            match id {
                3 => vec![2, 1],
                2 => vec![1],
                _ => vec![],
            }
        };
        let l: Local<u64, &'static str> = Local::new();
        l.set(1, "root");
        // Get from leaf walks up to root.
        assert_eq!(l.get_scoped(3, parents), Some("root"));
        // Direct hit at intermediate node.
        l.set(2, "mid");
        assert_eq!(l.get_scoped(3, parents), Some("mid"));
        // Direct hit on the queried node itself.
        l.set(3, "leaf");
        assert_eq!(l.get_scoped(3, parents), Some("leaf"));
    }

    #[test]
    fn get_scoped_returns_none_when_no_ancestor_has_data() {
        let parents = |_: u64| -> Vec<u64> { vec![] };
        let l: Local<u64, u32> = Local::new();
        assert_eq!(l.get_scoped(42, parents), None);
    }
}
