use super::Error;

use crate::constants::FILE_SHEET_TEMPLATES;
use crate::shared::{ActionContext, ActionInstance, Context, DEVICE_ACTIVE_PAGES, DEVICES, config_dir};
use crate::store::Store;
use crate::store::profiles::acquire_locks_mut;

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::command;

#[derive(Clone, Serialize, Deserialize)]
struct SheetTemplate {
	id: String,
	name: String,
	rows: u8,
	columns: u8,
	created_at: u64,
	updated_at: u64,
	slots: Vec<Option<ActionInstance>>,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct SheetTemplateMeta {
	id: String,
	name: String,
	rows: u8,
	columns: u8,
	created_at: u64,
	updated_at: u64,
	preview_map: Vec<bool>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct SheetTemplateStore {
	templates: Vec<SheetTemplate>,
}

impl crate::store::NotProfile for SheetTemplateStore {}

fn now_ts() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_secs())
		.unwrap_or(0)
}

fn load_store() -> Result<Store<SheetTemplateStore>, anyhow::Error> {
	Store::new(FILE_SHEET_TEMPLATES, &config_dir(), SheetTemplateStore::default())
}

fn to_meta(template: &SheetTemplate) -> SheetTemplateMeta {
	SheetTemplateMeta {
		id: template.id.clone(),
		name: template.name.clone(),
		rows: template.rows,
		columns: template.columns,
		created_at: template.created_at,
		updated_at: template.updated_at,
		preview_map: template.slots.iter().map(|slot| slot.is_some()).collect(),
	}
}

fn has_name_conflict(store: &SheetTemplateStore, name: &str, exclude_id: Option<&str>) -> bool {
	store.templates.iter().any(|template| {
		exclude_id.is_none_or(|id| id != template.id)
			&& template.name.eq_ignore_ascii_case(name)
	})
}

fn next_duplicate_name(store: &SheetTemplateStore, base_name: &str) -> String {
	let mut candidate = format!("{} Copy", base_name);
	let mut counter = 2;
	while has_name_conflict(store, &candidate, None) {
		candidate = format!("{} Copy {}", base_name, counter);
		counter += 1;
	}
	candidate
}

fn clone_with_context(mut instance: ActionInstance, context: ActionContext) -> ActionInstance {
	instance.context = context.clone();

	if let Some(children) = &mut instance.children {
		for (index, child) in children.iter_mut().enumerate() {
			let child_context = ActionContext {
				device: context.device.clone(),
				profile: context.profile.clone(),
				controller: context.controller.clone(),
				position: context.position,
				index: (index + 1) as u16,
			};
			*child = clone_with_context(child.clone(), child_context);
		}
	}

	if let Some(folder_slots) = &mut instance.folder_slots {
		for (position, slot) in folder_slots.iter_mut().enumerate() {
			if let Some(child) = slot {
				let child_context = ActionContext {
					device: context.device.clone(),
					profile: context.profile.clone(),
					controller: context.controller.clone(),
					position: position as u8,
					index: 0,
				};
				*slot = Some(clone_with_context(child.clone(), child_context));
			}
		}
	}

	instance
}

#[command]
pub async fn list_sheet_templates() -> Result<Vec<SheetTemplateMeta>, Error> {
	let mut store = load_store()?;
	store.value.templates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
	Ok(store.value.templates.iter().map(to_meta).collect())
}

#[command]
pub async fn save_current_page_as_sheet_template(device: String, name: String) -> Result<SheetTemplateMeta, Error> {
	let trimmed = name.trim();
	if trimmed.is_empty() {
		return Err(Error::new("SHEETS_TEMPLATE_NAME_EMPTY".to_owned()));
	}

	let mut locks = acquire_locks_mut().await;
	let device_info = DEVICES.get(&device).ok_or_else(|| Error::new(format!("device {device} not found")))?;
	let page_size = (device_info.rows * device_info.columns) as usize;
	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let current_page = DEVICE_ACTIVE_PAGES.get(&device).map(|p| *p as usize).unwrap_or(0);

	let slots = {
		let profile_store = locks.profile_stores.get_profile_store_mut(&device_info, &selected_profile).await?;
		let start = current_page * page_size;
		let end = start + page_size;
		if end > profile_store.value.keys.len() {
			return Err(Error::new("PAGES_INDEX_OUT_OF_BOUNDS".to_owned()));
		}
		profile_store.value.keys[start..end].to_vec()
	};

	let timestamp = now_ts();
	let id = format!("sheet_{}_{}", timestamp, trimmed.to_lowercase().replace(' ', "_"));
	let template = SheetTemplate {
		id: id.clone(),
		name: trimmed.to_owned(),
		rows: device_info.rows,
		columns: device_info.columns,
		created_at: timestamp,
		updated_at: timestamp,
		slots,
	};

	let mut store = load_store()?;
	store.value.templates.push(template.clone());
	store.save()?;

	Ok(to_meta(&template))
}

#[command]
pub async fn delete_sheet_template(id: String) -> Result<(), Error> {
	let mut store = load_store()?;
	store.value.templates.retain(|v| v.id != id);
	store.save()?;
	Ok(())
}

