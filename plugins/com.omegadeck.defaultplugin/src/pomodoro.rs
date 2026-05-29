use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use openaction::*;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PomodoroSettings {
    pub work_minutes: u32,
    pub break_minutes: u32,
    pub long_break_minutes: u32,
    pub sessions_before_long_break: u32,
}

impl Default for PomodoroSettings {
    fn default() -> Self {
        Self {
            work_minutes: 25,
            break_minutes: 5,
            long_break_minutes: 15,
            sessions_before_long_break: 4,
        }
    }
}

#[derive(Clone, PartialEq)]
enum Phase {
    Work,
    Break,
    LongBreak,
}

impl Phase {
    fn label(&self) -> &'static str {
        match self {
            Phase::Work => "",
            Phase::Break => "☕",
            Phase::LongBreak => "🌙",
        }
    }

    fn duration_secs(&self, settings: &PomodoroSettings) -> u64 {
        match self {
            Phase::Work => settings.work_minutes as u64 * 60,
            Phase::Break => settings.break_minutes as u64 * 60,
            Phase::LongBreak => settings.long_break_minutes as u64 * 60,
        }
    }
}

struct TimerState {
    phase: Phase,
    remaining_secs: u64,
    sessions_completed: u32,
    task: Option<JoinHandle<()>>,
    settings: PomodoroSettings,
    key_down_at: Option<Instant>,
}

impl TimerState {
    fn new(settings: PomodoroSettings) -> Self {
        let remaining_secs = settings.work_minutes as u64 * 60;
        Self {
            phase: Phase::Work,
            remaining_secs,
            sessions_completed: 0,
            task: None,
            settings,
            key_down_at: None,
        }
    }

    fn title(&self, running: bool) -> String {
        let mins = self.remaining_secs / 60;
        let secs = self.remaining_secs % 60;
        let prefix = if !running {
            if self.remaining_secs == self.phase.duration_secs(&self.settings) {
                "▶ ".to_string()
            } else {
                "⏸ ".to_string()
            }
        } else {
            String::new()
        };
        format!("{}{}{:02}:{:02}", prefix, self.phase.label(), mins, secs)
    }

    fn advance_phase(&mut self) {
        match self.phase {
            Phase::Work => {
                self.sessions_completed += 1;
                if self.sessions_completed % self.settings.sessions_before_long_break == 0 {
                    self.phase = Phase::LongBreak;
                } else {
                    self.phase = Phase::Break;
                }
            }
            Phase::Break | Phase::LongBreak => {
                self.phase = Phase::Work;
            }
        }
        self.remaining_secs = self.phase.duration_secs(&self.settings);
    }
}

// ── Global state ──────────────────────────────────────────────────────────────

type StateMap = Arc<Mutex<HashMap<InstanceId, TimerState>>>;

static STATES: LazyLock<StateMap> = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn get_or_create(instance_id: &InstanceId, settings: &PomodoroSettings) -> () {
    let mut map = STATES.lock().await;
    if !map.contains_key(instance_id) {
        map.insert(instance_id.clone(), TimerState::new(settings.clone()));
    }
}

async fn set_title_for(instance_id: &InstanceId, title: String) {
    if let Some(instance) = get_instance(instance_id.clone()).await {
        let _ = instance.set_title(Some(title), None).await;
    }
}

// ── Timer task ────────────────────────────────────────────────────────────────

fn spawn_timer(instance_id: InstanceId) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let mut map = STATES.lock().await;
            let Some(state) = map.get_mut(&instance_id) else { break };

            if state.remaining_secs > 0 {
                state.remaining_secs -= 1;
                let title = state.title(true);
                let iid = instance_id.clone();
                drop(map);
                set_title_for(&iid, title).await;
            } else {
                // Phase complete — advance and update display.
                state.advance_phase();
                let title = state.title(false);
                state.task = None;
                let iid = instance_id.clone();
                drop(map);
                set_title_for(&iid, title).await;
                break;
            }
        }
    })
}

// ── Action ────────────────────────────────────────────────────────────────────

pub struct PomodoroAction;
#[async_trait]
impl Action for PomodoroAction {
    const UUID: &'static str = "opendeck.builtin.pomodoro";
    type Settings = PomodoroSettings;

    async fn will_appear(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        // Update settings if they changed (e.g. user edited the property inspector).
        {
            let mut map = STATES.lock().await;
            let config_changed = map.get(&instance.instance_id).map_or(true, |state| {
                state.settings.work_minutes != settings.work_minutes
                    || state.settings.break_minutes != settings.break_minutes
                    || state.settings.long_break_minutes != settings.long_break_minutes
                    || state.settings.sessions_before_long_break != settings.sessions_before_long_break
            });
            if config_changed {
                if let Some(state) = map.get_mut(&instance.instance_id) {
                    if let Some(task) = state.task.take() { task.abort(); }
                }
                map.insert(instance.instance_id.clone(), TimerState::new(settings.clone()));
            }
        }
        let map = STATES.lock().await;
        if let Some(state) = map.get(&instance.instance_id) {
            let running = state.task.is_some();
            let title = state.title(running);
            drop(map);
            set_title_for(&instance.instance_id, title).await;
        }
        Ok(())
    }

    async fn key_down(&self, instance: &Instance, _settings: &Self::Settings) -> OpenActionResult<()> {
        let mut map = STATES.lock().await;
        if let Some(state) = map.get_mut(&instance.instance_id) {
            state.key_down_at = Some(Instant::now());
        }
        Ok(())
    }

    async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        get_or_create(&instance.instance_id, settings).await;

        let held_ms = {
            let map = STATES.lock().await;
            map.get(&instance.instance_id)
                .and_then(|s| s.key_down_at)
                .map(|t| t.elapsed().as_millis())
                .unwrap_or(0)
        };

        if held_ms >= 800 {
            // Long press → reset to beginning of current phase.
            let mut map = STATES.lock().await;
            if let Some(state) = map.get_mut(&instance.instance_id) {
                if let Some(task) = state.task.take() {
                    task.abort();
                }
                state.remaining_secs = state.phase.duration_secs(&state.settings);
                state.key_down_at = None;
                let title = state.title(false);
                let iid = instance.instance_id.clone();
                drop(map);
                set_title_for(&iid, title).await;
            }
            return Ok(());
        }

        // Short press → toggle play/pause.
        let mut map = STATES.lock().await;
        if let Some(state) = map.get_mut(&instance.instance_id) {
            state.key_down_at = None;
            if let Some(task) = state.task.take() {
                // Running → pause.
                task.abort();
                let title = state.title(false);
                let iid = instance.instance_id.clone();
                drop(map);
                set_title_for(&iid, title).await;
            } else {
                // Paused/idle → start.
                let iid = instance.instance_id.clone();
                let task = spawn_timer(iid.clone());
                state.task = Some(task);
                let title = state.title(true);
                drop(map);
                set_title_for(&iid, title).await;
            }
        }
        Ok(())
    }

    async fn dial_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
        self.key_up(instance, settings).await
    }

}
