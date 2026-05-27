use super::Error;

use crate::shared::{ActionInstance, DEVICE_ACTIVE_PAGES, DEVICES};
use crate::store::profiles::{LocksMut, acquire_locks_mut, save_profile};

use tauri::{AppHandle, Emitter, Manager, command};

#[derive(serde::Serialize, Clone)]
struct PageChangedPayload {
	device: String,
	page: u8,
}

pub async fn change_active_page(device: &str, new_page: u8, locks: &mut LocksMut<'_>, app: &AppHandle) -> Result<(), anyhow::Error> {
	let current_page = DEVICE_ACTIVE_PAGES.get(device).map(|p| *p).unwrap_or(0);
	if current_page == new_page {
		return Ok(());
	}

	let device_info = DEVICES.get(device).ok_or_else(|| anyhow::anyhow!("device not found"))?;
	let page_size = (device_info.rows * device_info.columns) as usize;
	let selected_profile = locks.device_stores.get_selected_profile(device)?;

	let (old_instances, new_instances, num_pages) = {
		let profile = &locks.profile_stores.get_profile_store(&device_info, &selected_profile)?.value;
		let num_pages = profile.num_pages;

		let old_start = current_page as usize * page_size;
		let old_end = (old_start + page_size).min(profile.keys.len());
		let old_instances: Vec<ActionInstance> = profile.keys[old_start..old_end].iter().flatten().cloned().collect();

		let new_start = new_page as usize * page_size;
		let new_end = (new_start + page_size).min(profile.keys.len());
		let new_instances: Vec<ActionInstance> = profile.keys[new_start..new_end].iter().flatten().cloned().collect();

		(old_instances, new_instances, num_pages)
	};

	if new_page >= num_pages {
		return Ok(());
	}

	for instance in &old_instances {
		fire_will_disappear(instance).await;
	}

	DEVICE_ACTIVE_PAGES.insert(device.to_owned(), new_page);

	for instance in &new_instances {
		fire_will_appear(instance).await;
	}

	let _ = app
		.get_webview_window("main")
		.unwrap()
		.emit("page_changed", PageChangedPayload { device: device.to_owned(), page: new_page });

	Ok(())
}

pub async fn fire_will_disappear(instance: &ActionInstance) {
	if !matches!(instance.action.uuid.as_str(), "opendeck.multiaction" | "opendeck.toggleaction") {
		let _ = crate::events::outbound::will_appear::will_disappear(instance, true).await;
	} else if let Some(children) = &instance.children {
		for child in children {
			let _ = crate::events::outbound::will_appear::will_disappear(child, true).await;
		}
	}
}

pub async fn fire_will_appear(instance: &ActionInstance) {
	if !matches!(instance.action.uuid.as_str(), "opendeck.multiaction" | "opendeck.toggleaction") {
		let _ = crate::events::outbound::will_appear::will_appear(instance).await;
	} else if let Some(children) = &instance.children {
		for child in children {
			let _ = crate::events::outbound::will_appear::will_appear(child).await;
		}
	}
}

#[command]
pub async fn get_active_page(device: String) -> u8 {
	DEVICE_ACTIVE_PAGES.get(&device).map(|p| *p).unwrap_or(0)
}

#[command]
pub async fn set_active_page(device: String, page: u8) -> Result<(), Error> {
	let app = crate::APP_HANDLE.get().unwrap();
	let mut locks = acquire_locks_mut().await;
	change_active_page(&device, page, &mut locks, app).await?;
	Ok(())
}

#[command]
pub async fn add_page(device: String) -> Result<u8, Error> {
	let mut locks = acquire_locks_mut().await;
	let device_info = DEVICES.get(&device).ok_or_else(|| Error::new(format!("device {device} not found")))?;
	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let num_pages = locks.profile_stores.add_page(&device_info, &selected_profile).await?;
	Ok(num_pages)
}

#[command]
pub async fn remove_last_page(device: String) -> Result<u8, Error> {
	let mut locks = acquire_locks_mut().await;
	let device_info = DEVICES.get(&device).ok_or_else(|| Error::new(format!("device {device} not found")))?;
	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let current_page = DEVICE_ACTIVE_PAGES.get(&device).map(|p| *p).unwrap_or(0);
	let num_pages = locks.profile_stores.remove_last_page(&device_info, &selected_profile).await?;
	if current_page >= num_pages {
		let app = crate::APP_HANDLE.get().unwrap();
		change_active_page(&device, num_pages - 1, &mut locks, app).await?;
	} else {
		save_profile(&device, &mut locks).await?;
	}
	Ok(num_pages)
}
