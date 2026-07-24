use crate::api::SearchResult;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct StepRailProps {
    pub active_step: u8,
    pub go_search: Signal<u8>,
    pub search_results: Signal<Vec<SearchResult>>,
}

#[component]
pub fn StepRail(props: StepRailProps) -> Element {
    rsx! {
        div { class: "step-rail",
            div {
                class: if props.active_step == 1 { "step-square active" } else { "step-square" },
                onclick: {
                    let mut go = props.go_search.clone();
                    let mut sr = props.search_results.clone();
                    move |_| {
                        go.set(1);
                        sr.set(Vec::new());
                    }
                },
                div { class: "step-inner" }
            }
            div { class: if props.active_step == 2 { "step-square active" } else { "step-square" },
                div { class: "step-inner" }
            }
            div { class: "step-square",
                div { class: "step-inner" }
            }
            div { class: "step-square",
                div { class: "step-inner" }
            }
        }
    }
}
