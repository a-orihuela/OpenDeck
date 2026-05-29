pub mod info_param;
pub mod manifest;
mod spawn;
mod webserver;
mod websocket;

use crate::APP_HANDLE;
use crate::shared::{CATEGORIES, Category, config_dir, convert_icon, log_dir};

use std::collections::HashMap;
use std::sync::{LazyLock, mpsc};
use std::{fs, path};

use tauri::{AppHandle, Manager};

use anyhow::anyhow;
use log::{error, warn};
use tokio::sync::{Mutex, RwLock};

pub use spawn::SpawnRequest;
use spawn::{PluginChildType, PluginInstance, ProcessHandle, kill_process, spawn_plugin, supervise_plugin};

pub static DEVICE_NAMESPACES: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static INSTANCES: LazyLock<Mutex<HashMap<String, PluginInstance>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub static PORT_BASE: LazyLock<u16> = LazyLock::new(|| {
	let lock_path = crate::shared::config_dir().join(crate::constants::FILE_PORTS_LOCK);

	// Try to reuse previously allocated ports from the lock file.
	if let Ok(content) = std::fs::read_to_string(&lock_path) {
		if let Ok(saved) = serde_json::from_str::<u16>(&content) {
			let ws_ok = std::net::TcpListener::bind(format!("0.0.0.0:{}", saved)).is_ok();
			let http_ok = std::net::TcpListener::bind(format!("0.0.0.0:{}", saved + 2)).is_ok();
			if ws_ok && http_ok {
				log::debug!("Reusing persisted ports {} and {}", saved, saved + 2);
				return saved;
			}
		}
	}

	// Scan for a free pair of ports and persist the result.
	let mut base = crate::constants::PORT_BASE;
	loop {
		let ws_ok = std::net::TcpListener::bind(format!("127.0.0.1:{}", base)).is_ok();
		let http_ok = std::net::TcpListener::bind(format!("127.0.0.1:{}", base + 2)).is_ok();
		if ws_ok && http_ok {
			log::debug!("Using ports {} and {}", base, base + 2);
			break;
		}
		base = base.saturating_add(1);
	}

	let _ = std::fs::write(&lock_path, serde_json::to_string(&base).unwrap());
	base
});

/// Initialise a plugin from a given directory.
pub async fn initialise_plugin(path: path::PathBuf, spawner_tx: mpsc::Sender<SpawnRequest>) -> anyhow::Result<()> {
	let plugin_uuid = path.file_name().unwrap().to_str().unwrap().to_owned();
	let plugin_uuid_2 = plugin_uuid.clone();

	let mut manifest = manifest::read_manifest(&path)?;

	if let Some(icon) = manifest.category_icon {
		let category_icon_path = path.join(icon);
		manifest.category_icon = Some(convert_icon(category_icon_path.to_string_lossy().to_string()));
	}

	for action in &mut manifest.actions {
		plugin_uuid.clone_into(&mut action.plugin);

		let action_icon_path = path.join(action.icon.clone());
		action.icon = convert_icon(action_icon_path.to_str().unwrap().to_owned());

		if !action.property_inspector.is_empty() {
			action.property_inspector = path.join(&action.property_inspector).to_string_lossy().to_string();
		} else if let Some(ref property_inspector) = manifest.property_inspector_path {
			action.property_inspector = path.join(property_inspector).to_string_lossy().to_string();
		}

		for state in &mut action.states {
			if state.image == "actionDefaultImage" {
				state.image.clone_from(&action.icon);
			} else {
				let state_icon = path.join(state.image.clone());
				state.image = convert_icon(state_icon.to_str().unwrap().to_owned());
			}

			match state.family.clone().to_lowercase().trim() {
				"arial" => "Liberation Sans",
				"arial black" => "Archivo Black",
				"comic sans ms" => "Comic Neue",
				"courier" | "Courier New" => "Courier Prime",
				"georgia" => "Tinos",
				"impact" => "Anton",
				"microsoft sans serif" | "Times New Roman" => "Liberation Serif",
				"tahoma" | "Verdana" => "Open Sans",
				"trebuchet ms" => "Fira Sans",
				_ => continue,
			}
			.clone_into(&mut state.family);
		}
	}

	{
		let mut categories = CATEGORIES.write().await;
		for action in manifest.actions {
			let cat_name = action.category.as_deref().unwrap_or(&manifest.category).to_owned();
			let cat_icon = if cat_name == manifest.category { manifest.category_icon.clone() } else { None };
			let cat = categories.entry(cat_name).or_insert_with(|| Category { icon: cat_icon, actions: vec![] });
			if let Some(index) = cat.actions.iter().position(|v| v.uuid == action.uuid) {
				cat.actions.remove(index);
			}
			cat.actions.push(action);
		}
	}

	if let Some(namespace) = manifest.device_namespace {
		DEVICE_NAMESPACES.write().await.insert(namespace, plugin_uuid.to_owned());
	}

	if let Some(caps) = manifest.capabilities {
		crate::shared::PLUGIN_CAPABILITIES.insert(plugin_uuid.clone(), caps);
	}

	#[cfg(target_os = "windows")]
	let platform = "windows";
	#[cfg(target_os = "macos")]
	let platform = "mac";
	#[cfg(target_os = "linux")]
	let platform = "linux";

	let mut code_path = manifest.code_path;
	let mut use_wine = false;
	let mut supported = false;

	for os in manifest.os {
		if os.platform == platform {
			#[cfg(target_os = "windows")]
			if manifest.code_path_windows.is_some() {
				code_path = manifest.code_path_windows.clone();
			}
			#[cfg(target_os = "macos")]
			if manifest.code_path_macos.is_some() {
				code_path = manifest.code_path_macos;
			}
			#[cfg(target_os = "linux")]
			if manifest.code_path_linux.is_some() {
				code_path = manifest.code_path_linux;
			}
			code_path = manifest.code_paths.and_then(|p| p.get(crate::built_info::TARGET).cloned()).or(code_path);

			use_wine = false;
			supported = true;
			break;
		} else if os.platform == "windows" {
			use_wine = true;
			supported = true;
		}
	}

	if code_path.is_none() && use_wine {
		code_path = manifest.code_path_windows;
	}

	if !supported || code_path.is_none() {
		return Err(anyhow!("unsupported on platform {}", platform));
	}

	if let Some(instance) = spawn_plugin(plugin_uuid.clone(), path.clone(), code_path.unwrap(), use_wine, manifest.name, manifest.version, spawner_tx).await? {
		INSTANCES.lock().await.insert(plugin_uuid, instance);
	}

	if let Some(applications) = manifest.applications_to_monitor
		&& let Some(applications) = applications.get(platform)
	{
		crate::application_watcher::start_monitoring(&plugin_uuid_2, applications).await;
	}

	Ok(())
}

