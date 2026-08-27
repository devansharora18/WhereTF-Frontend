use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct FiltersPanelProps {
    pub mode: Signal<String>,
    pub power_mode: Signal<bool>,
    pub spotlight_shortcut: Signal<String>,
    pub on_spotlight_shortcut_change: EventHandler<String>,
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
            div { class: "power-toggle-section",
                div { class: "panel-heading", style: "margin-top: 24px;", "Power Search" }
                div { class: "power-toggle-row",
                    span { class: "power-toggle-label", "HyDE Expansion" }
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
                }
                div { class: "power-toggle-desc", style: "font-size: 11px; opacity: 0.6; margin-top: 4px;",
                    if *props.power_mode.read() { "Uses WordNet HyDE query expansion (slower, more recall)" } else { "Direct search without expansion (fast)" }
                }
            }
            div { class: "power-toggle-section",
                div { class: "panel-heading", style: "margin-top: 24px;", "Spotlight" }
                div { class: "power-toggle-row",
                    span { class: "power-toggle-label", "Shortcut" }
                    SpotlightShortcutInput {
                        shortcut: props.spotlight_shortcut,
                        on_change: props.on_spotlight_shortcut_change.clone(),
                    }
                }
                div { class: "power-toggle-desc", style: "font-size: 11px; opacity: 0.6; margin-top: 4px;",
                    "Global hotkey to focus search from anywhere. Click to change, then press new combo."
                }
            }
        }
    }
}

#[component]
fn SpotlightShortcutInput(shortcut: Signal<String>, on_change: EventHandler<String>) -> Element {
    let mut capturing = use_signal(|| false);
    let display = if *capturing.read() {
        "Press shortcut...".to_string()
    } else {
        shortcut.read().clone()
    };
    rsx! {
        button {
            class: if *capturing.read() { "power-toggle-btn active" } else { "power-toggle-btn" },
            style: "min-width: 80px;",
            onclick: move |_| {
                capturing.set(true);
            },
            onkeydown: move |e: KeyboardEvent| {
                if !*capturing.read() { return; }
                let code = e.code().to_string();
                // Ignore pure modifier presses
                if code == "ControlLeft" || code == "ControlRight" || code == "AltLeft"
                    || code == "AltRight" || code == "ShiftLeft" || code == "ShiftRight"
                    || code == "MetaLeft" || code == "MetaRight" {
                    return;
                }
                let mods = e.modifiers();
                let ctrl = mods.ctrl();
                let alt = mods.alt();
                let shift = mods.shift();
                let meta = mods.meta();
                if let Some(new_shortcut) = crate::spotlight::event_to_shortcut_string(ctrl, alt, shift, meta, &code) {
                    on_change.call(new_shortcut);
                }
                capturing.set(false);
            },
            onblur: move |_| capturing.set(false),
            "{display}"
        }
    }
}
