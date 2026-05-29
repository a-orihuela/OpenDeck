use enigo::Key;
use openaction::*;
use serde::{Deserialize, Serialize};

use crate::util::{press_key, run_platform_command};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotMode {
    SystemShortcut,
    Command,
}

impl Default for ScreenshotMode {
    fn default() -> Self {
        Self::SystemShortcut
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct ScreenshotSettings {
    pub mode: ScreenshotMode,
    pub command: Option<String>,
}

pub struct ScreenshotAction;
#[async_trait]
impl Action for ScreenshotAction {
    const UUID: &'static str = "omegadeck.builtin.screenshot";
    type Settings = ScreenshotSettings;

    async fn key_up(&self, _instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        match settings.mode {
            ScreenshotMode::SystemShortcut => {
                #[cfg(not(target_os = "macos"))]
                {
                    if let Err(e) = press_key(Key::PrintScr).await {
                        log::warn!("Failed to press screenshot key: {e}");
                    }
                }
                #[cfg(target_os = "macos")]
                {
                    // Cmd+Shift+3 = full screenshot to Desktop on macOS
                    run_platform_command(
                        "",
                        "osascript -e 'tell application \"System Events\" to keystroke \"3\" using {command down, shift down}'",
                        "",
                    );
                }
            }
            ScreenshotMode::Command => {
                if let Some(cmd) = settings.command.as_deref()
                    && !cmd.trim().is_empty()
                {
                    run_platform_command(cmd, cmd, cmd);
                }
            }
        }
        Ok(())
    }

    async fn dial_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_up(instance, settings).await
    }
}
