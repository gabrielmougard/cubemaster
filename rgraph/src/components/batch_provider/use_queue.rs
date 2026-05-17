//! Port of `xyflow-react/src/components/BatchProvider/useQueue.ts`.
//!
//! Status: Phase 6 — implemented.
//!
//! Tiny queue + per-frame flush utility used by
//! [`crate::components::batch_provider::BatchProvider`].
//!
//! The TS source uses `requestAnimationFrame` to coalesce many
//! `setNodes`/`setEdges` calls into one store write per frame. In
//! Dioxus we flush on every render of the parent provider — Dioxus
//! already coalesces synchronous state updates within a render pass,
//! so the timing matches and we don't need a separate RAF tick.

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

/// Reference-counted FIFO queue for batched updates. `Rc<RefCell<…>>`
/// is the right choice because:
///   * The provider holds one persistent handle (cloned into context).
///   * Each consumer (hook) holds another handle to push items.
///   * Reads/writes happen single-threaded inside the Dioxus runtime,
///     so `RefCell` is sufficient.
pub struct Queue<T> {
    items: Rc<RefCell<Vec<T>>>,
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Queue {
            items: Rc::clone(&self.items),
        }
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Queue::new()
    }
}

impl<T> Queue<T> {
    /// Build an empty queue.
    #[must_use]
    pub fn new() -> Self {
        Queue {
            items: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Push an item onto the end of the queue.
    pub fn push(&self, item: T) {
        self.items.borrow_mut().push(item);
    }

    /// Drain every queued item and return it as a `Vec`. The queue is
    /// empty after this call.
    pub fn drain(&self) -> Vec<T> {
        std::mem::take(&mut *self.items.borrow_mut())
    }

    /// Returns `true` when the queue contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_drain_round_trip() {
        let q: Queue<i32> = Queue::new();
        assert!(q.is_empty());
        q.push(1);
        q.push(2);
        q.push(3);
        let items = q.drain();
        assert_eq!(items, vec![1, 2, 3]);
        assert!(q.is_empty());
    }

    #[test]
    fn cloned_queues_share_state() {
        let q1: Queue<i32> = Queue::new();
        let q2 = q1.clone();
        q1.push(10);
        q2.push(20);
        assert_eq!(q1.drain(), vec![10, 20]);
    }
}
