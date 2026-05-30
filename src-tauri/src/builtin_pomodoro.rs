/// Pomodoro timer — built-in action, managed entirely within the main process.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::builtin_actions::ActionEvent;
use crate::shared::{ActionContext, ActionInstance};

// ── Settings ─────────────────────────────────────────────────────────────────

fn default_true() -> bool { true }

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(default)]
struct Settings {
	work_minutes: u32,
	break_minutes: u32,
	long_break_minutes: u32,
	sessions_before_long_break: u32,
	#[serde(default = "default_true")]
	notify_system: bool,
	#[serde(default = "default_true")]
	notify_sound: bool,
	#[serde(default = "default_true")]
	show_on_key: bool,
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			work_minutes: 25,
			break_minutes: 5,
			long_break_minutes: 15,
			sessions_before_long_break: 4,
			notify_system: true,
			notify_sound: true,
			show_on_key: true,
		}
	}
}

// ── Phase ─────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Phase { Work, Break, LongBreak }

impl Phase {
	fn label(&self) -> &'static str {
		match self { Phase::Work => "", Phase::Break => "☕", Phase::LongBreak => "🌙" }
	}
	fn duration_secs(&self, s: &Settings) -> u64 {
		match self {
			Phase::Work => s.work_minutes as u64 * 60,
			Phase::Break => s.break_minutes as u64 * 60,
			Phase::LongBreak => s.long_break_minutes as u64 * 60,
		}
	}
}

// ── Timer state ───────────────────────────────────────────────────────────────

struct TimerState {
	phase: Phase,
	remaining_secs: u64,
	sessions_completed: u32,
	task: Option<JoinHandle<()>>,
	settings: Settings,
	key_down_at: Option<Instant>,
}

impl TimerState {
	fn new(s: Settings) -> Self {
		let remaining_secs = s.work_minutes as u64 * 60;
		Self { phase: Phase::Work, remaining_secs, sessions_completed: 0, task: None, settings: s, key_down_at: None }
	}
	fn title(&self, running: bool) -> String {
		let m = self.remaining_secs / 60;
		let s = self.remaining_secs % 60;
		let prefix = if !running {
			if self.remaining_secs == self.phase.duration_secs(&self.settings) { "▶ " } else { "⏸ " }
		} else { "" };
		format!("{}{}{m:02}:{s:02}", prefix, self.phase.label())
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
			Phase::Break | Phase::LongBreak => { self.phase = Phase::Work; }
		}
		self.remaining_secs = self.phase.duration_secs(&self.settings);
	}
}

// ── Global map ────────────────────────────────────────────────────────────────

type StateMap = Arc<Mutex<HashMap<String, TimerState>>>;
static STATES: LazyLock<StateMap> = LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

fn ctx_key(ctx: &ActionContext) -> String {
	format!("{}.{}.{}.{}.{}", ctx.device, ctx.profile, ctx.controller, ctx.position, ctx.index)
}

// ── Notifications ─────────────────────────────────────────────────────────────

fn notify_phase_change(phase: &Phase, settings: &Settings) {
	let (title, body) = match phase {
		Phase::Work => ("Pomodoro - Focus time!", "Time to work. Stay focused!"),
		Phase::Break => ("Pomodoro - Break time!", "Take a short break. You deserve it!"),
		Phase::LongBreak => ("Pomodoro - Long break!", "Take a longer break. Recharge!"),
	};

	if settings.notify_system {
		#[cfg(target_os = "linux")]
		let _ = std::process::Command::new("notify-send")
			.args(["--app-name=OmegaDeck", title, body])
			.spawn();
		#[cfg(target_os = "macos")]
		let _ = std::process::Command::new("osascript")
			.args(["-e", &format!("display notification \"{}\" with title \"{}\"", body, title)])
			.spawn();
		let _ = (title, body);
	}

	if settings.notify_sound {
		#[cfg(target_os = "linux")]
		let _ = std::process::Command::new("sh")
			.args(["-c", "paplay /usr/share/sounds/freedesktop/stereo/complete.oga 2>/dev/null || paplay /usr/share/sounds/freedesktop/stereo/bell.oga 2>/dev/null || aplay /usr/share/sounds/alsa/Front_Center.wav 2>/dev/null || true"])
			.spawn();
		#[cfg(target_os = "macos")]
		let _ = std::process::Command::new("afplay")
			.arg("/System/Library/Sounds/Glass.aiff")
			.spawn();
	}
}

