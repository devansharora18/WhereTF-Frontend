use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus_desktop::Config::new().with_window(
                dioxus_desktop::WindowBuilder::new()
                    .with_title("WhereTF")
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
