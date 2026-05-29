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
media_action!(MuteAction,       "opendeck.builtin.mute",       Key::VolumeMute);
media_action!(PlayPauseAction,  "opendeck.builtin.playpause",  Key::MediaPlayPause);
media_action!(NextTrackAction,  "opendeck.builtin.nexttrack",  Key::MediaNextTrack);
media_action!(PrevTrackAction,  "opendeck.builtin.prevtrack",  Key::MediaPrevTrack);
