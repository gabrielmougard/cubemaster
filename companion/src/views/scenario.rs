//! Experimental Scenario Editor.
//!
//! Lets a game master sketch a DnD-style scenario as a directed graph:
//! - **Situations** are nodes (Start / Scene / End).
//! - **Conditions** are edges — labelled triggers the GM fires while playing.
//!
//! Has two modes:
//! - **Edit** — drag nodes from the palette, connect them, label conditions,
//!   edit titles/descriptions in the inspector.
//! - **Play** — the graph turns read-only and outgoing conditions of the
//!   *current* node become buttons. Clicking one advances the current
//!   situation.
//!
//! Assets (videos / sounds projected onto the cube) are intentionally
//! left as a slot on each node for a future iteration.

use std::collections::HashMap;

use dioxus::prelude::*;
// rgraph::prelude::* exports an `Element` enum (`is_node` / `is_edge`
// view type) that collides with Dioxus' `Element` type alias. Import
// the rgraph pieces we need explicitly instead. (Note: rgraph's
// `<Background>` SVG renders unreliably in the Dioxus desktop webview,
// so we draw the dotted grid via CSS on `.scenario-canvas` and skip
// the component entirely.)
use rgraph::additional_components::controls::Controls;
use rgraph::additional_components::minimap::MiniMap;
use rgraph::container::rgraph::RGraph;
use rgraph::types::edges::Edge;
use rgraph::types::nodes::{BuiltInNodeData, Node};
use rgraph::utils::changes::{apply_edge_changes, apply_node_changes};
use rgraph_core::types::changes::NodeChange;
use rgraph_core::types::connection::{Connection, ConnectionMode};

use crate::components::icons::{IconEdit, IconPlay, IconPlus, IconRestart, IconTrash};
use crate::components::sidebar::{NavItem, Sidebar};

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScenarioMode {
    Edit,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Start,
    Scene,
    End,
}

