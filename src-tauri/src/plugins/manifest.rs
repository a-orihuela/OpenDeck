use std::collections::HashMap;

use crate::shared::Action;

use serde::Deserialize;
use serde_inline_default::serde_inline_default;

#[derive(Deserialize)]
pub struct OS {
	#[serde(alias = "Platform")]
	pub platform: String,
}

#[allow(dead_code)]
#[serde_inline_default]
#[derive(Deserialize)]
pub struct PluginManifest {
	#[serde(alias = "Name")]
	pub name: String,

	#[serde(alias = "Author")]
	pub author: String,

	#[serde(alias = "Version")]
	pub version: String,

	#[serde(alias = "Icon")]
	pub icon: String,

	#[serde_inline_default("Custom".to_owned())]
	#[serde(alias = "Category")]
	pub category: String,

	#[serde(alias = "CategoryIcon")]
	pub category_icon: Option<String>,

	#[serde(alias = "Actions")]
	pub actions: Vec<Action>,

	#[serde(alias = "OS")]
	pub os: Vec<OS>,

	#[serde(alias = "CodePath")]
	pub code_path: Option<String>,

	#[serde(alias = "CodePaths")]
	pub code_paths: Option<HashMap<String, String>>,

	#[serde(alias = "CodePathWin")]
	pub code_path_windows: Option<String>,

	#[serde(alias = "CodePathMac")]
	pub code_path_macos: Option<String>,

	#[serde(alias = "CodePathLin")]
	pub code_path_linux: Option<String>,

	#[serde(alias = "PropertyInspectorPath")]
	pub property_inspector_path: Option<String>,

	#[serde(alias = "DeviceNamespace")]
	pub device_namespace: Option<String>,

	#[serde(alias = "ApplicationsToMonitor")]
	pub applications_to_monitor: Option<HashMap<String, Vec<String>>>,

	#[serde(alias = "HasSettingsInterface")]
	pub has_settings_interface: Option<bool>,

	#[serde(alias = "Capabilities")]
	pub capabilities: Option<Vec<String>>,

	/// Per-action category overrides, keyed by action UUID. Populated from the raw
	/// JSON in `read_manifest` because `serde_inline_default` on `Action` prevents
	/// the alias from being applied during nested deserialization.
	#[serde(skip)]
	pub action_categories: HashMap<String, String>,
}

pub fn read_manifest(base_path: &std::path::Path) -> Result<PluginManifest, anyhow::Error> {
	use anyhow::Context;

	let mut manifest: serde_json::Value = serde_json::from_str(
		std::fs::read_to_string(base_path.join("manifest.json"))
			.context("failed to read manifest")?
			.trim_start_matches("\u{feff}"),
	)
	.context("failed to parse manifest")?;

	let platform_overrides_path = base_path.join(format!("manifest.{}.json", std::env::consts::OS));
	if platform_overrides_path.exists()
		&& let Ok(Ok(platform_overrides)) = std::fs::read(platform_overrides_path).map(|v| serde_json::from_slice(&v))
	{
		json_patch::merge(&mut manifest, &platform_overrides);
	}

	// Extract per-action categories from the raw JSON before `from_value` consumes it.
	// We can't rely on a `category` field on `shared::Action` because the `serde_inline_default`
	// macro's generated visitor doesn't carry through `#[serde(alias)]` on non-default fields.
	let action_categories: HashMap<String, String> = manifest["Actions"]
		.as_array()
		.map(|actions| {
			actions.iter().filter_map(|a| {
				let uuid = a.get("UUID").or_else(|| a.get("uuid"))
					.and_then(|v| v.as_str())?
					.to_owned();
				let category = a.get("Category").or_else(|| a.get("category"))
					.and_then(|v| v.as_str())?
					.to_owned();
				Some((uuid, category))
			})
			.collect()
		})
		.unwrap_or_default();

	let mut plugin_manifest: PluginManifest = serde_json::from_value(manifest).context("failed to parse manifest")?;
	plugin_manifest.action_categories = action_categories;
	Ok(plugin_manifest)
}

#[cfg(test)]
mod tests {
	use super::read_manifest;

	fn write_manifest(dir: &std::path::Path, json: &str) {
		std::fs::write(dir.join("manifest.json"), json).unwrap();
	}

	#[test]
	fn valid_manifest_parses_correctly() {
		let dir = tempfile::tempdir().unwrap();
		write_manifest(
			dir.path(),
			r#"{
				"Name": "Test Plugin",
				"Author": "Test Author",
				"Version": "1.0.0",
				"Icon": "icons/plugin",
				"OS": [{ "Platform": "linux" }],
				"Actions": [{
					"Name": "Test Action",
					"UUID": "com.test.action",
					"States": [{ "Image": "actionDefaultImage" }],
					"Controllers": ["Keypad"]
				}]
			}"#,
		);
		let manifest = read_manifest(dir.path()).unwrap();
		assert_eq!(manifest.name, "Test Plugin");
		assert_eq!(manifest.author, "Test Author");
		assert_eq!(manifest.version, "1.0.0");
		assert_eq!(manifest.actions.len(), 1);
		assert_eq!(manifest.actions[0].uuid, "com.test.action");
	}

	#[test]
	fn missing_manifest_file_returns_error() {
		let dir = tempfile::tempdir().unwrap();
		assert!(read_manifest(dir.path()).is_err());
	}

	#[test]
	fn invalid_json_returns_error() {
		let dir = tempfile::tempdir().unwrap();
		write_manifest(dir.path(), "this is not json");
		assert!(read_manifest(dir.path()).is_err());
	}

	#[test]
	fn missing_required_name_field_returns_error() {
		let dir = tempfile::tempdir().unwrap();
		write_manifest(
			dir.path(),
			r#"{ "Author": "A", "Version": "1.0.0", "Icon": "i", "OS": [], "Actions": [] }"#,
		);
		assert!(read_manifest(dir.path()).is_err());
	}
}
