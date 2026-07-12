use crate::api::SearchResult;
use dioxus::prelude::*;

const ARROW: Asset = asset!("/assets/arrow.svg");

#[derive(Props, Clone, PartialEq)]
pub struct SearchBarProps {
    pub on_search: EventHandler<String>,
    pub search_results: Vec<SearchResult>,
}

#[component]
pub fn SearchBar(props: SearchBarProps) -> Element {
    let mut query = use_signal(String::new);

    let do_search = {
        let query = query.clone();
        let on_search = props.on_search.clone();
        move || {
            let q = query.read().clone();
            if !q.trim().is_empty() {
                on_search.call(q);
            }
        }
    };

    rsx! {
        div { class: "main-content",
            div { class: "search-wrapper",
                div { class: "search-bar",
                    input {
                        class: "search-input",
                        r#type: "text",
                        placeholder: "Search your documents...",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                        onkeydown: {
                            let do_search = do_search.clone();
                            move |e| {
                                if e.key() == Key::Enter {
                                    do_search();
                                }
                            }
                        },
                    }
                    button {
                        class: "search-btn",
                        onclick: move |_| do_search(),
                        img { class: "arrow-icon", src: "{ARROW}" }
                    }
                }
            }
            if !props.search_results.is_empty() {
                div { class: "search-results",
                    for result in &props.search_results {
                        div { class: "result-item",
                            div { class: "result-path", "{result.file_path}" }
                            div { class: "result-content", "{result.content}" }
                            div { class: "result-score", "Score: {result.score:.4}" }
                        }
                    }
                }
            }
        }
    }
}
