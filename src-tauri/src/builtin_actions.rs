/// Built-in action handlers — same actions that were previously in the default plugin,
/// now executed directly inside the main process so they behave exactly like the
/// hardcoded Actions and Navigation entries (no install/sync mechanism needed).

use std::io::Read;
use std::sync::LazyLock;
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings, agent::Agent};
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use crate::shared::ActionInstance;

// ── Shared enigo instance ────────────────────────────────────────────────────

static ENIGO: LazyLock<Mutex<Option<Enigo>>> = LazyLock::new(|| Mutex::new(None));

async fn press_key(key: Key) -> Result<(), anyhow::Error> {
	let mut guard = ENIGO.lock().await;
	std::thread::spawn(move || -> Result<(), anyhow::Error> {
		if guard.is_none() {
			guard.replace(Enigo::new(&EnigoSettings::default())?);
		}
		let enigo = guard.as_mut().unwrap();
		enigo.key(key, Direction::Click)?;
		Ok(())
	})
	.join()
	.unwrap_or(Ok(()))
}

async fn simulate_input(value: &str) -> Result<(), anyhow::Error> {
	if value.trim().is_empty() {
		return Ok(());
	}
	let value = normalize_legacy_input_dsl(value);
	let mut guard = ENIGO.lock().await;
	std::thread::spawn(move || -> Result<(), anyhow::Error> {
		if guard.is_none() {
			guard.replace(Enigo::new(&EnigoSettings::default())?);
		}
		let enigo = guard.as_mut().unwrap();
		let tokens: Vec<enigo::agent::Token> = ron::from_str(&value)?;
		for token in tokens {
			enigo.execute(&token).map_err(|e| anyhow::anyhow!("{e:?}"))?;
		}
		Ok(())
	})
	.join()
	.unwrap_or(Ok(()))
}

#[cfg(target_os = "linux")]
fn try_playerctl(action: &str) -> bool {
	std::process::Command::new("playerctl")
		.arg(action)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.map(|status| status.success())
		.unwrap_or(false)
}

fn normalize_legacy_input_dsl(value: &str) -> String {
	let mut normalized = value.to_owned();

	for (from, to) in [
		("ControlLeft", "LControl"),
		("ControlRight", "RControl"),
		("ShiftLeft", "LShift"),
		("ShiftRight", "RShift"),
		("AltRight", "Alt"),
		("MetaLeft", "Meta"),
		("MetaRight", "Meta"),
		("Enter", "Return"),
		("PrintScreen", "PrintScr"),
	] {
		normalized = normalized.replace(from, to);
	}

	for letter in 'A'..='Z' {
		let from = format!("Key{letter}");
		let to = format!("Unicode('{}')", letter.to_ascii_lowercase());
		normalized = normalized.replace(&from, &to);
	}

	for d in '0'..='9' {
		let from = format!("Digit{d}");
		let to = format!("Unicode('{d}')");
		normalized = normalized.replace(&from, &to);
	}

	normalized
}

fn run_platform_command(linux: &str, macos: &str, windows: &str) {
	#[cfg(target_os = "linux")]
	let _ = std::process::Command::new("sh").arg("-c").arg(linux).spawn();
	#[cfg(target_os = "macos")]
	let _ = std::process::Command::new("sh").arg("-c").arg(macos).spawn();
	#[cfg(windows)]
	let _ = std::process::Command::new("cmd").args(["/C", windows]).spawn();
	let _ = (linux, macos, windows);
}

// ── Event type ───────────────────────────────────────────────────────────────

