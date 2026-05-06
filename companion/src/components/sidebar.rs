use dioxus::prelude::*;

use super::icons::*;
use crate::Route;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Discover,
    Dashboard,
    Settings,
}

impl NavItem {
    fn route(self) -> Route {
        match self {
            NavItem::Discover => Route::Discover,
            NavItem::Dashboard => Route::Dashboard,
            NavItem::Settings => Route::Settings,
        }
    }
}

#[component]
pub fn Sidebar(active: NavItem) -> Element {
    rsx! {
        nav { class: "sidebar",
            div { class: "sidebar-brand",
                IconCube { class: "brand-icon".to_string() }
                span { class: "brand-text", "CubeMaster" }
            }

            div { class: "sidebar-nav",
                SidebarItem {
                    item: NavItem::Discover,
                    label: "Discover",
                    active: active == NavItem::Discover,
                }
                SidebarItem {
                    item: NavItem::Dashboard,
                    label: "Dashboard",
                    active: active == NavItem::Dashboard,
                }
                SidebarItem {
                    item: NavItem::Settings,
                    label: "Settings",
                    active: active == NavItem::Settings,
                }
            }

            div { class: "sidebar-footer",
                span { class: "version-text", "v0.1.0" }
            }
        }
    }
}

#[component]
fn SidebarItem(item: NavItem, label: &'static str, active: bool) -> Element {
    let class = if active {
        "sidebar-item active"
    } else {
        "sidebar-item"
    };

    rsx! {
        Link {
            class: "{class}",
            to: item.route(),
            match item {
                NavItem::Discover => rsx! { IconSearch { class: "nav-icon".to_string() } },
                NavItem::Dashboard => rsx! { IconCube { class: "nav-icon".to_string() } },
                NavItem::Settings => rsx! { IconSettings { class: "nav-icon".to_string() } },
            }
            span { class: "nav-label", "{label}" }
        }
    }
}
