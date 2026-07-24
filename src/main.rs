mod api;
mod components;

use std::collections::HashMap;

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
                        .with_inner_size(dioxus_desktop::LogicalSize::new(960.0, 680.0)),
                )
        })
        .launch(app);
}

fn app() -> Element {
    let mut files = use_signal(Vec::new);
    let selected_file = use_signal(|| None::<String>);
    let search_results = use_signal(|| Vec::<api::SearchResult>::new());
    let mode = use_signal(|| "hybrid".to_string());
    let uploading = use_signal(|| false);
    let file_paths = use_signal(HashMap::<String, String>::new);

    let on_search = move |query: String| {
        let mode = mode.clone();
        let mut search_results = search_results.clone();
        spawn(async move {
            match api::search(&query, &mode(), 10).await {
                Ok(resp) => search_results.set(resp.results),
                Err(_) => {}
            }
        });
    };

    let on_open_file = use_callback(move |filename: String| {
        let p = file_paths.read().get(&filename).cloned();
        if let Some(path) = p {
            let _ = open::that(&path);
        }
    });

    let on_upload = {
        let files = files.clone();
        let uploading = uploading.clone();
        let file_paths = file_paths.clone();
        move |_| {
            let mut files = files.clone();
            let mut uploading = uploading.clone();
            let mut file_paths = file_paths.clone();
            spawn(async move {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    uploading.set(true);
                    let path_str = path.to_string_lossy().to_string();
                    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    file_paths.set({
                        let mut m = file_paths.peek().clone();
                        m.insert(filename, path_str.clone());
                        m
                    });
                    match api::upload_file(&path_str).await {
                        Ok(_) => {
                            for _ in 0..8 {
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                if let Ok(f) = api::get_all_files().await {
                                    files.set(f.clone());
                                    if !f.is_empty() {
                                        break;
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                    uploading.set(false);
                }
            });
        }
    };

    use_effect(move || {
        spawn(async move {
            if let Ok(f) = api::get_all_files().await {
                files.set(f);
            }
        });
    });

    rsx! {
        div { class: "app-layout",
            Sidebar {
                files: files.read().clone(),
                selected_file: selected_file.clone(),
                on_upload: on_upload,
                uploading: uploading(),
                mode: mode.clone(),
                on_open_file: on_open_file.clone(),
            }
            div { class: "main-area",
                SearchBar {
                    on_search: on_search,
                    search_results: search_results.read().clone(),
                    on_open_file: on_open_file.clone(),
                }
            }
        }
    }
}
