use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};
use walkdir::WalkDir;

use crate::api;

static WATCHERS: std::sync::OnceLock<Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>>> =
    std::sync::OnceLock::new();

fn watchers_map() -> Arc<Mutex<HashMap<String, notify::RecommendedWatcher>>> {
    WATCHERS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

fn debounce_map() -> Arc<Mutex<HashMap<PathBuf, Instant>>> {
    static DEBOUNCE: std::sync::OnceLock<Arc<Mutex<HashMap<PathBuf, Instant>>>> =
        std::sync::OnceLock::new();
    DEBOUNCE
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

const DEBOUNCE_SECS: f64 = 1.5;

fn should_process(path: &Path) -> bool {
    let now = Instant::now();
    let debounce = debounce_map();
    let mut map = debounce.lock().unwrap();
    if let Some(last) = map.get(path) {
        if now.duration_since(*last).as_secs_f64() < DEBOUNCE_SECS {
            return false;
        }
    }
    map.insert(path.to_path_buf(), now);
    // Clean old entries occasionally
    if map.len() > 1000 {
        map.retain(|_, t| now.duration_since(*t).as_secs_f64() < 10.0);
    }
    true
}

pub fn calculate_file_hash(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(path).map_err(|e| format!("open failed: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read failed: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

async fn handle_created(path: &Path) {
    if !should_process(path) {
        return;
    }
    // Wait a bit for file to be fully written (like python's time.sleep(1))
    tokio::time::sleep(Duration::from_secs(1)).await;
    if !path.exists() || path.is_dir() {
        return;
    }
    println!("[watcher] Created: {:?}", path);
    let p = path.to_string_lossy().to_string();
    match api::upload_file(&p).await {
        Ok(r) => println!("[watcher] upload ok: {:?}", r),
        Err(e) => eprintln!("[watcher] upload failed for {:?}: {}", path, e),
    }
}

async fn handle_modified(path: &Path) {
    if !should_process(path) {
        return;
    }
    if !path.exists() || path.is_dir() {
        return;
    }
    println!("[watcher] Modified: {:?}", path);
    // For modify, the python does delete+upload (modify). We'll try that.
    let p = path.to_string_lossy().to_string();
    // First try to get hash and check needs_indexing to avoid redundant upload
    if let Ok(hash) = calculate_file_hash(path) {
        if let Ok(needs) = api::needs_indexing(&p, &hash).await {
            if !needs {
                println!("[watcher] Skipping modified (no change): {:?}", path);
                return;
            }
        }
    }
    // Use upload (which will create or update). The backend's process_file_task handles
    // existing file_path by setting state=processing and re-indexing.
    match api::upload_file(&p).await {
        Ok(r) => println!("[watcher] re-index ok: {:?}", r),
        Err(e) => eprintln!("[watcher] re-index failed for {:?}: {}", path, e),
    }
}

async fn handle_deleted(path: &Path) {
    // Don't debounce deletes - they are distinct
    println!("[watcher] Deleted: {:?}", path);
    let p = path.to_string_lossy().to_string();
    match api::delete_file_by_path(&p).await {
        Ok(r) => println!("[watcher] delete ok: {:?}", r),
        Err(e) => eprintln!("[watcher] delete failed for {:?}: {}", path, e),
    }
}

async fn handle_moved(from: &Path, to: &Path) {
    println!("[watcher] Moved: {:?} -> {:?}", from, to);
    let old = from.to_string_lossy().to_string();
    let new = to.to_string_lossy().to_string();
    match api::rename_file(&old, &new).await {
        Ok(r) => println!("[watcher] rename ok: {:?}", r),
        Err(e) => {
            eprintln!("[watcher] rename failed: {}, trying upload+delete", e);
            // Fallback: upload new, delete old
            let _ = api::upload_file(&new).await;
            let _ = api::delete_file_by_path(&old).await;
        }
    }
}

pub async fn scan_folder(folder_path: &str) {
    let path = Path::new(folder_path);
    if !path.exists() {
        eprintln!("[watcher] scan_folder: not found {:?}", folder_path);
        return;
    }
    println!("[watcher] Scanning folder: {}", folder_path);
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let file_path = entry.path();
        let p = file_path.to_string_lossy().to_string();
        match calculate_file_hash(file_path) {
            Ok(hash) => match api::needs_indexing(&p, &hash).await {
                Ok(true) => {
                    println!("[watcher] Indexing: {}", p);
                    if let Err(e) = api::upload_file(&p).await {
                        eprintln!("[watcher] Failed to index {}: {}", p, e);
                    }
                }
                Ok(false) => {
                    // Skipping
                }
                Err(e) => eprintln!("[watcher] needs_indexing check failed for {}: {}", p, e),
            },
            Err(e) => eprintln!("[watcher] hash failed for {}: {}", p, e),
        }
        // Small yield to avoid blocking
        tokio::task::yield_now().await;
    }
    println!("[watcher] Scan complete for {}", folder_path);
}

pub fn watch_folder(folder_path: String) {
    let path = PathBuf::from(&folder_path);
    if !path.exists() {
        eprintln!("[watcher] watch_folder: path does not exist: {}", folder_path);
        return;
    }
    // Avoid duplicate watchers
    {
        let watchers = watchers_map();
        let map = watchers.lock().unwrap();
        if map.contains_key(&folder_path) {
            println!("[watcher] Already watching {}", folder_path);
            return;
        }
    }

    println!("[watcher] Starting watcher for {}", folder_path);

    // Spawn a thread for the watcher (notify is sync, needs thread)
    let folder_clone = folder_path.clone();
    std::thread::spawn(move || {
        // Setup watcher FIRST so events are not missed during initial scan
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: notify::RecommendedWatcher =
            notify::Watcher::new(tx, notify::Config::default()).unwrap();
        watcher
            .watch(Path::new(&folder_clone), RecursiveMode::Recursive)
            .unwrap();

        // Store watcher so it isn't dropped
        {
            let watchers = watchers_map();
            let mut map = watchers.lock().unwrap();
            map.insert(folder_clone.clone(), watcher);
        }

        println!("[watcher] Watching {} (recursive)", folder_clone);

        // Initial scan in background (don't block watcher)
        let folder_clone2 = folder_clone.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                scan_folder(&folder_clone2).await;
            });
        });

        // Event loop
        for res in rx {
            match res {
                Ok(event) => {
                    // Spawn async handling for each event
                    let event_clone = event.clone();
                    // Use a new runtime for async api calls
                    // We can't easily spawn tokio from sync thread, so use block_on with new runtime
                    // Instead, we spawn via tokio::task if runtime exists, but we're in sync thread.
                    // So we handle by spawning a temporary runtime
                    let rt2 = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt2.block_on(async {
                        handle_event(event_clone).await;
                    });
                }
                Err(e) => eprintln!("[watcher] watch error: {:?}", e),
            }
        }
    });
}

