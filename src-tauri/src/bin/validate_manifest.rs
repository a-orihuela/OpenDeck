/// validate-manifest: pre-flight checker for OpenDeck plugin manifests.
///
/// Usage:
///   cargo run --bin validate-manifest -- path/to/plugin.sdPlugin
///   cargo run --bin validate-manifest -- path/to/manifest.json
///
/// Parses manifest.json using the same logic as the main application, then
/// runs semantic checks and prints a pass/fail checklist with a final verdict.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_inline_default::serde_inline_default;

// Minimal local copies of the manifest types to keep this binary self-contained
// and free of Tauri/AppHandle dependencies.

#[derive(Deserialize)]
struct OS {
	#[serde(alias = "Platform")]
	platform: String,
}

#[derive(Deserialize, Default)]
#[allow(dead_code)]
struct ActionState {
	#[serde(alias = "Image")]
	image: Option<String>,
}

#[derive(Deserialize)]
struct Action {
	#[serde(alias = "Name")]
	name: Option<String>,
	#[serde(alias = "UUID")]
	uuid: Option<String>,
	#[serde(alias = "States")]
	states: Option<Vec<ActionState>>,
}

#[allow(dead_code)]
#[serde_inline_default]
#[derive(Deserialize)]
struct PluginManifest {
	#[serde(alias = "Name")]
	name: Option<String>,
	#[serde(alias = "Author")]
	author: Option<String>,
	#[serde(alias = "Version")]
	version: Option<String>,
	#[serde(alias = "Icon")]
	icon: Option<String>,
	#[serde(alias = "Actions")]
	actions: Option<Vec<Action>>,
	#[serde(alias = "OS")]
	os: Option<Vec<OS>>,
	#[serde(alias = "CodePath")]
	code_path: Option<String>,
	#[serde(alias = "CodePaths")]
	code_paths: Option<HashMap<String, String>>,
	#[serde(alias = "CodePathWin")]
	code_path_windows: Option<String>,
	#[serde(alias = "CodePathMac")]
	code_path_macos: Option<String>,
	#[serde(alias = "CodePathLin")]
	code_path_linux: Option<String>,
}

fn check(label: &str, pass: bool, detail: &str) -> bool {
	let icon = if pass { "✓" } else { "✗" };
	if pass {
		println!("  {icon} {label}");
	} else {
		println!("  {icon} {label}: {detail}");
	}
	pass
}

fn resolve_manifest_dir(arg: &str) -> PathBuf {
	let p = Path::new(arg);
	if p.is_file() && p.file_name().map(|f| f == "manifest.json").unwrap_or(false) {
		p.parent().unwrap_or(Path::new(".")).to_path_buf()
	} else {
		p.to_path_buf()
	}
}

fn read_manifest(base_path: &Path) -> Result<PluginManifest, String> {
	let manifest_path = base_path.join("manifest.json");
	let content = std::fs::read_to_string(&manifest_path).map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
	let mut value: serde_json::Value = serde_json::from_str(content.trim_start_matches('\u{feff}')).map_err(|e| format!("JSON parse error: {e}"))?;

	let platform_overrides_path = base_path.join(format!("manifest.{}.json", std::env::consts::OS));
	if platform_overrides_path.exists() {
		if let Ok(Ok(overrides)) = std::fs::read(&platform_overrides_path).map(|v| serde_json::from_slice::<serde_json::Value>(&v)) {
			json_patch::merge(&mut value, &overrides);
		}
	}

	serde_json::from_value(value).map_err(|e| format!("manifest deserialization error: {e}"))
}