impl NodeKind {
    fn type_str(self) -> &'static str {
        match self {
            NodeKind::Start => "input",
            NodeKind::Scene => "default",
            NodeKind::End => "output",
        }
    }
    fn label(self) -> &'static str {
        match self {
            NodeKind::Start => "Start",
            NodeKind::Scene => "Scene",
            NodeKind::End => "End",
        }
    }
    fn default_title(self) -> &'static str {
        match self {
            NodeKind::Start => "Opening",
            NodeKind::Scene => "Untitled scene",
            NodeKind::End => "Ending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeMeta {
    kind: NodeKind,
    title: String,
    description: String,
    /// Placeholder for future asset binding (video / sound id on the cube).
    asset_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct EdgeMeta {
    /// GM-facing trigger label, e.g. "Player picks the lock".
    label: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn new_node_id(nodes: &[Node<BuiltInNodeData>]) -> String {
    let mut n: u32 = (nodes.len() as u32) + 1;
    loop {
        let id = format!("n{n}");
        if !nodes.iter().any(|x| x.id == id) {
            return id;
        }
        n += 1;
    }
}

fn new_edge_id(edges: &[Edge<()>]) -> String {
    let mut n: u32 = (edges.len() as u32) + 1;
    loop {
        let id = format!("e{n}");
        if !edges.iter().any(|x| x.id == id) {
            return id;
        }
        n += 1;
    }
}

fn make_node(id: &str, x: f64, y: f64, kind: NodeKind, title: &str) -> Node<BuiltInNodeData> {
    let mut n = Node::<BuiltInNodeData>::minimal(id, x, y);
    n.type_ = Some(kind.type_str().to_string());
    n.data = BuiltInNodeData::Labelled {
        label: title.to_string(),
    };
    n
}

fn make_edge(id: &str, source: &str, target: &str) -> Edge<()> {
    Edge::<()>::minimal(id, source, target)
}

fn edge_from_connection(id: &str, conn: &Connection) -> Edge<()> {
    let mut e = make_edge(id, &conn.source, &conn.target);
    e.source_handle = conn.source_handle.clone();
    e.target_handle = conn.target_handle.clone();
    e
}

fn demo_state() -> (
    Vec<Node<BuiltInNodeData>>,
    Vec<Edge<()>>,
    HashMap<String, NodeMeta>,
    HashMap<String, EdgeMeta>,
) {
    let nodes = vec![
        make_node("n1", 60.0, 80.0, NodeKind::Start, "The Tavern"),
        make_node("n2", 320.0, 40.0, NodeKind::Scene, "Suspicious patron"),
        make_node("n3", 320.0, 200.0, NodeKind::Scene, "Cellar trapdoor"),
        make_node("n4", 600.0, 120.0, NodeKind::End, "Captured by orcs"),
    ];

    let edges = vec![
        make_edge("e1", "n1", "n2"),
        make_edge("e2", "n1", "n3"),
        make_edge("e3", "n2", "n4"),
        make_edge("e4", "n3", "n4"),
    ];

    let mut node_meta = HashMap::new();
    node_meta.insert(
        "n1".into(),
        NodeMeta {
            kind: NodeKind::Start,
            title: "The Tavern".into(),
            description: "A smoky inn at the edge of the kingdom. The party meets here at dusk.".into(),
            asset_hint: "tavern_loop.mp4 / tavern_ambience.wav".into(),
        },
    );
    node_meta.insert(
        "n2".into(),
        NodeMeta {
            kind: NodeKind::Scene,
            title: "Suspicious patron".into(),
            description: "A hooded figure watches the party from the corner. He clearly knows something.".into(),
            asset_hint: "patron_silhouette.png".into(),
        },
    );
    node_meta.insert(
        "n3".into(),
        NodeMeta {
            kind: NodeKind::Scene,
            title: "Cellar trapdoor".into(),
            description: "Beneath a rug behind the bar, a worn iron ring hints at a hidden passage.".into(),
            asset_hint: "wood_creak.wav".into(),
        },
    );
    node_meta.insert(
        "n4".into(),
        NodeMeta {
            kind: NodeKind::End,
            title: "Captured by orcs".into(),
            description: "The party is overwhelmed and dragged into the night. (Try again?)".into(),
            asset_hint: "drums_of_war.wav".into(),
        },
    );

    let mut edge_meta = HashMap::new();
    edge_meta.insert(
        "e1".into(),
        EdgeMeta {
            label: "Party approaches the patron".into(),
        },
    );
    edge_meta.insert(
        "e2".into(),
        EdgeMeta {
            label: "Party searches the back room".into(),
        },
    );
    edge_meta.insert(
        "e3".into(),
        EdgeMeta {
            label: "Patron lures them outside".into(),
        },
    );
    edge_meta.insert(
        "e4".into(),
        EdgeMeta {
            label: "Trap triggers".into(),
        },
    );

    (nodes, edges, node_meta, edge_meta)
}

// ---------------------------------------------------------------------------
// Context menu state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum ContextTarget {
    Node(String),
    Edge(String),
}

#[derive(Debug, Clone, PartialEq)]
struct ContextMenuState {
    /// Viewport-relative click coordinates (clientX/Y) — we render the
    /// menu as a `position: fixed` overlay so these map directly.
    x: f64,
    y: f64,
    target: ContextTarget,
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
pub fn ScenarioView() -> Element {
    let (init_nodes, init_edges, init_node_meta, init_edge_meta) = demo_state();

    let nodes = use_signal(|| init_nodes);
    let edges = use_signal(|| init_edges);
    let node_meta = use_signal(|| init_node_meta);
    let edge_meta = use_signal(|| init_edge_meta);

    let mode = use_signal(|| ScenarioMode::Edit);
    let play_current = use_signal(|| Option::<String>::None);
    let context_menu = use_signal(|| Option::<ContextMenuState>::None);

    // The rgraph DOM uses `react-flow__*` class names, but the upstream
    // CSS that ships in `rgraph-core/assets/` is written against the
    // `xy-flow__*` prefix (xyflow's framework-agnostic source). Re-emit
    // it with the prefix swapped so layout/handle/edge rules apply.
    //
    // (`rgraph::styles::{BASE_CSS, STYLE_CSS}` currently only contain
    // `@import` directives that point outside this repo and resolve to
    // nothing at runtime — we bypass them entirely.)
    let rgraph_css = use_memo(|| {
        let raw = concat!(
            include_str!("../../../rgraph-core/assets/init.css"),
            "\n",
            include_str!("../../../rgraph-core/assets/base.css"),
            "\n",
            include_str!("../../../rgraph-core/assets/style.css"),
            "\n",
            include_str!("../../../rgraph-core/assets/node-resizer.css"),
        );
        raw.replace("xy-flow", "react-flow")
    });

    rsx! {
        style { {rgraph_css} }
        style { {include_str!("../../assets/scenario.css")} }

        div { class: "app-layout",
            Sidebar { active: NavItem::Scenario }
            main { class: "main-content scenario-main",
                ScenarioToolbar { mode, play_current, nodes }
                div { class: "scenario-body",
                    if *mode.read() == ScenarioMode::Edit {
                        EditPalette { nodes, edges, node_meta, edge_meta }
                    } else {
                        PlayCurrentCard {
                            nodes,
                            node_meta,
                            play_current,
                        }
                    }

                    GraphCanvas {
                        nodes,
                        edges,
                        node_meta,
                        edge_meta,
                        mode,
                        play_current,
                        context_menu,
                    }

                    if *mode.read() == ScenarioMode::Edit {
                        Inspector { nodes, edges, node_meta, edge_meta }
                    } else {
                        PlayTransitions {
                            edges,
                            edge_meta,
                            play_current,
                            nodes,
                            node_meta,
                        }
                    }
                }
            }

            // Floating context menu overlay. Rendered at the app-layout
            // level so its `position: fixed` placement isn't clipped by
            // the canvas grid cell.
            if context_menu.read().is_some() {
                ScenarioContextMenu {
                    nodes,
                    edges,
                    node_meta,
                    edge_meta,
                    context_menu,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

#[component]
fn ScenarioToolbar(
    mode: Signal<ScenarioMode>,
    play_current: Signal<Option<String>>,
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
) -> Element {
    let current_mode = *mode.read();

    rsx! {
        div { class: "scenario-toolbar",
            div { class: "scenario-title-block",
                h1 { class: "view-title", "Scenario Editor" }
                span { class: "scenario-subtitle",
                    "Sketch a branching DnD scenario, then run it from the GM seat."
                }
            }

            div { class: "scenario-mode-switch", role: "tablist",
                button {
                    class: if current_mode == ScenarioMode::Edit { "mode-btn active" } else { "mode-btn" },
                    onclick: move |_| {
                        // Drop the "current situation" highlight so it
                        // doesn't bleed into edit selection.
                        let mut snap = nodes.peek().clone();
                        for n in snap.iter_mut() {
                            n.selected = Some(false);
                        }
                        nodes.set(snap);
                        play_current.set(None);
                        mode.set(ScenarioMode::Edit);
                    },
                    IconEdit { class: "mode-icon".to_string() }
                    span { "Edit" }
                }
                button {
                    class: if current_mode == ScenarioMode::Play { "mode-btn active" } else { "mode-btn" },
                    onclick: move |_| {
                        // Enter play mode at the first Start node, or fall
                        // back to the first node if none is marked Start.
                        let mut snap = nodes.peek().clone();
                        let start_id = snap
                            .iter()
                            .find(|n| n.type_.as_deref() == Some("input"))
                            .or_else(|| snap.first())
                            .map(|n| n.id.clone());
                        for n in snap.iter_mut() {
                            n.selected = Some(Some(&n.id) == start_id.as_ref());
                        }
                        nodes.set(snap);
                        play_current.set(start_id);
                        mode.set(ScenarioMode::Play);
                    },
                    IconPlay { class: "mode-icon".to_string() }
                    span { "Play" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Edit-mode left palette
// ---------------------------------------------------------------------------

#[component]
fn EditPalette(
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    edges: Signal<Vec<Edge<()>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
    edge_meta: Signal<HashMap<String, EdgeMeta>>,
) -> Element {
    let mut add_node = move |kind: NodeKind| {
        // Drop the new node near the center of the visible canvas. Without
        // a viewport handle from outside `<RGraph>` we fall back to an
        // offset grid so consecutive adds don't overlap.
        let mut new_nodes = nodes.peek().clone();
        let n_count = new_nodes.len();
        let x = 120.0 + ((n_count as f64) % 4.0) * 60.0;
        let y = 120.0 + ((n_count as f64 / 4.0).floor()) * 60.0;
        let id = new_node_id(&new_nodes);
        let title = kind.default_title();
        new_nodes.push(make_node(&id, x, y, kind, title));
        nodes.set(new_nodes);

        let mut meta = node_meta.peek().clone();
        meta.insert(
            id,
            NodeMeta {
                kind,
                title: title.into(),
                description: String::new(),
                asset_hint: String::new(),
            },
        );
        node_meta.set(meta);
    };

    let reset_demo = move |_: MouseEvent| {
        let (n, e, nm, em) = demo_state();
        nodes.set(n);
        edges.set(e);
        node_meta.set(nm);
        edge_meta.set(em);
    };

    rsx! {
        aside { class: "scenario-pane scenario-pane-left",
            div { class: "pane-section",
                span { class: "pane-section-title", "Add a situation" }
                button {
                    class: "palette-btn palette-btn-start",
                    onclick: move |_| add_node(NodeKind::Start),
                    IconPlus { class: "palette-btn-icon".to_string() }
                    div { class: "palette-btn-text",
                        strong { "Start" }
                        small { "Where the scenario begins. One source." }
                    }
                }
                button {
                    class: "palette-btn palette-btn-scene",
                    onclick: move |_| add_node(NodeKind::Scene),
                    IconPlus { class: "palette-btn-icon".to_string() }
                    div { class: "palette-btn-text",
                        strong { "Scene" }
                        small { "An intermediate situation. In + out handles." }
                    }
                }
                button {
                    class: "palette-btn palette-btn-end",
                    onclick: move |_| add_node(NodeKind::End),
                    IconPlus { class: "palette-btn-icon".to_string() }
                    div { class: "palette-btn-text",
                        strong { "Ending" }
                        small { "A terminal outcome. Target-only." }
                    }
                }
            }

            div { class: "pane-section",
                span { class: "pane-section-title", "How to wire it up" }
                ol { class: "pane-hint-list",
                    li {
                        "Click the source handle (small dot) of a node, then click the target handle of another node to draw a condition."
                    }
                    li { "Drag a node by its body to move it around." }
                    li { "Click a node or edge to edit it on the right." }
                    li { "Backspace removes the selected node or edge." }
                    li { "Hit ", strong { "Play" }, " to walk the GM through it." }
                }
            }

            div { class: "pane-section",
                button {
                    class: "btn btn-ghost btn-sm pane-reset-btn",
                    onclick: reset_demo,
                    IconRestart { class: "btn-icon".to_string() }
                    span { "Load demo scenario" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Play-mode left "current situation" card
// ---------------------------------------------------------------------------

#[component]
fn PlayCurrentCard(
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
    play_current: Signal<Option<String>>,
) -> Element {
    let current_id = play_current.read().clone();
    let snap_nodes = nodes.read().clone();
    let snap_meta = node_meta.read().clone();

    let current_node = current_id
        .as_ref()
        .and_then(|id| snap_nodes.iter().find(|n| &n.id == id).cloned());
    let current_meta = current_id.as_ref().and_then(|id| snap_meta.get(id).cloned());

    rsx! {
        aside { class: "scenario-pane scenario-pane-left",
            if let (Some(node), Some(meta)) = (current_node, current_meta) {
                div { class: "play-current-card",
                    span { class: "play-current-tag",
                        match meta.kind {
                            NodeKind::Start => "Start",
                            NodeKind::Scene => "Scene",
                            NodeKind::End => "Ending",
                        }
                    }
                    h2 { class: "play-current-title", "{meta.title}" }
                    p { class: "play-current-description",
                        if meta.description.is_empty() {
                            "No description yet — add one in Edit mode."
                        } else {
                            "{meta.description}"
                        }
                    }
                    div { class: "play-current-asset",
                        span { class: "play-current-asset-label", "Cube asset" }
                        span { class: "play-current-asset-value",
                            if meta.asset_hint.is_empty() {
                                "(none)"
                            } else {
                                "{meta.asset_hint}"
                            }
                        }
                    }
                    div { class: "play-current-id-row",
                        span { class: "play-current-id-label", "Node" }
                        code { "{node.id}" }
                    }
                }
            } else {
                div { class: "play-current-card empty",
                    span { class: "play-current-tag", "Idle" }
                    h2 { class: "play-current-title", "No situation active" }
                    p { class: "play-current-description",
                        "Press ", strong { "Restart" }, " or pick a node from the graph."
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Right inspector (edit mode)
// ---------------------------------------------------------------------------

#[component]
fn Inspector(
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    edges: Signal<Vec<Edge<()>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
    edge_meta: Signal<HashMap<String, EdgeMeta>>,
) -> Element {
    // Find the (single) selected node or edge. rgraph multi-select is
    // possible; we just inspect the first match for now.
    let selected_node_id: Option<String> = nodes
        .read()
        .iter()
        .find(|n| n.selected.unwrap_or(false))
        .map(|n| n.id.clone());
    let selected_edge_id: Option<String> = edges
        .read()
        .iter()
        .find(|e| e.selected.unwrap_or(false))
        .map(|e| e.id.clone());

    rsx! {
        aside { class: "scenario-pane scenario-pane-right",
            if let Some(node_id) = selected_node_id {
                NodeInspector { node_id, nodes, node_meta }
            } else if let Some(edge_id) = selected_edge_id {
                EdgeInspector { edge_id, edges, edge_meta, nodes, node_meta }
            } else {
                div { class: "pane-section pane-empty",
                    span { class: "pane-section-title", "Inspector" }
                    p { class: "pane-empty-text",
                        "Click a situation or condition to edit it. Drag from a handle to create a new condition."
                    }
                }
            }
        }
    }
}

#[component]
fn NodeInspector(
    node_id: String,
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
) -> Element {
    let meta = node_meta
        .read()
        .get(&node_id)
        .cloned()
        .unwrap_or(NodeMeta {
            kind: NodeKind::Scene,
            title: String::new(),
            description: String::new(),
            asset_hint: String::new(),
        });
    let kind_label = meta.kind.label();

    let node_id_title = node_id.clone();
    let on_title_input = {
        let node_id = node_id.clone();
        move |evt: FormEvent| {
            let value = evt.value();
            // Update the node label (visible on the canvas).
            let mut snap = nodes.peek().clone();
            if let Some(n) = snap.iter_mut().find(|n| n.id == node_id) {
                n.data = BuiltInNodeData::Labelled {
                    label: value.clone(),
                };
            }
            nodes.set(snap);
            // Update parallel metadata.
            let mut meta_map = node_meta.peek().clone();
            let entry = meta_map.entry(node_id.clone()).or_insert(NodeMeta {
                kind: NodeKind::Scene,
                title: String::new(),
                description: String::new(),
                asset_hint: String::new(),
            });
            entry.title = value;
            node_meta.set(meta_map);
        }
    };

    let node_id_desc = node_id.clone();
    let on_desc_input = move |evt: FormEvent| {
        let mut meta_map = node_meta.peek().clone();
        let entry = meta_map.entry(node_id_desc.clone()).or_insert(NodeMeta {
            kind: NodeKind::Scene,
            title: String::new(),
            description: String::new(),
            asset_hint: String::new(),
        });
        entry.description = evt.value();
        node_meta.set(meta_map);
    };

    let node_id_asset = node_id.clone();
    let on_asset_input = move |evt: FormEvent| {
        let mut meta_map = node_meta.peek().clone();
        let entry = meta_map.entry(node_id_asset.clone()).or_insert(NodeMeta {
            kind: NodeKind::Scene,
            title: String::new(),
            description: String::new(),
            asset_hint: String::new(),
        });
        entry.asset_hint = evt.value();
        node_meta.set(meta_map);
    };

    let node_id_delete = node_id.clone();
    let on_delete = move |_: MouseEvent| {
        let mut snap = nodes.peek().clone();
        snap.retain(|n| n.id != node_id_delete);
        nodes.set(snap);
        let mut meta_map = node_meta.peek().clone();
        meta_map.remove(&node_id_delete);
        node_meta.set(meta_map);
    };

    rsx! {
        div { class: "pane-section",
            div { class: "pane-section-header",
                span { class: "pane-section-title", "Situation" }
                span { class: "pane-section-pill pane-pill-{kind_label.to_lowercase()}", "{kind_label}" }
            }

            label { class: "field-label", "Title" }
            input {
                class: "field-input",
                value: meta.title.clone(),
                oninput: on_title_input,
                placeholder: "Untitled scene",
            }

            label { class: "field-label", "Description" }
            textarea {
                class: "field-textarea",
                rows: "4",
                value: "{meta.description}",
                oninput: on_desc_input,
                placeholder: "Hidden information, what the PCs see, GM notes...",
            }

            label { class: "field-label", "Cube asset (placeholder)" }
            input {
                class: "field-input",
                value: meta.asset_hint.clone(),
                oninput: on_asset_input,
                placeholder: "e.g. tavern_loop.mp4",
            }
            p { class: "field-hint",
                "Asset binding is not wired to the cube yet — this is a stub for the next iteration."
            }

            div { class: "field-row",
                span { class: "field-label-inline", "Node id" }
                code { class: "field-code", "{node_id_title}" }
            }

            div { class: "pane-actions",
                button {
                    class: "btn btn-danger btn-sm",
                    onclick: on_delete,
                    IconTrash { class: "btn-icon".to_string() }
                    span { "Delete situation" }
                }
            }
        }
    }
}

#[component]
fn EdgeInspector(
    edge_id: String,
    edges: Signal<Vec<Edge<()>>>,
    edge_meta: Signal<HashMap<String, EdgeMeta>>,
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
) -> Element {
    let edge_snap = edges
        .read()
        .iter()
        .find(|e| e.id == edge_id)
        .cloned();
    let Some(edge) = edge_snap else {
        return rsx! { div { class: "pane-section pane-empty", "(edge not found)" } };
    };

    let nm = node_meta.read().clone();
    let source_title = nm
        .get(&edge.source)
        .map(|m| m.title.clone())
        .unwrap_or_else(|| edge.source.clone());
    let target_title = nm
        .get(&edge.target)
        .map(|m| m.title.clone())
        .unwrap_or_else(|| edge.target.clone());

    let meta = edge_meta.read().get(&edge_id).cloned().unwrap_or_default();

    let edge_id_input = edge_id.clone();
    let on_label_input = move |evt: FormEvent| {
        let mut snap = edge_meta.peek().clone();
        let entry = snap.entry(edge_id_input.clone()).or_default();
        entry.label = evt.value();
        edge_meta.set(snap);
    };

    let edge_id_anim = edge_id.clone();
    let animated = edge.animated.unwrap_or(false);
    let on_animate_toggle = move |_: MouseEvent| {
        let mut snap = edges.peek().clone();
        if let Some(e) = snap.iter_mut().find(|e| e.id == edge_id_anim) {
            e.animated = Some(!e.animated.unwrap_or(false));
        }
        edges.set(snap);
    };

    let edge_id_del = edge_id.clone();
    let on_delete = move |_: MouseEvent| {
        let mut snap = edges.peek().clone();
        snap.retain(|e| e.id != edge_id_del);
        edges.set(snap);
        let mut meta_snap = edge_meta.peek().clone();
        meta_snap.remove(&edge_id_del);
        edge_meta.set(meta_snap);
    };

    rsx! {
        div { class: "pane-section",
            div { class: "pane-section-header",
                span { class: "pane-section-title", "Condition" }
                span { class: "pane-section-pill pane-pill-edge", "edge" }
            }

            div { class: "edge-flow",
                span { class: "edge-flow-node", "{source_title}" }
                span { class: "edge-flow-arrow", "→" }
                span { class: "edge-flow-node", "{target_title}" }
            }

            label { class: "field-label", "Trigger label" }
            input {
                class: "field-input",
                value: "{meta.label}",
                oninput: on_label_input,
                placeholder: "e.g. Player picks the lock",
            }
            p { class: "field-hint",
                "Shown as a button in Play mode when the GM is on the source situation."
            }

            div { class: "field-row",
                label { class: "field-checkbox",
                    input {
                        r#type: "checkbox",
                        checked: animated,
                        onclick: on_animate_toggle,
                    }
                    span { "Animate this edge" }
                }
            }

            div { class: "pane-actions",
                button {
                    class: "btn btn-danger btn-sm",
                    onclick: on_delete,
                    IconTrash { class: "btn-icon".to_string() }
                    span { "Delete condition" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Right play panel — outgoing conditions as buttons
// ---------------------------------------------------------------------------

#[component]
fn PlayTransitions(
    edges: Signal<Vec<Edge<()>>>,
    edge_meta: Signal<HashMap<String, EdgeMeta>>,
    play_current: Signal<Option<String>>,
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
) -> Element {
    let current_id = play_current.read().clone();
    let edges_snap = edges.read().clone();
    let meta_snap = edge_meta.read().clone();
    let nm_snap = node_meta.read().clone();

    let outgoing: Vec<Edge<()>> = match &current_id {
        Some(id) => edges_snap
            .iter()
            .filter(|e| &e.source == id)
            .cloned()
            .collect(),
        None => vec![],
    };

    rsx! {
        aside { class: "scenario-pane scenario-pane-right",
            div { class: "pane-section",
                span { class: "pane-section-title", "Available transitions" }
                if current_id.is_none() {
                    p { class: "pane-empty-text",
                        "No situation active. Hit Restart in the toolbar to begin."
                    }
                } else if outgoing.is_empty() {
                    div { class: "play-deadend",
                        strong { "Dead end." }
                        p { "This situation has no outgoing condition. The scenario stops here." }
                    }
                } else {
                    div { class: "play-buttons",
                        for e in outgoing.into_iter() {
                            {
                                let edge_id = e.id.clone();
                                let target_id = e.target.clone();
                                let label = meta_snap.get(&edge_id)
                                    .map(|m| m.label.clone())
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or_else(|| format!("Transition to {}", e.target));
                                let target_title = nm_snap.get(&target_id)
                                    .map(|m| m.title.clone())
                                    .unwrap_or_else(|| target_id.clone());
                                let take = move |_: MouseEvent| {
                                    // Set the new "current" node, and reflect
                                    // it visually by toggling the `selected`
                                    // flag.
                                    play_current.set(Some(target_id.clone()));
                                    let mut snap = nodes.peek().clone();
                                    for n in snap.iter_mut() {
                                        n.selected = Some(n.id == target_id);
                                    }
                                    nodes.set(snap);
                                };
                                rsx! {
                                    button {
                                        key: "{edge_id}",
                                        class: "play-trigger-btn",
                                        onclick: take,
                                        span { class: "play-trigger-label", "{label}" }
                                        span { class: "play-trigger-target",
                                            "→ {target_title}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "pane-section",
                button {
                    class: "btn btn-ghost btn-sm",
                    onclick: move |_| {
                        let snap = nodes.peek().clone();
                        let start_id = snap
                            .iter()
                            .find(|n| n.type_.as_deref() == Some("input"))
                            .or_else(|| snap.first())
                            .map(|n| n.id.clone());
                        play_current.set(start_id.clone());
                        let mut nsnap = nodes.peek().clone();
                        for n in nsnap.iter_mut() {
                            n.selected = Some(Some(&n.id) == start_id.as_ref());
                        }
                        nodes.set(nsnap);
                    },
                    IconRestart { class: "btn-icon".to_string() }
                    span { "Restart from Start" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Graph canvas
// ---------------------------------------------------------------------------

#[component]
fn GraphCanvas(
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    edges: Signal<Vec<Edge<()>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
    edge_meta: Signal<HashMap<String, EdgeMeta>>,
    mode: Signal<ScenarioMode>,
    play_current: Signal<Option<String>>,
    context_menu: Signal<Option<ContextMenuState>>,
) -> Element {
    let current_mode = *mode.read();
    let is_play = current_mode == ScenarioMode::Play;

    let on_nodes_change = move |changes: Vec<NodeChange<BuiltInNodeData>>| {
        // Filter destructive changes in play mode so the GM can't
        // accidentally move/remove a node mid-scenario.
        let allowed: Vec<_> = if is_play {
            changes
                .into_iter()
                .filter(|c| matches!(c, NodeChange::Select { .. }))
                .collect()
        } else {
            changes
        };
        if allowed.is_empty() {
            return;
        }
        let next = apply_node_changes(allowed, nodes.peek().clone());
        nodes.set(next);
    };

    let on_edges_change = move |changes: Vec<rgraph_core::types::changes::EdgeChange<()>>| {
        if is_play {
            // Allow selection only — no edits while playing.
            let allowed: Vec<_> = changes
                .into_iter()
                .filter(|c| matches!(c, rgraph_core::types::changes::EdgeChange::Select { .. }))
                .collect();
            if allowed.is_empty() {
                return;
            }
            let next = apply_edge_changes(allowed, edges.peek().clone());
            edges.set(next);
            return;
        }

        // In edit mode: also drop the parallel edge_meta entry on remove.
        let mut to_remove: Vec<String> = Vec::new();
        for c in &changes {
            if let rgraph_core::types::changes::EdgeChange::Remove { id } = c {
                to_remove.push(id.clone());
            }
        }
        let next = apply_edge_changes(changes, edges.peek().clone());
        edges.set(next);
        if !to_remove.is_empty() {
            let mut em = edge_meta.peek().clone();
            for id in to_remove {
                em.remove(&id);
            }
            edge_meta.set(em);
        }
    };

    let on_connect = move |conn: Connection| {
        if is_play {
            return;
        }
        let mut snap = edges.peek().clone();
        let id = new_edge_id(&snap);
        snap.push(edge_from_connection(&id, &conn));
        edges.set(snap);
        // Default the new edge's label to an empty string. The inspector
        // shows a placeholder until the GM fills it in.
        let mut em = edge_meta.peek().clone();
        em.insert(id, EdgeMeta::default());
        edge_meta.set(em);
    };

    let on_pane_click = move |_: std::rc::Rc<dioxus::prelude::Event<rgraph::types::nodes::MouseData>>| {
        // Pane click closes the context menu either way.
        if context_menu.peek().is_some() {
            context_menu.set(None);
        }

        if is_play {
            return;
        }

        // Clear inspector selection by deselecting everything.
        let mut snap_n = nodes.peek().clone();
        let mut changed = false;
        for n in snap_n.iter_mut() {
            if n.selected.unwrap_or(false) {
                n.selected = Some(false);
                changed = true;
            }
        }
        if changed {
            nodes.set(snap_n);
        }
        let mut snap_e = edges.peek().clone();
        let mut changed = false;
        for e in snap_e.iter_mut() {
            if e.selected.unwrap_or(false) {
                e.selected = Some(false);
                changed = true;
            }
        }
        if changed {
            edges.set(snap_e);
        }
    };

    let on_node_context_menu = move |args: rgraph::types::nodes::NodeMouseHandlerArgs<BuiltInNodeData>| {
        if is_play {
            return;
        }

        use dioxus::html::point_interaction::InteractionLocation;
        // `prevent_default` so the webview doesn't pop its own
        // browser-style context menu over ours.
        args.event.prevent_default();
        let coords = args.event.client_coordinates();
        // Also select the node so the inspector reflects what the
        // menu acts on (matches the user's "selected entity opens
        // inspector" expectation).
        let mut snap = nodes.peek().clone();
        for n in snap.iter_mut() {
            n.selected = Some(n.id == args.node.id);
        }

        nodes.set(snap);
        let mut snap_e = edges.peek().clone();
        for e in snap_e.iter_mut() {
            if e.selected.unwrap_or(false) {
                e.selected = Some(false);
            }
        }

        edges.set(snap_e);
        context_menu.set(Some(ContextMenuState {
            x: coords.x,
            y: coords.y,
            target: ContextTarget::Node(args.node.id.clone()),
        }));
    };

    let on_edge_context_menu = move |args: rgraph::types::edges::EdgeMouseHandlerArgs<()>| {
        if is_play {
            return;
        }

        use dioxus::html::point_interaction::InteractionLocation;
        args.event.prevent_default();
        let coords = args.event.client_coordinates();
        let mut snap = edges.peek().clone();
        for e in snap.iter_mut() {
            e.selected = Some(e.id == args.edge.id);
        }

        edges.set(snap);
        let mut snap_n = nodes.peek().clone();
        for n in snap_n.iter_mut() {
            if n.selected.unwrap_or(false) {
                n.selected = Some(false);
            }
        }

        nodes.set(snap_n);
        context_menu.set(Some(ContextMenuState {
            x: coords.x,
            y: coords.y,
            target: ContextTarget::Edge(args.edge.id.clone()),
        }));
    };

    // Suppress unused-var warning when the meta signals aren't consumed
    // directly in this component (they're owned at the view root).
    let _ = node_meta;
    let _ = edge_meta;
    let _ = play_current;

    let canvas_class = if is_play {
        "scenario-canvas scenario-canvas-play"
    } else {
        "scenario-canvas"
    };

    rsx! {
        div { class: "{canvas_class}",
            RGraph::<BuiltInNodeData, ()> {
                id: "scenario-graph",
                nodes: nodes.read().clone(),
                edges: edges.read().clone(),
                nodes_draggable: Some(!is_play),
                nodes_connectable: Some(!is_play),
                edges_reconnectable: Some(!is_play),
                // Loose lets the user wire any handle to any other handle
                // — convenient while exploring the editor, since the
                // built-in node types don't all carry a source AND a
                // target (Start has source-only, End has target-only).
                connection_mode: Some(ConnectionMode::Loose),
                fit_view: Some(true),
                min_zoom: 0.3,
                max_zoom: 1.8,
                on_nodes_change: on_nodes_change,
                on_edges_change: on_edges_change,
                on_connect: on_connect,
                on_pane_click: on_pane_click,
                on_node_context_menu: on_node_context_menu,
                on_edge_context_menu: on_edge_context_menu,
                Controls::<BuiltInNodeData, ()> {}
                MiniMap::<BuiltInNodeData> { pannable: true }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Context menu overlay
// ---------------------------------------------------------------------------

#[component]
fn ScenarioContextMenu(
    nodes: Signal<Vec<Node<BuiltInNodeData>>>,
    edges: Signal<Vec<Edge<()>>>,
    node_meta: Signal<HashMap<String, NodeMeta>>,
    edge_meta: Signal<HashMap<String, EdgeMeta>>,
    context_menu: Signal<Option<ContextMenuState>>,
) -> Element {
    let Some(state) = context_menu.read().clone() else {
        return rsx! {};
    };

    let mut close = move || context_menu.set(None);

    // Delete: remove the targeted entity (and clean parallel metadata).
    let delete = {
        let target = state.target.clone();
        move || {
            match &target {
                ContextTarget::Node(id) => {
                    let mut snap = nodes.peek().clone();
                    snap.retain(|n| &n.id != id);
                    nodes.set(snap);
                    // Drop any edges connected to the removed node.
                    let mut esnap = edges.peek().clone();
                    let dropped: Vec<String> = esnap
                        .iter()
                        .filter(|e| &e.source == id || &e.target == id)
                        .map(|e| e.id.clone())
                        .collect();
                    esnap.retain(|e| &e.source != id && &e.target != id);
                    edges.set(esnap);
                    let mut em = edge_meta.peek().clone();
                    for did in dropped {
                        em.remove(&did);
                    }

                    edge_meta.set(em);
                    let mut nm = node_meta.peek().clone();
                    nm.remove(id);
                    node_meta.set(nm);
                }
                ContextTarget::Edge(id) => {
                    let mut snap = edges.peek().clone();
                    snap.retain(|e| &e.id != id);
                    edges.set(snap);
                    let mut em = edge_meta.peek().clone();
                    em.remove(id);
                    edge_meta.set(em);
                }
            }
            close();
        }
    };

    // Create child: only applies to a node. Add a new Scene node next
    // to the source and an edge from source → new node.
    let create_child = {
        let target = state.target.clone();
        move || {
            if let ContextTarget::Node(src_id) = &target {
                let snap = nodes.peek().clone();
                let Some(src) = snap.iter().find(|n| &n.id == src_id).cloned() else {
                    close();
                    return;
                };
                let mut next_nodes = snap;
                let new_id = new_node_id(&next_nodes);
                let kind = NodeKind::Scene;
                let new_x = src.position.x + 220.0;
                let new_y = src.position.y + 60.0;
                next_nodes.push(make_node(&new_id, new_x, new_y, kind, kind.default_title()));
                nodes.set(next_nodes);

                let mut nm = node_meta.peek().clone();
                nm.insert(
                    new_id.clone(),
                    NodeMeta {
                        kind,
                        title: kind.default_title().into(),
                        description: String::new(),
                        asset_hint: String::new(),
                    },
                );
                node_meta.set(nm);

                let mut esnap = edges.peek().clone();
                let eid = new_edge_id(&esnap);
                esnap.push(make_edge(&eid, src_id, &new_id));
                edges.set(esnap);
                edge_meta.set({
                    let mut em = edge_meta.peek().clone();
                    em.insert(eid, EdgeMeta::default());
                    em
                });
            }
            close();
        }
    };

    let is_node = matches!(state.target, ContextTarget::Node(_));

    let on_keydown = {
        let mut delete_kb = delete.clone();
        let mut create_kb = create_child.clone();
        move |evt: Event<KeyboardData>| {
            let key = evt.key().to_string();
            match key.as_str() {
                "Backspace" | "Delete" | "d" | "D" => {
                    evt.prevent_default();
                    delete_kb();
                }
                "c" | "C" if is_node => {
                    evt.prevent_default();
                    create_kb();
                }
                "Escape" => {
                    evt.prevent_default();
                    context_menu.set(None);
                }
                _ => {}
            }
        }
    };

    // Inset slightly so the menu doesn't sit exactly under the cursor.
    let left = state.x + 2.0;
    let top = state.y + 2.0;

    let mut delete_click = delete.clone();
    let mut create_click = create_child.clone();

    rsx! {
        // Click-out backdrop. Catches clicks outside the menu and
        // closes it. `pointer-events: auto` here makes sure we still
        // intercept the click even though the menu visually sits on
        // top of the canvas.
        div {
            class: "scenario-ctxmenu-backdrop",
            onclick: move |_| context_menu.set(None),
            oncontextmenu: move |evt| {
                evt.prevent_default();
                context_menu.set(None);
            },
        }

        div {
            class: "scenario-ctxmenu",
            style: "left: {left}px; top: {top}px;",
            tabindex: "0",
            autofocus: true,
            onkeydown: on_keydown,
            onclick: move |evt: Event<MouseData>| evt.stop_propagation(),

            div { class: "scenario-ctxmenu-header",
                match &state.target {
                    ContextTarget::Node(_) => "Situation",
                    ContextTarget::Edge(_) => "Condition",
                }
            }
            if is_node {
                button {
                    class: "scenario-ctxmenu-item",
                    onclick: move |_| create_click(),
                    span { class: "scenario-ctxmenu-label",
                        crate::components::icons::IconPlus { class: "scenario-ctxmenu-icon".to_string() }
                        "Create child"
                    }
                    span { class: "scenario-ctxmenu-kbd", "C" }
                }
            }
            button {
                class: "scenario-ctxmenu-item scenario-ctxmenu-item-danger",
                onclick: move |_| delete_click(),
                span { class: "scenario-ctxmenu-label",
                    crate::components::icons::IconTrash { class: "scenario-ctxmenu-icon".to_string() }
                    "Delete"
                }
                span { class: "scenario-ctxmenu-kbd", "⌫" }
            }
        }
    }
}
