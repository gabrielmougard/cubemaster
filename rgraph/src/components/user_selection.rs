//! Port of `xyflow-react/src/components/UserSelection/index.tsx`.
//!
//! Status: Phase 4 — implemented.
//!
//! Renders the translucent rectangle drawn by the user during marquee
//! selection. Hidden when `user_selection_active` is `false` or
//! `user_selection_rect` is `None`.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

#[derive(Props, Clone, PartialEq)]
pub struct UserSelectionProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn UserSelection<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: UserSelectionProps<N, E>,
) -> Element {
    let _ = props;
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let active = *store.user_selection_active.read();
    let rect = *store.user_selection_rect.read();

    let Some(rect) = rect else {
        return rsx! {};
    };
    if !active {
        return rsx! {};
    }

    let style = format!(
        "width:{}px;height:{}px;transform:translate({}px,{}px);",
        rect.rect.width, rect.rect.height, rect.rect.x, rect.rect.y
    );

    rsx! {
        div {
            class: "react-flow__selection react-flow__container",
            style: "{style}",
        }
    }
}
