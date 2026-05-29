use openaction::*;
use serde::{Deserialize, Serialize};

use crate::util::run_platform_command;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SleepSettings;

pub struct SleepAction;
#[async_trait]
impl Action for SleepAction {
    const UUID: &'static str = "omegadeck.builtin.sleep";
    type Settings = SleepSettings;

    async fn key_up(&self, _instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        run_platform_command(
            "systemctl suspend",
            "pmset sleepnow",
            "rundll32.exe powrprof.dll,SetSuspendState 0,1,0",
        );
        Ok(())
    }

    async fn dial_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_up(instance, settings).await
    }
}