pub async fn deactivate_plugin(app: &AppHandle, uuid: &str) -> Result<(), anyhow::Error> {
	{
		let mut namespaces = DEVICE_NAMESPACES.write().await;
		if let Some((namespace, _)) = namespaces.clone().iter().find(|(_, plugin)| uuid == **plugin) {
			namespaces.remove(namespace);
			drop(namespaces);
			let devices = crate::shared::DEVICES.iter().map(|v| v.key().to_owned()).filter(|id| &id[..2] == namespace).collect::<Vec<_>>();
			for device in devices {
				crate::events::inbound::devices::deregister_device("", crate::events::inbound::PayloadEvent { payload: device }).await?;
			}
			crate::events::frontend::update_devices().await;
		}
	}

	crate::application_watcher::stop_monitoring(uuid).await;

	if let Some(instance) = INSTANCES.lock().await.remove(uuid) {
		match instance {
			PluginInstance::Webview => {
				if let Some(window) = app.get_webview_window(&uuid.replace('.', "_")) {
					window.close()?;
					tokio::time::sleep(std::time::Duration::from_millis(10)).await;
				}
			}
			PluginInstance::Node(handle) | PluginInstance::Wine(handle) | PluginInstance::Native(handle) => {
				let _ = handle.kill_tx.send(());
				kill_process(handle.pid);
			}
		}
		Ok(())
	} else {
		Err(anyhow!("instance of plugin {} not found", uuid))
	}
}

#[cfg(windows)]
pub async fn deactivate_plugins() {
	let uuids = {
		let instances = INSTANCES.lock().await;
		instances.keys().cloned().collect::<Vec<_>>()
	};

	let app = APP_HANDLE.get().unwrap();
	for uuid in uuids {
		let _ = deactivate_plugin(app, &uuid).await;
	}
}

