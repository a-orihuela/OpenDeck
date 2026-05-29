use openaction::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct BrightnessDownSettings {
    pub step: u8,
}

impl Default for BrightnessDownSettings {
    fn default() -> Self {
        Self { step: 10 }
    }
}

async fn decrease(step: u8) -> OpenActionResult<()> {
    send_arbitrary_json(json!({
        "event": "deviceBrightness",
        "action": "decrease",
        "value": step,
    }))
    .await
}

pub struct BrightnessDownAction;
#[async_trait]
impl Action for BrightnessDownAction {
    const UUID: &'static str = "omegadeck.builtin.brightnessdown";
    type Settings = BrightnessDownSettings;

    async fn key_up(&self, _instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        decrease(settings.step).await
    }

    async fn dial_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_up(instance, settings).await
    }

    async fn dial_rotate(
        &self,
        _instance: &Instance,
        settings: &Self::Settings,
        ticks: i16,
        _pressed: bool,
    ) -> OpenActionResult<()> {
        let (action, value) = if ticks < 0 {
            ("decrease", ticks.unsigned_abs() as u8 * settings.step)
        } else {
            ("increase", ticks.unsigned_abs() as u8 * settings.step)
        };
        send_arbitrary_json(json!({
            "event": "deviceBrightness",
            "action": action,
            "value": value,
        }))
        .await
    }
}
