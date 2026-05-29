use super::{GenericInstancePayload, send_to_plugin};

use crate::constants::{ACTION_FOLDER, ACTION_MULTIACTION, ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE, ACTION_TOGGLEACTION};
use crate::events::frontend::instances::{key_moved, update_state};
use crate::events::frontend::pages::change_active_page;
use crate::shared::{ActionContext, Context, DEVICE_ACTIVE_FOLDER, DEVICE_ACTIVE_PAGES, DEVICES};
use tauri::{Emitter, Manager};
use crate::store::profiles::{acquire_locks_mut, get_slot_mut, save_profile};

use std::sync::LazyLock;
use std::time::Duration;

use dashmap::DashMap;
use serde::Serialize;

static KEY_DOWN_TARGETS: LazyLock<DashMap<(String, u8), Context>> = LazyLock::new(DashMap::new);

#[derive(Serialize)]
struct KeyEvent {
	event: &'static str,
	action: String,
	context: ActionContext,
	device: String,
	payload: GenericInstancePayload,
}

/// Map a raw hardware key index to a profile position accounting for the active page.
fn page_position(device: &str, key: u8) -> u8 {
	let page = DEVICE_ACTIVE_PAGES.get(device).map(|p| *p).unwrap_or(0);
	if page == 0 {
		return key;
	}
	let Some(info) = DEVICES.get(device) else { return key };
	let page_size = info.rows * info.columns;
	(page * page_size).saturating_add(key)
}

pub async fn key_down(device: &str, key: u8) -> Result<(), anyhow::Error> {
	// Folder mode: physical keys map directly to folder slots.
	if let Some(folder_ctx) = DEVICE_ACTIVE_FOLDER.get(device).map(|r| r.clone()) {
		return folder_key_down(device, key, folder_ctx).await;
	}

	let mut locks = acquire_locks_mut().await;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = Context {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Keypad".to_owned(),
		position: page_position(device, key),
	};

	let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone(), true).await;
	KEY_DOWN_TARGETS.insert((device.to_owned(), key), context.clone());

	// Extract the action UUID without holding a borrow on locks, so page-navigation
	// actions can call change_active_page (which also needs &mut locks) freely.
	let action_uuid = {
		get_slot_mut(&context, &mut locks)
			.await?
			.as_ref()
			.map(|inst| inst.action.uuid.clone())
	};
	let Some(action_uuid) = action_uuid else { return Ok(()) };

	if action_uuid == ACTION_FOLDER {
		// Drop write locks before calling enter_folder_internal — it acquires its own locks
		// internally, and holding them here would cause a write-lock re-entrance deadlock.
		drop(locks);
		let folder_context = ActionContext::from_context(context, 0);
		let app = crate::APP_HANDLE.get().unwrap();
		crate::events::frontend::folders::enter_folder_internal(device, folder_context, app).await?;
		// Remove the key_down target so the matching key_up doesn't fire a folder-slot action.
		KEY_DOWN_TARGETS.remove(&(device.to_owned(), key));
		return Ok(());
	}

	if action_uuid == ACTION_NEXTPAGE || action_uuid == ACTION_PREVIOUSPAGE {
		let num_pages = {
			let device_info = DEVICES.get(device).ok_or_else(|| anyhow::anyhow!("device not found"))?;
			locks.profile_stores.get_profile_store(&device_info, &context.profile)?.value.num_pages
		};
		let current_page = DEVICE_ACTIVE_PAGES.get(device).map(|p| *p).unwrap_or(0);
		let new_page = if action_uuid == ACTION_NEXTPAGE {
			(current_page + 1) % num_pages
		} else {
			(current_page + num_pages - 1) % num_pages
		};
		let app = crate::APP_HANDLE.get().unwrap();
		change_active_page(device, new_page, &mut locks, app).await?;
		return Ok(());
	}

	let Some(instance) = get_slot_mut(&context, &mut locks).await? else { return Ok(()) };
	if instance.action.uuid == ACTION_MULTIACTION {
		for child in instance.children.as_mut().unwrap() {
			send_to_plugin(
				&child.action.plugin,
				&KeyEvent {
					event: "keyDown",
					action: child.action.uuid.clone(),
					context: child.context.clone(),
					device: child.context.device.clone(),
					payload: GenericInstancePayload::new(child),
				},
			)
			.await?;

			tokio::time::sleep(Duration::from_millis(100)).await;

			if child.states.len() == 2 && !child.action.disable_automatic_states {
				child.current_state = (child.current_state + 1) % (child.states.len() as u16);
			}

			send_to_plugin(
				&child.action.plugin,
				&KeyEvent {
					event: "keyUp",
					action: child.action.uuid.clone(),
					context: child.context.clone(),
					device: child.context.device.clone(),
					payload: GenericInstancePayload::new(child),
				},
			)
			.await?;

			tokio::time::sleep(Duration::from_millis(100)).await;
		}

		let contexts = instance.children.as_ref().unwrap().iter().map(|x| x.context.clone()).collect::<Vec<_>>();
		for child in contexts {
			let _ = update_state(crate::APP_HANDLE.get().unwrap(), child, &mut locks).await;
		}

		save_profile(device, &mut locks).await?;
	} else if instance.action.uuid == ACTION_TOGGLEACTION {
		let children = instance.children.as_ref().unwrap();
		if children.is_empty() {
			return Ok(());
		}
		let child = &children[instance.current_state as usize];
		send_to_plugin(
			&child.action.plugin,
			&KeyEvent {
				event: "keyDown",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: GenericInstancePayload::new(child),
			},
		)
		.await?;
	} else if instance.action.uuid.starts_with("omegadeck.builtin.") {
		let new_state = crate::builtin_actions::handle(instance, crate::builtin_actions::ActionEvent::KeyDown).await?;
		if let Some(state) = new_state {
			instance.current_state = state;
			let ctx = instance.context.clone();
			update_state(crate::APP_HANDLE.get().unwrap(), ctx, &mut locks).await?;
		}
		save_profile(device, &mut locks).await?;
	} else {
		send_to_plugin(
			&instance.action.plugin,
			&KeyEvent {
				event: "keyDown",
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: GenericInstancePayload::new(instance),
			},
		)
		.await?;
	}

	Ok(())
}

