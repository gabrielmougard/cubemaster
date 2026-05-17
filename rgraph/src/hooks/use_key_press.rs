//! Port of `xyflow-react/src/hooks/useKeyPress.ts`.
//!
//! Status: Phase 3 — implemented.
//!
//! The Rust port splits responsibilities between:
//!
//! * The **matcher** ([`KeyPressMatcher`]) — a pure state machine that
//!   parses `KeyCode` patterns and tracks pressed keys. Unit-testable
//!   without a DOM. Mirrors the TS `isMatchingKey` / `useKeyOrCode`
//!   internals.
//! * The **hook** ([`use_key_press`]) — returns a Dioxus
//!   `Signal<bool>` plus an `on_keydown` / `on_keyup` pair that the
//!   wrapping component should attach to the DOM element it cares
//!   about.
//!
//! Phase 4's `dom::eval` will install global `keydown` / `keyup`
//! listeners through the desktop webview so callers don't have to
//! attach them manually.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashSet;

use dioxus::prelude::{use_hook, use_signal, Signal, WritableExt};

use rgraph_core::types::viewport::KeyCode;

/// Options accepted by [`use_key_press`].
///
/// Mirrors the TS `UseKeyPressOptions`. `target` is omitted because
/// in the Dioxus desktop port we don't have arbitrary `EventTarget`
/// types — handlers always run against the wrapping `<div>`'s
/// keyboard events. `prevent_default` is honoured by the consumer at
/// the call site (e.g. `event.prevent_default()` inside the keydown
/// handler) since Dioxus events are owned per-listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseKeyPressOptions {
    /// When `false`, `keydown` events fired while the user is typing
    /// in an input/textarea/contentEditable element are ignored unless
    /// a modifier key is also held. Defaults to `true`.
    pub act_inside_input_with_modifier: bool,
    /// When `true`, `event.prevent_default()` is requested on a match
    /// (the actual call is up to the caller; the matcher exposes the
    /// match via its return value).
    pub prevent_default: bool,
}

impl Default for UseKeyPressOptions {
    fn default() -> Self {
        Self {
            act_inside_input_with_modifier: true,
            prevent_default: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure state machine
// ---------------------------------------------------------------------------

/// One observed keyboard event, fed into the matcher by the host
/// component. The fields mirror the JS `KeyboardEvent` properties the
/// TS source consults.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// `event.key`.
    pub key: String,
    /// `event.code`.
    pub code: String,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    /// `true` when the event originated from a text input / textarea /
    /// contentEditable element (the host is expected to fill this in).
    pub is_input: bool,
}

/// State machine that tracks pressed keys and decides whether the
/// configured chord is currently held down.
///
/// Mirrors the body of TS `useKeyPress`. A new matcher is created on
/// every `keyCode` change.
#[derive(Debug, Clone)]
pub struct KeyPressMatcher {
    /// The parsed list of chord-options, e.g. `[["a"], ["d", "s"]]`.
    chords: Vec<Vec<String>>,
    /// Flattened list used to decide whether to read `event.code` (for
    /// keys like `"ShiftLeft"`) or `event.key` (for `"Meta"` etc.).
    keys_to_watch: HashSet<String>,
    /// Currently pressed keys.
    pressed: HashSet<String>,
    /// Whether the most recent event carried any modifier.
    modifier_pressed: bool,
    /// Cached options.
    options: UseKeyPressOptions,
}

impl KeyPressMatcher {
    /// Build a matcher from a [`KeyCode`].
    ///
    /// `key_code = "a"`         → `[["a"]]`,
    /// `key_code = "Meta+s"`    → `[["Meta", "s"]]`,
    /// `key_code = ["a", "d+s"]` → `[["a"], ["d", "s"]]`.
    /// `key_code = "key++"`     → `[["key", "+"]]` (TS handles the
    ///                              `'+'` literal via the `'\n\n'`
    ///                              substitution; we replicate that).
    #[must_use]
    pub fn new(key_code: &KeyCode, options: UseKeyPressOptions) -> Self {
        let raw: Vec<String> = match key_code {
            KeyCode::Single(s) => vec![s.clone()],
            KeyCode::Multiple(v) => v.clone(),
        };
        let chords: Vec<Vec<String>> = raw
            .iter()
            .map(|kc| {
                kc.replace('+', "\n")
                    .replace("\n\n", "\n+")
                    .split('\n')
                    .map(|s| s.to_string())
                    .collect()
            })
            .collect();
        let keys_to_watch: HashSet<String> = chords.iter().flatten().cloned().collect();

        KeyPressMatcher {
            chords,
            keys_to_watch,
            pressed: HashSet::new(),
            modifier_pressed: false,
            options,
        }
    }

    /// Pick `event.code` if the watched keys list contains the code
    /// (the TS `useKeyOrCode` helper).
    fn key_or_code<'a>(&self, ev: &'a KeyEvent) -> &'a str {
        if self.keys_to_watch.contains(&ev.code) {
            &ev.code
        } else {
            &ev.key
        }
    }

