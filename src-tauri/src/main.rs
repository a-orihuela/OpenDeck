// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application_watcher;
pub mod builtin_actions;
pub mod builtin_pomodoro;
pub mod constants;
mod device_sleep;
mod elgato;
mod events;
mod plugins;
mod shared;
mod store;
mod zip_extract;

mod built_info {
	include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use constants::{BINARY_NAME, PRODUCT_NAME, TRAY_ID, APP_ID, FILE_PORTS_LOCK};
use events::frontend;

use std::sync::OnceLock;

use tauri::{
	AppHandle, Builder, Manager, WindowEvent,
	menu::{IconMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
	tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_log::{Target, TargetKind};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

fn show_window(app: &AppHandle) -> Result<(), tauri::Error> {
	#[cfg(target_os = "macos")]
	{
		use tauri::ActivationPolicy;
		let _ = app.set_activation_policy(ActivationPolicy::Regular);
	}

	let window = app.get_webview_window("main").ok_or_else(|| tauri::Error::WebviewNotFound)?;
	window.show().and_then(|_| window.set_focus())
}

fn hide_window(app: &AppHandle) -> Result<(), tauri::Error> {
	let window = app.get_webview_window("main").ok_or_else(|| tauri::Error::WebviewNotFound)?;
	window.hide()?;

	#[cfg(target_os = "macos")]
	{
		use tauri::ActivationPolicy;
		let _ = app.set_activation_policy(ActivationPolicy::Accessory);
	}

	Ok(())
}

#[tokio::main]
async fn main() {
	log_panics::init();
	let _ = fix_path_env::fix();

	// Export TypeScript bindings for all shared types during development builds.
	// This regenerates src/lib/bindings.ts automatically on every debug build.
	//
	// Uses a plain identity format (no serde-alias processing) so the emitted
	// TypeScript reflects the serialize-time field names, which is what the
	// frontend receives over Tauri IPC.
	#[cfg(debug_assertions)]
	{
		use specta::Types;
		use specta_typescript::Typescript;

		struct SerializeFormat;
		impl specta::Format for SerializeFormat {
			fn map_types(&self, types: &specta::Types) -> Result<std::borrow::Cow<'_, specta::Types>, specta::FormatError> {
				Ok(std::borrow::Cow::Owned(types.clone()))
			}
			fn map_type(&self, _types: &specta::Types, dt: &specta::datatype::DataType) -> Result<std::borrow::Cow<'_, specta::datatype::DataType>, specta::FormatError> {
				Ok(std::borrow::Cow::Owned(dt.clone()))
			}
		}

		let types = Types::default()
			.register::<crate::shared::DeviceInfo>()
			.register::<crate::shared::Action>()
			.register::<crate::shared::ActionInstance>()
			.register::<crate::shared::Context>()
			.register::<crate::shared::Profile>()
			.register::<crate::shared::Category>()
			.register::<crate::store::Settings>();

		if let Err(e) = Typescript::default().export_to(
			concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/bindings.ts"),
			&types,
			SerializeFormat,
		) {
			eprintln!("Warning: failed to export TypeScript bindings: {e}");
		}
	}

	#[cfg(target_os = "linux")]
	// SAFETY: std::env::set_var can cause race conditions in multithreaded contexts. We have not spawned any other threads at this point.
	unsafe {
		std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
		std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
	}

	let app = match Builder::default()
		.invoke_handler(tauri::generate_handler![
			frontend::restart,
			frontend::get_devices,
			frontend::get_port_base,
			frontend::get_categories,
			frontend::get_localisations,
			frontend::get_applications,
			frontend::get_application_profiles,
			frontend::set_application_profiles,
			frontend::get_fonts,
			frontend::instances::create_instance,
			frontend::instances::move_instance,
			frontend::instances::remove_instance,
			frontend::instances::set_state,
			frontend::instances::set_instance_settings,
			frontend::instances::update_image,
			frontend::instances::trigger_virtual_press,
			frontend::folders::enter_folder,
			frontend::folders::exit_folder,
			frontend::pages::get_active_page,
			frontend::pages::set_active_page,
			frontend::pages::add_page,
			frontend::pages::remove_last_page,
			frontend::profiles::get_profiles,
			frontend::profiles::get_selected_profile,
			frontend::profiles::set_selected_profile,
			frontend::profiles::delete_profile,
			frontend::profiles::rename_profile,
			frontend::property_inspector::make_info,
			frontend::property_inspector::switch_property_inspector,
			frontend::property_inspector::open_url,
			frontend::plugins::list_plugins,
			frontend::plugins::install_plugin,
			frontend::plugins::remove_plugin,
			frontend::plugins::reload_plugin,
			frontend::plugins::show_settings_interface,
			frontend::settings::get_settings,
			frontend::settings::set_settings,
			frontend::settings::open_config_directory,
			frontend::settings::open_log_directory,
			frontend::settings::get_build_info,
			frontend::settings::backup_config_directory,
			frontend::settings::restore_config_directory,
		])
		.setup(|app| {
			APP_HANDLE.set(app.handle().clone()).unwrap();

			#[cfg(windows)]
			if !std::env::args().any(|v| v == "--hide") {
				let _ = app.get_webview_window("main").unwrap().show();
			}
			#[cfg(not(windows))]
			if std::env::args().any(|v| v == "--hide") {
				let _ = hide_window(app.handle());
			}


			let mut settings = store::get_settings();
			use std::cmp::Ordering;
			use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
			let current_version = semver::Version::parse(built_info::PKG_VERSION)?;
			let settings_version = semver::Version::parse(&settings.value.version)?;
			let cmp = (current_version.major, current_version.minor).cmp(&(settings_version.major, settings_version.minor));
			match cmp {
				Ordering::Less => {
					app.get_webview_window("main").unwrap().close().unwrap();
					app.dialog()
						.message(format!(
							"A newer version of {PRODUCT_NAME} created configuration files on this device. This version is v{}; please upgrade to v{} or newer.",
							built_info::PKG_VERSION,
							settings.value.version
						))
						.title(format!("{PRODUCT_NAME} upgrade required"))
						.kind(MessageDialogKind::Error)
						.show(|_| APP_HANDLE.get().unwrap().exit(1));
					return Ok(());
				}
				Ordering::Greater => {
					let old_version = settings.value.version.clone();
					settings.value.version = built_info::PKG_VERSION.to_owned();
					settings.save()?;
					if old_version == "0.0.0" {
						app.dialog()
							.message(format!("Thanks for installing {PRODUCT_NAME}!\n\nIf you have any issues, please open an issue on GitHub.\n\nEnjoy!"))
							.title(format!("{PRODUCT_NAME} has successfully been installed"))
							.kind(MessageDialogKind::Info)
							.show(|_| ());
					} else {
						app.dialog()
							.message(format!("{PRODUCT_NAME} has been updated to v{}!", built_info::PKG_VERSION))
							.title(format!("{PRODUCT_NAME} has successfully been updated"))
							.kind(MessageDialogKind::Info)
							.show(|_| ());
					}
				}
				_ => {}
			}

			tokio::spawn(async {
				loop {
					elgato::initialise_devices().await;
					tokio::time::sleep(std::time::Duration::from_secs(10)).await;
				}
			});
			plugins::initialise_plugins();
			application_watcher::init_application_watcher();
			device_sleep::init_device_sleep();

			let label = IconMenuItemBuilder::with_id("label", PRODUCT_NAME)
				.icon(app.default_window_icon().unwrap().clone())
				.enabled(false)
				.build(app)?;
			let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
			let hide = MenuItemBuilder::with_id("hide", "Hide").build(app)?;
			let restart = MenuItemBuilder::with_id("restart", "Restart").build(app)?;
			let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
			let separator = PredefinedMenuItem::separator(app)?;
			let menu = MenuBuilder::new(app).items(&[&label, &separator, &show, &hide, &separator, &restart, &quit]).build()?;
			let _tray = TrayIconBuilder::with_id(TRAY_ID)
				.menu(&menu)
				.icon(app.default_window_icon().unwrap().clone())
				.show_menu_on_left_click(false)
				.on_tray_icon_event(move |icon, event| {
					if let TrayIconEvent::Click { button, button_state, .. } = event {
						if button != MouseButton::Left || button_state != MouseButtonState::Down {
							return;
						}

						let app_handle = icon.app_handle();
						let window = app_handle.get_webview_window("main").unwrap();
						let _ = if window.is_visible().unwrap_or(false) { hide_window(app_handle) } else { show_window(app_handle) };
					}
				})
				.on_menu_event(move |app, event| {
					let _ = match event.id().as_ref() {
						"show" => show_window(app),
						"hide" => hide_window(app),
						"restart" => app.restart(),
						"quit" => {
							app.exit(0);
							Ok(())
						}
						_ => Ok(()),
					};
				})
				.build(app)?;

			#[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
			{
				use tauri_plugin_deep_link::DeepLinkExt;
				let _ = app.deep_link().register_all();
			}

			Ok(())
		})
		.plugin(
			tauri_plugin_log::Builder::default()
				.targets([Target::new(TargetKind::LogDir { file_name: None }), Target::new(TargetKind::Stdout)])
				.level(log::LevelFilter::Info)
				.level_for(BINARY_NAME, log::LevelFilter::Trace)
				.build(),
		)
		.plugin(tauri_plugin_cors_fetch::init())
		.plugin(
			tauri_plugin_single_instance::Builder::new()
				.callback(|app, args, _| {
					if let Some(pos) = args.iter().position(|x| x.starts_with("openaction://") || x.starts_with("streamdeck://"))
						&& let Ok(url) = reqwest::Url::parse(&args[pos])
						&& let Some(mut path) = url.path_segments()
					{
						if url.host_str() == Some("plugins")
							&& path.next() == Some("message")
							&& let Some(plugin_id) = path.next()
						{
							if !url.query_pairs().any(|(k, v)| k == url.scheme() && v == "hidden") {
								let _ = show_window(app);
							}

							let plugin_id = if url.scheme() == "streamdeck" { format!("{plugin_id}.sdPlugin") } else { plugin_id.to_owned() };
							let url = args[pos].clone();
							std::thread::spawn(move || {
								tauri::async_runtime::block_on(async move {
									if let Err(error) = events::outbound::deep_link::did_receive_deep_link(&plugin_id, url).await {
										log::error!("Failed to process deep link for plugin {plugin_id}: {error}");
									}
								});
							});
						}
					} else if let Some(pos) = args.iter().position(|x| x.to_lowercase().trim() == "--reload-plugin") {
						if args.len() > pos + 1 {
							let app = app.clone();
							let plugin_id = args[pos + 1].clone();
							std::thread::spawn(move || {
								tauri::async_runtime::block_on(frontend::plugins::reload_plugin(app, plugin_id));
							});
						}
					} else if let Some(pos) = args.iter().position(|x| x.to_lowercase().trim() == "--sleep-device") {
						if args.len() > pos + 1 {
							let device_id = args[pos + 1].clone();
							std::thread::spawn(move || {
								if let Err(error) = tauri::async_runtime::block_on(device_sleep::sleep_device(device_id)) {
									log::error!("Failed to sleep device: {error}");
								}
							});
						}
					} else if let Some(pos) = args.iter().position(|x| x.to_lowercase().trim() == "--wake-device") {
						if args.len() > pos + 1 {
							let device_id = args[pos + 1].clone();
							std::thread::spawn(move || {
								if let Err(error) = tauri::async_runtime::block_on(device_sleep::note_activity(&device_id)) {
									log::error!("Failed to wake device: {error}");
								}
							});
						}
					} else if let Some(pos) = args.iter().position(|x| x.to_lowercase().trim() == "--process-message") {
						if args.len() > pos + 1 {
							let message = args[pos + 1].clone();
							std::thread::spawn(move || {
								tauri::async_runtime::block_on(events::inbound::process_incoming_message(Ok(tokio_tungstenite::tungstenite::Message::Text(message.into())), "", true));
							});
						}
					} else {
						let _ = show_window(app);
					}
				})
				.dbus_id(APP_ID)
				.build(),
		)
		.plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hide"])))
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_deep_link::init())
		.on_window_event(|window, event| {
			if window.label() != "main" {
				return;
			}
			if let WindowEvent::CloseRequested { api, .. } = event {
				if store::get_settings().value.background {
					let _ = hide_window(window.app_handle());
					api.prevent_close();
				} else {
					window.app_handle().exit(0);
				}
			}
		})
		.build(tauri::generate_context!())
	{
		Ok(app) => app,
		Err(error) => panic!("failed to build Tauri application: {}", error),
	};

	app.run(|_app, event| {
		if let tauri::RunEvent::Exit = event {
			#[cfg(windows)]
			futures::executor::block_on(plugins::deactivate_plugins());
			tokio::spawn(elgato::reset_devices());
			let _ = std::fs::remove_file(shared::config_dir().join(FILE_PORTS_LOCK));
		}
	});
}
