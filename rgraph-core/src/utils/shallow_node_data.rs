//! Port of `xyflow-core/src/utils/shallow-node-data.ts`.
//!
//! Status: implemented (phase 2).
//!
//! This util is used in TS to short-circuit React re-renders by
//! comparing nodes by `id`, `type` and `Object.is(data)`. The Rust
//! equivalent works on the same fields and requires `D: PartialEq`
//! for the `data` comparison.

#![allow(clippy::module_name_repetitions)]

use crate::types::nodes::Node;

/// Compare two nodes shallowly: equal `id`, equal `type`, equal
/// `data`.
///
/// Mirrors the TS field check `_a[i].id !== _b[i].id || _a[i].type !==
/// _b[i].type || !Object.is(_a[i].data, _b[i].data)`.
#[must_use]
#[inline]
pub fn shallow_node_eq<D: Clone + PartialEq>(a: &Node<D>, b: &Node<D>) -> bool {
    a.id == b.id && a.type_ == b.type_ && a.data == b.data
}

/// Compare two slices of nodes shallowly.
///
/// Mirrors the array branch of the TS `shallowNodeData(a, b)`. Both
/// slices must have the same length and be equal element-wise via
/// [`shallow_node_eq`].
#[must_use]
pub fn shallow_node_data_slice<D: Clone + PartialEq>(a: &[Node<D>], b: &[Node<D>]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| shallow_node_eq(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: &str) -> Node<i32> {
        let mut node: Node<i32> = Node::minimal(id, 0.0, 0.0);
        node.data = 1;
        node
    }

    #[test]
    fn shallow_eq_compares_id_type_data() {
        let mut a = n("a");
        let mut b = n("a");
        assert!(shallow_node_eq(&a, &b));

        b.id = "b".into();
        assert!(!shallow_node_eq(&a, &b));
        b.id = "a".into();

        a.type_ = Some("input".into());
        b.type_ = Some("output".into());
        assert!(!shallow_node_eq(&a, &b));
        b.type_ = Some("input".into());
        assert!(shallow_node_eq(&a, &b));

        b.data = 2;
        assert!(!shallow_node_eq(&a, &b));
    }

    #[test]
    fn shallow_node_data_slice_works() {
        let a = vec![n("a"), n("b")];
        let b = vec![n("a"), n("b")];
        assert!(shallow_node_data_slice(&a, &b));
        let c = vec![n("a")];
        assert!(!shallow_node_data_slice(&a, &c));
        let d = vec![n("a"), n("c")];
        assert!(!shallow_node_data_slice(&a, &d));
    }
}
