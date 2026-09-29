//! Embedded backend sidecar support (feature = "embedded-backend").
//!
//! The Python backend is packaged as a single executable (PyInstaller) and
//! appended to the frontend binary by `scripts/embed_sidecar.sh`, followed by a
//! 32-byte trailer:
//!
//!   [ frontend ][ sidecar bytes ][ magic(16) | offset(u64 LE) | length(u64 LE) ]
//!
//! At runtime we read our own executable, locate the trailer, and extract the
//! sidecar to the user cache dir, then spawn it and health-check it. This avoids
//! embedding gigabytes via `include_bytes!` (which would blow up rustc).
#![cfg(feature = "embedded-backend")]

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const MAGIC: &[u8; 16] = b"WHERETFEMBEDSID\x01";
const TRAILER_LEN: u64 = 40; // magic(16) + offset(8) + length(8) + build_id(8)
const HEALTH_URL: &str = "http://127.0.0.1:8000/health";

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("wheretf")
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.exists()
    }
}

/// Extract the appended sidecar to the cache dir (idempotent).
pub fn extract() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut f = fs::File::open(&exe).map_err(|e| format!("open self: {e}"))?;
    let total = f.metadata().map_err(|e| e.to_string())?.len();
    if total < TRAILER_LEN {
        return Err("no embedded trailer".into());
    }
    f.seek(SeekFrom::End(-(TRAILER_LEN as i64))).map_err(|e| e.to_string())?;
    let mut tb = [0u8; 40];
    f.read_exact(&mut tb).map_err(|e| e.to_string())?;
    if &tb[0..16] != MAGIC {
        return Err("not an embedded build (magic mismatch); use launch.sh/podman mode".into());
    }
    let offset = u64::from_le_bytes(tb[16..24].try_into().unwrap());
    let length = u64::from_le_bytes(tb[24..32].try_into().unwrap());
    let build_id = u64::from_le_bytes(tb[32..40].try_into().unwrap());

    let out = cache_dir().join(if cfg!(windows) { "wheretf-backend.exe" } else { "wheretf-backend" });
    let marker = cache_dir().join("wheretf-backend.buildid");
    // Reuse only if size AND build id match (so upgrades re-extract).
    let marker_ok = fs::read_to_string(&marker)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        == Some(build_id);
    if marker_ok {
        if let Ok(m) = fs::metadata(&out) {
            if m.len() == length && is_executable(&out) {
                return Ok(out);
            }
        }
    }
    fs::create_dir_all(out.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = out.with_extension("part");
    f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut reader = f.take(length);
    let mut writer = fs::File::create(&tmp).map_err(|e| format!("create temp: {e}"))?;
    std::io::copy(&mut reader, &mut writer).map_err(|e| format!("extract: {e}"))?;
    drop(writer);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&tmp) {
            let mut perm = meta.permissions();
            perm.set_mode(0o755);
            let _ = fs::set_permissions(&tmp, perm);
        }
    }
    fs::rename(&tmp, &out).map_err(|e| format!("rename: {e}"))?;
    let _ = fs::write(&marker, build_id.to_string());
    println!("[embedded] extracted backend to {} (build {build_id})", out.display());
    Ok(out)
}

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("wheretf")
}

fn spawn_backend(bin: &PathBuf, tier: &str) -> Result<(), String> {
    let data = data_dir();
    // PyInstaller `onefile` re-extracts itself on every launch. On many Linux
    // distros /tmp is a RAM-backed tmpfs, so point TMPDIR at a disk cache dir
    // instead (otherwise repeated runs fill RAM with _MEI* dirs).
    let tmp = cache_dir().join("tmp");
    let _ = fs::remove_dir_all(&tmp); // clear any leaked previous extraction
    let _ = fs::create_dir_all(&tmp);
    let mut cmd = Command::new(bin);
    cmd.arg("--host").arg("127.0.0.1")
        .arg("--port").arg("8000")
        .arg("--tier").arg(tier)
        .arg("--data-dir").arg(&data)
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd.spawn().map_err(|e| format!("spawn sidecar: {e}"))?;
    println!("[embedded] backend pid {} (tier {tier})", child.id());
    Ok(())
}

fn healthy_blocking() -> bool {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| c.get(HEALTH_URL).send().ok())
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Spawn the embedded backend (in a thread) with the chosen tier and wait until healthy.
pub fn ensure_spawn(tier: &str) {
    let tier = tier.to_string();
    std::thread::spawn(move || {
        let tier = tier.as_str();
        if healthy_blocking() {
            println!("[embedded] backend already healthy");
            return;
        }
        // Give an instance that is still starting a moment to come up.
        for _ in 0..5 {
            std::thread::sleep(Duration::from_secs(1));
            if healthy_blocking() {
                println!("[embedded] backend healthy (existing instance)");
                return;
            }
        }
        // Ensure a single instance: stop any stale sidecar that isn't serving
        // (otherwise it keeps port 8000 and our fresh one can't bind).
        #[cfg(unix)]
        {
            let _ = Command::new("pkill")
                .arg("-x")
                .arg("wheretf-backend")
                .status();
            std::thread::sleep(Duration::from_secs(1));
        }
        let bin = match extract() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[embedded] {e}");
                return;
            }
        };
        if let Err(e) = spawn_backend(&bin, tier) {
            eprintln!("[embedded] {e}");
            return;
        }
        for i in 0..180 {
            std::thread::sleep(Duration::from_secs(1));
            if healthy_blocking() {
                println!("[embedded] backend healthy after {}s", i + 1);
                return;
            }
        }
        eprintln!("[embedded] backend did not become healthy in time");
    });
}
