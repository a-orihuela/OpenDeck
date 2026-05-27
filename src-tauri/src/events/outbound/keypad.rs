use super::{GenericInstancePayload, send_to_plugin};

use crate::events::frontend::instances::{key_moved, update_state};
use crate::events::frontend::pages::change_active_page;
use crate::shared::{ActionContext, Context, DEVICE_ACTIVE_PAGES, DEVICES};
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

	if action_uuid == "opendeck.nextpage" || action_uuid == "opendeck.previouspage" {
		let num_pages = {
			let device_info = DEVICES.get(device).ok_or_else(|| anyhow::anyhow!("device not found"))?;
			locks.profile_stores.get_profile_store(&device_info, &context.profile)?.value.num_pages
		};
		let current_page = DEVICE_ACTIVE_PAGES.get(device).map(|p| *p).unwrap_or(0);
		let new_page = if action_uuid == "opendeck.nextpage" {
			(current_page + 1) % num_pages
		} else {
			(current_page + num_pages - 1) % num_pages
		};
		let app = crate::APP_HANDLE.get().unwrap();
		change_active_page(device, new_page, &mut locks, app).await?;
		return Ok(());
	}

	let Some(instance) = get_slot_mut(&context, &mut locks).await? else { return Ok(()) };
	if instance.action.uuid == "opendeck.multiaction" {
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
	} else if instance.action.uuid == "opendeck.toggleaction" {
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

pub async fn key_up(device: &str, key: u8) -> Result<(), anyhow::Error> {
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

	if instance.action.uuid == "opendeck.toggleaction" {
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
	} else if instance.action.uuid != "opendeck.multiaction" {
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
