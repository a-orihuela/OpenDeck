use std::collections::HashMap;
use std::env::var;
use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Deserializer, Serialize, de::Visitor};
use serde_inline_default::serde_inline_default;

use dashmap::DashMap;
use tauri::Manager;
use tokio::sync::RwLock;

pub const PRODUCT_NAME: &str = include_str!("../../product_name.txt").trim_ascii();

pub fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), std::io::Error> {
	use std::fs;
	fs::create_dir_all(&dst)?;
	for entry in fs::read_dir(src)?.flatten() {
		if entry.file_type()?.is_dir() {
			copy_dir(entry.path(), dst.as_ref().join(entry.file_name()))?;
		} else {
			fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
		}
	}
	Ok(())
}

/// Metadata of a device.
#[serde_inline_default]
#[derive(Clone, Deserialize, Serialize, specta::Type)]
pub struct DeviceInfo {
	pub id: String,
	#[serde_inline_default(String::new())]
	pub plugin: String,
	pub name: String,
	pub rows: u8,
	pub columns: u8,
	pub encoders: u8,
	#[serde_inline_default(0)]
	pub touchpoints: u8,
	pub r#type: u8,
}

pub static DEVICES: LazyLock<DashMap<String, DeviceInfo>> = LazyLock::new(DashMap::new);

/// Per-plugin capability grants loaded from each plugin's manifest at startup.
pub static PLUGIN_CAPABILITIES: LazyLock<DashMap<String, Vec<String>>> = LazyLock::new(DashMap::new);

/// Returns `true` if the plugin identified by `uuid` has been granted `capability`.
pub fn has_capability(uuid: &str, capability: &str) -> bool {
	PLUGIN_CAPABILITIES.get(uuid).is_some_and(|caps| caps.iter().any(|c| c == capability))
}

/// Per-plugin crash counts and the start of the current crash window, used by the plugin supervisor.
pub static PLUGIN_CRASH_COUNTS: LazyLock<DashMap<String, (u8, std::time::Instant)>> = LazyLock::new(DashMap::new);

/// Active page index per device (runtime state, not persisted).
pub static DEVICE_ACTIVE_PAGES: LazyLock<DashMap<String, u8>> = LazyLock::new(DashMap::new);

/// Get the application configuration directory.
pub fn config_dir() -> std::path::PathBuf {
	let app_handle = crate::APP_HANDLE.get().unwrap();
	app_handle.path().app_config_dir().unwrap()
}

/// Get the application log directory.
pub fn log_dir() -> std::path::PathBuf {
	let app_handle = crate::APP_HANDLE.get().unwrap();
	app_handle.path().app_log_dir().unwrap()
}

/// Get whether or not the application is running inside the Flatpak sandbox.
pub fn is_flatpak() -> bool {
	var("FLATPAK_ID").is_ok() || var("container").map(|x| x.to_lowercase().trim() == "flatpak").unwrap_or(false)
}

/// Convert an icon specified in a plugin manifest to its full path.
pub fn convert_icon(path: String) -> String {
	if Path::new(&(path.clone() + ".svg")).exists() {
		path + ".svg"
	} else if Path::new(&(path.clone() + "@2x.png")).exists() {
		path + "@2x.png"
	} else {
		path + ".png"
	}
}

#[derive(Clone, Copy, Serialize)]
pub struct FontSize(pub u16);

impl specta::Type for FontSize {
	fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
		<u16 as specta::Type>::definition(types)
	}
}
impl<'de> Deserialize<'de> for FontSize {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct MyVisitor;

		impl Visitor<'_> for MyVisitor {
			type Value = FontSize;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				fmt.write_str("integer or string")
			}

			fn visit_u64<E>(self, val: u64) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Ok(FontSize(val as u16))
			}

			fn visit_str<E>(self, val: &str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				match val.parse::<u64>() {
					Ok(val) => self.visit_u64(val),
					Err(_) => Err(E::custom("failed to parse integer")),
				}
			}
		}

		deserializer.deserialize_any(MyVisitor)
	}
}

