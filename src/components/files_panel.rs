use crate::api::IndexedFile;
use dioxus::prelude::*;
use std::path::Path;

#[derive(Props, Clone, PartialEq)]
pub struct FilesPanelProps {
    pub files: Vec<IndexedFile>,
    pub uploading: bool,
    pub upload_trigger: Signal<i32>,
    pub on_open_file: EventHandler<String>,
}

#[component]
pub fn FilesPanel(props: FilesPanelProps) -> Element {
    rsx! {
        div { class: "files-panel",
            div { class: "panel-heading", "Indexed Files" }
            div {
                class: "panel-upload-btn",
                onclick: {
                    let mut ut = props.upload_trigger.clone();
                    move |_| { let v = *ut.peek(); ut.set(v + 1); }
                },
                if props.uploading { "Uploading..." } else { "+ add files" }
            }
            if props.files.is_empty() {
                div { class: "panel-empty", "No files indexed yet." }
            } else {
                for file in &props.files {
                    div {
                        class: "panel-file-row",
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
                        div { class: "panel-file-name", "{file.file_path}" }
                        div { class: "panel-file-type", "{file.mime_type}" }
                    }
                }
            }
        }
    }
}
