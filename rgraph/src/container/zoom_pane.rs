//! Port of `xyflow-react/src/container/ZoomPane/index.tsx`.
//!
//! Status: Phase 4 — implemented.
//!
//! `<ZoomPane>` is the heart of the viewport: it mounts an
//! [`rgraph_core::xypanzoom::XYPanZoom`] instance against its `<div>`,
//! wires Dioxus pointer / wheel events into the engine, and pushes
//! transform updates back into [`crate::store::RGraphStore`].
//!
//! The TS source lives at `xyflow-react/src/container/ZoomPane/index.tsx`.
//! Differences:
//!
//! * The TS version reads `domNode.getBoundingClientRect()` inside a
//!   `useEffect`. We do the same via Dioxus' `MountedData::get_client_rect`
//!   awaited from a spawned task triggered by the `onmounted` callback.
//! * d3-zoom's native `wheel.zoom`, `dblclick.zoom`, and pointer
//!   listeners are replaced by Dioxus event handlers that translate
//!   each event through [`crate::dom::wheel`] / [`crate::dom::pointer`]
//!   and forward to `XYPanZoom::handle_wheel` / `handle_pointer`.

#![allow(clippy::module_name_repetitions)]

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::events::{MountedData, PointerData, WheelData};
use dioxus::prelude::*;

use rgraph_core::types::geometry::{Rect, Transform};
use rgraph_core::types::panzoom::{
    OnDraggingChange, OnPanZoom, OnTransformChange, PanOnDrag, PanZoomInstance, PanZoomParams,
    PanZoomUpdateOptions,
};
use rgraph_core::types::viewport::{KeyCode, PanOnScrollMode, Viewport};
use rgraph_core::xypanzoom::XYPanZoom;
use rgraph_core::Promise;

use crate::context::use_rgraph_store;
use crate::dom::pointer::{from_dioxus as pointer_from_dioxus, PointerEventKind};
use crate::dom::{wheel as dom_wheel, PaneBounds};
use crate::hooks::use_key_press::use_key_press;
use crate::hooks::use_resize_handler::use_resize_handler;
use crate::store::{RGraphStore, SharedPanZoom};
use crate::types::component_props::OnViewportChange;

/// Concrete handle to the live `XYPanZoom` engine. We keep this in a
/// `Signal<Option<EngineHandle>>` alongside `store.pan_zoom` (which
/// stores the trait-object form) so wheel/pointer/bbox calls can hit
/// the engine directly without trait-object dynamic dispatch.
type EngineHandle = Rc<XYPanZoom<()>>;

