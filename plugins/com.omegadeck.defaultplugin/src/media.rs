use enigo::Key;
use openaction::*;
use serde::{Deserialize, Serialize};

use crate::util::press_key;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct NoSettings;

macro_rules! media_action {
    ($name:ident, $uuid:literal, $key:expr) => {
        pub struct $name;
        #[async_trait]
        impl Action for $name {
            const UUID: &'static str = $uuid;
            type Settings = NoSettings;

            async fn key_down(
                &self,
                _instance: &Instance,
                _settings: &Self::Settings,
            ) -> OpenActionResult<()> {
                if let Err(e) = press_key($key).await {
                    log::warn!("Failed to press media key: {e}");
                }
                Ok(())
            }

            async fn dial_down(
                &self,
                instance: &Instance,
                settings: &Self::Settings,
            ) -> OpenActionResult<()> {
                self.key_down(instance, settings).await
            }
        }
    };
}

media_action!(VolumeUpAction,   "opendeck.builtin.volumeup",   Key::VolumeUp);
media_action!(VolumeDownAction, "opendeck.builtin.volumedown", Key::VolumeDown);
media_action!(NextTrackAction,  "opendeck.builtin.nexttrack",  Key::MediaNextTrack);
media_action!(PrevTrackAction,  "opendeck.builtin.prevtrack",  Key::MediaPrevTrack);

pub struct MuteAction;
#[async_trait]
impl Action for MuteAction {
    const UUID: &'static str = "opendeck.builtin.mute";
    type Settings = NoSettings;

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        if let Err(e) = press_key(Key::VolumeMute).await {
            log::warn!("Failed to press media key: {e}");
        }
        let cur = instance.current_state_index.load(std::sync::atomic::Ordering::Relaxed).min(1);
        instance.set_state(1 - cur).await?;
        Ok(())
    }

    async fn dial_down(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }
}

pub struct PlayPauseAction;
#[async_trait]
impl Action for PlayPauseAction {
    const UUID: &'static str = "opendeck.builtin.playpause";
    type Settings = NoSettings;

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        if let Err(e) = press_key(Key::MediaPlayPause).await {
            log::warn!("Failed to press media key: {e}");
        }
        let cur = instance.current_state_index.load(std::sync::atomic::Ordering::Relaxed).min(1);
        instance.set_state(1 - cur).await?;
        Ok(())
    }

    async fn dial_down(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_down(instance, settings).await
    }
}
