//! Port of `xyflow-react/src/utils/general.ts`.
//!
//! Status: Phase 1 — implemented.
//!
//! The TS source provides three exports:
//!
//! 1. `isNode(element: unknown): element is Node` — type guard.
//! 2. `isEdge(element: unknown): element is Edge` — type guard.
//! 3. `fixedForwardRef<T, P>(render): (props & RefAttributes<T>) => JSX.Element`
//!    — a `forwardRef` helper.
//!
//! In Rust the static type system already keeps `Node<D>` and `Edge<D>`
//! distinct, so the TS guards are only useful when callers receive a
//! heterogeneous list — typically `Array<Node | Edge>`. We model that
//! union as [`Element`] and provide [`Element::is_node`] /
//! [`Element::is_edge`] alongside the public functions [`is_node`] and
//! [`is_edge`] that mirror the original signatures.
//!
//! `fixedForwardRef` has no Rust analogue (Dioxus components don't use
//! React's `forwardRef`), so it is not ported.

use rgraph_core::types::edges::Edge;
use rgraph_core::types::nodes::Node;

// ---------------------------------------------------------------------------
// PtrEq — `Arc<T>` newtype with pointer-based `PartialEq`.
// ---------------------------------------------------------------------------

/// Reference-counted handle whose equality is **pointer identity**
/// instead of structural value comparison.
///
/// `dioxus::prelude::Props` requires its fields to be `Clone +
/// PartialEq`. Some `rgraph_core` option bundles
/// (`FitViewOptionsBase`, `ViewportHelperFunctionOptions`,
/// `SetCenterOptions`, …) carry boxed `EaseFn` closures and are
/// therefore not directly comparable. Wrapping them in [`PtrEq<T>`]
/// gives us cheap clones and a memoisation-friendly `PartialEq` that
/// only returns `true` when both ends point at the same allocation —
/// which is exactly what users want when memoising callback-bearing
/// props (and matches what Zustand/React do with reference identity).
#[derive(Debug)]
pub struct PtrEq<T: ?Sized>(pub std::sync::Arc<T>);

impl<T: ?Sized> PtrEq<T> {
    /// Build a new [`PtrEq`] around `value`.
    #[inline]
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
        PtrEq(std::sync::Arc::new(value))
    }

    /// Borrow the wrapped value.
    #[inline]
    pub fn get(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> Clone for PtrEq<T> {
    #[inline]
    fn clone(&self) -> Self {
        PtrEq(self.0.clone())
    }
}

impl<T: ?Sized> PartialEq for PtrEq<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Eq for PtrEq<T> {}

impl<T> From<T> for PtrEq<T> {
    #[inline]
    fn from(v: T) -> Self {
        PtrEq::new(v)
    }
}

impl<T: ?Sized> From<std::sync::Arc<T>> for PtrEq<T> {
    #[inline]
    fn from(arc: std::sync::Arc<T>) -> Self {
        PtrEq(arc)
    }
}

impl<T: ?Sized> std::ops::Deref for PtrEq<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Element — `Node | Edge` union for `is_node` / `is_edge`.
// ---------------------------------------------------------------------------

/// A heterogeneous element accepted by `apply_*_changes` / store utilities.
///
/// Mirrors the TS type `Node | Edge` returned in places where both are
/// allowed (e.g. `<ReactFlow defaultElements>` historically). Useful to
/// recover the original-shape semantics of [`is_node`] / [`is_edge`].
///
/// `N` and `E` default to the framework's plain unit-data shapes so the
/// type can be spelled `Element` without further parameterisation when
/// the data types are uninteresting.
#[derive(Debug, Clone, PartialEq)]
pub enum Element<N: Clone = (), E: Clone = ()> {
    Node(Node<N>),
    Edge(Edge<E>),
}

impl<N: Clone, E: Clone> Element<N, E> {
    /// `true` if this element is the [`Element::Node`] variant.
    #[inline]
    #[must_use]
    pub fn is_node(&self) -> bool {
        matches!(self, Element::Node(_))
    }

    /// `true` if this element is the [`Element::Edge`] variant.
    #[inline]
    #[must_use]
    pub fn is_edge(&self) -> bool {
        matches!(self, Element::Edge(_))
    }

    /// Borrow as a [`Node`] if the variant matches.
    #[inline]
    #[must_use]
    pub fn as_node(&self) -> Option<&Node<N>> {
        if let Element::Node(n) = self { Some(n) } else { None }
    }

    /// Borrow as an [`Edge`] if the variant matches.
    #[inline]
    #[must_use]
    pub fn as_edge(&self) -> Option<&Edge<E>> {
        if let Element::Edge(e) = self { Some(e) } else { None }
    }
}

impl<N: Clone, E: Clone> From<Node<N>> for Element<N, E> {
    #[inline]
    fn from(n: Node<N>) -> Self {
        Element::Node(n)
    }
}

impl<N: Clone, E: Clone> From<Edge<E>> for Element<N, E> {
    #[inline]
    fn from(e: Edge<E>) -> Self {
        Element::Edge(e)
    }
}

/// Test whether an [`Element`] is usable as a [`Node`].
///
/// Mirrors the TS `isNode` type guard. In a typed Rust call site you
/// usually already know whether you hold a `Node<_>` or an `Edge<_>`,
/// but this helper still has value when iterating over an
/// `Element<N, E>` slice (e.g. user-supplied "elements" arrays).
///
/// # Examples
///
/// ```rust,ignore
/// use rgraph::utils::general::{is_node, Element};
/// use rgraph_core::Node;
///
/// let e: Element = Node::<()>::minimal("n1", 0.0, 0.0).into();
/// assert!(is_node(&e));
/// ```
#[inline]
#[must_use]
pub fn is_node<N: Clone, E: Clone>(element: &Element<N, E>) -> bool {
    element.is_node()
}

/// Test whether an [`Element`] is usable as an [`Edge`].
///
/// Mirrors the TS `isEdge` type guard. See [`is_node`] for notes about
/// the Rust analogue of TS's type guards.
#[inline]
#[must_use]
pub fn is_edge<N: Clone, E: Clone>(element: &Element<N, E>) -> bool {
    element.is_edge()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgraph_core::Edge;
    use rgraph_core::Node;

    #[test]
    fn element_node_discriminator() {
        let e: Element = Node::<()>::minimal("n1", 0.0, 0.0).into();
        assert!(is_node(&e));
        assert!(!is_edge(&e));
        assert!(e.as_node().is_some());
        assert!(e.as_edge().is_none());
    }

    #[test]
    fn element_edge_discriminator() {
        let e: Element = Edge::<()>::minimal("e1", "a", "b").into();
        assert!(!is_node(&e));
        assert!(is_edge(&e));
        assert!(e.as_node().is_none());
        assert!(e.as_edge().is_some());
    }
}
