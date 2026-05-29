use std::sync::LazyLock;

use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use tokio::sync::Mutex;

// Shared enigo instance across all actions that need it.
pub static ENIGO: LazyLock<Mutex<Option<Enigo>>> = LazyLock::new(|| Mutex::new(None));

/// Press and release a single key using enigo.
pub async fn press_key(key: Key) -> Result<(), anyhow::Error> {
    let mut guard = ENIGO.lock().await;
    std::thread::spawn(move || -> Result<(), anyhow::Error> {
        if guard.is_none() {
            guard.replace(Enigo::new(&EnigoSettings::default())?);
        }
        let enigo = guard.as_mut().unwrap();
        enigo.key(key, Direction::Click)?;
        Ok(())
    })
    .join()
    .unwrap_or(Ok(()))
}

/// Run a platform-specific shell command, ignoring the exit code.
pub fn run_platform_command(linux: &str, macos: &str, windows: &str) {
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("sh").arg("-c").arg(linux).spawn();

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("sh").arg("-c").arg(macos).spawn();

    #[cfg(windows)]
    let _ = std::process::Command::new("cmd").args(["/C", windows]).spawn();

    // Suppress unused variable warnings on each platform.
    let _ = (linux, macos, windows);
}