/// Props for [`ZoomPane`]. Mirrors the TS `ZoomPaneProps` (a subset of
/// `FlowRendererProps` plus `is_controlled_viewport`).
#[derive(Props, Clone, PartialEq)]
pub struct ZoomPaneProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    /// HTML id of the wrapper. Used by the resize-observer shim.
    pub id: String,

    #[props(default = true)]
    pub zoom_on_scroll: bool,
    #[props(default = true)]
    pub zoom_on_pinch: bool,
    #[props(default)]
    pub pan_on_scroll: bool,
    #[props(default = 0.5)]
    pub pan_on_scroll_speed: f64,
    #[props(default)]
    pub pan_on_scroll_mode: PanOnScrollMode,
    #[props(default = true)]
    pub zoom_on_double_click: bool,
    #[props(default = PanOnDrag::On)]
    pub pan_on_drag: PanOnDrag,
    #[props(default)]
    pub default_viewport: Viewport,
    #[props(default = 0.5)]
    pub min_zoom: f64,
    #[props(default = 2.0)]
    pub max_zoom: f64,
    #[props(default)]
    pub zoom_activation_key_code: Option<KeyCode>,
    #[props(default = true)]
    pub prevent_scrolling: bool,
    #[props(default = "nowheel".to_string())]
    pub no_wheel_class_name: String,
    #[props(default = "nopan".to_string())]
    pub no_pan_class_name: String,
    #[props(default)]
    pub on_viewport_change: Option<OnViewportChange>,
    #[props(default)]
    pub is_controlled_viewport: bool,
    #[props(default = 0.0)]
    pub pane_click_distance: f64,
    #[props(default)]
    pub selection_on_drag: Option<bool>,

    pub children: Element,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn ZoomPane<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: ZoomPaneProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let zoom_activation_key = use_key_press(
        props.zoom_activation_key_code.clone(),
        Default::default(),
    );

    // Wrapper bounding rect, populated on `onmounted` and updated when
    // the wrapper is resized.
    let bounds_signal: Signal<PaneBounds> = use_signal(PaneBounds::default);
    // Concrete engine handle, parallel to `store.pan_zoom` (which
    // holds a `Box<dyn PanZoomInstance>` trait object).
    let engine_signal: Signal<Option<EngineHandle>> = use_signal(|| None::<EngineHandle>);

    let on_resize = use_resize_handler::<N, E>(&props.id);
    let id = props.id.clone();
    let default_viewport = props.default_viewport;
    let min_zoom = props.min_zoom;
    let max_zoom = props.max_zoom;

    let on_mounted = {
        let id = id.clone();
        let mut bounds_signal = bounds_signal;
        let mut engine_signal = engine_signal;
        move |evt: Event<MountedData>| {
            on_resize.call(evt.clone());

            let store = store;
            let id = id.clone();
            spawn(async move {
                let Ok(rect) = evt.get_client_rect().await else { return };
                let bounds = PaneBounds {
                    x: rect.origin.x,
                    y: rect.origin.y,
                    width: rect.size.width,
                    height: rect.size.height,
                };
                bounds_signal.set(bounds);
                // Mirror into the shared store-level signal so
                // viewport-helper hooks can convert screen↔flow coords
                // without holding a local copy of the bbox.
                store.dom_bbox.clone().set(bounds);

                if let Some(existing) = &*engine_signal.peek() {
                    existing.set_dom_bbox(rect_to_core(bounds));
                    return;
                }

                let panzoom = build_panzoom(store, bounds, default_viewport, min_zoom, max_zoom);
                let viewport = panzoom.get_viewport();
                store.transform.clone().set(Transform::new(
                    viewport.x,
                    viewport.y,
                    viewport.zoom,
                ));
                store.dom_node_id.clone().set(Some(id));

                let engine = Rc::new(panzoom);

                // Wire the zoom-event listener that syncs the engine's
                // transform into `store.transform` so the `<Viewport>`
                // div re-renders on every pan / zoom tick. xypanzoom's
                // `update()` doesn't install this in the Rust port —
                // without it, dragging the pane (and the minimap) does
                // nothing visible.
                let store_for_zoom = store;
                engine.on_zoom_event("zoom", move |evt: &rgraph_zoom::ZoomEvent<(), ()>| {
                    use dioxus::prelude::WritableExt;
                    let t = Transform::new(
                        evt.transform.x,
                        evt.transform.y,
                        evt.transform.k,
                    );
                    store_for_zoom.transform.clone().set(t);
                });

                engine_signal.set(Some(engine.clone()));

                // Push the trait-object adapter into the store too so
                // hooks like `use_viewport_helper` can drive zooms.
                let adapter = XYPanZoomAdapter { inner: engine };
                let boxed: Box<dyn PanZoomInstance> = Box::new(adapter);
                store.pan_zoom.clone().set(Some(Rc::new(RefCell::new(boxed))));
            });
        }
    };

    // Reapply the runtime-options bundle whenever any tracked input
    // changes. Mirrors the TS effect at lines 109–149.
    {
        let pan_on_drag = props.pan_on_drag.clone();
        let no_pan = props.no_pan_class_name.clone();
        let no_wheel = props.no_wheel_class_name.clone();
        let zoom_activation = zoom_activation_key.pressed;
        let on_viewport_change = props.on_viewport_change;
        let is_controlled = props.is_controlled_viewport;
        let prevent_scrolling = props.prevent_scrolling;
        let pan_on_scroll = props.pan_on_scroll;
        let pan_on_scroll_mode = props.pan_on_scroll_mode;
        let pan_on_scroll_speed = props.pan_on_scroll_speed;
        let zoom_on_pinch = props.zoom_on_pinch;
        let zoom_on_scroll = props.zoom_on_scroll;
        let zoom_on_double_click = props.zoom_on_double_click;
        let pane_click_distance = props.pane_click_distance;
        let selection_on_drag = props.selection_on_drag;
        use_effect(move || {
            let user_selection_active = *store.user_selection_active.peek();
            let connection_in_progress = matches!(
                *store.connection.peek(),
                rgraph_core::types::connection::ConnectionState::InProgress(_)
            );
            let lib = store.lib.peek().clone();
            let zoom_activation_pressed = *zoom_activation.read();

            let Some(engine) = engine_signal.peek().clone() else { return };
            let opts = PanZoomUpdateOptions {
                no_wheel_class_name: no_wheel.clone(),
                no_pan_class_name: no_pan.clone(),
                on_pane_context_menu: None,
                prevent_scrolling,
                pan_on_scroll,
                pan_on_drag: pan_on_drag.clone(),
                pan_on_scroll_mode,
                pan_on_scroll_speed,
                user_selection_active,
                zoom_on_pinch,
                zoom_on_scroll,
                zoom_on_double_click,
                zoom_activation_key_pressed: zoom_activation_pressed,
                lib,
                on_transform_change: build_transform_change_callback(
                    store,
                    is_controlled,
                    on_viewport_change,
                ),
                connection_in_progress,
                pane_click_distance,
                selection_on_drag,
            };
            engine.update(&opts);
        });
    }

    // Translate Dioxus events into rgraph_zoom inputs.
    let on_wheel = {
        move |evt: Event<WheelData>| {
            let bounds = *bounds_signal.read();
            let input = dom_wheel::from_dioxus(&evt, bounds);
            if let Some(engine) = &*engine_signal.peek() {
                engine.handle_wheel(input);
            }
        }
    };

    let on_pointer_down = make_pointer_handler(engine_signal, bounds_signal, PointerEventKind::Down);
    let on_pointer_move_inner = make_pointer_handler(engine_signal, bounds_signal, PointerEventKind::Move);
    let on_pointer_up_inner = make_pointer_handler(engine_signal, bounds_signal, PointerEventKind::Up);
    let on_pointer_cancel_inner = make_pointer_handler(engine_signal, bounds_signal, PointerEventKind::Cancel);

    // Layer a connection-line state updater on top of the pan/zoom
    // pointer handlers so `<ConnectionLine>` can draw a live preview
    // from the source handle to the cursor while the user drags.
    let store_for_move = store;
    let mut on_pointer_move_inner = on_pointer_move_inner;
    let on_pointer_move = move |evt: Event<PointerData>| {
        use dioxus::html::point_interaction::InteractionLocation;
        use dioxus::prelude::{ReadableExt, WritableExt};
        use rgraph_core::types::connection::ConnectionState;
        use rgraph_core::types::geometry::XYPosition;
        // IMPORTANT: bind the cloned value to a local *before* we
        // touch the signal again — `peek()` returns a `Ref<…>` whose
        // lifetime in an `if let` scrutinee extends to the end of the
        // `if` body. Calling `.set(…)` inside that scope while the
        // read guard is still alive triggers `AlreadyBorrowed`.
        let conn_snapshot = store_for_move.connection.peek().clone();
        if let ConnectionState::InProgress(mut p) = conn_snapshot {
            let client = evt.client_coordinates();
            let bbox = *store_for_move.dom_bbox.peek();
            p.to = XYPosition::new(client.x - bbox.x, client.y - bbox.y);
            p.pointer = XYPosition::new(client.x, client.y);
            store_for_move
                .connection
                .clone()
                .set(ConnectionState::InProgress(p));
        }
        on_pointer_move_inner(evt);
    };

    let store_for_up = store;
    let mut on_pointer_up_inner = on_pointer_up_inner;
    let on_pointer_up = move |evt: Event<PointerData>| {
        use dioxus::prelude::{ReadableExt, WritableExt};
        use rgraph_core::types::connection::ConnectionState;
        // Pane-level pointer-up only fires when the release did NOT
        // happen on a handle (handle's own onpointerup `stop_propagation`s).
        // That means the user either:
        //   * was dragging a connection and dropped it on empty canvas, or
        //   * had a click-to-connect "first click" pending and clicked
        //     somewhere off-handle to cancel it.
        // Either way we clear both the in-progress preview and the
        // sticky "click-connecting" handle so the next attempt starts
        // from a clean slate (otherwise the source dot stays yellow
        // and new drags from elsewhere get rejected).
        let in_progress = matches!(
            &*store_for_up.connection.peek(),
            ConnectionState::InProgress(_)
        );
        if in_progress {
            store_for_up
                .connection
                .clone()
                .set(ConnectionState::NoConnection);
        }
        let has_click_start = store_for_up
            .connection_click_start_handle
            .peek()
            .is_some();
        if has_click_start {
            store_for_up
                .connection_click_start_handle
                .clone()
                .set(None);
        }
        on_pointer_up_inner(evt);
    };

    let store_for_cancel = store;
    let mut on_pointer_cancel_inner = on_pointer_cancel_inner;
    let on_pointer_cancel = move |evt: Event<PointerData>| {
        use dioxus::prelude::{ReadableExt, WritableExt};
        use rgraph_core::types::connection::ConnectionState;
        let in_progress = matches!(
            &*store_for_cancel.connection.peek(),
            ConnectionState::InProgress(_)
        );
        if in_progress {
            store_for_cancel
                .connection
                .clone()
                .set(ConnectionState::NoConnection);
        }
        let has_click_start = store_for_cancel
            .connection_click_start_handle
            .peek()
            .is_some();
        if has_click_start {
            store_for_cancel
                .connection_click_start_handle
                .clone()
                .set(None);
        }
        on_pointer_cancel_inner(evt);
    };

    let style = "position:absolute;width:100%;height:100%;top:0;left:0;";

    rsx! {
        div {
            id: "{props.id}",
            class: "react-flow__renderer",
            style: "{style}",
            onmounted: on_mounted,
            onwheel: on_wheel,
            onpointerdown: on_pointer_down,
            onpointermove: on_pointer_move,
            onpointerup: on_pointer_up,
            onpointercancel: on_pointer_cancel,
            {props.children}
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn rect_to_core(b: PaneBounds) -> Rect {
    Rect {
        x: b.x,
        y: b.y,
        width: b.width,
        height: b.height,
    }
}

fn build_panzoom<N, E>(
    store: RGraphStore<N, E>,
    bounds: PaneBounds,
    default_viewport: Viewport,
    min_zoom: f64,
    max_zoom: f64,
) -> XYPanZoom<()>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    // `RGraphStore: Copy` so the move-closure below captures by value
    // without `Send + Sync` constraints; the closure is invoked
    // single-threaded inside the engine.
    let on_dragging_change: OnDraggingChange = Box::new(move |dragging: bool| {
        use dioxus::prelude::{ReadableExt, WritableExt};
        if *store.pane_dragging.peek() != dragging {
            store.pane_dragging.clone().set(dragging);
        }
    });
    let on_pan_zoom_start: OnPanZoom = Box::new(|_evt, _vp| {});
    let on_pan_zoom: OnPanZoom = Box::new(|_evt, _vp| {});
    let on_pan_zoom_end: OnPanZoom = Box::new(|_evt, _vp| {});

    let params = PanZoomParams {
        min_zoom,
        max_zoom,
        viewport: default_viewport,
        translate_extent: rgraph_core::INFINITE_EXTENT,
        dom_bbox: rect_to_core(bounds),
        on_dragging_change,
        on_pan_zoom_start: Some(on_pan_zoom_start),
        on_pan_zoom: Some(on_pan_zoom),
        on_pan_zoom_end: Some(on_pan_zoom_end),
    };

    XYPanZoom::new((), params)
}

fn build_transform_change_callback<N, E>(
    store: RGraphStore<N, E>,
    is_controlled_viewport: bool,
    on_viewport_change: Option<OnViewportChange>,
) -> OnTransformChange
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    Box::new(move |t: Transform| {
        let viewport = Viewport {
            x: t.tx(),
            y: t.ty(),
            zoom: t.scale(),
        };
        if let Some(cb) = on_viewport_change {
            cb.call(viewport);
        }
        if !is_controlled_viewport {
            store.transform.clone().set(t);
        }
    })
}

