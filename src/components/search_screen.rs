use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SearchScreenProps {
    pub on_search: Signal<String>,
    pub file_count: u64,
    pub avg_time: f64,
    pub upload_trigger: Signal<i32>,
    pub power_mode: Signal<bool>,
}

#[component]
pub fn SearchScreen(props: SearchScreenProps) -> Element {
    rsx! {
        div { class: "search-screen",
            div { class: "search-heading", "Search your files." }
            super::search_bar::SearchBar {
                on_search: props.on_search.clone(),
                placeholder: "type to search".to_string(),
            }
            div { class: "search-stats",
                span { "{props.file_count} FILES INDEXED" }
                span { "\u{00B7}" }
                span { "{props.avg_time:.2}s AVG" }
                span { "\u{00B7}" }
                span {
                    class: "upload-link",
                    onclick: {
                        let mut ut = props.upload_trigger.clone();
                        move |_| { let v = *ut.peek(); ut.set(v + 1); }
                    },
                    "+ add files"
                }
            }
            div { class: "power-toggle-row",
                span { class: "power-toggle-label", "Power Search" }
                button {
                    class: if *props.power_mode.read() { "power-toggle-btn active" } else { "power-toggle-btn" },
                    onclick: {
                        let mut pm = props.power_mode.clone();
                        move |_| {
                            let cur = *pm.read();
                            pm.set(!cur);
                        }
                    },
                    if *props.power_mode.read() { "ON" } else { "OFF" }
                }
                span { class: "power-toggle-desc",
                    if *props.power_mode.read() { "HyDE + WordNet expansion" } else { "Direct search" }
                }
            }
        }
    }
}