async fn folder_key_down(device: &str, key: u8, folder_ctx: ActionContext) -> Result<(), anyhow::Error> {
	let page_size = DEVICES.get(device).map(|d| (d.rows * d.columns) as usize).unwrap_or(0);
	let close_pos = if page_size > 0 { (folder_ctx.position as usize % page_size) as u8 } else { folder_ctx.position };

	if key == close_pos {
		let app = crate::APP_HANDLE.get().unwrap();
		crate::events::frontend::folders::exit_folder_internal(device, app).await?;
		return Ok(());
	}

	let child = {
		let mut locks = acquire_locks_mut().await;
		let selected_profile = locks.device_stores.get_selected_profile(device)?;
		let virtual_context = Context {
			device: device.to_owned(),
			profile: selected_profile.to_owned(),
			controller: "Keypad".to_owned(),
			position: key,
		};

		let _ = key_moved(crate::APP_HANDLE.get().unwrap(), virtual_context.clone(), true).await;
		KEY_DOWN_TARGETS.insert((device.to_owned(), key), virtual_context);

		let folder_slot_ctx: Context = (&folder_ctx).into();
		let slot = get_slot_mut(&folder_slot_ctx, &mut locks).await?;
		slot.as_ref().and_then(|inst| inst.folder_slots.as_ref())
			.and_then(|slots| slots.get(key as usize))
			.and_then(|s| s.as_ref())
			.cloned()
	};

	if let Some(child) = child {
		send_to_plugin(
			&child.action.plugin,
			&KeyEvent {
				event: "keyDown",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: GenericInstancePayload::new(&child),
			},
		).await?;
	}

	Ok(())
}

