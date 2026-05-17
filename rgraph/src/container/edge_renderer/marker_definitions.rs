//! Port of `xyflow-react/src/container/EdgeRenderer/MarkerDefinitions.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! Emits an `<svg>` containing `<defs><marker>` blocks for every
//! unique custom edge marker used by the current edges list. The
//! marker ids are stable across renders thanks to
//! [`rgraph_core::utils::marker::create_marker_ids`].

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::utils::marker::{create_marker_ids, CreateMarkerIdsOptions};

use crate::container::edge_renderer::marker_symbols::MarkerSymbol;
use crate::context::use_rgraph_store;
use crate::store::RGraphStore;

#[derive(Props, Clone, PartialEq)]
pub struct MarkerDefinitionsProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    /// Default fill colour for arrow markers. TS allows `null` to fall
    /// back to the `--xy-edge-stroke` CSS variable; we represent that
    /// with `None`.
    #[props(default)]
    pub default_color: Option<String>,
    /// React Flow id — feeds into the marker dom-id prefix so multiple
    /// flows on a page don't collide.
    #[props(default)]
    pub rf_id: Option<String>,
    #[props(default)]
    pub _types: std::marker::PhantomData<(N, E)>,
}

#[component]
pub fn MarkerDefinitions<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: MarkerDefinitionsProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store::<N, E>();
    let edges = store.edges.read();
    let default_edge_options = store.default_edge_options.read();

    let markers = create_marker_ids(
        &edges,
        &CreateMarkerIdsOptions {
            id: props.rf_id.clone(),
            default_color: props.default_color.clone(),
            default_marker_start: default_edge_options
                .as_ref()
                .and_then(|o| o.marker_start.clone()),
            default_marker_end: default_edge_options
                .as_ref()
                .and_then(|o| o.marker_end.clone()),
        },
    );

    if markers.is_empty() {
        return rsx! {};
    }

    rsx! {
        svg {
            class: "react-flow__marker",
            "aria-hidden": "true",
            defs {
                for marker in markers.iter() {
                    {
                        let m = marker.marker.clone();
                        let id = marker.id.clone();
                        let width = m.width.unwrap_or(12.5);
                        let height = m.height.unwrap_or(12.5);
                        let units = m.marker_units.clone().unwrap_or_else(|| "strokeWidth".to_string());
                        let orient = m.orient.clone().unwrap_or_else(|| "auto-start-reverse".to_string());
                        let stroke_width = m.stroke_width.unwrap_or(1.0);
                        rsx! {
                            marker {
                                key: "{id}",
                                class: "react-flow__arrowhead",
                                id: "{id}",
                                "marker-width": "{width}",
                                "marker-height": "{height}",
                                "view-box": "-10 -10 20 20",
                                "marker-units": "{units}",
                                orient: "{orient}",
                                "ref-x": "0",
                                "ref-y": "0",
                                MarkerSymbol {
                                    type_: m.type_,
                                    color: m.color.clone(),
                                    stroke_width,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
