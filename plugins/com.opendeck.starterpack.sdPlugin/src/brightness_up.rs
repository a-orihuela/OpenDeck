use openaction::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct BrightnessUpSettings {
    pub step: u8,
}

impl Default for BrightnessUpSettings {
    fn default() -> Self {
        Self { step: 10 }
    }
}

async fn increase(step: u8) -> OpenActionResult<()> {
    send_arbitrary_json(json!({
        "event": "deviceBrightness",
        "action": "increase",
        "value": step,
    }))
    .await
}

pub struct BrightnessUpAction;
#[async_trait]
impl Action for BrightnessUpAction {
    const UUID: &'static str = "opendeck.builtin.brightnessup";
    type Settings = BrightnessUpSettings;

    async fn key_up(&self, _instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        increase(settings.step).await
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
        let (action, value) = if ticks > 0 {
            ("increase", ticks.unsigned_abs() as u8 * settings.step)
        } else {
            ("decrease", ticks.unsigned_abs() as u8 * settings.step)
        };
        send_arbitrary_json(json!({
            "event": "deviceBrightness",
            "action": action,
            "value": value,
        }))
        .await
    }
}