async fn handle_event(event: Event) {
    match event.kind {
        EventKind::Create(_) => {
            for path in event.paths {
                handle_created(&path).await;
            }
        }
        EventKind::Modify(_) => {
            // Distinguish rename vs modify via event paths length?
            // notify reports Modify for content changes and Move for renames
            for path in event.paths {
                handle_modified(&path).await;
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                handle_deleted(&path).await;
            }
        }
        _ => {
            // Handle Move (rename) which is a Modify with 2 paths or specific kind
            // notify 6.1 uses EventKind::Modify with history? Let's check for Any
            // For safety, check if event is a Move (has 2 paths)
            if event.paths.len() == 2 {
                // Likely a rename
                handle_moved(&event.paths[0], &event.paths[1]).await;
            } else {
                for path in event.paths {
                    // Fallback to modified
                    handle_modified(&path).await;
                }
            }
        }
    }
}

pub fn stop_watcher(folder_path: &str) {
    let watchers = watchers_map();
    let mut map = watchers.lock().unwrap();
    if let Some(watcher) = map.remove(folder_path) {
        drop(watcher);
        println!("[watcher] Stopped watching {}", folder_path);
    }
}

pub fn start_watchers_for_existing() {
    // Called on startup to watch all folders already in DB
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            match api::get_watched_folders().await {
                Ok(folders) => {
                    for f in folders {
                        watch_folder(f.folder_path);
                        // Small delay to avoid thundering herd
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
                Err(e) => eprintln!("[watcher] failed to fetch watched folders on startup: {}", e),
            }
        });
    });
}
