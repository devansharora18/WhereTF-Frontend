mod api;
#[cfg(feature = "bundled-backend")]
mod backend;
mod config;
mod portal_shortcut;
mod spotlight;
mod watcher;
mod components;

use std::collections::{HashMap, HashSet};
use std::time::Instant;
use components::{header::Header, step_rail::StepRail, search_screen::SearchScreen, results_screen::ResultsScreen, filters_panel::FiltersPanel, files_panel::FilesPanel, watchdog_panel::WatchdogPanel};
use dioxus::prelude::*;
use dioxus_desktop::WindowCloseBehaviour;

fn main() {
    // Auto-start backend only if bundled-backend feature is enabled.
    // For split mode (default), frontend is standalone and expects
    // the backend service to be running separately via `podman-compose up -d`
    // in WhereTF-backend/. Use `cargo run --features bundled-backend` for combined.
    #[cfg(feature = "bundled-backend")]
    backend::ensure_backend_blocking_spawn();

    let css = format!(
        "<style>{}{}{}{}{}{}{}{}{}{}</style>",
        include_str!("style.css"),
        include_str!("components/header.css"),
        include_str!("components/step_rail.css"),
        include_str!("components/search_bar.css"),
        include_str!("components/search_screen.css"),
        include_str!("components/results_screen.css"),
        include_str!("components/filters_panel.css"),
        include_str!("components/files_panel.css"),
        include_str!("components/watchdog_panel.css"),
        format!("body {{ background: #050606; }}")
    );

    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            dioxus_desktop::Config::new()
                .with_custom_head(css)
                .with_menu(None)
                .with_close_behaviour(WindowCloseBehaviour::WindowHides)
                .with_window(
                    dioxus_desktop::WindowBuilder::new()
                        .with_title("whereTF")
                        .with_theme(Some(dioxus_desktop::tao::window::Theme::Dark))
                        .with_inner_size(dioxus_desktop::LogicalSize::new(1100.0, 750.0)),
                )
        })
        .launch(app);
}