fn main() {
	let arg = std::env::args().nth(1).unwrap_or_else(|| {
		eprintln!("Usage: validate-manifest <plugin-dir|manifest.json>");
		std::process::exit(2);
	});

	let base = resolve_manifest_dir(&arg);
	println!("Validating: {}", base.display());
	println!();

	let manifest = match read_manifest(&base) {
		Ok(m) => m,
		Err(e) => {
			println!("  ✗ manifest.json is readable and valid JSON: {e}");
			println!();
			println!("VERDICT: FAIL (manifest could not be parsed)");
			std::process::exit(1);
		}
	};

	println!("  ✓ manifest.json is readable and valid JSON");
	println!();

	let mut passed = 0usize;
	let mut failed = 0usize;

	macro_rules! c {
		($label:expr, $pass:expr, $detail:expr) => {{
			let ok = check($label, $pass, $detail);
			if ok { passed += 1; } else { failed += 1; }
			ok
		}};
	}

	// Required fields
	println!("Required fields:");
	let name = manifest.name.as_deref().unwrap_or("").trim().to_owned();
	c!("Name is present and non-empty", !name.is_empty(), "Name field is missing or empty");
	let author = manifest.author.as_deref().unwrap_or("").trim().to_owned();
	c!("Author is present and non-empty", !author.is_empty(), "Author field is missing or empty");
	let version = manifest.version.as_deref().unwrap_or("").trim().to_owned();
	let version_ok = c!("Version is present and non-empty", !version.is_empty(), "Version field is missing or empty");
	if version_ok {
		c!("Version is valid semver", semver::Version::parse(&version).is_ok(), &format!("\"{version}\" is not a valid semver string"));
	} else {
		failed += 1;
		println!("  ✗ Version is valid semver: skipped (version missing)");
	}
	c!("Icon is present", manifest.icon.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false), "Icon field is missing or empty");
	println!();

	// OS support
	println!("Platform support:");
	let os_list = manifest.os.as_deref().unwrap_or(&[]);
	c!("At least one OS entry is present", !os_list.is_empty(), "OS array is missing or empty");
	let current_platform = match std::env::consts::OS {
		"windows" => "windows",
		"macos" => "mac",
		_ => "linux",
	};
	let supports_current = os_list.iter().any(|o| o.platform == current_platform || o.platform == "windows");
	c!(&format!("Supports current platform ({current_platform})"), supports_current, "no OS entry matches current platform");
	println!();

	// Actions
	println!("Actions:");
	let actions = manifest.actions.as_deref().unwrap_or(&[]);
	c!("At least one action is defined", !actions.is_empty(), "Actions array is missing or empty");
	for (i, action) in actions.iter().enumerate() {
		let action_name = action.name.as_deref().unwrap_or("(unnamed)");
		let label_name = format!("Action[{i}] \"{action_name}\" has Name");
		c!(&label_name, action.name.as_deref().map(|n| !n.trim().is_empty()).unwrap_or(false), "Name field is missing or empty");
		let label_uuid = format!("Action[{i}] \"{action_name}\" has UUID");
		c!(&label_uuid, action.uuid.as_deref().map(|u| !u.trim().is_empty()).unwrap_or(false), "UUID field is missing or empty");
		let states = action.states.as_deref().unwrap_or(&[]);
		let label_states = format!("Action[{i}] \"{action_name}\" has at least one State");
		c!(&label_states, !states.is_empty(), "States array is missing or empty");
	}
	println!();

	// CodePath exists on disk
	println!("Code path:");
	let code_path = match (std::env::consts::OS, manifest.code_path.as_deref(), manifest.code_path_windows.as_deref(), manifest.code_path_macos.as_deref(), manifest.code_path_linux.as_deref()) {
		("windows", _, Some(p), _, _) | ("windows", Some(p), None, _, _) => Some(p.to_owned()),
		("macos", _, _, Some(p), _) | ("macos", Some(p), _, None, _) => Some(p.to_owned()),
		(_, _, _, _, Some(p)) | (_, Some(p), _, _, None) => Some(p.to_owned()),
		_ => None,
	};
	if let Some(ref cp) = code_path {
		let full = base.join(cp);
		c!(&format!("CodePath \"{cp}\" exists on disk"), full.exists(), &format!("{} not found", full.display()));
	} else if let Some(ref paths) = manifest.code_paths {
		let target = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
		if let Some(cp) = paths.get(&target) {
			let full = base.join(cp);
			c!(&format!("CodePaths[\"{target}\"] \"{cp}\" exists on disk"), full.exists(), &format!("{} not found", full.display()));
		} else {
			failed += 1;
			println!("  ✗ CodePath: no entry in CodePaths for target \"{target}\"");
		}
	} else {
		failed += 1;
		println!("  ✗ CodePath: no CodePath or CodePaths field found");
	}
	println!();

	println!("Results: {passed} passed, {failed} failed");
	println!();
	if failed == 0 {
		println!("VERDICT: PASS");
	} else {
		println!("VERDICT: FAIL");
		std::process::exit(1);
	}
}
