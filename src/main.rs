mod components;

use components::{sidebar::Sidebar, search::SearchBar};
use dioxus::prelude::*;

const BG: Asset = asset!("/assets/bg.png");

fn main() {
    let global_css = include_str!("style.css");
    let sidebar_css = include_str!("components/sidebar.css");
    let search_css = include_str!("components/search.css");
    let bg_url = format!("background-image: url('{BG}'); background-color: #0f0f14; background-size: cover; background-position: center;");

    let css = format!(
        "<style>{global_css} body {{ {bg_url} }} {sidebar_css}{search_css}</style>"
    );

    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus_desktop::Config::new()
                .with_custom_head(css)
                .with_menu(None)
                .with_window(
                    dioxus_desktop::WindowBuilder::new()
                        .with_title("WhereTF")
                        .with_theme(Some(dioxus_desktop::tao::window::Theme::Dark))
                        .with_inner_size(dioxus_desktop::LogicalSize::new(800.0, 600.0)),
                )
        })
        .launch(app);
}

fn app() -> Element {
    rsx! {
        div { class: "app-layout",
            Sidebar {}
            div { class: "main-area",
                SearchBar {}
            }
        }
    }
}
