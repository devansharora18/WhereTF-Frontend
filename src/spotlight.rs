use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use std::str::FromStr;

/// Normalize a display key name ("K", "7", "F1") to the physical-code name
/// the `keyboard_types` `Code::from_str` expects ("KeyK", "Digit7", "F1").
fn normalize_code_name(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    // Single letter -> Key<UPPER>
    if trimmed.len() == 1 {
        let c = trimmed.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return format!("Key{}", c.to_ascii_uppercase());
        }
        if c.is_ascii_digit() {
            return format!("Digit{}", c);
        }
    }
    // If it already looks like a physical code ("KeyK", "Digit7", "Space", "F1"), pass through
    if trimmed.starts_with("Key")
        || trimmed.starts_with("Digit")
        || trimmed.starts_with("F") && trimmed.len() > 1
        || trimmed == "Space"
        || trimmed == "Enter"
        || trimmed == "Tab"
        || trimmed == "Escape"
    {
        return trimmed.to_string();
    }
    // Otherwise try uppercase single-letter style ("K" -> "KeyK")
    trimmed.to_ascii_uppercase()
}

pub fn parse_shortcut(s: &str) -> Option<HotKey> {
    // Format: "Alt+K", "Ctrl+Shift+P", "Cmd+K", "Ctrl+Shift+KeyP", etc.
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut mods = Modifiers::empty();
    let mut code_str = "";
    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "alt" | "option" => mods |= Modifiers::ALT,
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "cmd" | "command" | "win" | "meta" => mods |= Modifiers::SUPER,
            _ => code_str = part,
        }
    }
    if code_str.is_empty() {
        return None;
    }
    // Try to parse Code from string like "K", "A", "F1", "Space" -> "KeyK", "KeyA", "F1", "Space"
    let code_name = normalize_code_name(code_str);
    let code = Code::from_str(&code_name).ok()?;
    HotKey::new(Some(mods), code).into()
}

pub fn shortcut_to_string(hotkey: &HotKey) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mods = hotkey.mods;
    if mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    if mods.contains(Modifiers::SUPER) {
        #[cfg(target_os = "macos")]
        parts.push("Cmd".to_string());
        #[cfg(target_os = "windows")]
        parts.push("Win".to_string());
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        parts.push("Super".to_string());
    }
    let code_str = format!("{:?}", hotkey.key);
    let display = if code_str.starts_with("Key") && code_str.len() == 4 {
        code_str[3..].to_string()
    } else if code_str.starts_with("Digit") && code_str.len() == 6 {
        code_str[5..].to_string()
    } else {
        code_str
    };
    parts.push(display);
    parts.join("+")
}

// For capturing a new shortcut from a keyboard event in the UI.
// `code` should be the physical key code from the event (e.g. dioxus `e.code()`
// returns "KeyK", "Digit7", "F1"). Modifiers come from `e.modifiers()`.
pub fn event_to_shortcut_string(ctrl: bool, alt: bool, shift: bool, meta: bool, code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.is_empty()
        || trimmed == "Control"
        || trimmed == "Alt"
        || trimmed == "Shift"
        || trimmed == "Meta"
    {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    if ctrl { parts.push("Ctrl".to_string()); }
    if alt { parts.push("Alt".to_string()); }
    if shift { parts.push("Shift".to_string()); }
    if meta {
        #[cfg(target_os = "macos")]
        parts.push("Cmd".to_string());
        #[cfg(not(target_os = "macos"))]
        parts.push("Super".to_string());
    }
    // Store the compact, human-friendly form ("K", "7", "F1") rather than the
    // internal physical-code name ("KeyK", "Digit7"). parse_shortcut() accepts both.
    let compact = compact_code(trimmed);
    parts.push(compact);
    Some(parts.join("+"))
}

/// Convert a physical code name ("KeyK", "Digit7", "F1") to a compact display name.
pub fn compact_code(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("Key") {
        if rest.len() == 1 && rest.chars().next().unwrap().is_ascii_alphabetic() {
            return rest.to_uppercase();
        }
        return t.to_string();
    }
    if let Some(rest) = t.strip_prefix("Digit") {
        if rest.len() == 1 {
            return rest.to_string();
        }
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse() {
        assert!(parse_shortcut("Alt+K").is_some());
        assert!(parse_shortcut("Ctrl+Shift+P").is_some());
        assert!(parse_shortcut("Cmd+K").is_some());
    }
}
