//! Data-join — the algorithmic heart of d3-selection, ported to Rust.
//!
//! This module provides two layers:
//!
//! 1. [`bind_index`] / [`bind_key`] — faithful ports of d3-selection's
//!    `bindIndex` and `bindKey` algorithms in `selection/data.js`. Generic
//!    over a "node" type `N`. Returns a [`JoinResult`] carrying `enter`,
//!    `update`, `exit` slots plus the `next` insertion-order link.
//!
//! 2. [`KeyedDiff`] — a Dioxus-friendly API that runs `bind_key` against the
//!    *previous* set of keys (what's currently rendered) and a *new* slice
//!    of data, producing a plan with concrete indices the caller can use to
//!    drive a `for` loop in `rsx!{}` with stable `key={…}` attributes.
//!
//! Either layer is independently usable. The high-level [`KeyedDiff`] is
//! what most Dioxus apps want; the low-level functions are exposed for
//! faithful d3 parity, custom renderers, and unit-test infrastructure.
//!
//! # Background — what the data-join means
//!
//! Given a parent (think: a `<ul>`) holding zero or more children (its
//! `<li>`s) plus a fresh array of *data* items, d3 computes a three-way
//! partition:
//!
//! * **update** — pre-existing children that should be kept (their data
//!   gets refreshed).
//! * **enter** — new data items that have no existing child yet (need to
//!   be created).
//! * **exit** — pre-existing children whose data has gone away (need to be
//!   removed).
//!
//! The pairing is by *index* in the simple case (the `i`-th child gets the
//! `i`-th datum) or by an explicit *key function* if the caller wants
//! reordering / stable identity across re-renders.

use std::collections::HashMap;
use std::hash::Hash;

// ---------------------------------------------------------------------------
// JoinResult — output of bind_index / bind_key
// ---------------------------------------------------------------------------

/// One element of the `enter` slot. An incoming datum that is **not**
/// associated with any existing node, optionally carrying the next-update
/// index so the caller can place it before the right sibling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnterEntry<D> {
    /// Position in the `data` slice this entry corresponds to.
    pub data_index: usize,
    /// The datum itself.
    pub datum: D,
    /// Index into `update` of the *following* update slot, if any. Mirrors
    /// d3's `previous._next = next || null` link, used by `appendChild`/
    /// `insertBefore` to materialize children in the right place.
    pub next_update: Option<usize>,
}

/// Output of [`bind_index`] / [`bind_key`].
///
/// Lengths:
/// * `update.len() == data.len()` — one slot per incoming datum (`Some` if
///   matched to an existing node, `None` if entering).
/// * `enter.len() == data.len()` — sparse companion of `update`.
/// * `exit.len() == group.len()` — sparse: nodes that are leaving.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JoinResult<D, N> {
    /// `Some((node, datum))` for matched data, `None` for entering data.
    /// Same length as the input `data` slice.
    pub update: Vec<Option<(N, D)>>,
    /// `Some(EnterEntry)` for entering data, `None` for matched data.
    /// Same length as the input `data` slice.
    pub enter: Vec<Option<EnterEntry<D>>>,
    /// `Some(node)` for nodes that were not matched to any datum. Same
    /// length as the input `group` slice.
    pub exit: Vec<Option<N>>,
}

impl<D, N> JoinResult<D, N> {
    /// Iterator over `(data_index, &datum)` for entering items.
    pub fn entering(&self) -> impl Iterator<Item = (usize, &D)> {
        self.enter.iter().filter_map(|opt| opt.as_ref().map(|e| (e.data_index, &e.datum)))
    }

    /// Iterator over `(data_index, &node, &datum)` for kept items.
    pub fn updating(&self) -> impl Iterator<Item = (usize, &N, &D)> {
        self.update
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|(n, d)| (i, n, d)))
    }

    /// Iterator over `(group_index, &node)` for exiting items.
    pub fn exiting(&self) -> impl Iterator<Item = (usize, &N)> {
        self.exit
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|n| (i, n)))
    }

    /// Count of entering items.
    pub fn enter_count(&self) -> usize {
        self.enter.iter().filter(|x| x.is_some()).count()
    }
    /// Count of updating items.
    pub fn update_count(&self) -> usize {
        self.update.iter().filter(|x| x.is_some()).count()
    }
    /// Count of exiting items.
    pub fn exit_count(&self) -> usize {
        self.exit.iter().filter(|x| x.is_some()).count()
    }
}