pub enum ActionEvent {
	KeyDown,
	KeyUp,
	DialDown,
	DialUp,
	DialRotate(i16),
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

/// Handle a built-in action event. Returns `Some(new_state)` if the visual
/// state of the instance should be updated (e.g. Mute/PlayPause toggle).
pub async fn handle(instance: &ActionInstance, event: ActionEvent) -> anyhow::Result<Option<u16>> {
	let uuid = instance.action.uuid.as_str();
	let s = &instance.settings;

	match uuid {
		// ── Media ─────────────────────────────────────────────────────────
		"omegadeck.builtin.volumeup" => match event {
			ActionEvent::KeyDown | ActionEvent::DialDown => { let _ = press_key(Key::VolumeUp).await; }
			_ => {}
		},
		"omegadeck.builtin.volumedown" => match event {
			ActionEvent::KeyDown | ActionEvent::DialDown => { let _ = press_key(Key::VolumeDown).await; }
			_ => {}
		},
		"omegadeck.builtin.nexttrack" => match event {
			ActionEvent::KeyDown | ActionEvent::DialDown => {
				#[cfg(target_os = "linux")]
				if !try_playerctl("next") {
					let _ = press_key(Key::MediaNextTrack).await;
				}
				#[cfg(not(target_os = "linux"))]
				{ let _ = press_key(Key::MediaNextTrack).await; }
			}
			_ => {}
		},
		"omegadeck.builtin.prevtrack" => match event {
			ActionEvent::KeyDown | ActionEvent::DialDown => {
				#[cfg(target_os = "linux")]
				if !try_playerctl("previous") {
					let _ = press_key(Key::MediaPrevTrack).await;
				}
				#[cfg(not(target_os = "linux"))]
				{ let _ = press_key(Key::MediaPrevTrack).await; }
			}
			_ => {}
		},
		"omegadeck.builtin.mute" => match event {
			ActionEvent::KeyDown | ActionEvent::DialDown => {
				let _ = press_key(Key::VolumeMute).await;
				let cur = instance.current_state.min(1);
				return Ok(Some(1 - cur));
			}
			_ => {}
		},
		"omegadeck.builtin.playpause" => match event {
			ActionEvent::KeyDown | ActionEvent::DialDown => {
				let _ = press_key(Key::MediaPlayPause).await;
				let cur = instance.current_state.min(1);
				return Ok(Some(1 - cur));
			}
			_ => {}
		},

		// ── System ───────────────────────────────────────────────────────
		"omegadeck.builtin.lockscreen" => match event {
			ActionEvent::KeyUp | ActionEvent::DialUp => run_platform_command(
				"loginctl lock-session",
				"osascript -e 'tell application \"System Events\" to keystroke \"q\" using {command down, control down}'",
				"rundll32.exe user32.dll,LockWorkStation",
			),
			_ => {}
		},
		"omegadeck.builtin.sleep" => match event {
			ActionEvent::KeyUp | ActionEvent::DialUp => run_platform_command(
				"systemctl suspend",
				"pmset sleepnow",
				"rundll32.exe powrprof.dll,SetSuspendState 0,1,0",
			),
			_ => {}
		},
		"omegadeck.builtin.screenshot" => {
			let mode = s.get("mode").and_then(|v| v.as_str()).unwrap_or("system_shortcut");
			match event {
				ActionEvent::KeyUp | ActionEvent::DialUp => {
					if mode == "command" {
						if let Some(cmd) = s.get("command").and_then(|v| v.as_str()) {
							if !cmd.trim().is_empty() {
								run_platform_command(cmd, cmd, cmd);
							}
						}
					} else {
						#[cfg(not(target_os = "macos"))]
						{ let _ = press_key(Key::PrintScr).await; }
						#[cfg(target_os = "macos")]
						run_platform_command("", "osascript -e 'tell application \"System Events\" to keystroke \"3\" using {command down, shift down}'", "");
					}
				}
				_ => {}
			}
		},
		"omegadeck.builtin.brightnessup" => {
			let step = s.get("step").and_then(|v| v.as_u64()).unwrap_or(10) as i32;
			match event {
				ActionEvent::KeyUp | ActionEvent::DialUp => {
					device_brightness_change(step).await;
				}
				ActionEvent::DialRotate(ticks) => {
					device_brightness_change(ticks as i32 * step).await;
				}
				_ => {}
			}
		},
		"omegadeck.builtin.brightnessdown" => {
			let step = s.get("step").and_then(|v| v.as_u64()).unwrap_or(10) as i32;
			match event {
				ActionEvent::KeyUp | ActionEvent::DialUp => {
					device_brightness_change(-step).await;
				}
				ActionEvent::DialRotate(ticks) => {
					device_brightness_change(ticks as i32 * step).await;
				}
				_ => {}
			}
		},
		// ── Automation ───────────────────────────────────────────────────
		"omegadeck.builtin.runcommand" => {
			let cmd = match event {
				ActionEvent::KeyDown | ActionEvent::DialDown => s.get("down").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::KeyUp | ActionEvent::DialUp => s.get("up").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::DialRotate(ticks) => {
					let raw = s.get("rotate").and_then(|v| v.as_str()).unwrap_or("").to_owned();
					raw.replace("%d", &ticks.to_string())
				}
			};
			if !cmd.trim().is_empty() {
				let file_path = s.get("file").and_then(|v| v.as_str()).map(|v| v.to_owned());
				let show = s.get("show").and_then(|v| v.as_bool()).unwrap_or(false);
				let output_path = file_path.filter(|path| !path.trim().is_empty());
				if !show && output_path.is_none() {
					if let Err(e) = run_command_detached(&cmd) {
						log::warn!("run_command failed: {e}");
					}
				} else {
					let context = instance.context.clone();
					tokio::spawn(async move {
						match run_command_str(&cmd).await {
						Ok(output) => {
							if let Some(path) = output_path
								&& let Err(e) = tokio::fs::write(path, &output).await
							{
								log::warn!("run_command failed to write output file: {e}");
							}
							if show {
								set_runtime_title(&context, output.trim().to_owned()).await;
							}
						}
						Err(e) => {
							log::warn!("run_command failed: {e}");
						}
						}
					});
				}
			}
		},
		"omegadeck.builtin.openurl" => {
			let url = match event {
				ActionEvent::KeyDown | ActionEvent::DialDown => s.get("down").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::KeyUp | ActionEvent::DialUp => s.get("up").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::DialRotate(ticks) if ticks < 0 => s.get("anticlockwise").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::DialRotate(_) => s.get("clockwise").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
			};
			if !url.trim().is_empty() {
				let _ = open::that_detached(&url);
			}
		},
		"omegadeck.builtin.inputsimulation" => {
			let dsl = match event {
				ActionEvent::KeyDown | ActionEvent::DialDown => s.get("down").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::KeyUp | ActionEvent::DialUp => s.get("up").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::DialRotate(ticks) if ticks < 0 => s.get("anticlockwise").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
				ActionEvent::DialRotate(_) => s.get("clockwise").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
			};
			let repeats = match event {
				ActionEvent::DialRotate(ticks) => ticks.abs() as usize,
				_ => 1,
			};
			for _ in 0..repeats {
				if let Err(e) = simulate_input(&dsl).await {
					log::warn!("simulate_input failed: {e}");
				}
			}
		},

		// ── Productivity ─────────────────────────────────────────────────
		"omegadeck.builtin.switchprofile" => {
			let device = s.get("device").and_then(|v| v.as_str()).map(|s| s.to_owned())
				.unwrap_or_else(|| instance.context.device.clone());
			let profile = match event {
				ActionEvent::DialRotate(ticks) if ticks < 0 => s.get("anticlockwise").and_then(|v| v.as_str()).unwrap_or("Default").to_owned(),
				ActionEvent::DialRotate(_) => s.get("clockwise").and_then(|v| v.as_str()).unwrap_or("Default").to_owned(),
				ActionEvent::KeyUp | ActionEvent::DialUp => s.get("profile").and_then(|v| v.as_str()).unwrap_or("Default").to_owned(),
				_ => return Ok(None),
			};
			if let Some(app) = crate::APP_HANDLE.get() {
				let _ = app.get_webview_window("main").map(|w| {
					w.emit("switch_profile", serde_json::json!({ "device": device, "profile": profile }))
				});
			}
		},
		"omegadeck.builtin.pomodoro" => {
			// Pomodoro keeps its own timer state per-context; delegated to the pomodoro module.
			crate::builtin_pomodoro::handle(instance, event).await?;
		},

		_ => {}
	}

	Ok(None)
}

async fn device_brightness_change(delta: i32) {
	let current = crate::store::get_settings().value.brightness as i32;
	let new_brightness = (current + delta).clamp(5, 100) as u8;

	{
		let mut store = crate::store::SETTINGS_MUT.lock().await;
		store.value.brightness = new_brightness;
		let _ = store.save();
	}

	let _ = crate::events::outbound::devices::set_brightness(new_brightness).await;
}

async fn set_runtime_title(context: &crate::shared::ActionContext, title: String) {
	let Ok(mut locks) = tokio::time::timeout(
		Duration::from_millis(200),
		crate::store::profiles::acquire_locks_mut(),
	)
	.await
	else { return };

	let Ok(Some(instance)) = crate::store::profiles::get_instance_mut(context, &mut locks).await
	else { return };

	let idx = instance.current_state as usize;
	if idx >= instance.states.len() {
		return;
	}

	instance.states[idx].text = title;
	instance.states[idx].show = true;
	let clone = instance.clone();
	let _ = crate::events::frontend::instances::update_state(
		crate::APP_HANDLE.get().unwrap(),
		clone.context.clone(),
		&mut locks,
	)
	.await;
	let _ = crate::events::outbound::states::title_parameters_did_change(&clone, idx as u16).await;
}

async fn run_command_str(cmd: &str) -> Result<String, anyhow::Error> {
	let (mut reader, writer) = os_pipe::pipe()?;
	let writer2 = writer.try_clone()?;
	let mut command = build_shell_command(cmd);
	command.stdout(std::process::Stdio::from(writer));
	command.stderr(std::process::Stdio::from(writer2));
	command.spawn()?.wait()?;
	let mut output = String::new();
	reader.read_to_string(&mut output)?;
	log::debug!("run_command output: {}", output.trim());
	Ok(output)
}

fn run_command_detached(cmd: &str) -> Result<(), anyhow::Error> {
	let mut command = build_shell_command(cmd);
	command.stdout(std::process::Stdio::null());
	command.stderr(std::process::Stdio::null());
	command.stdin(std::process::Stdio::null());
	command.spawn()?;
	Ok(())
}

fn build_shell_command(cmd: &str) -> std::process::Command {
	#[cfg(unix)]
	let (program, args): (&str, Vec<&str>) = {
		let flatpak = std::env::var("FLATPAK_ID").is_ok()
			|| std::env::var("container").map(|x| x.to_lowercase().trim() == "flatpak").unwrap_or(false);
		let distrobox = std::env::var("CONTAINER_ID").is_ok();
		if flatpak {
			("flatpak-spawn", vec!["--host", "sh", "-c", cmd])
		} else if distrobox && !cmd.trim().starts_with("distrobox-host-exec") {
			("distrobox-host-exec", vec!["sh", "-c", cmd])
		} else {
			("sh", vec!["-c", cmd])
		}
	};
	#[cfg(windows)]
	let (program, args): (&str, Vec<&str>) = ("cmd", vec!["/C", cmd]);

	let mut command = std::process::Command::new(program);
	command.args(args);
	if let Some(home) = std::env::home_dir() {
		command.current_dir(home);
	}
	command
}
