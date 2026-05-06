#![cfg_attr(not(test), windows_subsystem = "windows")]

use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(server_only_config())
        .launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            style: "max-width: 600px; margin: 80px auto; font-family: sans-serif;",
            h1 { "CubeMaster Companion" }
            p { "Connect your CubeMaster device and manage sound banks." }
            p { style: "color: #888;", "Dioxus desktop app — MVP bootstrap" }
        }
    }
}

fn server_only_config() -> dioxus::desktop::Config {
    dioxus::desktop::Config::default()
}
