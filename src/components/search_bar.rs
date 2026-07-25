use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SearchBarProps {
    pub on_search: Signal<String>,
    pub placeholder: String,
}

#[component]
pub fn SearchBar(props: SearchBarProps) -> Element {
    let mut query = use_signal(String::new);

    let mut submit = {
        let mut os = props.on_search.clone();
        let query = query.clone();
        move || {
            let q = query.read().clone();
            if !q.trim().is_empty() {
                os.set(q);
            }
        }
    };

    rsx! {
        div { class: "search-input-row",
            input {
                class: "search-field",
                r#type: "text",
                placeholder: "{props.placeholder}",
                value: "{query}",
                autofocus: true,
                oninput: move |e| query.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Enter { submit(); }
                },
            }
            span { class: "shortcut-hint", "\u{2318}K" }
        }
    }
}