fn make_pointer_handler(
    engine_signal: Signal<Option<EngineHandle>>,
    bounds_signal: Signal<PaneBounds>,
    kind: PointerEventKind,
) -> impl FnMut(Event<PointerData>) + 'static {
    move |evt: Event<PointerData>| {
        let bounds = *bounds_signal.read();
        let input = pointer_from_dioxus::<()>(&evt, kind, bounds, None);
        if let Some(engine) = &*engine_signal.peek() {
            engine.handle_pointer(input);
        }
    }
}

// ---------------------------------------------------------------------------
// Trait-object adapter for the store-side `pan_zoom` slot.
// ---------------------------------------------------------------------------
//
// `RGraphStore::pan_zoom` is `Signal<Option<SharedPanZoom>>` where
// `SharedPanZoom = Rc<RefCell<Box<dyn PanZoomInstance>>>`. `XYPanZoom`
// methods take `&self`, but `PanZoomInstance` declares `&mut self`. We
// bridge that with a small wrapper that owns an `Rc<XYPanZoom<()>>`
// and forwards every call through the engine's interior-mutable API.

struct XYPanZoomAdapter {
    inner: Rc<XYPanZoom<()>>,
}

impl PanZoomInstance for XYPanZoomAdapter {
    fn update(&mut self, options: PanZoomUpdateOptions) {
        self.inner.update(&options);
    }
    fn destroy(&mut self) {
        self.inner.destroy();
    }
    fn get_viewport(&self) -> Viewport {
        self.inner.get_viewport()
    }
    fn set_viewport(
        &mut self,
        viewport: Viewport,
        options: Option<rgraph_core::types::panzoom::PanZoomTransformOptions>,
    ) -> Promise<bool> {
        self.inner.set_viewport(viewport, options)
    }
    fn set_viewport_constrained(
        &mut self,
        viewport: Viewport,
        extent: rgraph_core::types::geometry::CoordinateExtent,
        translate_extent: rgraph_core::types::geometry::CoordinateExtent,
    ) -> Promise<Option<Transform>> {
        self.inner
            .set_viewport_constrained(viewport, extent, translate_extent)
    }
    fn set_scale_extent(&mut self, scale_extent: (f64, f64)) {
        self.inner.set_scale_extent(scale_extent);
    }
    fn set_translate_extent(
        &mut self,
        translate_extent: rgraph_core::types::geometry::CoordinateExtent,
    ) {
        self.inner.set_translate_extent(translate_extent);
    }
    fn scale_to(
        &mut self,
        scale: f64,
        options: Option<rgraph_core::types::panzoom::PanZoomTransformOptions>,
    ) -> Promise<bool> {
        self.inner.scale_to(scale, options)
    }
    fn scale_by(
        &mut self,
        factor: f64,
        options: Option<rgraph_core::types::panzoom::PanZoomTransformOptions>,
    ) -> Promise<bool> {
        self.inner.scale_by(factor, options)
    }
    fn sync_viewport(&mut self, viewport: Viewport) {
        self.inner.sync_viewport(viewport);
    }
    fn set_click_distance(&mut self, distance: f64) {
        self.inner.set_click_distance(distance);
    }
}

// `SharedPanZoom` import retained for documentation cross-references.
#[allow(dead_code)]
type _Spz = SharedPanZoom;