pub async fn key_up(device: &str, key: u8) -> Result<(), anyhow::Error> {
	// Folder mode: physical keys map directly to folder slots.
	if let Some(folder_ctx) = DEVICE_ACTIVE_FOLDER.get(device).map(|r| r.clone()) {
		return folder_key_up(device, key, folder_ctx).await;
	}

	let mut locks = acquire_locks_mut().await;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;
	let context = Context {
		device: device.to_owned(),
		profile: selected_profile.to_owned(),
		controller: "Keypad".to_owned(),
		position: page_position(device, key),
	};

	let _ = key_moved(crate::APP_HANDLE.get().unwrap(), context.clone(), false).await;
	let Some((_, expected_context)) = KEY_DOWN_TARGETS.remove(&(device.to_owned(), key)) else {
		return Ok(());
	};
	if context != expected_context {
		return Ok(());
	}

	let slot = get_slot_mut(&context, &mut locks).await?;
	let Some(instance) = slot else { return Ok(()) };

	if instance.action.uuid == ACTION_TOGGLEACTION {
		let index = instance.current_state as usize;
		let children = instance.children.as_ref().unwrap();
		if children.is_empty() {
			return Ok(());
		}
		let child = &children[index];
		send_to_plugin(
			&child.action.plugin,
			&KeyEvent {
				event: "keyUp",
				action: child.action.uuid.clone(),
				context: child.context.clone(),
				device: child.context.device.clone(),
				payload: GenericInstancePayload::new(child),
			},
		)
		.await?;
		instance.current_state = ((index + 1) % instance.children.as_ref().unwrap().len()) as u16;
	} else if instance.action.uuid.starts_with("omegadeck.builtin.") {
		let new_state = crate::builtin_actions::handle(instance, crate::builtin_actions::ActionEvent::KeyUp).await?;
		if let Some(state) = new_state {
			instance.current_state = state;
		}
	} else if instance.action.uuid != ACTION_MULTIACTION {
		if instance.states.len() == 2 && !instance.action.disable_automatic_states {
			instance.current_state = (instance.current_state + 1) % (instance.states.len() as u16);
		}
		send_to_plugin(
			&instance.action.plugin,
			&KeyEvent {
				event: "keyUp",
				action: instance.action.uuid.clone(),
				context: instance.context.clone(),
				device: instance.context.device.clone(),
				payload: GenericInstancePayload::new(instance),
			},
		)
		.await?;
	};

	let _ = update_state(crate::APP_HANDLE.get().unwrap(), instance.context.clone(), &mut locks).await;
	save_profile(device, &mut locks).await?;

	Ok(())
}

async fn folder_key_up(device: &str, key: u8, folder_ctx: ActionContext) -> Result<(), anyhow::Error> {
	let folder_slot_ctx: Context = (&folder_ctx).into();

	// All lock-holding work is scoped here; locks are released before send_to_plugin.
	let child = {
		let mut locks = acquire_locks_mut().await;
		let selected_profile = locks.device_stores.get_selected_profile(device)?;
		let virtual_context = Context {
			device: device.to_owned(),
			profile: selected_profile.to_owned(),
			controller: "Keypad".to_owned(),
			position: key,
		};

		let _ = key_moved(crate::APP_HANDLE.get().unwrap(), virtual_context.clone(), false).await;
		let Some((_, expected)) = KEY_DOWN_TARGETS.remove(&(device.to_owned(), key)) else {
			return Ok(());
		};
		if virtual_context != expected {
			return Ok(());
		}

		let child = {
			let slot = get_slot_mut(&folder_slot_ctx, &mut locks).await?;
			slot.as_ref().and_then(|inst| inst.folder_slots.as_ref())
				.and_then(|slots| slots.get(key as usize))
				.and_then(|s| s.as_ref())
				.cloned()
		};

		let Some(mut child) = child else { return Ok(()) };

		// Advance automatic two-state toggle.
		if child.states.len() == 2 && !child.action.disable_automatic_states {
			let new_state = (child.current_state + 1) % (child.states.len() as u16);
			child.current_state = new_state;
			let slot = get_slot_mut(&folder_slot_ctx, &mut locks).await?;
			if let Some(inst) = slot {
				if let Some(slots) = inst.folder_slots.as_mut() {
					if let Some(Some(child_ref)) = slots.get_mut(key as usize) {
						child_ref.current_state = new_state;
					}
				}
			}
		}

		save_profile(device, &mut locks).await?;
		child
	}; // locks released here — before any plugin communication

	send_to_plugin(
		&child.action.plugin,
		&KeyEvent {
			event: "keyUp",
			action: child.action.uuid.clone(),
			context: child.context.clone(),
			device: child.context.device.clone(),
			payload: GenericInstancePayload::new(&child),
		},
	).await?;

	let app = crate::APP_HANDLE.get().unwrap();
	let window = app.get_webview_window("main").unwrap();
	let _ = window.emit("update_state", serde_json::json!({
		"context": child.context,
		"contents": &child,
	}));

	Ok(())
}
