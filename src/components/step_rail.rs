use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct StepRailProps {
    pub active_step: u8,
    pub active_setter: Signal<u8>,
}

#[component]
pub fn StepRail(props: StepRailProps) -> Element {
    rsx! {
        div { class: "step-rail",
            for i in 1..=4u8 {
                div {
                    class: if props.active_step == i { "step-square active" } else { "step-square" },
                    onclick: {
                        let mut setter = props.active_setter.clone();
                        move |_| setter.set(i)
                    },
                    div { class: "step-inner" }
                }
            }
        }
    }
}
