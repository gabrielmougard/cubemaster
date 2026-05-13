//! Rust port of [d3-dispatch](https://github.com/d3/d3-dispatch).
//!
//! A lightweight event dispatcher that maps named event types to ordered lists
//! of callbacks. Callbacks are identified by an optional dot-namespaced tag so
//! multiple subscribers to the same type can coexist and be removed
//! individually.
//!
//! # Design notes
//!
//! * Generic over the argument type `A`. Callbacks receive `&A`. Use a tuple
//!   for multiple arguments, e.g. `Dispatch<(Context, f32)>`.
//! * Uses [`Rc`] for shared callback ownership and an internal [`RefCell`] so
//!   that `call` can safely release its borrow before invoking callbacks —
//!   allowing callbacks to register or remove other callbacks on the same
//!   dispatcher.
//! * `on` takes `&self` (not `&mut self`); mutation goes through the internal
//!   [`RefCell`]. This lets callbacks hold an `Rc<Dispatch<A>>` and call `on`
//!   from within a dispatch.
//! * Faithful to d3-dispatch's "noop poke" semantics: removing or replacing
//!   a callback during dispatch (a) prevents the *removed* callback from
//!   firing later in the current cycle, and (b) prevents the *new*
//!   replacement callback from firing in the current cycle.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Reserved names (mirrors JS prototype-chain guard)
// ---------------------------------------------------------------------------

const RESERVED: &[&str] = &[
    "__proto__",
    "hasOwnProperty",
    "constructor",
    "toString",
    "valueOf",
];

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Validated decomposition of a "type.name" or "type" or ".name" token.
struct Typename {
    /// The event-type part (empty for bare `.name` tokens).
    type_: String,
    /// The tag part after the dot (empty when no dot is present).
    name: String,
}