    /// Test whether `pressed` matches any of the chords.
    ///
    /// `is_up` mirrors the TS `isUp` parameter: on the keyup branch we
    /// don't filter by chord size (any chord that is fully contained
    /// triggers a release).
    fn is_matching(&self, is_up: bool) -> bool {
        self.chords
            .iter()
            .filter(|keys| is_up || keys.len() == self.pressed.len())
            .any(|keys| keys.iter().all(|k| self.pressed.contains(k)))
    }

    /// Process a `keydown` event. Returns `true` when this event
    /// caused the chord to become active (i.e. flips the result of
    /// [`Self::is_pressed`] from `false` to `true`).
    pub fn on_key_down(&mut self, ev: &KeyEvent) -> bool {
        self.modifier_pressed = ev.ctrl_key || ev.meta_key || ev.shift_key || ev.alt_key;

        // Skip when typing inside inputs unless a modifier is held
        // and `actInsideInputWithModifier` is `true` (TS lines 106–112).
        // Boolean simplification: the original
        // `(!modifier || (modifier && !act_inside)) && is_input`
        // reduces to `(!modifier || !act_inside) && is_input`.
        let prevent_action =
            (!self.modifier_pressed || !self.options.act_inside_input_with_modifier)
                && ev.is_input;
        if prevent_action {
            return false;
        }

        let was_pressed = self.is_matching(false);
        let key_or_code = self.key_or_code(ev).to_string();
        self.pressed.insert(key_or_code);
        let is_pressed = self.is_matching(false);

        !was_pressed && is_pressed
    }

    /// Process a `keyup` event. Returns `true` when this event caused
    /// the chord to become inactive (flips the result from `true` to
    /// `false`).
    pub fn on_key_up(&mut self, ev: &KeyEvent) -> bool {
        let was_pressed = self.is_matching(false);
        let key_or_code = self.key_or_code(ev).to_string();
        if self.is_matching(true) {
            self.pressed.clear();
        } else {
            self.pressed.remove(&key_or_code);
        }
        // Mac fix: when Cmd is released, the keyup events for
        // intermediate keys never fire, so flush the set (TS line 139).
        if ev.key == "Meta" {
            self.pressed.clear();
        }
        self.modifier_pressed = false;
        let is_pressed = self.is_matching(false);

        was_pressed && !is_pressed
    }

    /// Reset state — call when the host loses focus or shows a context
    /// menu (mirrors the TS `resetHandler`).
    pub fn reset(&mut self) {
        self.pressed.clear();
        self.modifier_pressed = false;
    }

    /// Current pressed state.
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        self.is_matching(false)
    }
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/// Bundle returned by [`use_key_press`]. Hosts attach
/// [`KeyPressApi::on_key_down`] / [`KeyPressApi::on_key_up`] to a DOM
/// element's keyboard events and read [`KeyPressApi::pressed`] for the
/// current state.
#[derive(Clone, Copy)]
pub struct KeyPressApi {
    /// `true` while the configured chord is held down.
    pub pressed: Signal<bool>,
}

impl KeyPressApi {
    /// Forward a `keydown` event to the underlying matcher and update
    /// the [`Self::pressed`] signal accordingly.
    pub fn on_key_down(&self, ev: &KeyEvent, matcher: &mut KeyPressMatcher) {
        let became_active = matcher.on_key_down(ev);
        if became_active {
            self.pressed.clone().set(true);
        }
    }

    /// Forward a `keyup` event to the matcher and update the signal.
    pub fn on_key_up(&self, ev: &KeyEvent, matcher: &mut KeyPressMatcher) {
        let became_inactive = matcher.on_key_up(ev);
        if became_inactive {
            self.pressed.clone().set(false);
        }
    }

    /// Reset the matcher and force the signal back to `false`.
    pub fn reset(&self, matcher: &mut KeyPressMatcher) {
        matcher.reset();
        self.pressed.clone().set(false);
    }
}