/// Initialise plugins from the plugins directory.
pub fn initialise_plugins() {
	tokio::spawn(websocket::init_websocket_server());
	tokio::spawn(webserver::init_webserver(config_dir()));

	let plugin_dir = config_dir().join("plugins");
	let _ = fs::create_dir_all(&plugin_dir);
	let _ = fs::create_dir_all(log_dir().join("plugins"));

	if let Ok(Ok(entries)) = APP_HANDLE.get().unwrap().path().resolve("plugins", tauri::path::BaseDirectory::Resource).map(fs::read_dir) {
		for entry in entries.flatten() {
			if let Err(error) = (|| -> Result<(), anyhow::Error> {
				let builtin_version = semver::Version::parse(&serde_json::from_slice::<manifest::PluginManifest>(&fs::read(entry.path().join("manifest.json"))?)?.version)?;
				let existing_path = plugin_dir.join(entry.file_name());
				if (|| -> Result<(), anyhow::Error> {
					let existing_version = semver::Version::parse(&serde_json::from_slice::<manifest::PluginManifest>(&fs::read(existing_path.join("manifest.json"))?)?.version)?;
					if existing_version < builtin_version {
						Err(anyhow::anyhow!("builtin version is newer than existing version"))
					} else {
						Ok(())
					}
				})()
				.is_err()
				{
					if existing_path.exists() {
						fs::rename(&existing_path, existing_path.with_extension("old"))?;
					}
					if crate::shared::copy_dir(entry.path(), &existing_path).is_err() && existing_path.with_extension("old").exists() {
						fs::rename(existing_path.with_extension("old"), &existing_path)?;
					}
					let _ = fs::remove_dir_all(existing_path.with_extension("old"));
				}
				Ok(())
			})() {
				error!("Failed to upgrade builtin plugin {}: {}", entry.file_name().to_string_lossy(), error);
			}
		}
	}

	let entries = match fs::read_dir(&plugin_dir) {
		Ok(p) => p,
		Err(error) => {
			error!("Failed to read plugins directory at {}: {}", plugin_dir.display(), error);
			panic!()
		}
	};

	let (tx, rx) = mpsc::channel::<SpawnRequest>();
	APP_HANDLE.get().unwrap().manage(tx.clone());

	let tx_for_watchdogs = tx.clone();
	// Use a dedicated spawner thread so that plugin processes don't die due to PR_SET_PDEATHSIG when the parent Tokio worker exits
	std::thread::spawn(move || {
		for f in rx {
			match f() {
				Ok((plugin_uuid, child_type, mut command)) => match command.spawn() {
					Ok(child) => {
						let pid = child.id();
						let (kill_tx, kill_rx) = mpsc::sync_channel::<()>(1);
						let watchdog_uuid = plugin_uuid.clone();
						let watchdog_spawner_tx = tx_for_watchdogs.clone();
						std::thread::spawn(move || {
							supervise_plugin(child, watchdog_uuid, watchdog_spawner_tx, kill_rx);
						});
						INSTANCES.blocking_lock().insert(
							plugin_uuid,
							match child_type {
								PluginChildType::Wine => PluginInstance::Wine(ProcessHandle { pid, kill_tx }),
								PluginChildType::Native => PluginInstance::Native(ProcessHandle { pid, kill_tx }),
								PluginChildType::Node => PluginInstance::Node(ProcessHandle { pid, kill_tx }),
							},
						);
					}
					Err(error) => warn!("Failed to initialise plugin {}: {}", plugin_uuid, error),
				},
				Err(error) => warn!("Failed to initialise plugin: {}", error),
			}
		}
	});

	for entry in entries {
		if let Ok(entry) = entry {
			let path = match entry.metadata().unwrap().is_symlink() {
				true => entry.path().parent().unwrap_or_else(|| path::Path::new(".")).join(fs::read_link(entry.path()).unwrap()),
				false => entry.path(),
			};
			let metadata = fs::metadata(&path).unwrap();
			if metadata.is_dir() {
				let spawner_tx = tx.clone();
				tokio::spawn(async move {
					if let Err(error) = initialise_plugin(path.clone(), spawner_tx).await {
						warn!("Failed to initialise plugin at {}: {:#}", path.display(), error);
					}
				});
			}
		} else if let Err(error) = entry {
			warn!("Failed to read entry of plugins directory: {}", error)
		}
	}

	// On macOS, hidden WKWebView windows suspend JavaScript after ~7s.
	// Periodically eval a no-op to keep them alive.
	#[cfg(target_os = "macos")]
	tokio::spawn(async {
		use tauri::Manager;
		let app = APP_HANDLE.get().unwrap();
		loop {
			tokio::time::sleep(std::time::Duration::from_secs(3)).await;
			let instances = INSTANCES.lock().await;
			for (uuid, _) in instances.iter().filter(|(_, instance)| matches!(instance, PluginInstance::Webview)) {
				if let Some(window) = app.get_webview_window(&uuid.replace('.', "_")) {
					let _ = window.eval("void(0);");
				}
			}
		}
	});
}
