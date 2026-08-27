use crate::api::WatchedFolder;
use dioxus::prelude::*;
use std::collections::HashSet;

#[derive(Props, Clone, PartialEq)]
pub struct WatchdogPanelProps {
    pub folders: Vec<WatchedFolder>,
    pub add_trigger: Signal<i32>,
    pub on_remove: EventHandler<String>,
    pub on_toggle_pause: EventHandler<String>,
    pub paused: ReadSignal<HashSet<String>>,
    pub on_pause_all: EventHandler<()>,
    pub on_remove_all: EventHandler<()>,
}

#[component]
pub fn WatchdogPanel(props: WatchdogPanelProps) -> Element {
    let paused_set = props.paused.read().clone();
    let is_all_paused = !props.folders.is_empty()
        && props.folders.iter().all(|f| paused_set.contains(&f.folder_path));
    let has_folders = !props.folders.is_empty();

    rsx! {
        div { class: "watchdog-panel",
            div { class: "panel-heading", "Watched Folders" }
            div { class: "watchdog-actions",
                div {
                    class: "panel-upload-btn",
                    onclick: {
                        let mut at = props.add_trigger.clone();
                        move |_| { let v = *at.peek(); at.set(v + 1); }
                    },
                    "+ watch folder"
                }
                if has_folders {
                    button {
                        class: if is_all_paused { "watchdog-btn watchdog-btn-pause" } else { "watchdog-btn" },
                        onclick: move |_| props.on_pause_all.call(()),
                        if is_all_paused { "Resume All" } else { "Pause All" }
                    }
                    button {
                        class: "watchdog-btn watchdog-btn-danger",
                        onclick: move |_| props.on_remove_all.call(()),
                        "Remove All"
                    }
                }
            }
            if props.folders.is_empty() {
                div { class: "panel-empty", "No folders watched." }
            } else {
                for folder in props.folders.iter() {
                    {
                        let is_paused = paused_set.contains(&folder.folder_path);
                        let folder_path = folder.folder_path.clone();
                        rsx! {
                            div { class: "watchdog-row",
                                div { class: "watchdog-path", "{folder.folder_path}" }
                                div { class: if is_paused { "watchdog-status paused" } else { "watchdog-status" },
                                    if is_paused { "paused" } else { "active" }
                                }
                                button {
                                    class: "watchdog-btn watchdog-btn-small",
                                    onclick: {
                                        let path = folder_path.clone();
                                        let handler = props.on_toggle_pause.clone();
                                        move |_| handler.call(path.clone())
                                    },
                                    if is_paused { "Resume" } else { "Pause" }
                                }
                                button {
                                    class: "watchdog-btn watchdog-btn-small watchdog-btn-danger",
                                    onclick: {
                                        let path = folder_path.clone();
                                        let handler = props.on_remove.clone();
                                        move |_| handler.call(path.clone())
                                    },
                                    "Remove"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
