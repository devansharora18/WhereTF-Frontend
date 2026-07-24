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
    let mut query = use_signal(String::new);

    rsx! {
        div { class: "search-screen",
            div { class: "search-heading", "Search your files." }
            div { class: "search-input-row",
                input {
                    class: "search-field",
                    r#type: "text",
                    placeholder: "type to search",
                    value: "{query}",
                    autofocus: true,
                    oninput: move |e| query.set(e.value()),
                    onkeydown: {
                        let q = query.clone();
                        let mut os = props.on_search.clone();
                        move |e| {
                            if e.key() == Key::Enter {
                                let v = q.read().clone();
                                if !v.trim().is_empty() {
                                    os.set(v);
                                }
                            }
                        }
                    },
                }
                span { class: "shortcut-hint", "\u{2318}K" }
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
                        move |_| {
                            let v = *ut.peek();
                            ut.set(v + 1);
                        }
                    },
                    "+ add files"
                }
            }
        }
    }
}
