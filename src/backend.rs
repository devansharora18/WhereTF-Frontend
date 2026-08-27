use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const HEALTH_URL: &str = "http://127.0.0.1:8000/health";
const STARTUP_TIMEOUT_SECS: u64 = 120;
const POLL_INTERVAL_MS: u64 = 1000;

/// Find WhereTF-backend directory by probing candidates.
pub fn find_backend_dir() -> Option<PathBuf> {
    let candidates = [
        // When running `cargo run` from WhereTF-Frontend
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("WhereTF-backend"),
        // If backend is sibling (not nested)
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("WhereTF-backend"),
        // Relative to current exe (installed app)
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("WhereTF-backend")))
            .unwrap_or_default(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().and_then(|d| d.parent()).map(|d| d.join("WhereTF-backend")))
            .unwrap_or_default(),
        // Current working dir
        std::env::current_dir().unwrap_or_default().join("WhereTF-backend"),
        std::env::current_dir().unwrap_or_default(),
        PathBuf::from("WhereTF-backend"),
        PathBuf::from("."),
    ];

    for c in candidates {
        let dc = c.join("docker-compose.yml");
        let dc2 = c.join("compose.yml");
        if dc.exists() || dc2.exists() {
            // Ensure it also has app/main.py to confirm it's backend
            if c.join("app").join("main.py").exists() || c.join("app").exists() {
                return Some(c.canonicalize().unwrap_or(c));
            }
        }
    }
    None
}

pub async fn is_healthy() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build();
    if let Ok(c) = client {
        if let Ok(resp) = c.get(HEALTH_URL).send().await {
            return resp.status().is_success();
        }
    }
    false
}

pub fn is_healthy_blocking() -> bool {
    // Use curl-like quick check without async runtime
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build();
    if let Ok(c) = client {
        if let Ok(resp) = c.get(HEALTH_URL).send() {
            return resp.status().is_success();
        }
    }
    false
}

fn which(cmd: &str) -> Option<PathBuf> {
    if let Ok(paths) = std::env::var("PATH") {
        for p in std::env::split_paths(&paths) {
            let full = p.join(cmd);
            if full.exists() {
                return Some(full);
            }
        }
    }
    // Check common podman-compose location
    let home = std::env::var("HOME").unwrap_or_default();
    let local = PathBuf::from(home).join(".local/bin").join(cmd);
    if local.exists() {
        return Some(local);
    }
    None
}

fn images_exist() -> bool {
    // Check if required images are already present
    if let Ok(out) = Command::new("podman")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        // Need at least one backend variant + pgvector + redis
        let has_backend = s.contains("wheretf-backend");
        let has_pgvector = s.contains("pgvector");
        let has_redis = s.contains("redis");
        if has_backend && has_pgvector && has_redis {
            return true;
        }
        println!(
            "[backend] images_exist check: backend={}, pgvector={}, redis={}",
            has_backend, has_pgvector, has_redis
        );
    }
    false
}

fn find_images_tar(backend_dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    // Direct in backend dir (dev + offline bundle)
    candidates.push(backend_dir.join("backend-images.tar"));
    candidates.push(backend_dir.join("backend-images.tar.gz"));
    // Cargo manifest
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("WhereTF-backend/backend-images.tar"));
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("WhereTF-backend/backend-images.tar.gz"));
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("backend-images.tar"));
    // Next to exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("backend-images.tar"));
            candidates.push(parent.join("backend-images.tar.gz"));
            candidates.push(parent.join("WhereTF-backend/backend-images.tar"));
            candidates.push(parent.join("resources/backend-images.tar"));
            candidates.push(parent.join("resources/backend-images.tar.gz"));
            if let Some(gp) = parent.parent() {
                candidates.push(gp.join("backend-images.tar"));
                candidates.push(gp.join("WhereTF-backend/backend-images.tar"));
            }
        }
    }
    // Cwd
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("WhereTF-backend/backend-images.tar"));
        candidates.push(cwd.join("backend-images.tar"));
    }
    candidates.push(PathBuf::from("backend-images.tar"));
    candidates.push(PathBuf::from("WhereTF-backend/backend-images.tar"));

    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

