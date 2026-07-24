use crate::api::IndexedFile;
use dioxus::prelude::*;
use std::path::Path;

const LOGO: Asset = asset!("/assets/logo.svg");
const FILTER: Asset = asset!("/assets/filter.svg");
const FILE_ICON: Asset = asset!("/assets/file.svg");

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub files: Vec<IndexedFile>,
    pub selected_file: Signal<Option<String>>,
    pub on_upload: EventHandler<()>,
    pub uploading: bool,
    pub mode: Signal<String>,
    pub on_open_file: EventHandler<String>,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    let active_panel = use_signal(|| 0u8);

    rsx! {
        div { class: "sidebar",
            img { class: "sidebar-logo", src: "{LOGO}" }
            div { class: "sidebar-buttons",
                button {
                    class: if active_panel() == 1 { "sidebar-icon-btn active" } else { "sidebar-icon-btn" },
                    onclick: {
                        let mut ap = active_panel.clone();
                        move |_| ap.set(if ap() == 1 { 0 } else { 1 })
                    },
                    img { class: "sidebar-icon", src: "{FILTER}" }
                }
                button {
                    class: if active_panel() == 2 { "sidebar-icon-btn active" } else { "sidebar-icon-btn" },
                    onclick: {
                        let mut ap = active_panel.clone();
                        move |_| ap.set(if ap() == 2 { 0 } else { 2 })
                    },
                    img { class: "sidebar-icon", src: "{FILE_ICON}" }
                }
                button { class: "sidebar-icon-btn",
                    div { class: "sidebar-placeholder" }
                }
                button { class: "sidebar-icon-btn",
                    div { class: "sidebar-placeholder" }
                }
            }
            if active_panel() == 1 {
                div { class: "files-panel",
                    div { class: "files-panel-header", "Search Mode" }
                    div { class: "filter-options",
                        label {
                            class: "filter-option",
                            input {
                                r#type: "radio",
                                name: "search-mode",
                                value: "hybrid",
                                checked: props.mode.read().to_string() == "hybrid",
                                onchange: {
                                    let mut mode = props.mode.clone();
                                    move |_| mode.set("hybrid".to_string())
                                }
                            }
                            span { "Hybrid" }
                        }
                        label {
                            class: "filter-option",
                            input {
                                r#type: "radio",
                                name: "search-mode",
                                value: "vector",
                                checked: props.mode.read().to_string() == "vector",
                                onchange: {
                                    let mut mode = props.mode.clone();
                                    move |_| mode.set("vector".to_string())
                                }
                            }
                            span { "Vector" }
                        }
                        label {
                            class: "filter-option",
                            input {
                                r#type: "radio",
                                name: "search-mode",
                                value: "keyword",
                                checked: props.mode.read().to_string() == "keyword",
                                onchange: {
                                    let mut mode = props.mode.clone();
                                    move |_| mode.set("keyword".to_string())
                                }
                            }
                            span { "Keyword" }
                        }
                    }
                }
            }
            if active_panel() == 2 {
                div { class: "files-panel",
                    div { class: "files-panel-header", "Indexed Files" }
                    button {
                        class: "upload-btn",
                        onclick: move |_| props.on_upload.call(()),
                        disabled: props.uploading,
                        if props.uploading { "Uploading..." } else { "+ Add Files" }
                    }
                    if props.files.is_empty() {
                        div { class: "files-empty", "No files indexed yet." }
                    } else {
                        for file in &props.files {
                            div {
                                class: "file-item",
                                key: "{file.id}",
                                onclick: {
                                    let name = Path::new(&file.file_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let handler = props.on_open_file.clone();
                                    move |_| handler.call(name.clone())
                                },
                                div { class: "file-name", "{file.file_path}" }
                                div { class: "file-meta",
                                    if let Some(ref ctx) = file.context {
                                        span { class: "file-context", "{ctx}" }
                                    }
                                    for tag in &file.tags {
                                        span { class: "file-tag", "{tag}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