#[command]
pub async fn rename_sheet_template(id: String, name: String) -> Result<SheetTemplateMeta, Error> {
	let trimmed = name.trim();
	if trimmed.is_empty() {
		return Err(Error::new("SHEETS_TEMPLATE_NAME_EMPTY".to_owned()));
	}

	let mut store = load_store()?;
	if has_name_conflict(&store.value, trimmed, Some(&id)) {
		return Err(Error::new("SHEETS_TEMPLATE_NAME_EXISTS".to_owned()));
	}

	let template = store
		.value
		.templates
		.iter_mut()
		.find(|template| template.id == id)
		.ok_or_else(|| Error::new("SHEETS_TEMPLATE_NOT_FOUND".to_owned()))?;
	template.name = trimmed.to_owned();
	template.updated_at = now_ts();

	let meta = to_meta(template);
	store.save()?;
	Ok(meta)
}

#[command]
pub async fn duplicate_sheet_template(id: String) -> Result<SheetTemplateMeta, Error> {
	let mut store = load_store()?;
	let source = store
		.value
		.templates
		.iter()
		.find(|template| template.id == id)
		.cloned()
		.ok_or_else(|| Error::new("SHEETS_TEMPLATE_NOT_FOUND".to_owned()))?;

	let timestamp = now_ts();
	let new_name = next_duplicate_name(&store.value, &source.name);
	let nonce = store.value.templates.len();
	let duplicate = SheetTemplate {
		id: format!("sheet_{}_dup_{}", timestamp, nonce),
		name: new_name,
		rows: source.rows,
		columns: source.columns,
		created_at: timestamp,
		updated_at: timestamp,
		slots: source.slots,
	};

	let meta = to_meta(&duplicate);
	store.value.templates.push(duplicate);
	store.save()?;
	Ok(meta)
}

#[command]
pub async fn apply_sheet_template(device: String, template_id: String, page: Option<u8>) -> Result<(), Error> {
	let store = load_store()?;
	let template = store
		.value
		.templates
		.iter()
		.find(|v| v.id == template_id)
		.cloned()
		.ok_or_else(|| Error::new("SHEETS_TEMPLATE_NOT_FOUND".to_owned()))?;

	let mut locks = acquire_locks_mut().await;
	let device_info = DEVICES.get(&device).ok_or_else(|| Error::new(format!("device {device} not found")))?;
	if template.rows != device_info.rows || template.columns != device_info.columns {
		return Err(Error::new("SHEETS_TEMPLATE_LAYOUT_INCOMPATIBLE".to_owned()));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let profile_store = locks.profile_stores.get_profile_store_mut(&device_info, &selected_profile).await?;
	let page_size = (device_info.rows * device_info.columns) as usize;
	let target_page = page.unwrap_or_else(|| DEVICE_ACTIVE_PAGES.get(&device).map(|p| *p).unwrap_or(0));
	if target_page >= profile_store.value.num_pages {
		return Err(Error::new("PAGES_INDEX_OUT_OF_BOUNDS".to_owned()));
	}

	let start = target_page as usize * page_size;
	for index in 0..page_size {
		let context = Context {
			device: device.clone(),
			profile: selected_profile.clone(),
			controller: "Keypad".to_owned(),
			position: (start + index) as u8,
		};
		profile_store.value.keys[start + index] = template
			.slots
			.get(index)
			.cloned()
			.flatten()
			.map(|instance| clone_with_context(instance, ActionContext::from_context(context, 0)));
	}

	profile_store.save()?;
	Ok(())
}

#[command]
pub async fn insert_sheet_template_as_new_page(device: String, template_id: String) -> Result<u8, Error> {
	let store = load_store()?;
	let template = store
		.value
		.templates
		.iter()
		.find(|v| v.id == template_id)
		.cloned()
		.ok_or_else(|| Error::new("SHEETS_TEMPLATE_NOT_FOUND".to_owned()))?;

	let mut locks = acquire_locks_mut().await;
	let device_info = DEVICES.get(&device).ok_or_else(|| Error::new(format!("device {device} not found")))?;
	if template.rows != device_info.rows || template.columns != device_info.columns {
		return Err(Error::new("SHEETS_TEMPLATE_LAYOUT_INCOMPATIBLE".to_owned()));
	}

	let selected_profile = locks.device_stores.get_selected_profile(&device)?;
	let num_pages = locks.profile_stores.add_page(&device_info, &selected_profile).await?;
	let profile_store = locks.profile_stores.get_profile_store_mut(&device_info, &selected_profile).await?;
	let page_size = (device_info.rows * device_info.columns) as usize;
	let target_page = num_pages.saturating_sub(1);
	let start = target_page as usize * page_size;

	for index in 0..page_size {
		let context = Context {
			device: device.clone(),
			profile: selected_profile.clone(),
			controller: "Keypad".to_owned(),
			position: (start + index) as u8,
		};
		profile_store.value.keys[start + index] = template
			.slots
			.get(index)
			.cloned()
			.flatten()
			.map(|instance| clone_with_context(instance, ActionContext::from_context(context, 0)));
	}

	profile_store.save()?;
	Ok(num_pages)
}