// ---------------------------------------------------------------------------
// bind_index — d3's bindIndex
// ---------------------------------------------------------------------------

/// Index-based join. Pair the `i`-th data item with `group[i]` if it is
/// `Some`, otherwise the datum enters; any leftover nodes in `group` (when
/// `data.len() < group.len()`) exit.
///
/// Equivalent to d3-selection's `bindIndex`. The "next link" between
/// successive enter slots mirrors d3's loop that fills `previous._next`
/// with the index of the next non-null update slot.
pub fn bind_index<D: Clone, N: Clone>(group: &[Option<N>], data: &[D]) -> JoinResult<D, N> {
    let group_length = group.len();
    let data_length = data.len();

    let mut update: Vec<Option<(N, D)>> = (0..data_length).map(|_| None).collect();
    let mut enter: Vec<Option<EnterEntry<D>>> = (0..data_length).map(|_| None).collect();
    let mut exit: Vec<Option<N>> = (0..group_length).map(|_| None).collect();

    // Phase 1: pair each datum with its same-indexed node, if any.
    for i in 0..data_length {
        match group.get(i).and_then(|n| n.as_ref()) {
            Some(node) => {
                update[i] = Some((node.clone(), data[i].clone()));
            }
            None => {
                enter[i] = Some(EnterEntry {
                    data_index: i,
                    datum: data[i].clone(),
                    next_update: None,
                });
            }
        }
    }

    // Phase 2: any extra nodes (group_length > data_length) exit.
    if group_length > data_length {
        for (i, slot) in group.iter().enumerate().skip(data_length) {
            if let Some(node) = slot {
                exit[i] = Some(node.clone());
            }
        }
    }

    fill_next_links(&mut enter, &update);

    JoinResult { update, enter, exit }
}

// ---------------------------------------------------------------------------
// bind_key — d3's bindKey
// ---------------------------------------------------------------------------

/// Key-based join. Each incoming datum and each existing node yields a
/// hashable key; matching keys are paired into `update`. Unmatched data
/// goes into `enter`; unmatched nodes (or duplicates) go into `exit`.
///
/// `node_key` returns the key for an existing node — typically by reading
/// some attribute or its bound datum. `data_key` returns the key for an
/// incoming datum.
///
/// Mirrors d3's `bindKey` exactly:
///
/// * If two existing nodes share a key, the *second* one is sent to
///   `exit` (the first wins update).
/// * If two data items share a key, the second is sent to `enter` (with
///   no node match).
/// * After matching, nodes that were never picked up by any datum are
///   sent to `exit` — this preserves d3's third loop that re-checks
///   `nodeByKeyValue` to differentiate "matched and consumed" from
///   "matched but bumped by a duplicate".
pub fn bind_key<D, N, K, FN, FD>(
    group: &[Option<N>],
    data: &[D],
    mut node_key: FN,
    mut data_key: FD,
) -> JoinResult<D, N>
where
    D: Clone,
    N: Clone,
    K: Hash + Eq + Clone,
    FN: FnMut(usize, &N) -> K,
    FD: FnMut(usize, &D) -> K,
{
    let group_length = group.len();
    let data_length = data.len();

    let mut update: Vec<Option<(N, D)>> = (0..data_length).map(|_| None).collect();
    let mut enter: Vec<Option<EnterEntry<D>>> = (0..data_length).map(|_| None).collect();
    let mut exit: Vec<Option<N>> = (0..group_length).map(|_| None).collect();

    // 1) Compute the key for each existing node. First-come wins; later
    //    duplicates go to exit. Track the per-node key in `key_values` so
    //    we can resolve "was this node consumed?" later.
    let mut by_key: HashMap<K, (usize, N)> = HashMap::with_capacity(group_length);
    let mut key_values: Vec<Option<K>> = (0..group_length).map(|_| None).collect();
    for (i, slot) in group.iter().enumerate() {
        if let Some(node) = slot {
            let k = node_key(i, node);
            key_values[i] = Some(k.clone());
            // Use the entry API so we get a single hash lookup instead of
            // contains_key + insert (clippy::map_entry).
            use std::collections::hash_map::Entry;
            match by_key.entry(k) {
                Entry::Occupied(_) => { exit[i] = Some(node.clone()); }
                Entry::Vacant(v) => { v.insert((i, node.clone())); }
            }
        }
    }

    // 2) Walk data and try to consume nodes from `by_key`. Consumed nodes
    //    are removed so a duplicate datum cannot re-bind to the same node.
    for (i, datum) in data.iter().enumerate() {
        let k = data_key(i, datum);
        if let Some((_node_index, node)) = by_key.remove(&k) {
            update[i] = Some((node, datum.clone()));
        } else {
            enter[i] = Some(EnterEntry { data_index: i, datum: datum.clone(), next_update: None });
        }
    }

    // 3) Any node still in `by_key` (i.e. its key was never consumed by
    //    any datum) goes to exit. Walk `key_values` to preserve original
    //    group ordering of exits.
    for (i, slot) in group.iter().enumerate() {
        if exit[i].is_some() { continue; }
        if let (Some(node), Some(k)) = (slot, &key_values[i])
            && let Some((idx, _)) = by_key.get(k)
            && *idx == i
        {
            // If the key is still in `by_key` AND the entry's index equals
            // i (i.e. this *was* the unique bearer of this key), exit it.
            // After step 2 we removed entries that were consumed, so any
            // leftover with this key matches an unconsumed first-occurrence.
            exit[i] = Some(node.clone());
        }
    }

    fill_next_links(&mut enter, &update);
    JoinResult { update, enter, exit }
}

