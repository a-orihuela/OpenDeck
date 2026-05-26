use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::{fs, path};

use crate::shared::{config_dir, is_flatpak, log_dir};

use super::PORT_BASE;

pub enum PluginChildType {
	Wine,
	Native,
	Node,
}

pub struct ProcessHandle {
	pub pid: u32,
	pub kill_tx: mpsc::SyncSender<()>,
}

pub enum PluginInstance {
	Webview,
	Wine(ProcessHandle),
	Native(ProcessHandle),
	Node(ProcessHandle),
}

pub type SpawnRequest = Box<dyn FnOnce() -> Result<(String, PluginChildType, Command), anyhow::Error> + Send>;

/// Attach a kernel-enforced "die when parent dies" signal to a plugin child process.
#[cfg(target_os = "linux")]
fn attach_parent_death_signal(command: &mut Command) {
	use std::os::unix::process::CommandExt;
	// SAFETY: `libc::prctl` is async-signal-safe.
	unsafe {
		command.pre_exec(move || {
			if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong) != 0 {
				return Err(std::io::Error::last_os_error());
			}
			Ok(())
		});
	}
}

/// Kill a process by PID without requiring ownership of the `Child`.
pub fn kill_process(pid: u32) {
	#[cfg(unix)]
	unsafe {
		libc::kill(pid as libc::pid_t, libc::SIGTERM);
	}
	#[cfg(windows)]
	unsafe {
		use windows_sys::Win32::Foundation::CloseHandle;
		use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
		let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
		if !handle.is_null() {
			TerminateProcess(handle, 1);
			CloseHandle(handle);
		}
	}
}

/// Watch a spawned plugin process and restart it with exponential backoff on unexpected exit.
pub fn supervise_plugin(mut child: Child, uuid: String, spawner_tx: mpsc::Sender<SpawnRequest>, kill_rx: mpsc::Receiver<()>) {
	const MAX_CRASHES_PER_WINDOW: u8 = 5;
	const CRASH_WINDOW_SECS: u64 = 60;

	let _ = child.wait();

	if kill_rx.try_recv().is_ok() {
		return;
	}

	let crash_count = {
		let mut entry = crate::shared::PLUGIN_CRASH_COUNTS.entry(uuid.clone()).or_insert((0u8, std::time::Instant::now()));
		if entry.1.elapsed().as_secs() >= CRASH_WINDOW_SECS {
			entry.0 = 0;
			entry.1 = std::time::Instant::now();
		}
		entry.0 = entry.0.saturating_add(1);
		if entry.0 == 3 {
			log::warn!("plugin \"{uuid}\" has crashed 3 times rapidly; it may be unstable");
		}
		entry.0
	};

	if crash_count > MAX_CRASHES_PER_WINDOW {
		log::error!("plugin \"{uuid}\" has crashed too many times within {CRASH_WINDOW_SECS}s; giving up");
		return;
	}

	let delay_secs = 1u64.checked_shl(crash_count as u32).unwrap_or(32).min(32);
	std::thread::sleep(std::time::Duration::from_secs(delay_secs));

	if kill_rx.try_recv().is_ok() {
		return;
	}

	let path = config_dir().join("plugins").join(&uuid);
	tauri::async_runtime::spawn(async move {
		if let Err(e) = super::initialise_plugin(path, spawner_tx).await {
			log::warn!("failed to restart plugin \"{uuid}\": {e:#}");
		}
	});
}

