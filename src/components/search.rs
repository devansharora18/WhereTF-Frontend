use dioxus::prelude::*;

const ARROW: Asset = asset!("/assets/arrow.svg");

#[component]
pub fn SearchBar() -> Element {
    rsx! {
        div { class: "search-wrapper",
            div { class: "search-bar",
                input {
                    class: "search-input",
                    r#type: "text",
                    placeholder: "Search...",
                }
                button { class: "search-btn",
                    img { class: "arrow-icon", src: "{ARROW}" }
                }
            }
        }
    }
}