// ---------------------------------------------------------------------------
// helper: link enter slots to the following update slot
// ---------------------------------------------------------------------------

fn fill_next_links<D, N>(
    enter: &mut [Option<EnterEntry<D>>],
    update: &[Option<(N, D)>],
) {
    let n = enter.len();
    let mut i1 = 0usize;
    for (i0, slot) in enter.iter_mut().enumerate() {
        if slot.is_some() {
            if i0 >= i1 { i1 = i0 + 1; }
            // Find next update slot at or after i1.
            while i1 < n && update.get(i1).map(|s| s.is_none()).unwrap_or(true) {
                i1 += 1;
            }
            let next = if i1 < n { Some(i1) } else { None };
            if let Some(entry) = slot.as_mut() {
                entry.next_update = next;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// KeyedDiff — Dioxus-friendly API
// ---------------------------------------------------------------------------

/// Plan returned by [`KeyedDiff::diff`] describing what a Dioxus render
/// should do given the previous keys and the new data.
///
/// Semantics:
/// * `enter` — new keys, in the order they appear in `new_data`. Each
///   entry carries the datum's index in `new_data` so the caller can
///   render `rsx!{ key: {key}, … }` with the right datum.
/// * `update` — keys that existed before *and* still exist. Same shape.
/// * `exit` — keys that existed before but no longer appear. The caller
///   typically does nothing with these (Dioxus removes them by their
///   absence in the next render); they're returned for completeness so
///   callers can fire `on_exit` callbacks (e.g. for transitions).
/// * `order` — final render order: a vector of `(K, datum_index)` tuples
///   in the order the renderer should emit them. This is exactly the
///   sequence produced by walking `new_data` from start to end, but
///   exposed as a separate field so callers don't have to recompute it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffPlan<K: Clone + Eq + Hash, D: Clone> {
    pub enter: Vec<DiffEntry<K, D>>,
    pub update: Vec<DiffEntry<K, D>>,
    pub exit: Vec<K>,
    pub order: Vec<(K, usize)>,
}

/// One enter or update entry in a [`DiffPlan`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffEntry<K: Clone + Eq + Hash, D: Clone> {
    /// Stable key of this item.
    pub key: K,
    /// Index into the input `new_data` slice this entry corresponds to.
    pub data_index: usize,
    /// Datum value (clone of `new_data[data_index]`).
    pub datum: D,
}

/// Stateful keyed-list reconciler.
///
/// Construct one per renderable list and call [`KeyedDiff::diff`] on each
/// render with the new data. The reconciler remembers the previous keys
/// so it can produce a precise enter/update/exit plan.
///
/// # Example (conceptual)
///
/// ```ignore
/// // In a Dioxus component holding `cubes: Vec<Cube>` and
/// // `mut diff: KeyedDiff<CubeId, Cube>`:
/// let plan = diff.diff(&cubes, |c| c.id);
/// rsx! {
///     for entry in plan.order.iter() {
///         div { key: "{entry.0}", /* … */ }
///     }
/// }
/// ```
pub struct KeyedDiff<K: Clone + Eq + Hash, D: Clone> {
    /// Keys in the order they were last rendered.
    prev_keys: Vec<K>,
    _marker: std::marker::PhantomData<D>,
}

impl<K: Clone + Eq + Hash, D: Clone> KeyedDiff<K, D> {
    /// Construct an empty reconciler.
    pub fn new() -> Self {
        KeyedDiff { prev_keys: Vec::new(), _marker: std::marker::PhantomData }
    }

    /// Construct a reconciler pre-seeded with the given keys (useful when
    /// a list is hydrated from external state and the first `diff` should
    /// not classify everything as `enter`).
    pub fn from_keys<I: IntoIterator<Item = K>>(keys: I) -> Self {
        KeyedDiff {
            prev_keys: keys.into_iter().collect(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the keys remembered from the previous diff, in render order.
    pub fn keys(&self) -> &[K] { &self.prev_keys }
    pub fn len(&self) -> usize { self.prev_keys.len() }
    pub fn is_empty(&self) -> bool { self.prev_keys.is_empty() }

    /// Compute a [`DiffPlan`] against `new_data`, identifying each datum
    /// by `key_of(datum)`. After this call, `self`'s remembered keys are
    /// updated to reflect the new render.
    ///
    /// Semantics match d3's `bindKey`:
    /// * If `new_data` contains duplicate keys, the second occurrence is
    ///   classified as `enter` (no existing item matches it).
    /// * If, somehow, the previous keys contained duplicates (shouldn't
    ///   happen for well-keyed lists), the first occurrence matches
    ///   first; later duplicates fall through to `exit`.
    pub fn diff<F>(&mut self, new_data: &[D], mut key_of: F) -> DiffPlan<K, D>
    where
        F: FnMut(&D) -> K,
    {
        // Build the previous-key index. `prev_index` lets us detect
        // duplicates among prior keys (preserving d3's "first occurrence
        // wins" rule).
        let mut prev_index: HashMap<K, usize> = HashMap::with_capacity(self.prev_keys.len());
        for (i, k) in self.prev_keys.iter().enumerate() {
            prev_index.entry(k.clone()).or_insert(i);
        }

        let mut consumed: HashMap<K, bool> = HashMap::with_capacity(self.prev_keys.len());
        let mut enter: Vec<DiffEntry<K, D>> = Vec::new();
        let mut update: Vec<DiffEntry<K, D>> = Vec::new();
        let mut order: Vec<(K, usize)> = Vec::with_capacity(new_data.len());
        let mut new_keys: Vec<K> = Vec::with_capacity(new_data.len());

        for (i, datum) in new_data.iter().enumerate() {
            let k = key_of(datum);
            order.push((k.clone(), i));
            new_keys.push(k.clone());
            let already_used = consumed.get(&k).copied().unwrap_or(false);
            if !already_used && prev_index.contains_key(&k) {
                consumed.insert(k.clone(), true);
                update.push(DiffEntry { key: k, data_index: i, datum: datum.clone() });
            } else {
                enter.push(DiffEntry { key: k, data_index: i, datum: datum.clone() });
            }
        }

        // exit = prev_keys whose key was never consumed by any new datum.
        let mut exit: Vec<K> = Vec::new();
        let mut seen_exit: HashMap<K, bool> = HashMap::new();
        for k in &self.prev_keys {
            if consumed.get(k).copied().unwrap_or(false) { continue; }
            // Don't emit duplicates of the same exiting key.
            if seen_exit.insert(k.clone(), true).is_some() { continue; }
            exit.push(k.clone());
        }

        // Update remembered keys for next round.
        self.prev_keys = new_keys;

        DiffPlan { enter, update, exit, order }
    }

    /// Forget the remembered keys without producing a plan. After this,
    /// the next [`KeyedDiff::diff`] will classify everything as `enter`.
    pub fn reset(&mut self) { self.prev_keys.clear(); }
}

impl<K: Clone + Eq + Hash, D: Clone> Default for KeyedDiff<K, D> {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- bind_index -----

    #[test]
    fn bind_index_pairs_data_to_existing() {
        let group = vec![Some("a"), Some("b"), Some("c")];
        let data = vec!["foo", "bar", "baz"];
        let r = bind_index(&group, &data);
        assert_eq!(r.update_count(), 3);
        assert_eq!(r.enter_count(), 0);
        assert_eq!(r.exit_count(), 0);
        assert_eq!(r.update[0], Some(("a", "foo")));
        assert_eq!(r.update[1], Some(("b", "bar")));
        assert_eq!(r.update[2], Some(("c", "baz")));
    }

    #[test]
    fn bind_index_extra_data_enters() {
        let group = vec![Some("a"), Some("b")];
        let data = vec!["foo", "bar", "baz"];
        let r = bind_index(&group, &data);
        assert_eq!(r.update_count(), 2);
        assert_eq!(r.enter_count(), 1);
        assert_eq!(r.exit_count(), 0);
        let e = r.enter[2].as_ref().unwrap();
        assert_eq!(e.data_index, 2);
        assert_eq!(e.datum, "baz");
        // No following update -> next_update is None.
        assert_eq!(e.next_update, None);
    }

    #[test]
    fn bind_index_extra_nodes_exit() {
        let group = vec![Some("a"), Some("b"), Some("c")];
        let data = vec!["foo", "bar"];
        let r = bind_index(&group, &data);
        assert_eq!(r.update_count(), 2);
        assert_eq!(r.enter_count(), 0);
        assert_eq!(r.exit_count(), 1);
        assert_eq!(r.exit[2], Some("c"));
    }

    #[test]
    fn bind_index_sparse_group_creates_enter() {
        // group = [None, "b", None] data = ["x", "y", "z"]
        // -> update[1] = (b, y); enter[0]=x, enter[2]=z
        let group: Vec<Option<&str>> = vec![None, Some("b"), None];
        let data = vec!["x", "y", "z"];
        let r = bind_index(&group, &data);
        assert_eq!(r.update[0], None);
        assert_eq!(r.update[1], Some(("b", "y")));
        assert_eq!(r.update[2], None);
        let e0 = r.enter[0].as_ref().unwrap();
        let e2 = r.enter[2].as_ref().unwrap();
        assert_eq!(e0.datum, "x");
        assert_eq!(e2.datum, "z");
        // next_update for enter[0] should point to update[1]
        assert_eq!(e0.next_update, Some(1));
        // next_update for enter[2] has no following update -> None
        assert_eq!(e2.next_update, None);
    }

    // ----- bind_key -----

    fn bind_by_str_id<'a>(
        group: &[Option<&'a str>],
        data: &[&'a str],
    ) -> JoinResult<&'a str, &'a str> {
        bind_key(
            group,
            data,
            |_, n| (*n).to_owned(),
            |_, d| (*d).to_owned(),
        )
    }

    #[test]
    fn bind_key_matches_by_key() {
        // d3 test: data ["one","four","three"], key=id, group=[one,two,three]
        // expected: groups [one, _, three], enter[1]="four", exit[1]=two.
        let group = vec![Some("one"), Some("two"), Some("three")];
        let data = vec!["one", "four", "three"];
        let r = bind_by_str_id(&group, &data);
        assert_eq!(r.update[0], Some(("one", "one")));
        assert_eq!(r.update[1], None);
        assert_eq!(r.update[2], Some(("three", "three")));
        let e = r.enter[1].as_ref().unwrap();
        assert_eq!(e.datum, "four");
        // following update slot is at index 2
        assert_eq!(e.next_update, Some(2));
        assert_eq!(r.exit[0], None);
        assert_eq!(r.exit[1], Some("two"));
        assert_eq!(r.exit[2], None);
    }

    #[test]
    fn bind_key_duplicate_node_keys_go_to_exit() {
        // d3 test: 3 nodes with name attrs ["foo","foo","bar"], data=["foo"]
        // Expected: groups [one], exit [_, two, three].
        // We'll model node keys directly: nodes are ("one","foo"),("two","foo"),("three","bar")
        let group: Vec<Option<&'static str>> =
            vec![Some("one"), Some("two"), Some("three")];
        let names = ["foo", "foo", "bar"];
        let data = vec!["foo"];
        let r = bind_key(
            &group,
            &data,
            |i, _| names[i].to_owned(),
            |_, d| (*d).to_owned(),
        );
        assert_eq!(r.update[0], Some(("one", "foo"))); // first-key wins
        assert_eq!(r.exit[0], None);
        assert_eq!(r.exit[1], Some("two"));   // duplicate key -> exit
        assert_eq!(r.exit[2], Some("three")); // never matched -> exit
    }

    #[test]
    fn bind_key_duplicate_data_keys_go_to_enter() {
        // d3 test: data=["one","one","two"], key=id, group=[one,two,three]
        // Expected: groups [one, _, two], enter[1]={"one", next=two},
        // exit[2]=three.
        let group = vec![Some("one"), Some("two"), Some("three")];
        let data = vec!["one", "one", "two"];
        let r = bind_by_str_id(&group, &data);
        assert_eq!(r.update[0], Some(("one", "one")));
        assert_eq!(r.update[1], None);
        assert_eq!(r.update[2], Some(("two", "two")));
        let e1 = r.enter[1].as_ref().unwrap();
        assert_eq!(e1.datum, "one");
        // next_update = index 2 (which is the "two" update slot)
        assert_eq!(e1.next_update, Some(2));
        assert_eq!(r.exit[2], Some("three"));
    }

    #[test]
    fn bind_key_reordering() {
        // d3 test: data ["four","three","one","five","two"]
        // expected: groups [_, three, one, _, two];
        //           enter[0]={four, next=three(idx=1)},
        //           enter[3]={five, next=two(idx=4)};
        //           exit empty.
        let group = vec![Some("one"), Some("two"), Some("three")];
        let data = vec!["four", "three", "one", "five", "two"];
        let r = bind_by_str_id(&group, &data);
        assert_eq!(r.update[0], None);
        assert_eq!(r.update[1], Some(("three", "three")));
        assert_eq!(r.update[2], Some(("one", "one")));
        assert_eq!(r.update[3], None);
        assert_eq!(r.update[4], Some(("two", "two")));
        let e0 = r.enter[0].as_ref().unwrap();
        let e3 = r.enter[3].as_ref().unwrap();
        assert_eq!(e0.datum, "four");
        assert_eq!(e0.next_update, Some(1));
        assert_eq!(e3.datum, "five");
        assert_eq!(e3.next_update, Some(4));
        assert!(r.exit.iter().all(|s| s.is_none()));
    }

    #[test]
    fn bind_key_all_data_replaced() {
        // d3 test: groups=[a,b,c], data=["x","y","z"], all new keys
        let group = vec![Some("a"), Some("b"), Some("c")];
        let data = vec!["x", "y", "z"];
        let r = bind_by_str_id(&group, &data);
        // All data enters
        assert_eq!(r.enter_count(), 3);
        assert_eq!(r.update_count(), 0);
        // All nodes exit
        assert_eq!(r.exit_count(), 3);
        assert_eq!(r.exit[0], Some("a"));
        assert_eq!(r.exit[1], Some("b"));
        assert_eq!(r.exit[2], Some("c"));
    }

    #[test]
    fn bind_key_empty_group() {
        let group: Vec<Option<&'static str>> = vec![];
        let data = vec!["x", "y"];
        let r = bind_by_str_id(&group, &data);
        assert_eq!(r.enter_count(), 2);
        assert_eq!(r.update_count(), 0);
        assert_eq!(r.exit_count(), 0);
    }

    #[test]
    fn bind_key_empty_data() {
        let group = vec![Some("a"), Some("b")];
        let data: Vec<&'static str> = vec![];
        let r = bind_by_str_id(&group, &data);
        assert_eq!(r.enter_count(), 0);
        assert_eq!(r.update_count(), 0);
        assert_eq!(r.exit_count(), 2);
    }

    // ----- KeyedDiff -----

    #[test]
    fn keyed_diff_first_render_is_all_enter() {
        let mut d: KeyedDiff<u64, (u64, &'static str)> = KeyedDiff::new();
        let data = vec![(1u64, "a"), (2, "b"), (3, "c")];
        let plan = d.diff(&data, |it| it.0);
        assert_eq!(plan.enter.len(), 3);
        assert_eq!(plan.update.len(), 0);
        assert_eq!(plan.exit.len(), 0);
        assert_eq!(d.keys(), &[1, 2, 3]);
    }

    #[test]
    fn keyed_diff_unchanged_is_all_update() {
        let mut d: KeyedDiff<u64, (u64, i32)> = KeyedDiff::new();
        let v1 = vec![(1u64, 10), (2, 20)];
        d.diff(&v1, |it| it.0);
        let v2 = vec![(1u64, 11), (2, 22)];
        let plan = d.diff(&v2, |it| it.0);
        assert_eq!(plan.enter.len(), 0);
        assert_eq!(plan.update.len(), 2);
        assert_eq!(plan.exit.len(), 0);
        // Update carries the new datum, not the old.
        assert_eq!(plan.update[0].datum, (1, 11));
        assert_eq!(plan.update[1].datum, (2, 22));
    }

    #[test]
    fn keyed_diff_partial() {
        let mut d: KeyedDiff<u64, (u64, &'static str)> = KeyedDiff::new();
        d.diff(&[(1, "a"), (2, "b"), (3, "c")], |it| it.0);
        let plan = d.diff(&[(2, "b"), (3, "c"), (4, "d")], |it| it.0);
        // 1 exited; 2,3 update; 4 enters
        assert_eq!(plan.exit, vec![1]);
        assert_eq!(plan.update.iter().map(|e| e.key).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(plan.enter.iter().map(|e| e.key).collect::<Vec<_>>(), vec![4]);
        // Order matches new_data
        assert_eq!(plan.order, vec![(2, 0), (3, 1), (4, 2)]);
        // State updated for next round.
        assert_eq!(d.keys(), &[2, 3, 4]);
    }

    #[test]
    fn keyed_diff_reorder_only() {
        let mut d: KeyedDiff<u64, (u64, &'static str)> = KeyedDiff::new();
        d.diff(&[(1, "a"), (2, "b"), (3, "c")], |it| it.0);
        let plan = d.diff(&[(3, "c"), (1, "a"), (2, "b")], |it| it.0);
        assert_eq!(plan.enter.len(), 0);
        assert_eq!(plan.exit.len(), 0);
        assert_eq!(plan.update.len(), 3);
        assert_eq!(plan.order, vec![(3, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn keyed_diff_duplicate_data_keys_enter_second() {
        let mut d: KeyedDiff<&'static str, (&'static str, u32)> = KeyedDiff::new();
        d.diff(&[("a", 1)], |it| it.0);
        let plan = d.diff(&[("a", 2), ("a", 3)], |it| it.0);
        // First "a" matches existing -> update.
        // Second "a" is a duplicate in new data -> enter.
        assert_eq!(plan.update.len(), 1);
        assert_eq!(plan.enter.len(), 1);
        assert_eq!(plan.update[0].datum, ("a", 2));
        assert_eq!(plan.enter[0].datum, ("a", 3));
    }

    #[test]
    fn keyed_diff_reset() {
        let mut d: KeyedDiff<u64, (u64, &'static str)> = KeyedDiff::new();
        d.diff(&[(1, "a")], |it| it.0);
        d.reset();
        let plan = d.diff(&[(1, "b")], |it| it.0);
        assert_eq!(plan.enter.len(), 1); // forgotten state -> entering
    }

    #[test]
    fn keyed_diff_from_keys_seeds_state() {
        let mut d: KeyedDiff<u64, (u64, &'static str)> = KeyedDiff::from_keys(vec![1, 2]);
        let plan = d.diff(&[(1, "a"), (3, "c")], |it| it.0);
        assert_eq!(plan.update.len(), 1);
        assert_eq!(plan.enter.len(), 1);
        assert_eq!(plan.exit, vec![2]);
    }
}