fn app() -> Element {
    let files = use_signal(Vec::new);
    let search_results = use_signal(|| Vec::<api::SearchResult>::new());
    let mode = use_signal(|| "hybrid".to_string());
    let power_mode = use_signal(|| false);
    let spotlight_shortcut = use_signal(|| config::get_shortcut());
    let mut spotlight_visible = use_signal(|| false);
    let uploading = use_signal(|| false);
    let file_paths = use_signal(HashMap::<String, String>::new);
    let active_step = use_signal(|| 1u8);
    let query_text = use_signal(String::new);
    let search_time = use_signal(|| 0.0);
    let file_count = use_signal(|| 0u64);
    let search_query = use_signal(String::new);
    let upload_trigger = use_signal(|| 0i32);
    let watch_trigger = use_signal(|| 0i32);
    let watch_folders = use_signal(Vec::<api::WatchedFolder>::new);
    let paused_folders = use_signal(|| std::collections::HashSet::<String>::new());
    let backend_ready = use_signal(|| false);
    let backend_error = use_signal(|| Option::<String>::None);

    // Ensure backend is running.
    // - With `bundled-backend` feature (combined app): auto-starts via podman-compose
    // - Without (default, split): just checks health, shows "Backend not available" if down
    #[cfg(feature = "bundled-backend")]
    use_effect(move || {
        let mut backend_ready = backend_ready.clone();
        let mut backend_error = backend_error.clone();
        spawn(async move {
            if backend::is_healthy().await {
                backend_ready.set(true);
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if backend::is_healthy().await {
                backend_ready.set(true);
                return;
            }
            match backend::ensure_backend_async().await {
                Ok(_) => backend_ready.set(true),
                Err(e) => backend_error.set(Some(e)),
            }
        });
    });
    #[cfg(not(feature = "bundled-backend"))]
    use_effect(move || {
        let mut backend_ready = backend_ready.clone();
        let mut backend_error = backend_error.clone();
        spawn(async move {
            // Simple health check without auto-start (split mode)
            // Poll for 5s, then show error if still not healthy
            for _ in 0..5 {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(1))
                    .build();
                if let Ok(c) = client {
                    if let Ok(resp) = c.get("http://127.0.0.1:8000/health").send().await {
                        if resp.status().is_success() {
                            backend_ready.set(true);
                            return;
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            // After 5s, still not healthy -> show message but keep polling in background
            // Check once more synchronously via api
            if let Ok(healthy) = reqwest::get("http://127.0.0.1:8000/health").await {
                if healthy.status().is_success() {
                    backend_ready.set(true);
                    return;
                }
            }
            backend_error.set(Some(
                "Backend not running. Please start it with: cd WhereTF-backend && podman-compose up -d (or docker-compose up -d)".to_string(),
            ));
            // Keep polling every 5s in background to auto-recover when service starts
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(1))
                    .build();
                if let Ok(c) = client {
                    if let Ok(resp) = c.get("http://127.0.0.1:8000/health").send().await {
                        if resp.status().is_success() {
                            backend_ready.set(true);
                            backend_error.set(None);
                            break;
                        }
                    }
                }
            }
        });
    });

    use_effect(move || {
        let q = search_query.read().clone();
        let is_power = power_mode();
        if q.is_empty() { return; }
        let mode = mode.read().clone();
        let mut search_results = search_results.clone();
        let mut search_time = search_time.clone();
        let mut query_text = query_text.clone();
        query_text.set(q.clone());
        spawn(async move {
            let start = Instant::now();
            let resp = if is_power {
                api::power_search(&q, &mode, 10).await
            } else {
                api::search(&q, &mode, 10).await
            };
            match resp {
                Ok(resp) => {
                    search_time.set(start.elapsed().as_secs_f64());
                    search_results.set(resp.results);
                }
                Err(_) => {}
            }
        });
    });

    use_effect(move || {
        let count = upload_trigger();
        if count <= 0 { return; }
        if uploading() { return; }
        let mut uploading = uploading.clone();
        let mut files = files.clone();
        let mut file_count = file_count.clone();
        let mut file_paths = file_paths.clone();
        spawn(async move {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                uploading.set(true);
                let path_str = path.to_string_lossy().to_string();
                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                file_paths.set({
                    let mut m = file_paths.peek().clone();
                    m.insert(filename, path_str.clone());
                    m
                });
                let _ = api::upload_file(&path_str).await;
                for _ in 0..8 {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    if let Ok(f) = api::get_all_files().await {
                        file_count.set(f.len() as u64);
                        files.set(f.clone());
                        if !f.is_empty() { break; }
                    }
                }
                uploading.set(false);
            }
        });
    });

    use_effect(move || {
        if !backend_ready() { return; }
        let mut files = files.clone();
        let mut file_count = file_count.clone();
        let mut wf = watch_folders.clone();
        spawn(async move {
            if let Ok(f) = api::get_all_files().await {
                file_count.set(f.len() as u64);
                files.set(f);
            }
            if let Ok(w) = api::get_watched_folders().await {
                wf.set(w.clone());
                // Start local filesystem watchers for each watched folder (Rust notify)
                for folder in w {
                    watcher::watch_folder(folder.folder_path);
                }
            }
        });
    });

    use_effect(move || {
        let count = watch_trigger();
        if count <= 0 { return; }
        let mut wf = watch_folders.clone();
        spawn(async move {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let p = path.to_string_lossy().to_string();
                match api::add_watch_folder(&p).await {
                    Ok(_) => {
                        // Start watching this folder locally (Rust notify)
                        watcher::watch_folder(p.clone());
                        if let Ok(w) = api::get_watched_folders().await {
                            wf.set(w);
                        }
                    }
                    Err(e) => {
                        eprintln!("[watcher] add_watch_folder failed: {}", e);
                    }
                }
            }
        });
    });

    let on_open_file = use_callback(move |path: String| {
        // If it's already an absolute path (contains / or \), open directly
        // Otherwise, try to resolve via file_paths map (for legacy upload flow)
        if path.contains('/') || path.contains('\\') {
            if std::path::Path::new(&path).exists() {
                let _ = open::that(&path);
            } else {
                // Fallback: try lookup by file_name
                let fname = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path)
                    .to_string();
                if let Some(p) = file_paths.read().get(&fname).cloned() {
                    let _ = open::that(&p);
                } else {
                    // Last resort: try to open as-is (will show OS error if not found)
                    let _ = open::that(&path);
                }
            }
        } else {
            // It's a file_name, lookup in map
            if let Some(p) = file_paths.read().get(&path).cloned() {
                let _ = open::that(&p);
            }
        }
    });

    let on_remove_folder = {
        let watch_folders = watch_folders.clone();
        let paused_folders = paused_folders.clone();
        use_callback(move |folder_path: String| {
            let mut watch_folders = watch_folders.clone();
            let mut paused_folders = paused_folders.clone();
            let folder_path_clone = folder_path.clone();
            spawn(async move {
                let _ = api::remove_watch_folder(&folder_path_clone).await;
                watcher::stop_watcher(&folder_path_clone);
                paused_folders.write().remove(&folder_path_clone);
                if let Ok(w) = api::get_watched_folders().await {
                    watch_folders.set(w);
                }
            });
        })
    };

    let on_toggle_pause = {
        let paused_folders = paused_folders.clone();
        use_callback(move |folder_path: String| {
            let mut paused_folders = paused_folders.clone();
            let is_paused = paused_folders.read().contains(&folder_path);
            if is_paused {
                watcher::watch_folder(folder_path.clone());
                paused_folders.write().remove(&folder_path);
            } else {
                watcher::stop_watcher(&folder_path);
                paused_folders.write().insert(folder_path);
            }
        })
    };

    let on_pause_all = {
        let watch_folders = watch_folders.clone();
        let paused_folders = paused_folders.clone();
        use_callback(move |_: ()| {
            let mut paused_folders = paused_folders.clone();
            let folders = watch_folders.read().clone();
            let is_all_paused = !folders.is_empty()
                && folders.iter().all(|f| paused_folders.read().contains(&f.folder_path));
            if is_all_paused {
                for f in &folders {
                    watcher::watch_folder(f.folder_path.clone());
                }
                paused_folders.set(HashSet::new());
            } else {
                for f in &folders {
                    watcher::stop_watcher(&f.folder_path);
                }
                let set: HashSet<String> = folders.into_iter().map(|f| f.folder_path).collect();
                paused_folders.set(set);
            }
        })
    };

    let on_remove_all = {
        let watch_folders = watch_folders.clone();
        let paused_folders = paused_folders.clone();
        use_callback(move |_: ()| {
            let mut watch_folders = watch_folders.clone();
            let mut paused_folders = paused_folders.clone();
            spawn(async move {
                let folders = watch_folders.read().clone();
                for f in folders {
                    let _ = api::remove_watch_folder(&f.folder_path).await;
                    watcher::stop_watcher(&f.folder_path);
                }
                paused_folders.set(HashSet::new());
                if let Ok(w) = api::get_watched_folders().await {
                    watch_folders.set(w);
                } else {
                    watch_folders.set(Vec::new());
                }
            });
        })
    };

    let on_spotlight_shortcut_change = {
        let spotlight_shortcut = spotlight_shortcut.clone();
        use_callback(move |new_shortcut: String| {
            config::set_shortcut(&new_shortcut);
            let mut sc = spotlight_shortcut.clone();
            sc.set(new_shortcut.clone());
            println!("[spotlight] shortcut changed to {}, restart required", new_shortcut);
        })
    };

    // Spotlight trigger channel.
    // Multiple sources (X11 global hotkey, XDG portal for Wayland, tray menu,
    // in-app key) all send `()` to this one Sender; a single UI-side poller
    // toggles `spotlight_visible`. Created once via use_hook.
    let spotlight_trigger = use_hook(move || {
        let (tx, rx) = std::sync::mpsc::channel::<()>();

        // UI-side poller: forward triggers to the spotlight_visible signal.
        let mut spotlight_visible = spotlight_visible.clone();
        let mut active_step = active_step.clone();
        let mut search_query = search_query.clone();
        let mut search_results = search_results.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(60)).await;
                while rx.try_recv().is_ok() {
                    let cur = *spotlight_visible.read();
                    let next = !cur;
                    if next {
                        // Opening spotlight: start fresh, just the searchbar
                        search_query.set(String::new());
                        search_results.set(Vec::new());
                    }
                    spotlight_visible.set(next);
                    active_step.set(1);
                    println!("[spotlight] toggled visible -> {}", next);
                }
            }
        });

        // #1 X11 global hotkey backend (works on X11 + non-GNOME Wayland/XWayland).
        let shortcut = config::get_shortcut();
        let tx_x11 = tx.clone();
        std::thread::spawn(move || {
            use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
            let Some(hotkey) = crate::spotlight::parse_shortcut(&shortcut) else {
                eprintln!("[spotlight] invalid shortcut: {}", shortcut);
                return;
            };
            let manager = match GlobalHotKeyManager::new() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[spotlight] X11 hotkey manager error: {:?}", e);
                    return;
                }
            };
            if let Err(e) = manager.register(hotkey) {
                eprintln!(
                    "[spotlight] X11 global hotkey register failed ({}); GNOME Wayland needs \
                     the portal fallback instead",
                    e
                );
                return;
            }
            println!("[spotlight] X11 global hotkey registered: {}", shortcut);
            let receiver = GlobalHotKeyEvent::receiver();
            loop {
                if let Ok(event) = receiver.recv() {
                    if event.id == hotkey.id() {
                        let _ = tx_x11.send(());
                    }
                }
            }
        });

        tx
    });

    // #2 XDG Desktop Portal (GlobalShortcuts) - the Wayland/GNOME path.
    {
        let tx = spotlight_trigger.clone();
        let shortcut = config::get_shortcut();
        spawn(async move {
            match crate::portal_shortcut::bind(tx, shortcut, "open-spotlight").await {
                Ok(_) => {}
                Err(e) => eprintln!("[spotlight] XDG portal not available: {e}"),
            }
        });
    }

    // #3 Tray menu "Open Spotlight…" as a guaranteed path on every desktop.
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        use dioxus_desktop::trayicon;
        let tx = spotlight_trigger.clone();
        let _tray = use_hook(move || {
            use dioxus_desktop::trayicon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
            use dioxus_desktop::trayicon::TrayIconBuilder;
            let menu = Menu::new();
            let open_item = MenuItem::with_id("wheretf-open", "Open Spotlight…", true, None);
            let _ = menu.append(&open_item);
            let _ = menu.append(&PredefinedMenuItem::separator());
            let _ = menu.append(&PredefinedMenuItem::quit(None));
            let open_id = open_item.id().clone();
            let t = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_tooltip("WhereTF")
                .build()
                .ok();
            // Drive tray clicks into the spotlight channel.
            std::thread::spawn(move || {
                let rx = MenuEvent::receiver();
                loop {
                    if let Ok(ev) = rx.recv() {
                        if ev.id == open_id {
                            let _ = tx.send(());
                        }
                    }
                }
            });
            t
        });
    }

    // In-window Alt+K fallback (works whenever the app window is focused, including
    // on GNOME Wayland where a true global grab from a non-installed dev binary isn't
    // permitted). Uses the same trigger channel as X11/portal/tray.
    let spotlight_tx_inapp = spotlight_trigger.clone();
    let spotlight_inapp_handler = move |e: KeyboardEvent| {
        if e.modifiers().alt() && e.code().to_string() == "KeyK" {
            let _ = spotlight_tx_inapp.send(());
        }
    };

    // Build the floating Spotlight window once (frameless, always-on-top, centered,
    // hidden until shown). Store its handle so the show/hide effect can control it.
    let desktop_window = dioxus_desktop::use_window();
    let spotlight_window = use_signal(|| None::<dioxus_desktop::DesktopContext>);
    {
        let mut spotlight_window = spotlight_window.clone();
        use_effect(move || {
            let desktop_window = desktop_window.clone();
            spawn(async move {
                use dioxus_desktop::tao::dpi::{PhysicalPosition, PhysicalSize, Position};
                use dioxus_desktop::tao::window::WindowBuilder;
                use dioxus_desktop::Config;
                // Center it on the primary monitor.
                let (w, h) = desktop_window
                    .current_monitor()
                    .map(|m| m.size().to_logical::<f64>(1.0))
                    .map(|s| (s.width as i32, s.height as i32))
                    .unwrap_or((1280, 800));
                let (sw, sh) = (1230, 420); // 1.7x wider pill, room for live results
                let pos = Position::Physical(PhysicalPosition::new((w - sw) / 2, (h - sh) / 2));
                let css = format!(
                    "<style>{}</style>",
                    include_str!("components/spotlight_window.css")
                );
                let wb = WindowBuilder::new()
                    .with_title("WhereTF Spotlight")
                    .with_inner_size(PhysicalSize::new(sw, sh))
                    .with_position(pos)
                    .with_decorations(false)
                    .with_always_on_top(true)
                    .with_transparent(true)
                    .with_visible(false);
                let cfg = Config::new().with_window(wb).with_custom_head(css);
                let dom = dioxus_core::VirtualDom::new(components::spotlight_window::SpotlightWindow);
                let win = dioxus_desktop::window().new_window(dom, cfg).await;
                println!("[spotlight] floating window ready");
                spotlight_window.set(Some(win));
            });
        });
    }

    // Show/hide the floating spotlight window in response to triggers.
    let mut spotlight_window = spotlight_window.clone();
    use_effect(move || {
        let v = spotlight_visible();
        if let Some(win) = spotlight_window.read().as_ref() {
            if v {
                win.set_always_on_top(true);
                win.set_visible(true);
                win.set_focus();
                println!("[spotlight] floating window shown");
            } else {
                win.set_visible(false);
            }
        }
    });

    let view_label = match active_step() {
        1 => "/ 01 \u{00B7} SEARCH".to_string(),
        2 => "/ 02 \u{00B7} FILTERS".to_string(),
        3 => "/ 03 \u{00B7} FILES".to_string(),
        4 => "/ 04 \u{00B7} WATCH".to_string(),
        _ => "/ \u{00B7}".to_string(),
    };

    rsx! {
        // The main app window always renders normally. Spotlight is a separate
        // floating window (ctrl + top spotlight_window handle) shown/hidden above.
        div { id: "main",
              onkeydown: spotlight_inapp_handler,
              div { class: "app-shell",
                Header { view_label: view_label }
                if !backend_ready() {
                    div { class: "backend-loading", style: "padding: 48px; font-family: var(--font-mono); color: var(--text-secondary);",
                        if let Some(err) = backend_error.read().as_ref() {
                            div { style: "color: #ff6b6b; margin-bottom: 12px;", "Backend failed to start:" }
                            div { style: "font-size: 12px; word-break: break-all;", "{err}" }
                            div { style: "margin-top: 16px; font-size: 12px;", "Ensure podman and podman-compose are installed and WhereTF-backend/docker-compose.yml exists." }
                            div { style: "margin-top: 12px; font-size: 12px;", "You can also run manually: cd WhereTF-backend && podman-compose up -d" }
                        } else {
                            div { "Starting backend — db + api + redis + worker" }
                            div { style: "margin-top: 8px; font-size: 12px; opacity: 0.7;", "First launch may take 60-90s (Jina CLIP model download)..." }
                        }
                    }
                } else {
                    div { class: "app-body",
                        StepRail {
                            active_step: active_step(),
                            active_setter: active_step.clone(),
                        }
                        div { class: "app-content",
                        if active_step() == 1 {
                            if search_results.read().is_empty() {
                                SearchScreen {
                                    on_search: search_query.clone(),
                                    file_count: file_count(),
                                    avg_time: 0.04,
                                    upload_trigger: upload_trigger.clone(),
                                    power_mode: power_mode,
                                }
                            } else {
                                ResultsScreen {
                                    query: query_text.read().clone(),
                                    search_time: search_time(),
                                    results: search_results.read().clone(),
                                    on_open_file: on_open_file.clone(),
                                    on_search: search_query.clone(),
                                }
                            }
                        } else if active_step() == 2 {
                            FiltersPanel {
                                mode: mode.clone(),
                                power_mode: power_mode,
                                spotlight_shortcut: spotlight_shortcut,
                                on_spotlight_shortcut_change: on_spotlight_shortcut_change.clone(),
                            }
                        } else if active_step() == 3 {
                            FilesPanel {
                                files: files.read().clone(),
                                uploading: uploading(),
                                upload_trigger: upload_trigger.clone(),
                                on_open_file: on_open_file.clone(),
                            }
                        } else if active_step() == 4 {
                            WatchdogPanel {
                                folders: watch_folders.read().clone(),
                                add_trigger: watch_trigger.clone(),
                                on_remove: on_remove_folder.clone(),
                                on_toggle_pause: on_toggle_pause.clone(),
                                paused: paused_folders,
                                on_pause_all: on_pause_all.clone(),
                                on_remove_all: on_remove_all.clone(),
                            }
                        }
                    }
                }
            }
            }
        }
    }
}
