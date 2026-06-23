use dioxus::prelude::*;

const DARK_CSS: &str = r#"
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
        background: #0f0f14;
        color: #e0e0e0;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        display: flex;
        justify-content: center;
        align-items: center;
        min-height: 100vh;
    }
    button {
        background: #2a2a3e;
        color: #e0e0e0;
        border: 1px solid #3a3a50;
        border-radius: 6px;
        padding: 8px 20px;
        font-size: 14px;
        cursor: pointer;
        transition: background 0.2s;
    }
    button:hover { background: #3a3a50; }
    h1 { margin-bottom: 12px; font-size: 24px; }
    p { margin-bottom: 12px; font-size: 16px; }
"#;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus_desktop::Config::new()
                .with_custom_head(format!("<style>{DARK_CSS}</style>"))
                .with_window(
                    dioxus_desktop::WindowBuilder::new()
                        .with_title("WhereTF")
                        .with_theme(Some(dioxus_desktop::tao::window::Theme::Dark))
                        .with_inner_size(dioxus_desktop::LogicalSize::new(800.0, 600.0)),
                )
        })
        .launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div {
            h1 { "WhereTF" }
            p { "Count: {count}" }
            button {
                onclick: move |_| count += 1,
                "Increment"
            }
        }
    }
}
