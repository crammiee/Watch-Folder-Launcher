// Suppress the console window on Windows in release builds.
// Use `cargo build --features console` to keep stdout during development.
#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]

#[cfg(windows)]
mod tray;

use chrono::Local;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebouncedEventKind};
use serde::Deserialize;
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub watch_folder: String,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    pub file_extensions: Vec<String>,
    pub app_display_name: String,
    pub launch_command: String,
    #[serde(default)]
    pub launch_args: Vec<String>,
    /// Process name to check before launching (e.g. "myapp.exe" on Windows).
    /// Leave empty to skip the "already running" check.
    #[serde(default)]
    pub process_pattern: String,
    #[serde(default)]
    pub log_file: String,
}

fn default_debounce_ms() -> u64 {
    2000
}

pub fn log_msg(message: &str, log_file: &str) {
    let line = format!("[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S"), message);
    println!("{line}");
    if !log_file.is_empty() {
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_file) {
            let _ = writeln!(f, "{line}");
        }
    }
}

fn is_app_running(pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    is_app_running_impl(pattern)
}

#[cfg(target_os = "macos")]
fn is_app_running_impl(pattern: &str) -> bool {
    Command::new("pgrep")
        .args(["-f", pattern])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn is_app_running_impl(pattern: &str) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {pattern}"), "/NH"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_lowercase()
                .contains(&pattern.to_lowercase())
        })
        .unwrap_or(false)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_app_running_impl(_pattern: &str) -> bool {
    false
}

fn launch_app(config: &Config, dry_run: bool) {
    if config.launch_command.is_empty() {
        log_msg("launchCommand is empty — set it in config.json", &config.log_file);
        return;
    }
    if dry_run {
        log_msg(
            &format!("[dry-run] Would launch: {} {:?}", config.launch_command, config.launch_args),
            &config.log_file,
        );
        return;
    }
    match Command::new(&config.launch_command)
        .args(&config.launch_args)
        .spawn()
    {
        Ok(_) => log_msg(&format!("Launched {}", config.app_display_name), &config.log_file),
        Err(e) => log_msg(
            &format!("Failed to launch {}: {e}", config.app_display_name),
            &config.log_file,
        ),
    }
}

pub fn run_watcher(config: Arc<Config>, active: Arc<AtomicBool>, dry_run: bool) {
    let extensions: Vec<String> = config
        .file_extensions
        .iter()
        .map(|e| e.to_lowercase())
        .collect();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer =
        match new_debouncer(Duration::from_millis(config.debounce_ms), None, tx) {
            Ok(d) => d,
            Err(e) => {
                log_msg(&format!("Failed to create watcher: {e}"), &config.log_file);
                return;
            }
        };

    if let Err(e) = debouncer
        .watcher()
        .watch(Path::new(&config.watch_folder), RecursiveMode::NonRecursive)
    {
        log_msg(
            &format!("Failed to watch '{}': {e}", config.watch_folder),
            &config.log_file,
        );
        return;
    }

    log_msg(&format!("Watching: {}", config.watch_folder), &config.log_file);

    for result in rx {
        match result {
            Ok(events) => {
                for event in events {
                    if event.kind != DebouncedEventKind::Any {
                        continue;
                    }
                    if !active.load(Ordering::Relaxed) {
                        continue; // paused via tray
                    }

                    let ext = event
                        .path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| format!(".{}", e.to_lowercase()))
                        .unwrap_or_default();

                    if !extensions.contains(&ext) {
                        continue;
                    }

                    let filename = event
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    log_msg(&format!("Detected: {filename}"), &config.log_file);

                    if is_app_running(&config.process_pattern) {
                        log_msg(
                            &format!("{} already running — skipping", config.app_display_name),
                            &config.log_file,
                        );
                    } else {
                        launch_app(&config, dry_run);
                    }
                }
            }
            Err(errors) => {
                for e in errors {
                    log_msg(&format!("Watch error: {e:?}"), &config.log_file);
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());

    // --config flag overrides; otherwise look next to the exe, then fall back to CWD.
    let config_path = args
        .iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("config.json")))
                .unwrap_or_else(|| std::path::PathBuf::from("config.json"))
        });

    let config_str = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", config_path.display()));
    let config: Config =
        serde_json::from_str(&config_str).expect("Invalid config.json — check field names/types");
    let config = Arc::new(config);

    let active = Arc::new(AtomicBool::new(true));

    {
        let config = Arc::clone(&config);
        let active = Arc::clone(&active);
        std::thread::spawn(move || run_watcher(config, active, dry_run));
    }

    #[cfg(windows)]
    tray::run(Arc::clone(&config), Arc::clone(&active));

    // Non-Windows: headless loop (useful for development/testing in WSL)
    #[cfg(not(windows))]
    {
        log_msg(
            &format!(
                "Headless mode — watching {} | Ctrl+C to stop",
                config.watch_folder
            ),
            &config.log_file,
        );
        loop {
            std::thread::sleep(Duration::from_secs(3600));
        }
    }
}
