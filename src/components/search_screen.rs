use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SearchScreenProps {
    pub on_search: Signal<String>,
    pub file_count: u64,
    pub avg_time: f64,
    pub upload_trigger: Signal<i32>,
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
        }
    }
}
