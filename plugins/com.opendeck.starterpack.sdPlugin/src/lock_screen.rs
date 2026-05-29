use openaction::*;
use serde::{Deserialize, Serialize};

use crate::util::run_platform_command;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct LockScreenSettings;

pub struct LockScreenAction;
#[async_trait]
impl Action for LockScreenAction {
    const UUID: &'static str = "opendeck.builtin.lockscreen";
    type Settings = LockScreenSettings;

    async fn key_up(&self, _instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        run_platform_command(
            "loginctl lock-session",
            "osascript -e 'tell application \"System Events\" to keystroke \"q\" using {command down, control down}'",
            "rundll32.exe user32.dll,LockWorkStation",
        );
        Ok(())
    }

    async fn dial_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_up(instance, settings).await
    }
}
