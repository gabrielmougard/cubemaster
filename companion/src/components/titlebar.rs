use dioxus::prelude::*;
use dioxus::desktop::window;

/// Custom titlebar replacing the OS window decorations.
/// Provides drag-to-move and minimize/maximize/close buttons.
#[component]
pub fn Titlebar() -> Element {
    let on_minimize = move |_| {
        window().window.set_minimized(true);
    };
    let on_maximize = move |_| {
        window().toggle_maximized();
    };
    let on_close = move |_| {
        window().close();
    };
    let on_drag = move |_| {
        window().drag();
    };

    rsx! {
        div {
            class: "titlebar",
            onmousedown: on_drag,
            div { class: "titlebar-title", "CubeMaster" }
            div { class: "titlebar-controls",
                button {
                    class: "titlebar-btn titlebar-minimize",
                    onmousedown: |e| e.stop_propagation(),
                    onclick: on_minimize,
                    svg {
                        width: "12",
                        height: "12",
                        view_box: "0 0 12 12",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "1.5",
                        line { x1: "2", y1: "6", x2: "10", y2: "6" }
                    }
                }
                button {
                    class: "titlebar-btn titlebar-maximize",
                    onmousedown: |e| e.stop_propagation(),
                    onclick: on_maximize,
                    svg {
                        width: "12",
                        height: "12",
                        view_box: "0 0 12 12",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "1.5",
                        rect { x: "2", y: "2", width: "8", height: "8", rx: "1" }
                    }
                }
                button {
                    class: "titlebar-btn titlebar-close",
                    onmousedown: |e| e.stop_propagation(),
                    onclick: on_close,
                    svg {
                        width: "12",
                        height: "12",
                        view_box: "0 0 12 12",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "1.5",
                        line { x1: "2", y1: "2", x2: "10", y2: "10" }
                        line { x1: "10", y1: "2", x2: "2", y2: "10" }
                    }
                }
            }
        }
    }
}
