use dioxus::prelude::*;

const BG: Asset = asset!("/assets/bg.png");
const ARROW: Asset = asset!("/assets/arrow.svg");

fn main() {
    let css = format!(
        "<style>
            * {{ margin: 0; padding: 0; box-sizing: border-box; }}
            body {{
                background-image: url('{BG}');
                background-color: #0f0f14;
                background-size: cover;
                background-position: center;
                color: #e0e0e0;
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                min-height: 100vh;
            }}
            .search-wrapper {{
                display: flex;
                justify-content: center;
                padding-top: 32px;
            }}
            .search-bar {{
                display: flex;
                align-items: center;
                width: 480px;
                height: 52px;
                padding: 6px 14px 6px 24px;
                background: rgba(226, 165, 138, 0.15);
                backdrop-filter: blur(16px);
                -webkit-backdrop-filter: blur(16px);
                border: 1px solid rgba(226, 165, 138, 0.3);
                border-radius: 28px;
                transition: all 0.3s ease;
                box-shadow:
                    0 8px 32px rgba(226, 165, 138, 0.1),
                    inset 0 1px 1px rgba(255, 255, 255, 0.08);
            }}
            .search-bar:focus-within {{
                background: rgba(226, 165, 138, 0.22);
                border-color: rgba(226, 165, 138, 0.5);
                box-shadow:
                    0 8px 40px rgba(226, 165, 138, 0.18),
                    inset 0 1px 1px rgba(255, 255, 255, 0.12);
            }}
            .search-input {{
                flex: 1;
                background: transparent;
                border: none;
                outline: none;
                font-size: 16px;
                color: #fff;
            }}
            .search-input::placeholder {{
                color: rgba(226, 165, 138, 0.5);
            }}
            .search-btn {{
                width: 40px;
                height: 40px;
                border: none;
                background: transparent;
                border-radius: 50%;
                cursor: pointer;
                flex-shrink: 0;
                transition: background 0.2s;
                padding: 0;
                display: flex;
                align-items: center;
                justify-content: center;
            }}
            .search-btn:hover {{
                background: rgba(226, 165, 138, 0.18);
            }}
            .arrow-icon {{
                width: 35px;
                height: 35px;
            }}
        </style>"
    );

    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus_desktop::Config::new()
                .with_custom_head(css)
                .with_menu(None)
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
    rsx! {
        div { class: "search-wrapper",
            div { class: "search-bar",
                input {
                    class: "search-input",
                    r#type: "text",
                    placeholder: "Search...",
                }
                button { class: "search-btn",
                    img {
                        class: "arrow-icon",
                        src: "{ARROW}",
                    }
                }
            }
        }
    }
}
