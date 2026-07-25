mod api;
mod components;

use std::collections::HashMap;
use std::time::Instant;
use components::{header::Header, step_rail::StepRail, search_screen::SearchScreen, results_screen::ResultsScreen, filters_panel::FiltersPanel, files_panel::FilesPanel};
use dioxus::prelude::*;

fn main() {
    let css = format!(
        "<style>{}{}{}{}{}{}{}{}{}</style>",
        include_str!("style.css"),
        include_str!("components/header.css"),
        include_str!("components/step_rail.css"),
        include_str!("components/search_bar.css"),
        include_str!("components/search_screen.css"),
        include_str!("components/results_screen.css"),
        include_str!("components/filters_panel.css"),
        include_str!("components/files_panel.css"),
        format!("body {{ background: #050606; }}")
    );

    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus_desktop::Config::new()
                .with_custom_head(css)
                .with_menu(None)
                .with_window(
                    dioxus_desktop::WindowBuilder::new()
                        .with_title("whereTF")
                        .with_theme(Some(dioxus_desktop::tao::window::Theme::Dark))
                        .with_inner_size(dioxus_desktop::LogicalSize::new(1100.0, 750.0)),
                )
        })
        .launch(app);
}

fn app() -> Element {
    let files = use_signal(Vec::new);
    let search_results = use_signal(|| Vec::<api::SearchResult>::new());
    let mode = use_signal(|| "hybrid".to_string());
    let uploading = use_signal(|| false);
    let file_paths = use_signal(HashMap::<String, String>::new);
    let active_step = use_signal(|| 1u8);
    let query_text = use_signal(String::new);
    let search_time = use_signal(|| 0.0);
    let file_count = use_signal(|| 0u64);
    let search_query = use_signal(String::new);
    let upload_trigger = use_signal(|| 0i32);

    use_effect(move || {
        let q = search_query.read().clone();
        if q.is_empty() { return; }
        let mode = mode.read().clone();
        let mut search_results = search_results.clone();
        let mut search_time = search_time.clone();
        let mut query_text = query_text.clone();
        query_text.set(q.clone());
        spawn(async move {
            let start = Instant::now();
            match api::search(&q, &mode, 10).await {
                Ok(resp) => {
                    search_time.set(start.elapsed().as_secs_f64());
                    search_results.set(resp.results);
                }
                Err(_) => {}
            }
        });
    });

    use_effect(move || {
        let count = upload_trigger();
        if count <= 0 { return; }
        if uploading() { return; }
        let mut uploading = uploading.clone();
        let mut files = files.clone();
        let mut file_count = file_count.clone();
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
                let _ = api::upload_file(&path_str).await;
                for _ in 0..8 {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    if let Ok(f) = api::get_all_files().await {
                        file_count.set(f.len() as u64);
                        files.set(f.clone());
                        if !f.is_empty() { break; }
                    }
                }
                uploading.set(false);
            }
        });
    });

    use_effect(move || {
        let mut files = files.clone();
        let mut file_count = file_count.clone();
        spawn(async move {
            if let Ok(f) = api::get_all_files().await {
                file_count.set(f.len() as u64);
                files.set(f);
            }
        });
    });

    let on_open_file = use_callback(move |filename: String| {
        let p = file_paths.read().get(&filename).cloned();
        if let Some(path) = p {
            let _ = open::that(&path);
        }
    });

    let view_label = match active_step() {
        1 => "/ 01 \u{00B7} SEARCH".to_string(),
        2 => "/ 02 \u{00B7} FILTERS".to_string(),
        3 => "/ 03 \u{00B7} FILES".to_string(),
        _ => "/ \u{00B7}".to_string(),
    };

    rsx! {
        div { id: "main",
            div { class: "app-shell",
                Header { view_label: view_label }
                div { class: "app-body",
                    StepRail {
                        active_step: active_step(),
                        active_setter: active_step.clone(),
                    }
                    div { class: "app-content",
                        if active_step() == 1 {
                            if search_results.read().is_empty() {
                                SearchScreen {
                                    on_search: search_query.clone(),
                                    file_count: file_count(),
                                    avg_time: 0.04,
                                    upload_trigger: upload_trigger.clone(),
                                }
                            } else {
                                ResultsScreen {
                                    query: query_text.read().clone(),
                                    search_time: search_time(),
                                    results: search_results.read().clone(),
                                    on_open_file: on_open_file.clone(),
                                    on_search: search_query.clone(),
                                }
                            }
                        } else if active_step() == 2 {
                            FiltersPanel {
                                mode: mode.clone(),
                            }
                        } else if active_step() == 3 {
                            FilesPanel {
                                files: files.read().clone(),
                                uploading: uploading(),
                                upload_trigger: upload_trigger.clone(),
                                on_open_file: on_open_file.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}
