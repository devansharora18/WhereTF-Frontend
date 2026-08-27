use crate::api::SearchResult;
use dioxus::prelude::*;
use std::path::Path;

#[derive(Props, Clone, PartialEq)]
pub struct ResultsScreenProps {
    pub query: String,
    pub search_time: f64,
    pub results: Vec<SearchResult>,
    pub on_open_file: EventHandler<String>,
    pub on_search: Signal<String>,
}

#[component]
pub fn ResultsScreen(props: ResultsScreenProps) -> Element {
    let unique: Vec<&SearchResult> = {
        let mut seen = std::collections::HashSet::new();
        props.results.iter().filter(|r| seen.insert(&r.file_path)).collect()
    };
    let best = unique.first().copied();
    let related: &[&SearchResult] = if unique.len() > 1 {
        &unique[1..]
    } else {
        &[]
    };

    rsx! {
        div { class: "results-screen",
            super::search_bar::SearchBar {
                on_search: props.on_search,
                placeholder: props.query.clone(),
            }
            div { class: "results-time", "{props.search_time:.2}s" }

            if let Some(best) = best {
                div { class: "primary-result",
                    div { class: "primary-result-heading",
                        div { class: "primary-result-label", "Most relevant file" }
                        div { class: "primary-result-score", "{score_fmt(best.score)}%" }
                    }
                    div {
                        class: "primary-result-path",
                        onclick: {
                            let path = best.file_path.clone();
                            let h = props.on_open_file.clone();
                            move |_| h.call(path.clone())
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
                            let path = r.file_path.clone();
                            let h = props.on_open_file.clone();
                            move |_| h.call(path.clone())
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
