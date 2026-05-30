use super::Error;

use crate::constants::{ACTION_FOLDER, ACTION_MULTIACTION, ACTION_TOGGLEACTION};
use crate::shared::{Action, ActionContext, ActionInstance, ActionState, Context, DEVICE_ACTIVE_FOLDER, config_dir};
use crate::store::profiles::{LocksMut, acquire_locks, acquire_locks_mut, get_instance_mut, get_slot, get_slot_mut, save_profile};

use tauri::{AppHandle, Emitter, Manager, command};
use tokio::fs::remove_dir_all;

fn normalise_action_states(action: &Action) -> Vec<ActionState> {
	action
		.states
		.iter()
		.cloned()
		.map(|mut state| {
			if state.image == "actionDefaultImage" {
				state.image = action.icon.clone();
			}
			state
		})
		.collect()
}

#[command]
pub async fn create_instance(app: AppHandle, action: Action, context: Context) -> Result<Option<ActionInstance>, Error> {
	// In folder mode: route keypad drops to the open folder's child slots.
	if context.controller == "Keypad" {
		if let Some(folder_ctx) = DEVICE_ACTIVE_FOLDER.get(&context.device).map(|r| r.clone()) {
			return crate::events::frontend::folders::create_folder_child_impl(
				&context.device,
				folder_ctx,
				context.position as usize,
				action,
			).await;
		}
	}

	if !action.controllers.contains(&context.controller) {
		return Ok(None);
	}

	let mut locks = acquire_locks_mut().await;
	let slot = get_slot_mut(&context, &mut locks).await?;

	if let Some(parent) = slot {
		let Some(children) = &mut parent.children else { return Ok(None) };
		let index = match children.last() {
			None => 1,
			Some(instance) => instance.context.index + 1,
		};

		let instance = ActionInstance {
			action: action.clone(),
			context: ActionContext::from_context(context.clone(), index),
			states: normalise_action_states(&action),
			current_state: 0,
			settings: serde_json::Value::Object(serde_json::Map::new()),
			children: None,
			folder_slots: None,
		};
		children.push(instance.clone());

		if parent.action.uuid == ACTION_TOGGLEACTION && parent.states.len() < children.len() {
			parent.states.push(crate::shared::ActionState {
				image: "omegadeck/toggle-action.png".to_owned(),
				..Default::default()
			});
			let _ = update_state(&app, parent.context.clone(), &mut locks).await;
		}

		save_profile(&context.device, &mut locks).await?;
		drop(locks);
		let _ = crate::events::outbound::will_appear::will_appear(&instance).await;

		let locks = acquire_locks().await;
		let slot = get_slot(&context, &locks).await?.clone();
		Ok(slot)
	} else {
		let folder_slots = if action.uuid == ACTION_FOLDER {
			let page_size = crate::shared::DEVICES.get(&context.device)
				.map(|d| (d.rows * d.columns) as usize)
				.unwrap_or(15);
			Some(vec![None; page_size])
		} else {
			None
		};

		let instance = ActionInstance {
			action: action.clone(),
			context: ActionContext::from_context(context.clone(), 0),
			states: normalise_action_states(&action),
			current_state: 0,
			settings: serde_json::Value::Object(serde_json::Map::new()),
			children: if matches!(action.uuid.as_str(), ACTION_MULTIACTION | ACTION_TOGGLEACTION) {
				Some(vec![])
			} else {
				None
			},
			folder_slots,
		};

		*slot = Some(instance.clone());
		let slot = slot.clone();

		save_profile(&context.device, &mut locks).await?;
		let _ = crate::events::outbound::will_appear::will_appear(&instance).await;

		Ok(slot)
	}
}

fn instance_images_dir(context: &ActionContext) -> std::path::PathBuf {
	config_dir()
		.join("images")
		.join(&context.device)
		.join(&context.profile)
		.join(format!("{}.{}.{}", context.controller, context.position, context.index))
}

