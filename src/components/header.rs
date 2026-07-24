use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct HeaderProps {
    pub view_label: String,
}

#[component]
pub fn Header(props: HeaderProps) -> Element {
    rsx! {
        div { class: "header",
            div { class: "header-logo", "whereTF" }
            div { class: "header-view", "{props.view_label}" }
        }
    }
}