fn ensure_images_loaded(backend_dir: &Path) {
    if images_exist() {
        println!("[backend] images already present, skipping load");
        return;
    }
    println!("[backend] images missing, probing for bundled offline tar...");
    let Some(tar) = find_images_tar(backend_dir) else {
        println!("[backend] no bundled tar found, will rely on podman-compose pull/build (requires internet)");
        return;
    };
    println!(
        "[backend] found bundled tar at {:?} ({} bytes), loading via podman load (may take 30-60s for 1.9GB)...",
        tar,
        tar.metadata().map(|m| m.len()).unwrap_or(0)
    );
    // podman load handles both .tar and .tar.gz
    let out = Command::new("podman")
        .args(["load", "-i"])
        .arg(&tar)
        .output();
    match out {
        Ok(o) => {
            println!("[backend] podman load stdout: {}", String::from_utf8_lossy(&o.stdout));
            let stderr = String::from_utf8_lossy(&o.stderr);
            if !stderr.is_empty() {
                eprintln!("[backend] podman load stderr: {}", stderr);
            }
            if o.status.success() {
                println!("[backend] offline images loaded successfully");
            } else {
                eprintln!("[backend] podman load failed with status {:?}", o.status);
            }
        }
        Err(e) => eprintln!("[backend] failed to spawn podman load: {}", e),
    }
}

fn run_compose_up(backend_dir: &Path) -> Result<(), String> {
    // Try order: podman-compose -> podman compose -> docker-compose -> docker compose
    let attempts: Vec<Vec<String>> = vec![
        vec!["podman-compose".into(), "up".into(), "-d".into()],
        vec!["podman".into(), "compose".into(), "up".into(), "-d".into()],
        vec!["docker-compose".into(), "up".into(), "-d".into()],
        vec!["docker".into(), "compose".into(), "up".into(), "-d".into()],
    ];

    let mut last_err = String::new();
    for args in attempts {
        let bin = &args[0];
        // For "podman compose" style, bin is podman/compose, need to check podman exists
        let check_bin = if bin == "podman" && args.len() > 1 && args[1] == "compose" {
            "podman"
        } else {
            bin.as_str()
        };
        if which(check_bin).is_none() && Command::new(check_bin).arg("--version").output().is_err() {
            last_err = format!("{} not found", check_bin);
            continue;
        }

        let mut cmd = Command::new(&args[0]);
        for a in &args[1..] {
            cmd.arg(a);
        }
        cmd.current_dir(backend_dir);
        // For podman rootless, ensure env
        println!("[backend] trying: {:?} in {:?}", args, backend_dir);
        match cmd.output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("[backend] stdout: {}", stdout);
                if !stderr.is_empty() {
                    eprintln!("[backend] stderr: {}", stderr);
                }
                if out.status.success() {
                    return Ok(());
                } else {
                    last_err = format!("{:?} failed: {}", args, stderr);
                    // Try next
                }
            }
            Err(e) => {
                last_err = format!("failed to spawn {:?}: {}", args, e);
            }
        }
    }
    Err(last_err)
}

/// Blocking version to call from main() before dioxus launch.
/// Spawns a thread so it doesn't block UI.
pub fn ensure_backend_blocking_spawn() {
    std::thread::spawn(|| {
        if is_healthy_blocking() {
            println!("[backend] already healthy at {}", HEALTH_URL);
            return;
        }
        println!("[backend] not healthy, attempting to start...");
        let Some(dir) = find_backend_dir() else {
            eprintln!("[backend] could not find WhereTF-backend directory (probed CARGO_MANIFEST_DIR, exe parent, cwd)");
            return;
        };
        println!("[backend] found backend dir: {:?}", dir);
        // For fully offline single file: load bundled tar if images missing
        ensure_images_loaded(&dir);
        if let Err(e) = run_compose_up(&dir) {
            eprintln!("[backend] compose up failed: {}", e);
            return;
        }
        // Poll health
        for i in 0..STARTUP_TIMEOUT_SECS {
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            if is_healthy_blocking() {
                println!("[backend] healthy after {}s", i + 1);
                return;
            }
            if i % 10 == 0 {
                println!("[backend] waiting for health... {}s", i + 1);
            }
        }
        eprintln!("[backend] timed out waiting for health at {}", HEALTH_URL);
    });
}

pub async fn ensure_backend_async() -> Result<(), String> {
    if is_healthy().await {
        return Ok(());
    }
    let Some(dir) = find_backend_dir() else {
        return Err("WhereTF-backend directory not found".into());
    };
    ensure_images_loaded(&dir);
    run_compose_up(&dir)?;
    // Poll async
    for i in 0..STARTUP_TIMEOUT_SECS {
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        if is_healthy().await {
            println!("[backend] async healthy after {}s", i + 1);
            return Ok(());
        }
    }
    Err(format!("timed out waiting for {}", HEALTH_URL))
}
