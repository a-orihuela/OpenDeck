mod brightness_down;
mod brightness_up;
mod input_simulation;
mod lock_screen;
mod media;
mod open_url;
mod pomodoro;
mod run_command;
mod screenshot;
mod sleep;
mod switch_profile;
mod util;

use openaction::*;

struct GlobalEventHandler;
#[async_trait]
impl global_events::GlobalEventHandler for GlobalEventHandler {
    async fn device_did_connect(
        &self,
        _event: global_events::DeviceDidConnectEvent,
    ) -> OpenActionResult<()> {
        switch_profile::update_devices().await
    }

    async fn device_did_disconnect(
        &self,
        _event: global_events::DeviceDidDisconnectEvent,
    ) -> OpenActionResult<()> {
        switch_profile::update_devices().await
    }
}

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    {
        use simplelog::*;
        if let Err(error) = TermLogger::init(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Stdout,
            ColorChoice::Never,
        ) {
            eprintln!("Logger initialization failed: {}", error);
        }
    }

    global_events::set_global_event_handler(&GlobalEventHandler);

    // Original actions (renamed UUIDs)
    register_action(input_simulation::InputSimulationAction).await;
    register_action(open_url::OpenUrlAction).await;
    register_action(run_command::RunCommandAction).await;
    register_action(switch_profile::SwitchProfileAction).await;

    // Brightness
    register_action(brightness_up::BrightnessUpAction).await;
    register_action(brightness_down::BrightnessDownAction).await;

    // Media & volume
    register_action(media::VolumeUpAction).await;
    register_action(media::VolumeDownAction).await;
    register_action(media::MuteAction).await;
    register_action(media::PlayPauseAction).await;
    register_action(media::NextTrackAction).await;
    register_action(media::PrevTrackAction).await;

    // System
    register_action(lock_screen::LockScreenAction).await;
    register_action(sleep::SleepAction).await;
    register_action(screenshot::ScreenshotAction).await;

    // Pomodoro
    register_action(pomodoro::PomodoroAction).await;

    run(std::env::args().collect()).await
}