/// A state of an action.
#[derive(Clone, Serialize, Deserialize, specta::Type)]
#[serde(default)]
pub struct ActionState {
	#[serde(alias = "Image")]
	pub image: String,
	// Note: this is not a real manifest property; it is only used internally.
	#[serde(alias = "ImageScale")]
	pub image_scale: u8,
	// Note: this is not a real manifest property; it is only used internally.
	#[serde(alias = "BackgroundColour")]
	pub background_colour: String,
	#[serde(alias = "Name")]
	pub name: String,
	#[serde(alias = "Title")]
	pub text: String,
	#[serde(alias = "ShowTitle")]
	pub show: bool,
	#[serde(alias = "TitleColor")]
	pub colour: String,
	// Note: this is not a real manifest property; it is only used internally.
	#[serde(alias = "TitleStroke")]
	pub stroke_colour: String,
	#[serde(alias = "TitleAlignment")]
	pub alignment: String,
	#[serde(alias = "FontFamily")]
	pub family: String,
	#[serde(alias = "FontStyle")]
	pub style: String,
	#[serde(alias = "FontSize")]
	pub size: FontSize,
	// Note: this is not a real manifest property; it is only used internally.
	#[serde(alias = "StrokeSize")]
	pub stroke_size: FontSize,
	#[serde(alias = "FontUnderline")]
	pub underline: bool,
}

impl Default for ActionState {
	fn default() -> Self {
		Self {
			image: "actionDefaultImage".to_owned(),
			image_scale: 100,
			background_colour: "#000000".to_owned(),
			name: String::new(),
			text: String::new(),
			show: true,
			colour: "#FFFFFF".to_owned(),
			stroke_colour: "#000000".to_owned(),
			alignment: "middle".to_owned(),
			family: "Liberation Sans".to_owned(),
			style: "Regular".to_owned(),
			size: FontSize(16),
			stroke_size: FontSize(3),
			underline: false,
		}
	}
}

#[serde_inline_default]
#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct Category {
	pub icon: Option<String>,
	pub actions: Vec<Action>,
}

/// An action, deserialised from the plugin manifest.
#[serde_inline_default]
#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct Action {
	#[serde(alias = "Name")]
	pub name: String,

	#[serde(alias = "UUID")]
	pub uuid: String,

	#[serde_inline_default(String::new())]
	pub plugin: String,

	#[serde_inline_default(String::new())]
	#[serde(alias = "Tooltip")]
	pub tooltip: String,

	#[serde_inline_default(String::new())]
	#[serde(alias = "Icon")]
	pub icon: String,

	#[serde_inline_default(false)]
	#[serde(alias = "DisableAutomaticStates")]
	pub disable_automatic_states: bool,

	#[serde_inline_default(true)]
	#[serde(alias = "VisibleInActionsList")]
	pub visible_in_action_list: bool,

	#[serde_inline_default(true)]
	#[serde(alias = "SupportedInMultiActions")]
	pub supported_in_multi_actions: bool,

	#[serde_inline_default(String::new())]
	#[serde(alias = "PropertyInspectorPath")]
	pub property_inspector: String,

	#[serde_inline_default(vec!["Keypad".to_owned()])]
	#[serde(alias = "Controllers")]
	pub controllers: Vec<String>,

	#[serde(alias = "States")]
	pub states: Vec<ActionState>,
}

/// Location metadata of a slot.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Context {
	pub device: String,
	pub profile: String,
	pub controller: String,
	pub position: u8,
}

/// Information about the slot and index an instance is located in.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct ActionContext {
	pub device: String,
	pub profile: String,
	pub controller: String,
	pub position: u8,
	pub index: u16,
}

impl std::fmt::Display for ActionContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}.{}.{}.{}.{}", self.device, self.profile, self.controller, self.position, self.index)
	}
}

impl std::str::FromStr for ActionContext {
	type Err = anyhow::Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let segments: Vec<&str> = s.split('.').collect();
		if segments.len() < 5 {
			return Err(anyhow::anyhow!("not enough segments"));
		}
		let device = segments[0].to_owned();
		let profile = segments[1].to_owned();
		let controller = segments[2].to_owned();
		let position = u8::from_str(segments[3])?;
		let index = u16::from_str(segments[4])?;
		Ok(Self {
			device,
			profile,
			controller,
			position,
			index,
		})
	}
}

impl specta::Type for ActionContext {
	fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
		<String as specta::Type>::definition(types)
	}
}

impl ActionContext {
	pub fn from_context(context: Context, index: u16) -> Self {
		Self {
			device: context.device,
			profile: context.profile,
			controller: context.controller,
			position: context.position,
			index,
		}
	}
}

impl From<ActionContext> for Context {
	fn from(value: ActionContext) -> Self {
		Self {
			device: value.device,
			profile: value.profile,
			controller: value.controller,
			position: value.position,
		}
	}
}

