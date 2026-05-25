use dioxus::prelude::*;

#[component]
pub fn IconBluetooth(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "m7 7 10 10-5 5V2l5 5L7 17" }
        }
    }
}

#[component]
pub fn IconWifi(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M5 12.55a11 11 0 0 1 14.08 0" }
            path { d: "M1.42 9a16 16 0 0 1 21.16 0" }
            path { d: "M8.53 16.11a6 6 0 0 1 6.95 0" }
            circle { cx: "12", cy: "20", r: "1" }
        }
    }
}

#[component]
pub fn IconCube(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" }
            polyline { points: "3.27 6.96 12 12.01 20.73 6.96" }
            line { x1: "12", y1: "22.08", x2: "12", y2: "12" }
        }
    }
}

#[component]
pub fn IconSettings(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "12", cy: "12", r: "3" }
            path { d: "M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" }
        }
    }
}

#[component]
pub fn IconSearch(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "11", cy: "11", r: "8" }
            line { x1: "21", y1: "21", x2: "16.65", y2: "16.65" }
        }
    }
}

#[component]
pub fn IconSignal(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            line { x1: "2", y1: "20", x2: "2", y2: "16" }
            line { x1: "6", y1: "20", x2: "6", y2: "12" }
            line { x1: "10", y1: "20", x2: "10", y2: "8" }
            line { x1: "14", y1: "20", x2: "14", y2: "4" }
        }
    }
}

#[component]
pub fn IconRefresh(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            polyline { points: "23 4 23 10 17 10" }
            path { d: "M20.49 15a9 9 0 1 1-2.12-9.36L23 10" }
        }
    }
}

#[component]
pub fn IconLink(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" }
            path { d: "M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" }
        }
    }
}

#[component]
pub fn IconScenario(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            circle { cx: "5", cy: "6", r: "2.2" }
            circle { cx: "19", cy: "6", r: "2.2" }
            circle { cx: "12", cy: "13", r: "2.4" }
            circle { cx: "6", cy: "19", r: "2" }
            circle { cx: "18", cy: "19", r: "2" }
            path { d: "M7 7.3 10.4 11.7" }
            path { d: "M17 7.3 13.6 11.7" }
            path { d: "M10.6 14.6 7.6 17.4" }
            path { d: "M13.4 14.6 16.4 17.4" }
        }
    }
}

#[component]
pub fn IconPlay(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "currentColor",
            stroke: "none",
            polygon { points: "6,4 20,12 6,20" }
        }
    }
}

#[component]
pub fn IconEdit(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 20h9" }
            path { d: "M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z" }
        }
    }
}

#[component]
pub fn IconPlus(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            line { x1: "12", y1: "5", x2: "12", y2: "19" }
            line { x1: "5", y1: "12", x2: "19", y2: "12" }
        }
    }
}

#[component]
pub fn IconTrash(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            polyline { points: "3 6 5 6 21 6" }
            path { d: "M19 6 17.4 19.1A2 2 0 0 1 15.4 21H8.6a2 2 0 0 1-2-1.9L5 6" }
            path { d: "M10 11v6" }
            path { d: "M14 11v6" }
            path { d: "M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" }
        }
    }
}

#[component]
pub fn IconRestart(class: Option<String>) -> Element {
    let cls = class.unwrap_or_default();
    rsx! {
        svg {
            class: "{cls}",
            xmlns: "http://www.w3.org/2000/svg",
            width: "24",
            height: "24",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M3 12a9 9 0 1 0 3-6.7" }
            polyline { points: "3 4 3 9 8 9" }
        }
    }
}