#[command]
pub async fn move_instance(source: Context, destination: Context, retain: bool) -> Result<Option<ActionInstance>, Error> {
	if source.controller != destination.controller {
		return Ok(None);
	}

	{
		let locks = crate::store::profiles::acquire_locks().await;
		let dst = crate::store::profiles::get_slot(&destination, &locks).await?;
		if dst.is_some() {
			return Ok(None);
		}
	}

	let mut locks = acquire_locks_mut().await;
	let src = get_slot_mut(&source, &mut locks).await?;

	let Some(mut new) = src.clone() else {
		return Ok(None);
	};
	new.context = ActionContext::from_context(destination.clone(), 0);
	if let Some(children) = &mut new.children {
		for (index, instance) in children.iter_mut().enumerate() {
			instance.context = ActionContext::from_context(destination.clone(), index as u16 + 1);
			for (i, state) in instance.states.iter_mut().enumerate() {
				if !instance.action.states[i].image.is_empty() {
					if instance.action.states[i].image == "actionDefaultImage" {
						state.image = instance.action.icon.clone();
					} else {
						state.image = instance.action.states[i].image.clone();
					}
				} else {
					state.image = instance.action.icon.clone();
				}
			}
		}
	}

	let old_dir = instance_images_dir(&src.as_ref().unwrap().context);
	let new_dir = instance_images_dir(&new.context);
	let _ = tokio::fs::create_dir_all(&new_dir).await;
	if let Ok(files) = old_dir.read_dir() {
		for file in files.flatten() {
			let _ = tokio::fs::copy(file.path(), new_dir.join(file.file_name())).await;
		}
	}
	for state in new.states.iter_mut() {
		let path = std::path::Path::new(&state.image);
		if path.starts_with(&old_dir) {
			state.image = new_dir.join(path.strip_prefix(&old_dir).unwrap()).to_string_lossy().into_owned();
		}
	}

	let dst = get_slot_mut(&destination, &mut locks).await?;
	*dst = Some(new.clone());

	if !retain {
		let src = get_slot_mut(&source, &mut locks).await?;
		if let Some(old) = src {
			let _ = crate::events::outbound::will_appear::will_disappear(old, true).await;
			let _ = remove_dir_all(instance_images_dir(&old.context)).await;
		}
		*src = None;
	}

	let _ = crate::events::outbound::will_appear::will_appear(&new).await;

	save_profile(&destination.device, &mut locks).await?;

	Ok(Some(new))
}

#[command]
pub async fn remove_instance(context: ActionContext) -> Result<(), Error> {
	// In folder mode: route keypad removes to the open folder's child slots.
	if context.controller == "Keypad" {
		if let Some(folder_ctx) = DEVICE_ACTIVE_FOLDER.get(&context.device).map(|r| r.clone()) {
			return crate::events::frontend::folders::remove_folder_child_impl(
				&context.device,
				folder_ctx,
				context.position as usize,
			).await;
		}
	}

	let mut locks = acquire_locks_mut().await;
	let slot = get_slot_mut(&(&context).into(), &mut locks).await?;
	let Some(instance) = slot else {
		return Ok(());
	};

	if instance.context == context {
		let _ = crate::events::outbound::will_appear::will_disappear(instance, true).await;
		if let Some(children) = &instance.children {
			for child in children {
				let _ = crate::events::outbound::will_appear::will_disappear(child, true).await;
				let _ = remove_dir_all(instance_images_dir(&child.context)).await;
			}
		}
		let _ = remove_dir_all(instance_images_dir(&instance.context)).await;
		*slot = None;
	} else {
		let children = instance.children.as_mut().unwrap();
		for (index, instance) in children.iter().enumerate() {
			if instance.context == context {
				let _ = crate::events::outbound::will_appear::will_disappear(instance, true).await;
				let _ = remove_dir_all(instance_images_dir(&instance.context)).await;
				children.remove(index);
				break;
			}
		}
		if instance.action.uuid == ACTION_TOGGLEACTION {
			if instance.current_state as usize >= children.len() {
				instance.current_state = if children.is_empty() { 0 } else { children.len() as u16 - 1 };
			}
			if !children.is_empty() {
				instance.states.pop();
				let _ = update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await;
			}
		}
	}

	save_profile(&context.device, &mut locks).await?;

	Ok(())
}

#[derive(Clone, serde::Serialize)]
struct UpdateStateEvent {
	context: ActionContext,
	contents: Option<ActionInstance>,
}