impl From<&ActionContext> for Context {
	fn from(value: &ActionContext) -> Self {
		Self::from(value.clone())
	}
}

/// An instance of an action.
#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct ActionInstance {
	pub action: Action,
	pub context: ActionContext,
	pub states: Vec<ActionState>,
	pub current_state: u16,
	#[specta(type = specta_typescript::Any)]
	pub settings: serde_json::Value,
	pub children: Option<Vec<ActionInstance>>,
}

#[serde_inline_default]
#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct Profile {
	pub id: String,
	pub keys: Vec<Option<ActionInstance>>,
	pub sliders: Vec<Option<ActionInstance>>,
	#[serde_inline_default(1u8)]
	pub num_pages: u8,
}

/// A map of category names to a list of actions in that category.
pub static CATEGORIES: LazyLock<RwLock<HashMap<String, Category>>> = LazyLock::new(|| {
	let mut hashmap = HashMap::new();
	hashmap.insert(
		PRODUCT_NAME.to_owned(),
		Category {
			icon: None,
			actions: vec![
				serde_json::from_value(serde_json::json!(
					{
						"name": "Multi Action",
						"icon": "opendeck/multi-action.png",
						"plugin": "opendeck",
						"uuid": "opendeck.multiaction",
						"tooltip": "Execute multiple actions",
						"controllers": [ "Keypad" ],
						"states": [ { "image": "opendeck/multi-action.png" } ],
						"supported_in_multi_actions": false
					}
				))
				.unwrap(),
				serde_json::from_value(serde_json::json!(
					{
						"name": "Toggle Action",
						"icon": "opendeck/toggle-action.png",
						"plugin": "opendeck",
						"uuid": "opendeck.toggleaction",
						"tooltip": "Cycle through multiple actions",
						"controllers": [ "Keypad" ],
						"states": [ { "image": "opendeck/toggle-action.png" } ],
						"supported_in_multi_actions": false
					}
				))
				.unwrap(),
			],
		},
	);
	hashmap.insert(
		"Navigation".to_owned(),
		Category {
			icon: None,
			actions: vec![
				serde_json::from_value(serde_json::json!(
					{
						"name": "Next Page",
						"icon": "opendeck/next-page.svg",
						"plugin": "opendeck",
						"uuid": "opendeck.nextpage",
						"tooltip": "Go to the next page",
						"controllers": [ "Keypad" ],
						"states": [ { "image": "opendeck/next-page.svg" } ],
						"supported_in_multi_actions": false
					}
				))
				.unwrap(),
				serde_json::from_value(serde_json::json!(
					{
						"name": "Previous Page",
						"icon": "opendeck/previous-page.svg",
						"plugin": "opendeck",
						"uuid": "opendeck.previouspage",
						"tooltip": "Go to the previous page",
						"controllers": [ "Keypad" ],
						"states": [ { "image": "opendeck/previous-page.svg" } ],
						"supported_in_multi_actions": false
					}
				))
				.unwrap(),
			],
		},
	);
	RwLock::new(hashmap)
});

#[cfg(test)]
mod tests {
	use super::ActionContext;
	use std::str::FromStr;

	fn sample_context() -> ActionContext {
		ActionContext {
			device: "dev1".to_owned(),
			profile: "Default".to_owned(),
			controller: "Keypad".to_owned(),
			position: 3,
			index: 0,
		}
	}

	#[test]
	fn action_context_display_round_trip() {
		let ctx = sample_context();
		let serialised = ctx.to_string();
		let parsed = ActionContext::from_str(&serialised).unwrap();
		assert_eq!(ctx, parsed);
	}

	#[test]
	fn action_context_display_format() {
		let ctx = sample_context();
		assert_eq!(ctx.to_string(), "dev1.Default.Keypad.3.0");
	}

	#[test]
	fn action_context_from_str_too_few_segments() {
		assert!(ActionContext::from_str("dev1.Default.Keypad").is_err());
	}

	#[test]
	fn action_context_from_str_invalid_position() {
		assert!(ActionContext::from_str("dev1.Default.Keypad.notanumber.0").is_err());
	}

	#[test]
	fn action_context_from_str_with_high_index() {
		let ctx = ActionContext::from_str("ABC12345.Gaming.Encoder.7.255").unwrap();
		assert_eq!(ctx.device, "ABC12345");
		assert_eq!(ctx.profile, "Gaming");
		assert_eq!(ctx.controller, "Encoder");
		assert_eq!(ctx.position, 7);
		assert_eq!(ctx.index, 255);
	}
}