fn validate_type_name(name: &str) {
    if name.is_empty() {
        panic!("illegal type: {name}");
    }

    for ch in name.chars() {
        if ch.is_whitespace() || ch == '.' {
            panic!("illegal type: {name}");
        }
    }

    for &reserved in RESERVED {
        if reserved == name {
            panic!("illegal type: {name}");
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Type alias for the boxed callback held inside the dispatcher.
pub type Callback<A> = Rc<dyn Fn(&A) + 'static>;

/// One named subscription slot. The value is held in a [`RefCell`] so we can
/// "poke" it to `None` when the slot is removed or replaced — mirroring the
/// JS implementation that overwrites the entry with `noop` before slicing it
/// out of the array. This is what allows in-flight dispatches to skip
/// callbacks that were removed or replaced *during* the same dispatch cycle.
struct Entry<A> {
    name: String,
    value: RefCell<Option<Callback<A>>>,
}

/// A per-event-type ordered subscription list. Stored behind an [`Rc`] so that
/// `call` can clone a cheap pointer to the *current* list and iterate it even
/// if `on()` swaps in a new list mid-dispatch.
type EntryList<A> = Rc<Vec<Rc<Entry<A>>>>;

/// A dynamic event dispatcher for named typed callbacks.
///
/// `A` is the argument type passed to every callback on dispatch. For multiple
/// arguments, use a tuple, e.g. `Dispatch<(i32, &'static str)>`.
///
/// # Examples
///
/// ```
/// use rgraph_dispatch::Dispatch;
/// use std::rc::Rc;
/// use std::cell::Cell;
///
/// let d = Dispatch::new(&["foo", "bar"]);
/// let count = Rc::new(Cell::new(0u32));
/// let c = count.clone();
/// d.on("foo", Some(Rc::new(move |_: &()| { c.set(c.get() + 1); })));
/// d.call("foo", &());
/// assert_eq!(count.get(), 1);
/// ```
pub struct Dispatch<A = ()> {
    /// Maps each registered type to its current ordered list of entries.
    ///
    /// The outer [`RefCell`] permits `on()` to take `&self`. The inner [`Rc`]
    /// makes "swap in a fresh list while old iterations keep walking the
    /// previous one" both cheap and safe.
    types: RefCell<HashMap<String, EntryList<A>>>,
    /// Tracks whether we are inside `call`/`apply`. Currently informational;
    /// reserved for future diagnostics.
    in_dispatch: Cell<u32>,
}

impl<A> Dispatch<A> {
    /// Creates a new dispatcher with the given event type names.
    ///
    /// # Panics
    ///
    /// Panics if any name is empty, contains whitespace or a dot, duplicates
    /// another name, or matches a reserved identifier.
    pub fn new(type_names: &[&str]) -> Self {
        let mut map = HashMap::with_capacity(type_names.len());
        for &name in type_names {
            validate_type_name(name);
            if map.contains_key(name) {
                panic!("illegal type: {name}");
            }

            map.insert(name.to_owned(), Rc::new(Vec::<Rc<Entry<A>>>::new()));
        }

        Dispatch {
            types: RefCell::new(map),
            in_dispatch: Cell::new(0),
        }
    }

    /// Parses a space-separated list of `type`, `type.name`, or `.name` tokens.
    /// Panics on an unknown non-empty type.
    fn parse_typenames(&self, typenames: &str) -> Vec<Typename> {
        let types = self.types.borrow();
        let mut result = Vec::new();
        for part in typenames.split_whitespace() {
            let (type_, name) = match part.find('.') {
                Some(i) => (part[..i].to_owned(), part[i + 1..].to_owned()),
                None => (part.to_owned(), String::new()),
            };
            if !type_.is_empty() && !types.contains_key(&type_) {
                panic!("unknown type: {type_}");
            }

            result.push(Typename { type_, name });
        }
        result
    }

    /// Build a new list that is `current` with any entry whose name equals
    /// `name` removed (and the existing entry's value poked to `None` so that
    /// any in-flight dispatch skips it). If `replacement` is `Some`, append
    /// it as a fresh `Entry`.
    fn rebuild_list(
        current: &EntryList<A>,
        name: &str,
        replacement: Option<&Callback<A>>,
    ) -> EntryList<A> {
        let mut out: Vec<Rc<Entry<A>>> = Vec::with_capacity(current.len() + 1);
        for slot in current.iter() {
            if slot.name == name {
                // Poke the *existing* entry to None so any concurrent
                // iteration over `current` skips it.
                *slot.value.borrow_mut() = None;
                continue;
            }
            out.push(Rc::clone(slot));
        }

        if let Some(cb) = replacement {
            out.push(Rc::new(Entry {
                name: name.to_owned(),
                value: RefCell::new(Some(Rc::clone(cb))),
            }));
        }

        Rc::new(out)
    }

    /// Registers, removes, or replaces the callback for the given typename(s).
    ///
    /// `typename` may be a space-separated list of tokens. Each token can be:
    ///
    /// * `"type"` — matches the unnamed (empty-tag) slot for that event type.
    /// * `"type.name"` — matches the named slot.
    /// * `".name"` — when `callback` is `None`, removes every slot with that
    ///   tag across **all** registered types. When `callback` is `Some`, the
    ///   entry is silently ignored (no type to store against).
    ///
    /// Setting a callback that already has the same tag removes the old entry
    /// and appends the new one (moving it to the end of the invocation order).
    ///
    /// Returns `&Self` for chaining.
    ///
    /// # Panics
    ///
    /// Panics if a non-empty type name is unknown.
    pub fn on(&self, typename: &str, callback: Option<Callback<A>>) -> &Self {
        let parsed = self.parse_typenames(typename);
        let mut types = self.types.borrow_mut();
        for tn in &parsed {
            if tn.type_.is_empty() {
                // ".name" token — only acts on remove
                if callback.is_none() {
                    let keys: Vec<String> = types.keys().cloned().collect();
                    for k in keys {
                        let cur = Rc::clone(types.get(&k).unwrap());
                        let new_list = Self::rebuild_list(&cur, &tn.name, None);
                        types.insert(k, new_list);
                    }
                }
            } else {
                let cur = Rc::clone(types.get(&tn.type_).unwrap());
                let new_list = Self::rebuild_list(&cur, &tn.name, callback.as_ref());
                types.insert(tn.type_.clone(), new_list);
            }
        }

        self
    }

    /// Returns the first registered callback matching the given typename(s),
    /// or `None` if no match is found.
    ///
    /// When `typename` is space-separated, the first matching token wins.
    /// Bare `.name` tokens always return `None`.
    ///
    /// # Panics
    ///
    /// Panics if a non-empty type name is unknown.
    pub fn callback(&self, typename: &str) -> Option<Callback<A>> {
        let parsed = self.parse_typenames(typename);
        let types = self.types.borrow();
        for tn in &parsed {
            if !tn.type_.is_empty() && let Some(list) = types.get(&tn.type_) {
                for entry in list.iter() {
                    if entry.name == tn.name && let Some(ref cb) = *entry.value.borrow() {
                        return Some(Rc::clone(cb));
                    }
                }
            }
        }

        None
    }

    /// Invokes every callback registered for `type_` in registration order,
    /// passing `args` to each one.
    ///
    /// During iteration, modifications to the per-type list (via `on` from
    /// within a callback) install a *new* list in storage; the *current*
    /// dispatch keeps walking the old list. Entries that were removed or
    /// replaced are poked to `None` in that old list so they are skipped.
    ///
    /// # Panics
    ///
    /// Panics if `type_` is not a registered event type.
    pub fn call(&self, type_: &str, args: &A) {
        // Cheap clone of the Rc — does not deep-copy entries.
        let list: EntryList<A> = {
            let types = self.types.borrow();
            Rc::clone(
                types
                    .get(type_)
                    .unwrap_or_else(|| panic!("unknown type: {type_}")),
            )
        };
        self.in_dispatch.set(self.in_dispatch.get() + 1);
        // Iterate by index over the snapshot list. Entry values may be
        // mutated to None by concurrent `on()` calls.
        for entry in list.iter() {
            // Clone out the inner Rc so the borrow on the entry's value is
            // released before we call into user code.
            let cb_opt: Option<Callback<A>> = entry.value.borrow().clone();
            if let Some(cb) = cb_opt {
                cb(args);
            }
        }

        self.in_dispatch.set(self.in_dispatch.get() - 1);
    }

    /// Equivalent to [`call`](Self::call).
    ///
    /// In JavaScript d3-dispatch, `apply` accepts an explicit args array while
    /// `call` uses variadic arguments. In Rust both collapse to the same
    /// signature.
    pub fn apply(&self, type_: &str, args: &A) {
        self.call(type_, args);
    }

    /// Returns an isolated copy of this dispatcher.
    ///
    /// The copy shares the same [`Rc`]-wrapped callbacks but owns independent
    /// per-type lists. Mutations on the copy (via `on`) do not affect the
    /// original, and vice-versa.
    ///
    /// Important: the per-entry "noop poke" mechanism is per-list, so the
    /// copy must allocate fresh `Entry` slots that point at the same callback
    /// pointers — otherwise removing a slot in the copy would also poke the
    /// original's entry to `None`.
    pub fn copy(&self) -> Self {
        let src = self.types.borrow();
        let mut dst = HashMap::with_capacity(src.len());
        for (k, list) in src.iter() {
            let mut new_vec: Vec<Rc<Entry<A>>> = Vec::with_capacity(list.len());
            for entry in list.iter() {
                let v = entry.value.borrow().clone();
                new_vec.push(Rc::new(Entry {
                    name: entry.name.clone(),
                    value: RefCell::new(v),
                }));
            }

            dst.insert(k.clone(), Rc::new(new_vec));
        }

        Dispatch {
            types: RefCell::new(dst),
            in_dispatch: Cell::new(0),
        }
    }
}

impl<A> Clone for Dispatch<A> {
    fn clone(&self) -> Self {
        self.copy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type NoArgs = ();

    /// Convenience: wrap a plain closure into the callback type.
    fn wrap<F: Fn(&NoArgs) + 'static>(f: F) -> Rc<dyn Fn(&NoArgs) + 'static> {
        Rc::new(f)
    }

    #[test]
    fn dispatch_returns_dispatch_with_specified_types() {
        let d = Dispatch::<NoArgs>::new(&["foo", "bar"]);
        let t = d.types.borrow();
        assert!(t.contains_key("foo"));
        assert!(t.contains_key("bar"));
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn dispatch_allows_type_name_colliding_with_dispatch_method() {
        let d = Dispatch::<NoArgs>::new(&["on"]);
        assert!(d.types.borrow().contains_key("on"));
    }

    #[test]
    #[should_panic(expected = "illegal type: __proto__")]
    fn dispatch_rejects_proto() { Dispatch::<NoArgs>::new(&["__proto__"]); }

    #[test]
    #[should_panic(expected = "illegal type: hasOwnProperty")]
    fn dispatch_rejects_has_own_property() { Dispatch::<NoArgs>::new(&["hasOwnProperty"]); }

    #[test]
    #[should_panic(expected = "illegal type:")]
    fn dispatch_rejects_empty() { Dispatch::<NoArgs>::new(&[""]); }

    #[test]
    #[should_panic(expected = "illegal type: foo.bar")]
    fn dispatch_rejects_dotted() { Dispatch::<NoArgs>::new(&["foo.bar"]); }

    #[test]
    #[should_panic(expected = "illegal type: foo bar")]
    fn dispatch_rejects_space() { Dispatch::<NoArgs>::new(&["foo bar"]); }

    #[test]
    #[should_panic(expected = "illegal type: foo\tbar")]
    fn dispatch_rejects_tab() { Dispatch::<NoArgs>::new(&["foo\tbar"]); }

    #[test]
    #[should_panic(expected = "illegal type: foo")]
    fn dispatch_rejects_duplicate() { Dispatch::<NoArgs>::new(&["foo", "foo"]); }

    #[test]
    fn call_invokes_correct_type() {
        let foo = Rc::new(RefCell::new(0u32));
        let bar = Rc::new(RefCell::new(0u32));
        let (f, b) = (foo.clone(), bar.clone());
        let d = Dispatch::new(&["foo", "bar"]);
        d.on("foo", Some(wrap(move |_| { *f.borrow_mut() += 1; })));
        d.on("bar", Some(wrap(move |_| { *b.borrow_mut() += 1; })));
        d.call("foo", &());
        assert_eq!(*foo.borrow(), 1);
        assert_eq!(*bar.borrow(), 0);
        d.call("foo", &());
        d.call("bar", &());
        assert_eq!(*foo.borrow(), 2);
        assert_eq!(*bar.borrow(), 1);
    }

    #[test]
    fn call_passes_arguments() {
        let results: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
        let r = results.clone();
        let d: Dispatch<(String, String)> = Dispatch::new(&["foo"]);
        d.on("foo", Some(Rc::new(move |(ctx, arg): &(String, String)| {
            r.borrow_mut().push((ctx.clone(), arg.clone()));
        })));
        d.call("foo", &("ctx1".into(), "arg1".into()));
        d.call("foo", &("ctx2".into(), "arg2".into()));
        let r = results.borrow();
        assert_eq!(r[0], ("ctx1".into(), "arg1".into()));
        assert_eq!(r[1], ("ctx2".into(), "arg2".into()));
    }

    #[test]
    fn call_invokes_in_order() {
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let d = Dispatch::new(&["foo"]);
        let (l1, l2, l3, l4) = (log.clone(), log.clone(), log.clone(), log.clone());
        d.on("foo.a", Some(wrap(move |_| { l1.borrow_mut().push("A"); })));
        d.on("foo.b", Some(wrap(move |_| { l2.borrow_mut().push("B"); })));
        d.call("foo", &());                                                   // [A B]
        d.on("foo.c", Some(wrap(move |_| { l3.borrow_mut().push("C"); })));
        d.on("foo.a", Some(wrap(move |_| { l4.borrow_mut().push("A"); }))); // move .a to end
        d.call("foo", &());                                                   // [B C A]
        assert_eq!(&*log.borrow(), &["A", "B", "B", "C", "A"]);
    }

    #[test]
    fn call_returns_unit() {
        let d = Dispatch::<NoArgs>::new(&["foo"]);
        let () = d.call("foo", &());
    }

    #[test]
    fn apply_invokes_correct_type() {
        let foo = Rc::new(RefCell::new(0u32));
        let bar = Rc::new(RefCell::new(0u32));
        let (f, b) = (foo.clone(), bar.clone());
        let d = Dispatch::new(&["foo", "bar"]);
        d.on("foo", Some(wrap(move |_| { *f.borrow_mut() += 1; })));
        d.on("bar", Some(wrap(move |_| { *b.borrow_mut() += 1; })));
        d.apply("foo", &());
        assert_eq!(*foo.borrow(), 1);
        assert_eq!(*bar.borrow(), 0);
        d.apply("foo", &());
        d.apply("bar", &());
        assert_eq!(*foo.borrow(), 2);
        assert_eq!(*bar.borrow(), 1);
    }

    #[test]
    fn apply_passes_arguments() {
        let results: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
        let r = results.clone();
        let d: Dispatch<(String, String)> = Dispatch::new(&["foo"]);
        d.on("foo", Some(Rc::new(move |(ctx, arg): &(String, String)| {
            r.borrow_mut().push((ctx.clone(), arg.clone()));
        })));
        d.apply("foo", &("ctx1".into(), "arg1".into()));
        d.apply("foo", &("ctx2".into(), "arg2".into()));
        let r = results.borrow();
        assert_eq!(r[0], ("ctx1".into(), "arg1".into()));
        assert_eq!(r[1], ("ctx2".into(), "arg2".into()));
    }

    #[test]
    fn apply_invokes_in_order() {
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let d = Dispatch::new(&["foo"]);
        let (l1, l2, l3, l4) = (log.clone(), log.clone(), log.clone(), log.clone());
        d.on("foo.a", Some(wrap(move |_| { l1.borrow_mut().push("A"); })));
        d.on("foo.b", Some(wrap(move |_| { l2.borrow_mut().push("B"); })));
        d.apply("foo", &());
        d.on("foo.c", Some(wrap(move |_| { l3.borrow_mut().push("C"); })));
        d.on("foo.a", Some(wrap(move |_| { l4.borrow_mut().push("A"); })));
        d.apply("foo", &());
        assert_eq!(&*log.borrow(), &["A", "B", "B", "C", "A"]);
    }

    #[test]
    fn on_returns_self() {
        let d = Dispatch::<NoArgs>::new(&["foo"]);
        let r = d.on("foo", Some(wrap(|_| {})));
        assert!(std::ptr::eq(r, &d));
    }

    #[test]
    fn on_replaces_existing_callback() {
        let foo = Rc::new(RefCell::new(0u32));
        let bar = Rc::new(RefCell::new(0u32));
        let (f, b) = (foo.clone(), bar.clone());
        let d = Dispatch::new(&["foo"]);
        d.on("foo", Some(wrap(move |_| { *f.borrow_mut() += 1; })));
        d.call("foo", &());
        assert_eq!(*foo.borrow(), 1);
        d.on("foo", Some(wrap(move |_| { *b.borrow_mut() += 1; })));
        d.call("foo", &());
        assert_eq!(*foo.borrow(), 1);
        assert_eq!(*bar.borrow(), 1);
    }

    #[test]
    fn on_replacing_with_same_callback_has_no_effect() {
        let count = Rc::new(RefCell::new(0u32));
        let c = count.clone();
        let cb = wrap(move |_| { *c.borrow_mut() += 1; });
        let d = Dispatch::new(&["foo"]);
        d.on("foo", Some(Rc::clone(&cb)));
        d.call("foo", &());
        assert_eq!(*count.borrow(), 1);
        // Re-registering the same tag three times: list should still have 1 entry
        d.on("foo", Some(Rc::clone(&cb)))
         .on("foo", Some(Rc::clone(&cb)))
         .on("foo", Some(Rc::clone(&cb)));
        d.call("foo", &());
        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn on_trailing_dot_is_equivalent_to_bare_type() {
        let foos = Rc::new(RefCell::new(0u32));
        let bars = Rc::new(RefCell::new(0u32));
        let (f, b) = (foos.clone(), bars.clone());
        let foo_cb = wrap(move |_| { *f.borrow_mut() += 1; });
        let bar_cb = wrap(move |_| { *b.borrow_mut() += 1; });
        let d = Dispatch::new(&["foo"]);

        d.on("foo.", Some(Rc::clone(&foo_cb)));
        assert!(Rc::ptr_eq(&d.callback("foo.").unwrap(), &foo_cb));
        assert!(Rc::ptr_eq(&d.callback("foo").unwrap(),  &foo_cb));

        d.on("foo.", Some(Rc::clone(&bar_cb)));
        assert!(Rc::ptr_eq(&d.callback("foo.").unwrap(), &bar_cb));
        assert!(Rc::ptr_eq(&d.callback("foo").unwrap(),  &bar_cb));

        d.call("foo", &());
        assert_eq!(*foos.borrow(), 0);
        assert_eq!(*bars.borrow(), 1);

        // Remove by ".name" (empty name == "")
        d.on(".", None);
        assert!(d.callback("foo").is_none());
        d.call("foo", &());
        assert_eq!(*bars.borrow(), 1); // not incremented again
    }

    #[test]
    fn on_null_removes_callback() {
        let count = Rc::new(RefCell::new(0u32));
        let c = count.clone();
        let d = Dispatch::new(&["foo", "bar"]);
        d.on("foo", Some(wrap(move |_| { *c.borrow_mut() += 1; })));
        d.call("foo", &());
        assert_eq!(*count.borrow(), 1);
        d.on("foo", None);
        d.call("foo", &());
        assert_eq!(*count.borrow(), 1);
    }

    #[test]
    fn on_null_does_not_remove_shared_callback() {
        let count = Rc::new(RefCell::new(0u32));
        let c = count.clone();
        let cb = wrap(move |_| { *c.borrow_mut() += 1; });
        let d = Dispatch::new(&["foo", "bar"]);
        d.on("foo", Some(Rc::clone(&cb)));
        d.on("bar", Some(Rc::clone(&cb)));
        d.call("foo", &());
        d.call("bar", &());
        assert_eq!(*count.borrow(), 2);
        d.on("foo", None);
        d.call("bar", &());
        assert_eq!(*count.borrow(), 3);
    }

    #[test]
    fn on_null_removing_missing_callback_has_no_effect() {
        let count = Rc::new(RefCell::new(0u32));
        let c = count.clone();
        let cb = wrap(move |_| { *c.borrow_mut() += 1; });
        let d = Dispatch::new(&["foo"]);
        d.on("foo.a", None)
         .on("foo", Some(Rc::clone(&cb)))
         .on("foo", None)
         .on("foo", None);
        d.call("foo", &());
        assert_eq!(*count.borrow(), 0);
    }

    /// A callback removes another callback before it runs; the removed
    /// callback must not be invoked in the same dispatch cycle.
    #[test]
    fn on_null_during_callback_does_not_invoke_removed() {
        let a = Rc::new(RefCell::new(0u32));
        let b = Rc::new(RefCell::new(0u32));
        let (a1, b1) = (a.clone(), b.clone());

        let d = Rc::new(Dispatch::new(&["foo"]));

        let dd = d.clone();
        d.on("foo.A", Some(wrap(move |_| {
            *a1.borrow_mut() += 1;
            dd.on("foo.B", None); // remove B before it executes
        })));
        d.on("foo.B", Some(wrap(move |_| { *b1.borrow_mut() += 1; })));

        d.call("foo", &());
        assert_eq!(*a.borrow(), 1);
        assert_eq!(*b.borrow(), 0);
    }

    /// A callback replaces another callback; neither the old nor new version
    /// should fire in the same dispatch cycle.
    #[test]
    fn on_replace_during_callback_does_not_invoke_old_or_new() {
        let a = Rc::new(RefCell::new(0u32));
        let b = Rc::new(RefCell::new(0u32));
        let c = Rc::new(RefCell::new(0u32));
        let (a1, b1, c1) = (a.clone(), b.clone(), c.clone());

        let d = Rc::new(Dispatch::new(&["foo"]));

        // A replaces B with C
        let dd = d.clone();
        d.on("foo.A", Some(wrap(move |_| {
            *a1.borrow_mut() += 1;
            let c2 = c1.clone();
            dd.on("foo.B", Some(wrap(move |_| { *c2.borrow_mut() += 1; })));
        })));
        d.on("foo.B", Some(wrap(move |_| { *b1.borrow_mut() += 1; })));

        d.call("foo", &());
        assert_eq!(*a.borrow(), 1);
        assert_eq!(*b.borrow(), 0); // old B not called
        assert_eq!(*c.borrow(), 0); // new C not called either
    }

    /// A callback adds a new callback; the new one must not fire in the same
    /// dispatch cycle.
    #[test]
    fn on_add_during_callback_does_not_invoke_new() {
        let a = Rc::new(RefCell::new(0u32));
        let b = Rc::new(RefCell::new(0u32));
        let (a1, b1) = (a.clone(), b.clone());

        let d = Rc::new(Dispatch::new(&["foo"]));

        // A adds B
        let dd = d.clone();
        d.on("foo.A", Some(wrap(move |_| {
            *a1.borrow_mut() += 1;
            let b2 = b1.clone();
            dd.on("foo.B", Some(wrap(move |_| { *b2.borrow_mut() += 1; })));
        })));

        d.call("foo", &());
        assert_eq!(*a.borrow(), 1);
        assert_eq!(*b.borrow(), 0);
    }

    #[test]
    fn on_space_separated_types_adds_for_both() {
        let count = Rc::new(RefCell::new(0u32));
        let c = count.clone();
        let cb = wrap(move |_| { *c.borrow_mut() += 1; });
        let d = Dispatch::new(&["foo", "bar"]);
        d.on("foo bar", Some(Rc::clone(&cb)));
        assert!(Rc::ptr_eq(&d.callback("foo").unwrap(), &cb));
        assert!(Rc::ptr_eq(&d.callback("bar").unwrap(), &cb));
        d.call("foo", &());
        assert_eq!(*count.borrow(), 1);
        d.call("bar", &());
        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn on_space_separated_typenames_adds_for_both() {
        let count = Rc::new(RefCell::new(0u32));
        let c = count.clone();
        let cb = wrap(move |_| { *c.borrow_mut() += 1; });
        let d = Dispatch::new(&["foo"]);
        d.on("foo.one foo.two", Some(Rc::clone(&cb)));
        assert!(Rc::ptr_eq(&d.callback("foo.one").unwrap(), &cb));
        assert!(Rc::ptr_eq(&d.callback("foo.two").unwrap(), &cb));
        d.call("foo", &());
        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn callback_space_separated_returns_first_match() {
        let foo_cb = wrap(|_| {});
        let bar_cb = wrap(|_| {});
        let d = Dispatch::new(&["foo", "bar"]);

        d.on("foo", Some(Rc::clone(&foo_cb)));
        assert!(Rc::ptr_eq(&d.callback("foo bar").unwrap(), &foo_cb));
        assert!(Rc::ptr_eq(&d.callback("bar foo").unwrap(), &foo_cb));

        d.on("foo", None);
        d.on("bar", Some(Rc::clone(&bar_cb)));
        assert!(Rc::ptr_eq(&d.callback("foo bar").unwrap(), &bar_cb));
        assert!(Rc::ptr_eq(&d.callback("bar foo").unwrap(), &bar_cb));
    }

    #[test]
    fn callback_typename_space_separated_returns_first_match() {
        let foo_cb = wrap(|_| {});
        let bar_cb = wrap(|_| {});
        let d = Dispatch::new(&["foo"]);

        d.on("foo.one", Some(Rc::clone(&foo_cb)));
        assert!(Rc::ptr_eq(&d.callback("foo.one foo.two").unwrap(), &foo_cb));
        assert!(Rc::ptr_eq(&d.callback("foo.two foo.one").unwrap(), &foo_cb));
        assert!(Rc::ptr_eq(&d.callback("foo foo.one").unwrap(),     &foo_cb));
        assert!(Rc::ptr_eq(&d.callback("foo.one foo").unwrap(),     &foo_cb));

        d.on("foo.one", None);
        d.on("foo.two", Some(Rc::clone(&bar_cb)));
        assert!(Rc::ptr_eq(&d.callback("foo.one foo.two").unwrap(), &bar_cb));
        assert!(Rc::ptr_eq(&d.callback("foo.two foo.one").unwrap(), &bar_cb));
        assert!(Rc::ptr_eq(&d.callback("foo foo.two").unwrap(),     &bar_cb));
        assert!(Rc::ptr_eq(&d.callback("foo.two foo").unwrap(),     &bar_cb));
    }

    #[test]
    fn callback_named_slots_are_independent() {
        let d = Dispatch::new(&["foo"]);
        let (ca, cb, cc) = (wrap(|_| {}), wrap(|_| {}), wrap(|_| {}));
        d.on("foo.a", Some(Rc::clone(&ca)));
        d.on("foo.b", Some(Rc::clone(&cb)));
        d.on("foo",   Some(Rc::clone(&cc)));
        assert!(d.callback("foo.a").is_some());
        assert!(d.callback("foo.b").is_some());
        assert!(d.callback("foo").is_some());
        assert!(!Rc::ptr_eq(&d.callback("foo.a").unwrap(), &d.callback("foo.b").unwrap()));
        assert!(!Rc::ptr_eq(&d.callback("foo.a").unwrap(), &d.callback("foo").unwrap()));
    }

    #[test]
    fn callback_bare_dot_name_returns_none() {
        let d = Dispatch::new(&["foo"]);
        d.on("foo.a", Some(wrap(|_| {})));
        assert!(d.callback(".a").is_none());
    }

    #[test]
    fn on_dot_name_null_removes_from_all_types() {
        let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let d = Dispatch::new(&["foo", "bar"]);
        let (r1, r2, r3) = (log.clone(), log.clone(), log.clone());
        d.on("foo.a", Some(wrap(move |_| { r1.borrow_mut().push(1); })));
        d.on("bar.a", Some(wrap(move |_| { r2.borrow_mut().push(2); })));
        d.on("foo",   Some(wrap(move |_| { r3.borrow_mut().push(3); })));
        d.on(".a", None);
        d.call("foo", &());
        d.call("bar", &());
        assert_eq!(&*log.borrow(), &[3u32]);
    }

    #[test]
    fn on_dot_name_null_removes_named_across_types_via_space_sep() {
        let d = Dispatch::new(&["foo"]);
        let cb = wrap(|_| {});
        d.on("foo.one", Some(Rc::clone(&cb)));
        d.on("foo.two", Some(Rc::clone(&cb)));
        d.on("foo.one foo.two", None);
        assert!(d.callback("foo.one").is_none());
        assert!(d.callback("foo.two").is_none());
    }

    #[test]
    fn on_dot_name_set_has_no_effect() {
        let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let d = Dispatch::new(&["foo", "bar"]);
        let (r0, r1, r2) = (log.clone(), log.clone(), log.clone());
        d.on(".a",    Some(wrap(move |_| { r0.borrow_mut().push(0); }))); // ignored
        d.on("foo.a", Some(wrap(move |_| { r1.borrow_mut().push(1); })));
        d.on("bar",   Some(wrap(move |_| { r2.borrow_mut().push(2); })));
        d.call("foo", &());
        d.call("bar", &());
        assert_eq!(&*log.borrow(), &[1u32, 2]);
        assert!(d.callback(".a").is_none());
    }

    #[test]
    fn copy_returns_isolated_copy() {
        let foo_cb = wrap(|_| {});
        let bar_cb = wrap(|_| {});
        let d0 = Dispatch::new(&["foo", "bar"]);
        d0.on("foo", Some(Rc::clone(&foo_cb)));
        d0.on("bar", Some(Rc::clone(&bar_cb)));

        let d1 = d0.copy();
        assert!(Rc::ptr_eq(&d1.callback("foo").unwrap(), &foo_cb));
        assert!(Rc::ptr_eq(&d1.callback("bar").unwrap(), &bar_cb));

        // Changes to d1 don't affect d0
        d1.on("bar", None);
        assert!(d1.callback("bar").is_none());
        assert!(Rc::ptr_eq(&d0.callback("bar").unwrap(), &bar_cb));

        // Changes to d0 don't affect d1
        d0.on("foo", None);
        assert!(d0.callback("foo").is_none());
        assert!(Rc::ptr_eq(&d1.callback("foo").unwrap(), &foo_cb));
    }

    #[test]
    #[should_panic(expected = "unknown type: bar")]
    fn on_unknown_type_panics() {
        Dispatch::<NoArgs>::new(&["foo"]).on("bar", Some(wrap(|_| {})));
    }

    #[test]
    #[should_panic(expected = "unknown type: bar")]
    fn callback_unknown_type_panics() {
        Dispatch::<NoArgs>::new(&["foo"]).callback("bar");
    }
}
