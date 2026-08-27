use crate::api;
use dioxus::prelude::*;
use std::time::Duration;

/// Floating, always-on-top desktop search bar (macOS Spotlight-style).
/// A pill-shaped input with live results as you type. Runs in its own window.
#[component]
pub fn SpotlightWindow() -> Element {
    let query = use_signal(String::new);
    let results = use_signal(|| Vec::<api::SearchResult>::new());
    let window = dioxus_desktop::use_window();

    // Live search: runs whenever the query changes (debounced).
    use_effect(move || {
        let q = query.read().clone();
        let mut results = results.clone();
        if q.trim().is_empty() {
            results.set(Vec::new());
            return;
        }
        // Debounce typing.
        let qq = q.clone();
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(180)).await;
            if let Ok(resp) = api::search(&qq, "hybrid", 8).await {
                results.set(resp.results);
            }
        });
    });

    rsx! {
        div { class: "sw-root",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    window.set_visible(false);
                }
            },
            div { class: "sw-pill",
                span { class: "sw-icon", "\u{2315}" }
                input {
                    class: "sw-input",
                    r#type: "text",
                    placeholder: "Search your files\u{2026}",
                    autofocus: true,
                    value: "{query}",
                    oninput: {
                        let mut q = query.clone();
                        move |e| q.set(e.value())
                    },
                }
            }
            if !query.read().trim().is_empty() {
                div { class: "sw-results",
                    if results.read().is_empty() {
                        div { class: "sw-empty", "No results" }
                    } else {
                        for r in results.read().iter() {
                            div {
                                class: "sw-result",
                                onclick: {
                                    let path = r.file_path.clone();
                                    let window = window.clone();
                                    move |_| {
                                        let _ = open::that(&path);
                                        window.set_visible(false);
                                    }
                                },
                                span { class: "sw-result-name", "{r.file_path}" }
                                span { class: "sw-result-score", "{r.score:.2}" }
                            }
                        }
                    }
                }
            }
        }
    }
}