pub async fn update_state(app: &AppHandle, context: ActionContext, locks: &mut LocksMut<'_>) -> Result<(), anyhow::Error> {
	let window = app.get_webview_window("main").unwrap();
	window.emit(
		"update_state",
		UpdateStateEvent {
			contents: get_instance_mut(&context, locks).await?.cloned(),
			context,
		},
	)?;
	Ok(())
}

#[command]
pub async fn set_state(context: ActionContext, index: u16, state: ActionState) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	let reference = get_instance_mut(&context, &mut locks).await?.unwrap();
	reference.states[index as usize] = state;
	let clone = reference.clone();
	save_profile(&context.device, &mut locks).await?;
	crate::events::outbound::states::title_parameters_did_change(&clone, index).await?;
	Ok(())
}

#[command]
pub async fn set_instance_settings(context: ActionContext, settings: serde_json::Value) -> Result<(), Error> {
	let mut locks = acquire_locks_mut().await;
	let reference = get_instance_mut(&context, &mut locks).await?.unwrap();
	reference.settings = settings;
	let clone = reference.clone();
	save_profile(&context.device, &mut locks).await?;
	crate::events::outbound::settings::did_receive_settings(&clone, true).await?;
	Ok(())
}

#[command]
pub async fn update_image(mut context: Context, image: Option<String>) {
	if Some(&context.profile) != crate::store::profiles::DEVICE_STORES.write().await.get_selected_profile(&context.device).ok().as_ref() {
		return;
	}

	// Translate the page-aware flat profile position to the physical device position.
	// The frontend encodes keypad positions as: active_page * page_size + physical_key.
	// For devices with touchpoints, the touchpoint at index i is at num_pages * page_size + i,
	// which maps to the physical position page_size + i.
	if context.controller == "Keypad" {
		if let Some(device_info) = crate::shared::DEVICES.get(&context.device) {
			let page_size = (device_info.rows * device_info.columns) as usize;
			if page_size > 0 {
				// In folder mode: positions are already physical (0..page_size-1); skip translation.
				if !DEVICE_ACTIVE_FOLDER.contains_key(&context.device) {
					let flat = context.position as usize;
					let current_page = crate::shared::DEVICE_ACTIVE_PAGES.get(&context.device).map(|p| *p as usize).unwrap_or(0);

					if device_info.touchpoints > 0 {
						let num_pages = {
							let profile_stores = crate::store::profiles::PROFILE_STORES.read().await;
							profile_stores
								.get_profile_store(&device_info, &context.profile)
								.map(|s| s.value.num_pages as usize)
								.unwrap_or(1)
						};
						let touchpoint_start = num_pages * page_size;
						if flat >= touchpoint_start {
							context.position = (page_size + flat - touchpoint_start) as u8;
						} else {
							if flat / page_size != current_page {
								return;
							}
							context.position = (flat % page_size) as u8;
						}
					} else {
						if flat / page_size != current_page {
							return;
						}
						context.position = (flat % page_size) as u8;
					}
				}
			}
		}
	}

	if let Err(error) = crate::events::outbound::devices::update_image(context, image).await {
		log::warn!("Failed to update device image: {}", error);
	}
}

#[command]
pub async fn trigger_virtual_press(context: Context) -> Result<(), Error> {
	let event = || crate::events::inbound::PayloadEvent {
		payload: crate::events::inbound::devices::PressPayload {
			device: context.device.clone(),
			position: context.position,
		},
	};
	match context.controller.as_str() {
		"Keypad" => {
			crate::events::inbound::devices::key_down(event()).await?;
			tokio::time::sleep(std::time::Duration::from_millis(100)).await;
			crate::events::inbound::devices::key_up(event()).await?;
		}
		"Encoder" => {
			crate::events::inbound::devices::encoder_down(event()).await?;
			tokio::time::sleep(std::time::Duration::from_millis(100)).await;
			crate::events::inbound::devices::encoder_up(event()).await?;
		}
		_ => {}
	}

	Ok(())
}

#[derive(Clone, serde::Serialize)]
struct KeyMovedEvent {
	context: Context,
	pressed: bool,
}

pub async fn key_moved(app: &AppHandle, context: Context, pressed: bool) -> Result<(), anyhow::Error> {
	let window = app.get_webview_window("main").unwrap();
	window.emit("key_moved", KeyMovedEvent { context, pressed })?;
	Ok(())
}
