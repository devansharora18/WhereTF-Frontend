use dioxus::prelude::*;

const LOGO: Asset = asset!("/assets/logo.svg");
const FILTER: Asset = asset!("/assets/filter.svg");

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        div { class: "sidebar",
            img { class: "sidebar-logo", src: "{LOGO}" }
            div { class: "sidebar-buttons",
                button { class: "sidebar-icon-btn active",
                    img { class: "sidebar-icon", src: "{FILTER}" }
                }
                button { class: "sidebar-icon-btn",
                    div { class: "sidebar-placeholder" }
                }
                button { class: "sidebar-icon-btn",
                    div { class: "sidebar-placeholder" }
                }
                button { class: "sidebar-icon-btn",
                    div { class: "sidebar-placeholder" }
                }
            }
        }
    }
}
