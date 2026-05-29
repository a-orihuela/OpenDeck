use super::Error;

use crate::constants::{ACTION_FOLDER, ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE};
use crate::events::frontend::pages::{fire_will_appear, fire_will_disappear};
use crate::shared::{Action, ActionContext, ActionInstance, DEVICE_ACTIVE_FOLDER, DEVICE_ACTIVE_PAGES, DEVICES};
use crate::store::profiles::{acquire_locks_mut, get_slot_mut, save_profile};

use tauri::{AppHandle, Emitter, Manager, command};

#[derive(Clone, serde::Serialize)]
struct FolderOpenedEvent {
	device: String,
	folder_context: ActionContext,
}

pub async fn enter_folder_internal(device: &str, folder_context: ActionContext, app: &AppHandle) -> Result<(), anyhow::Error> {
	let device_info = DEVICES.get(device).ok_or_else(|| anyhow::anyhow!("device not found"))?;
	let page_size = (device_info.rows * device_info.columns) as usize;

	let (page_instances, folder_children) = {
		let mut locks = acquire_locks_mut().await;
		let selected_profile = locks.device_stores.get_selected_profile(device)?.to_owned();
		let current_page = DEVICE_ACTIVE_PAGES.get(device).map(|p| *p as usize).unwrap_or(0);

		let profile = &locks.profile_stores.get_profile_store(&device_info, &selected_profile)?.value;
		let start = current_page * page_size;
		let end = (start + page_size).min(profile.keys.len());
		let page: Vec<ActionInstance> = profile.keys[start..end].iter().flatten().cloned().collect();

		let folder_flat = folder_context.position as usize;
		let children = profile.keys
			.get(folder_flat)
			.and_then(|s| s.as_ref())
			.and_then(|inst| inst.folder_slots.as_ref())
			.map(|slots| slots.iter().flatten().cloned().collect::<Vec<_>>())
			.unwrap_or_default();

		(page, children)
	};

	for instance in &page_instances {
		fire_will_disappear(instance).await;
	}

	// Set active folder BEFORE will_appear so that update_image routes correctly.
	DEVICE_ACTIVE_FOLDER.insert(device.to_owned(), folder_context.clone());

	for child in &folder_children {
		let _ = crate::events::outbound::will_appear::will_appear(child).await;
	}

	let _ = app.get_webview_window("main").unwrap().emit("folder_opened", FolderOpenedEvent {
		device: device.to_owned(),
		folder_context,
	});

	Ok(())
}

pub async fn exit_folder_internal(device: &str, app: &AppHandle) -> Result<(), anyhow::Error> {
	let Some((_, folder_context)) = DEVICE_ACTIVE_FOLDER.remove(device) else {
		return Ok(());
	};

	let device_info = DEVICES.get(device).ok_or_else(|| anyhow::anyhow!("device not found"))?;
	let page_size = (device_info.rows * device_info.columns) as usize;

	let (folder_children, page_instances) = {
		let mut locks = acquire_locks_mut().await;
		let selected_profile = locks.device_stores.get_selected_profile(device)?.to_owned();
		let current_page = DEVICE_ACTIVE_PAGES.get(device).map(|p| *p as usize).unwrap_or(0);
		let profile = &locks.profile_stores.get_profile_store(&device_info, &selected_profile)?.value;

		let folder_flat = folder_context.position as usize;
		let children = profile.keys
			.get(folder_flat)
			.and_then(|s| s.as_ref())
			.and_then(|inst| inst.folder_slots.as_ref())
			.map(|slots| slots.iter().flatten().cloned().collect::<Vec<_>>())
			.unwrap_or_default();

		let start = current_page * page_size;
		let end = (start + page_size).min(profile.keys.len());
		let page: Vec<ActionInstance> = profile.keys[start..end].iter().flatten().cloned().collect();

		(children, page)
	};

	for child in &folder_children {
		let _ = crate::events::outbound::will_appear::will_disappear(child, true).await;
	}

	for instance in &page_instances {
		fire_will_appear(instance).await;
	}

	let _ = app.get_webview_window("main").unwrap().emit("folder_closed", serde_json::json!({ "device": device }));

	Ok(())
}

#[command]
pub async fn enter_folder(device: String, folder_context: ActionContext) -> Result<(), Error> {
	let app = crate::APP_HANDLE.get().unwrap();
	enter_folder_internal(&device, folder_context, app).await?;
	Ok(())
}

#[command]
pub async fn exit_folder(device: String) -> Result<(), Error> {
	let app = crate::APP_HANDLE.get().unwrap();
	exit_folder_internal(&device, app).await?;
	Ok(())
}

fn is_forbidden_in_folder(uuid: &str) -> bool {
	matches!(uuid, ACTION_NEXTPAGE | ACTION_PREVIOUSPAGE | ACTION_FOLDER)
}

pub async fn create_folder_child_impl(
	device: &str,
	folder_context: ActionContext,
	slot_index: usize,
	action: Action,
) -> Result<Option<ActionInstance>, Error> {
	if is_forbidden_in_folder(&action.uuid) || !action.controllers.contains(&"Keypad".to_owned()) {
		return Ok(None);
	}

	let child_context = ActionContext {
		device: folder_context.device.clone(),
		profile: folder_context.profile.clone(),
		controller: folder_context.controller.clone(),
		position: slot_index as u8,
		index: 0,
	};

	let instance = ActionInstance {
		action: action.clone(),
		context: child_context,
		states: action.states.clone(),
		current_state: 0,
		settings: serde_json::Value::Object(serde_json::Map::new()),
		children: None,
		folder_slots: None,
	};

	{
		let mut locks = acquire_locks_mut().await;
		let ctx: crate::shared::Context = (&folder_context).into();
		let folder_slot = get_slot_mut(&ctx, &mut locks).await?;
		let Some(folder_inst) = folder_slot else { return Ok(None) };

		let folder_slots = match &mut folder_inst.folder_slots {
			Some(slots) => slots,
			None => return Ok(None),
		};

		if slot_index >= folder_slots.len() { return Ok(None) }
		if folder_slots[slot_index].is_some() { return Ok(None) }

		folder_slots[slot_index] = Some(instance.clone());
		save_profile(device, &mut locks).await?;
	}

	let _ = crate::events::outbound::will_appear::will_appear(&instance).await;
	Ok(Some(instance))
}

pub async fn remove_folder_child_impl(
	device: &str,
	folder_context: ActionContext,
	slot_index: usize,
) -> Result<(), Error> {
	let child = {
		let mut locks = acquire_locks_mut().await;
		let ctx: crate::shared::Context = (&folder_context).into();
		let folder_slot = get_slot_mut(&ctx, &mut locks).await?;
		let Some(folder_inst) = folder_slot else { return Ok(()) };

		let folder_slots = match &mut folder_inst.folder_slots {
			Some(slots) => slots,
			None => return Ok(()),
		};

		if slot_index >= folder_slots.len() { return Ok(()) }

		let child = folder_slots[slot_index].take();
		save_profile(device, &mut locks).await?;
		child
	};

	if let Some(child) = child {
		let _ = crate::events::outbound::will_appear::will_disappear(&child, true).await;
	}

	Ok(())
}
