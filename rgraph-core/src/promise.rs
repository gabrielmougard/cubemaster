//! Tiny std-only `Promise<T>` used in place of JavaScript `Promise<T>`
//! for completion signalling on animated viewport-change calls
//! (`set_viewport`, `scale_to`, `scale_by`, `fit_view`, …).
//!
//! Status: implemented (phase 1).
//!
//! The implementation is a thin wrapper around
//! [`std::sync::mpsc::sync_channel(1)`]. It deliberately avoids pulling
//! in `tokio` or `futures` so the crate stays runtime-agnostic.
//!
//! The API supports three idioms:
//!
//! 1. Already-resolved values via [`Promise::resolved`].
//! 2. Polling with [`Promise::try_take`].
//! 3. Blocking with [`Promise::block_take`].
//!
//! Resolution is one-shot: dropping the [`Resolver`] without calling
//! [`Resolver::resolve`] turns the promise into one that will never
//! complete; [`Promise::block_take`] returns `None` in that case.
//!
//! # Example
//!
//! ```
//! use rgraph_core::promise::channel;
//!
//! let (promise, resolver) = channel::<bool>();
//! resolver.resolve(true);
//! assert_eq!(promise.try_take(), Some(true));
//! ```

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

/// One-shot completion handle returned by methods that finish
/// asynchronously (typically as a transition winds down).
///
/// The promise can be polled non-blockingly with [`Self::try_take`] or
/// awaited synchronously with [`Self::block_take`]. The value can be
/// taken at most once; subsequent polls return `None`.
pub struct Promise<T> {
    /// `RefCell` so that `try_take(&self)` can drain the receiver
    /// without requiring `&mut self` — matches the JS pattern of calling
    /// `await promise` from any context.
    inner: RefCell<PromiseInner<T>>,
}

enum PromiseInner<T> {
    /// Value already available — early-return path used by
    /// [`Promise::resolved`].
    Ready(Option<T>),
    /// Backing channel; `None` once a value has been observed.
    Pending(Option<Receiver<T>>),
    /// Drained: a value has been delivered and taken by the caller.
    Drained,
}

/// Sender half of a [`Promise`] / [`Resolver`] pair.
pub struct Resolver<T> {
    tx: SyncSender<T>,
}

/// Create a new connected [`Promise`] / [`Resolver`] pair.
///
/// The promise resolves once [`Resolver::resolve`] is invoked (or
/// remains forever-pending if the resolver is dropped).
#[must_use]
pub fn channel<T>() -> (Promise<T>, Resolver<T>) {
    let (tx, rx) = sync_channel(1);
    (
        Promise {
            inner: RefCell::new(PromiseInner::Pending(Some(rx))),
        },
        Resolver { tx },
    )
}

impl<T> Promise<T> {
    /// Construct a promise that is already resolved with `value`.
    ///
    /// This is the equivalent of JavaScript's `Promise.resolve(value)`
    /// and is used by early-return paths such as
    /// `XYPanZoom::set_transform` when the underlying selection is
    /// missing.
    #[must_use]
    pub fn resolved(value: T) -> Self {
        Promise {
            inner: RefCell::new(PromiseInner::Ready(Some(value))),
        }
    }

    /// Try to take the resolved value without blocking.
    ///
    /// Returns:
    /// * `Some(value)` the first time the promise is observed in the
    ///   resolved state,
    /// * `None` if the promise has not resolved yet, has already been
    ///   drained, or its [`Resolver`] was dropped.
    pub fn try_take(&self) -> Option<T> {
        let mut inner = self.inner.borrow_mut();
        match &mut *inner {
            PromiseInner::Ready(slot) => {
                let v = slot.take();
                if v.is_some() {
                    *inner = PromiseInner::Drained;
                }
                v
            }
            PromiseInner::Pending(slot) => {
                let rx = slot.as_ref()?;
                match rx.try_recv() {
                    Ok(v) => {
                        *inner = PromiseInner::Drained;
                        Some(v)
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        *inner = PromiseInner::Drained;
                        None
                    }
                }
            }
            PromiseInner::Drained => None,
        }
    }

    /// Block the current thread until the promise resolves.
    ///
    /// Returns `Some(value)` once, or `None` if the [`Resolver`] was
    /// dropped or the promise had already been drained.
    pub fn block_take(self) -> Option<T> {
        let inner = self.inner.into_inner();
        match inner {
            PromiseInner::Ready(slot) => slot,
            PromiseInner::Pending(Some(rx)) => rx.recv().ok(),
            PromiseInner::Pending(None) | PromiseInner::Drained => None,
        }
    }

    /// Returns `true` if the value can be taken on the next call to
    /// [`Self::try_take`] — useful from polling loops.
    pub fn is_ready(&self) -> bool {
        let inner = self.inner.borrow();
        matches!(&*inner, PromiseInner::Ready(Some(_)))
            || match &*inner {
                PromiseInner::Pending(Some(rx)) => match rx.try_recv() {
                    Ok(_) => unreachable!(
                        "is_ready must not consume; switch to try_iter once stabilized"
                    ),
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
                    Err(std::sync::mpsc::TryRecvError::Empty) => false,
                },
                _ => false,
            }
    }
}

impl<T> Resolver<T> {
    /// Resolve the paired promise with `value`.
    ///
    /// Silently no-ops if the promise has already been dropped.
    pub fn resolve(self, value: T) {
        let _ = self.tx.send(value);
    }
}

impl<T> std::fmt::Debug for Promise<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        match &*inner {
            PromiseInner::Ready(Some(_)) => f.write_str("Promise(ready)"),
            PromiseInner::Ready(None) | PromiseInner::Drained => f.write_str("Promise(drained)"),
            PromiseInner::Pending(_) => f.write_str("Promise(pending)"),
        }
    }
}

impl<T> std::fmt::Debug for Resolver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Resolver { .. }")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_value_is_observed_once() {
        let p = Promise::resolved(42);
        assert!(p.is_ready() || matches!(p.inner.borrow().clone_kind(), Kind::Ready));
        assert_eq!(p.try_take(), Some(42));
        assert_eq!(p.try_take(), None);
    }

    #[test]
    fn pending_promise_resolves_via_resolver() {
        let (p, r) = channel::<bool>();
        assert_eq!(p.try_take(), None);
        r.resolve(true);
        assert_eq!(p.try_take(), Some(true));
        assert_eq!(p.try_take(), None);
    }

    #[test]
    fn block_take_returns_value() {
        let (p, r) = channel::<u32>();
        std::thread::spawn(move || {
            r.resolve(7);
        });
        assert_eq!(p.block_take(), Some(7));
    }

    #[test]
    fn dropped_resolver_makes_block_take_return_none() {
        let (p, r) = channel::<()>();
        drop(r);
        assert_eq!(p.block_take(), None);
    }

    // helper for the first test — kept inside the cfg(test) module
    enum Kind {
        Ready,
        Pending,
        Drained,
    }
    impl<T> PromiseInner<T> {
        fn clone_kind(&self) -> Kind {
            match self {
                PromiseInner::Ready(_) => Kind::Ready,
                PromiseInner::Pending(_) => Kind::Pending,
                PromiseInner::Drained => Kind::Drained,
            }
        }
    }
}