/// Schedule a title update on a separate task to avoid profile-lock contention
/// when called from within a key-event handler that already holds the locks.
fn update_title_later(ctx: ActionContext, title: String) {
	tokio::spawn(async move {
		tokio::time::sleep(Duration::from_millis(100)).await;
		set_title_for_context(&ctx, title).await;
	});
}

// ── Title update ──────────────────────────────────────────────────────────────

async fn set_title_for_context(ctx: &ActionContext, title: String) {
	let Ok(mut locks) = tokio::time::timeout(
		Duration::from_millis(200),
		crate::store::profiles::acquire_locks_mut(),
	)
	.await
	else { return };

	let Ok(Some(instance)) = crate::store::profiles::get_instance_mut(ctx, &mut locks).await
	else { return };

	let idx = instance.current_state as usize;
	if idx < instance.states.len() {
		instance.states[idx].text = title;
		instance.states[idx].show = true;
		let clone = instance.clone();
		let _ = crate::events::outbound::states::title_parameters_did_change(&clone, idx as u16).await;
	}
}

// ── Timer task ────────────────────────────────────────────────────────────────

fn spawn_timer(ctx: ActionContext, key: String, settings: Settings) -> JoinHandle<()> {
	tokio::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_secs(1)).await;
			let mut map = STATES.lock().await;
			let Some(state) = map.get_mut(&key) else { break };
			if state.remaining_secs > 0 {
				state.remaining_secs -= 1;
				if settings.show_on_key {
					let title = state.title(true);
					drop(map);
					set_title_for_context(&ctx, title).await;
				} else {
					drop(map);
				}
			} else {
				state.advance_phase();
				let new_phase = state.phase.clone();
				let title = state.title(false);
				state.task = None;
				drop(map);
				notify_phase_change(&new_phase, &settings);
				if settings.show_on_key {
					set_title_for_context(&ctx, title).await;
				}
				break;
			}
		}
	})
}

// ── Public API ────────────────────────────────────────────────────────────────

pub async fn handle(instance: &ActionInstance, event: ActionEvent) -> anyhow::Result<Option<u16>> {
	let ctx = instance.context.clone();
	let key = ctx_key(&ctx);
	let s: Settings = serde_json::from_value(instance.settings.clone()).unwrap_or_default();

	{
		let mut map = STATES.lock().await;
		let state = map.entry(key.clone()).or_insert_with(|| TimerState::new(s.clone()));
		if state.settings != s {
			if let Some(task) = state.task.take() {
				task.abort();
			}
			*state = TimerState::new(s.clone());
		}
	}

	match event {
		ActionEvent::KeyDown => {
			let mut map = STATES.lock().await;
			map.entry(key).or_insert_with(|| TimerState::new(s)).key_down_at = Some(Instant::now());
		}
		ActionEvent::KeyUp | ActionEvent::DialUp => {
			let mut map = STATES.lock().await;
			let state = map.entry(key.clone()).or_insert_with(|| TimerState::new(s));

			let held_ms = state.key_down_at.map(|t| t.elapsed().as_millis()).unwrap_or(0);
			state.key_down_at = None;

			if held_ms >= 800 {
				if let Some(task) = state.task.take() { task.abort(); }
				state.remaining_secs = state.phase.duration_secs(&state.settings);
				let title = state.title(false);
				drop(map);
				update_title_later(ctx.clone(), title);
			} else if let Some(task) = state.task.take() {
				task.abort();
				let title = state.title(false);
				drop(map);
				update_title_later(ctx.clone(), title);
			} else {
				let settings_clone = state.settings.clone();
				let task = spawn_timer(ctx.clone(), key.clone(), settings_clone);
				state.task = Some(task);
				let title = state.title(true);
				drop(map);
				update_title_later(ctx.clone(), title);
			}
		}
		_ => {}
	}

	Ok(None)
}
