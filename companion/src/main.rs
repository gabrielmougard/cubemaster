#![cfg_attr(not(test), windows_subsystem = "windows")]

mod ble;
mod components;
mod state;
mod store;
mod views;
mod wifi;

use dioxus::prelude::*;
use views::dashboard::DashboardView;
use views::discover::DiscoverView;
use views::settings::SettingsView;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("cubemaster_companion=debug,btleplug=info")
        .init();

    LaunchBuilder::desktop()
        .with_cfg(desktop_config())
        .launch(app);
}

fn app() -> Element {
    state::provide_app_state();

    rsx! {
        style { {include_str!("../assets/style.css")} }
        components::titlebar::Titlebar {}
        Router::<Route> {}
    }
}

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[route("/")]
    Discover,
    #[route("/dashboard")]
    Dashboard,
    #[route("/settings")]
    Settings,
}

#[component]
fn Discover() -> Element {
    rsx! { DiscoverView {} }
}

#[component]
fn Dashboard() -> Element {
    rsx! { DashboardView {} }
}

#[component]
fn Settings() -> Element {
    rsx! { SettingsView {} }
}

fn desktop_config() -> dioxus::desktop::Config {
    dioxus::desktop::Config::default()
        .with_window(
            dioxus::desktop::WindowBuilder::new()
                .with_title("CubeMaster")
                .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
                .with_min_inner_size(dioxus::desktop::LogicalSize::new(800.0, 500.0))
                .with_decorations(false),
        )
}
