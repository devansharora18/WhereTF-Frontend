//! XDG Desktop Portal global shortcut (works on Wayland, e.g. GNOME).
//! Falls back gracefully if the portal is unavailable.

use std::sync::mpsc::Sender;
use futures::StreamExt;

pub async fn bind(
    tx: Sender<()>,
    shortcut: String,
    id: &'static str,
) -> Result<(), String> {
    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};

    let shortcuts = GlobalShortcuts::new()
        .await
        .map_err(|e| format!("portal connect failed: {e}"))?;

    let session = shortcuts
        .create_session()
        .await
        .map_err(|e| format!("create_session failed: {e}"))?;

    let ns = NewShortcut::new(id, "Open WhereTF Spotlight".to_string())
        .preferred_trigger(Some(shortcut.as_str()));
    let bound_req = shortcuts
        .bind_shortcuts(&session, std::slice::from_ref(&ns), None)
        .await
        .map_err(|e| format!("bind_shortcuts failed: {e}"))?;
    let bound = bound_req
        .response()
        .map_err(|e| format!("bind response failed: {e}"))?;
    for sc in bound.shortcuts() {
        println!(
            "[spotlight] portal bound '{}' ({}): {}",
            sc.id(),
            sc.description(),
            sc.trigger_description()
        );
    }

    let mut activated = shortcuts
        .receive_activated()
        .await
        .map_err(|e| format!("receive_activated failed: {e}"))?;
    println!("[spotlight] portal listener ready for global shortcut '{shortcut}'");
    while let Some(ev) = activated.next().await {
        if ev.shortcut_id() == id {
            let _ = tx.send(());
        }
    }
    Ok(())
}