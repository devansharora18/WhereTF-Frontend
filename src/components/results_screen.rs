use crate::api::SearchResult;
use dioxus::prelude::*;
use std::path::Path;

#[derive(Props, Clone, PartialEq)]
pub struct ResultsScreenProps {
    pub query: String,
    pub search_time: f64,
    pub results: Vec<SearchResult>,
    pub on_open_file: EventHandler<String>,
}

#[component]
pub fn ResultsScreen(props: ResultsScreenProps) -> Element {
    let best = props.results.first();
    let related = if props.results.len() > 1 {
        &props.results[1..]
    } else {
        &[]
    };

    rsx! {
        div { class: "results-screen",
            div { class: "results-query-row",
                span { class: "results-query-text", "{props.query}" }
                span { class: "cursor" }
                span { class: "results-query-time", "{props.search_time:.2}s" }
            }

            if let Some(best) = best {
                div { class: "primary-result",
                    div { class: "primary-result-heading",
                        div { class: "primary-result-label", "Most relevant file" }
                        div { class: "primary-result-score", "{score_fmt(best.score)}%" }
                    }
                    div {
                        class: "primary-result-path",
                        onclick: {
                            let name = file_name(&best.file_path);
                        let h = props.on_open_file.clone();
                        move |_| h.call(name.clone())
                        },
                        "{best.file_path}"
                    }
                }
            }

            if !related.is_empty() {
                hr { class: "section-divider" }
                div { class: "related-heading", "RELATED \u{2014} {related.len()}" }

                for r in related {
                    div {
                        class: "result-row",
                        onclick: {
                            let name = file_name(&r.file_path);
                        let h = props.on_open_file.clone();
                        move |_| h.call(name.clone())
                        },
                        div { class: "result-row-name", "{file_name(&r.file_path)}" }
                        div { class: "result-row-score", "{score_fmt(r.score)}%" }
                    }
                }
            }
        }
    }
}

fn score_fmt(score: f64) -> String {
    format!("{:.0}", (score * 100.0).min(99.0).max(1.0))
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string()
}