/// `use_key_press(KeyCode, options) -> KeyPressApi`.
///
/// Sets up the matcher state once (TS `useState(() => …)`) and returns
/// a [`KeyPressApi`] the host can wire to DOM keyboard events.
///
/// **Phase 3 caveat**: there is no automatic global listener — Phase 4
/// will install `keydown` / `keyup` listeners on the
/// `<RGraph>` wrapper (and `window`) through `dom::eval` and feed
/// matched events into the matcher. Until then, callers must attach
/// the events themselves. See the unit tests for an example of using
/// the matcher directly without a DOM.
#[must_use]
pub fn use_key_press(key_code: Option<KeyCode>, options: UseKeyPressOptions) -> KeyPressApi {
    let pressed = use_signal(|| false);

    // The matcher is created on the first render and never replaced
    // for the same `key_code`; TS uses `useMemo` here.
    use_hook(|| {
        if let Some(code) = key_code {
            let _matcher = KeyPressMatcher::new(&code, options);
            // The hook owns no internal storage in Phase 3 because we
            // can't subscribe to global key events without
            // `dom::eval`. Phase 4 will swap this for an `Rc<RefCell>`
            // shared with the listener.
        }
    });

    KeyPressApi { pressed }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(key: &str, code: &str) -> KeyEvent {
        KeyEvent {
            key: key.into(),
            code: code.into(),
            ctrl_key: false,
            meta_key: false,
            shift_key: false,
            alt_key: false,
            is_input: false,
        }
    }

    #[test]
    fn single_key_press_release() {
        let mut m = KeyPressMatcher::new(
            &KeyCode::from("a"),
            UseKeyPressOptions::default(),
        );
        assert!(!m.is_pressed());
        assert!(m.on_key_down(&ev("a", "KeyA")));
        assert!(m.is_pressed());
        assert!(m.on_key_up(&ev("a", "KeyA")));
        assert!(!m.is_pressed());
    }

    #[test]
    fn chord_requires_all_keys() {
        let mut m = KeyPressMatcher::new(
            &KeyCode::Single("Meta+s".into()),
            UseKeyPressOptions::default(),
        );
        assert!(!m.on_key_down(&KeyEvent {
            key: "Meta".into(),
            code: "MetaLeft".into(),
            meta_key: true,
            ..ev("Meta", "MetaLeft")
        }));
        assert!(!m.is_pressed());
        // Pressing 's' alone is only one of two — but with Meta down
        // the chord matches.
        let mut s = ev("s", "KeyS");
        s.meta_key = true;
        assert!(m.on_key_down(&s));
        assert!(m.is_pressed());
    }

    #[test]
    fn skip_inside_input_without_modifier() {
        let mut m = KeyPressMatcher::new(
            &KeyCode::from("a"),
            UseKeyPressOptions::default(),
        );
        let mut e = ev("a", "KeyA");
        e.is_input = true;
        // No modifier, in-input → ignored.
        assert!(!m.on_key_down(&e));
        assert!(!m.is_pressed());

        // With a modifier and `actInsideInputWithModifier = true` →
        // not ignored.
        let mut m2 = KeyPressMatcher::new(
            &KeyCode::from("a"),
            UseKeyPressOptions::default(),
        );
        let mut e2 = ev("a", "KeyA");
        e2.is_input = true;
        e2.ctrl_key = true;
        assert!(m2.on_key_down(&e2));
    }

    #[test]
    fn meta_release_clears_pressed_set() {
        let mut m = KeyPressMatcher::new(
            &KeyCode::Single("Meta+s".into()),
            UseKeyPressOptions::default(),
        );
        let mut meta = ev("Meta", "MetaLeft");
        meta.meta_key = true;
        m.on_key_down(&meta);
        let mut s = ev("s", "KeyS");
        s.meta_key = true;
        m.on_key_down(&s);
        assert!(m.is_pressed());
        // Releasing Meta should clear everything.
        let _ = m.on_key_up(&meta);
        assert!(!m.is_pressed());
    }

    #[test]
    fn multiple_chord_options() {
        let mut m = KeyPressMatcher::new(
            &KeyCode::Multiple(vec!["a".into(), "d+s".into()]),
            UseKeyPressOptions::default(),
        );
        // First chord: just "a".
        assert!(m.on_key_down(&ev("a", "KeyA")));
        assert!(m.is_pressed());
        m.on_key_up(&ev("a", "KeyA"));
        assert!(!m.is_pressed());
        // Second chord: "d+s".
        assert!(!m.on_key_down(&ev("d", "KeyD")));
        assert!(m.on_key_down(&ev("s", "KeyS")));
        assert!(m.is_pressed());
    }

    #[test]
    fn reset_drops_pressed_state() {
        let mut m = KeyPressMatcher::new(
            &KeyCode::from("a"),
            UseKeyPressOptions::default(),
        );
        m.on_key_down(&ev("a", "KeyA"));
        assert!(m.is_pressed());
        m.reset();
        assert!(!m.is_pressed());
    }
}
