use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct FiltersPanelProps {
    pub mode: Signal<String>,
}

#[component]
pub fn FiltersPanel(props: FiltersPanelProps) -> Element {
    rsx! {
        div { class: "filters-panel",
            div { class: "panel-heading", "Search Mode" }
            div { class: "filter-options",
                for (label, value) in [
                    ("Hybrid — vector + keyword", "hybrid"),
                    ("Vector — semantic similarity", "vector"),
                    ("Keyword — full-text match", "keyword"),
                ] {
                    label {
                        class: if props.mode.read().to_string() == value { "filter-option selected" } else { "filter-option" },
                        onclick: {
                            let mut m = props.mode.clone();
                            let v = value.to_string();
                            move |_| m.set(v.clone())
                        },
                        div { class: "filter-radio" }
                        div { class: "filter-label",
                            div { class: "filter-name", "{label.split('\u{2014}').next().unwrap_or(label)}" }
                            div { class: "filter-desc", "{label.split('\u{2014}').nth(1).unwrap_or(\"\").trim()}" }
                        }
                    }
                }
            }
        }
    }
}