/// Dispatch process spawning for a plugin based on its code path type.
///
/// Returns `Some(PluginInstance::Webview)` for HTML/webview plugins (registered immediately).
/// Returns `None` for Node/Wine/native plugins, which are registered asynchronously by the
/// spawner thread after the process starts.
pub async fn spawn_plugin(
	plugin_uuid: String,
	path: path::PathBuf,
	code_path: String,
	use_wine: bool,
	manifest_name: String,
	manifest_version: String,
	spawner_tx: mpsc::Sender<SpawnRequest>,
) -> anyhow::Result<Option<PluginInstance>> {
	use anyhow::anyhow;

	let args = [
		"-port".to_owned(),
		PORT_BASE.to_string(),
		"-pluginUUID".to_owned(),
		plugin_uuid.clone(),
		"-registerEvent".to_owned(),
		"registerPlugin".to_owned(),
		"-info".to_owned(),
	];

	if code_path.to_lowercase().ends_with(".html") || code_path.to_lowercase().ends_with(".htm") || code_path.to_lowercase().ends_with(".xhtml") {
		let url = format!("http://localhost:{}/", *PORT_BASE + 2) + path.join(&code_path).to_str().unwrap();
		let window = tauri::WebviewWindowBuilder::new(crate::APP_HANDLE.get().unwrap(), plugin_uuid.replace('.', "_"), tauri::WebviewUrl::External(url.parse()?))
			.title(manifest_name)
			.visible(false)
			.build()?;

		if fs::exists(path.join("debug")).unwrap_or(false) {
			let _ = window.show();
			window.open_devtools();
		}

		let info = super::info_param::make_info(plugin_uuid.clone(), manifest_version, false).await;
		window.eval(format!(
			r#"const opendeckInit = () => {{
				try {{
					if (document.readyState !== "complete") throw new Error("not ready");
					if (typeof connectOpenActionSocket === "function") connectOpenActionSocket({port}, "{uuid}", "{event}", `{info}`);
					else connectElgatoStreamDeckSocket({port}, "{uuid}", "{event}", `{info}`);
				}} catch (e) {{
					setTimeout(opendeckInit, 10);
				}}
			}};
			opendeckInit();
			"#,
			port = *PORT_BASE,
			uuid = plugin_uuid,
			event = "registerPlugin",
			info = serde_json::to_string(&info)?
		))?;

		Ok(Some(PluginInstance::Webview))
	} else if code_path.to_lowercase().ends_with(".js") || code_path.to_lowercase().ends_with(".mjs") || code_path.to_lowercase().ends_with(".cjs") {
		let command = if is_flatpak() { "flatpak-spawn" } else { "node" };
		let extra_args = if is_flatpak() { vec!["--host", "node"] } else { vec![] };
		let version_output = Command::new(command).args(&extra_args).arg("--version").output();
		if version_output.is_err() || String::from_utf8(version_output.unwrap().stdout).unwrap().trim() < "v20.0.0" {
			return Err(anyhow!("Node.js version 20.0.0 or higher is required"));
		}

		let info = super::info_param::make_info(plugin_uuid.clone(), manifest_version, true).await;
		let log_file = fs::File::create(log_dir().join("plugins").join(format!("{plugin_uuid}.log")))?;

		spawner_tx
			.send(Box::new(move || {
				let mut command = Command::new(command);
				command
					.current_dir(path)
					.args(extra_args)
					.arg(code_path)
					.args(args)
					.arg(serde_json::to_string(&info)?)
					.stdout(Stdio::from(log_file.try_clone()?))
					.stderr(Stdio::from(log_file));
				#[cfg(target_os = "linux")]
				attach_parent_death_signal(&mut command);
				#[cfg(target_os = "windows")]
				{
					use std::os::windows::process::CommandExt;
					command.creation_flags(0x08000000);
				}
				Ok((plugin_uuid, PluginChildType::Node, command))
			}))
			.map_err(|e| anyhow!(e.to_string()))?;
		Ok(None)
	} else if use_wine {
		let command = if is_flatpak() { "flatpak-spawn" } else { "wine" };
		let extra_args = if is_flatpak() { vec!["--host", "wine"] } else { vec![] };
		let result = Command::new(command)
			.args(&extra_args)
			.arg("--version")
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.spawn()
			.and_then(|mut child| child.wait())
			.map(|status| status.success());
		if !matches!(result, Ok(true)) {
			return Err(anyhow!("failed to detect an installation of Wine"));
		}

		let info = super::info_param::make_info(plugin_uuid.clone(), manifest_version, true).await;
		let log_file = fs::File::create(log_dir().join("plugins").join(format!("{plugin_uuid}.log")))?;

		spawner_tx
			.send(Box::new(move || {
				let mut command = Command::new(command);
				command
					.current_dir(&path)
					.args(extra_args)
					.arg(code_path)
					.args(args)
					.arg(serde_json::to_string(&info)?)
					.stdout(Stdio::from(log_file.try_clone()?))
					.stderr(Stdio::from(log_file));
				if crate::store::get_settings().value.separatewine {
					command.env("WINEPREFIX", path.join("wineprefix").to_string_lossy().to_string());
				} else {
					let _ = fs::remove_dir_all(path.join("wineprefix"));
				}
				#[cfg(target_os = "linux")]
				attach_parent_death_signal(&mut command);
				Ok((plugin_uuid, PluginChildType::Wine, command))
			}))
			.map_err(|e| anyhow!(e.to_string()))?;
		Ok(None)
	} else {
		let info = super::info_param::make_info(plugin_uuid.clone(), manifest_version, false).await;
		let log_file = fs::File::create(log_dir().join("plugins").join(format!("{plugin_uuid}.log")))?;

		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			fs::set_permissions(path.join(&code_path), fs::Permissions::from_mode(0o755))?;
		}

		spawner_tx
			.send(Box::new(move || {
				let mut command = Command::new(path.join(code_path));
				command
					.current_dir(path)
					.args(args)
					.arg(serde_json::to_string(&info)?)
					.stdout(Stdio::from(log_file.try_clone()?))
					.stderr(Stdio::from(log_file));
				#[cfg(target_os = "linux")]
				attach_parent_death_signal(&mut command);
				#[cfg(target_os = "windows")]
				{
					use std::os::windows::process::CommandExt;
					command.creation_flags(0x08000000);
				}
				Ok((plugin_uuid, PluginChildType::Native, command))
			}))
			.map_err(|e| anyhow!(e.to_string()))?;
		Ok(None)
	}
}
