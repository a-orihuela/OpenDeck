// ── App identity ─────────────────────────────────────────────────────────────
pub const APP_ID: &str = "com.opendeck.app";
pub const PRODUCT_NAME: &str = include_str!("../../product_name.txt").trim_ascii();
/// Tauri app identifier (tauri.conf.json → "identifier"). Controls data paths.
/// Note: this differs from APP_ID which is used for D-Bus/Flatpak.
pub const BINARY_NAME: &str = "opendeck";
pub const TRAY_ID: &str = "opendeck";

// ── Networking ────────────────────────────────────────────────────────────────
pub const PORT_BASE: u16 = 57116;
pub const PORT_WEBSERVER_OFFSET: u16 = 2;

// ── Config directory / file names ─────────────────────────────────────────────
pub const DIR_PLUGINS: &str = "plugins";
pub const DIR_PROFILES: &str = "profiles";
pub const DIR_SETTINGS: &str = "settings";
pub const FILE_PORTS_LOCK: &str = "ports.json";

// ── Built-in plugin identifier ────────────────────────────────────────────────
pub const BUILTIN_PLUGIN: &str = "opendeck";

// ── Built-in action UUIDs ─────────────────────────────────────────────────────
pub const ACTION_MULTIACTION: &str = "opendeck.multiaction";
pub const ACTION_TOGGLEACTION: &str = "opendeck.toggleaction";
pub const ACTION_NEXTPAGE: &str = "opendeck.nextpage";
pub const ACTION_PREVIOUSPAGE: &str = "opendeck.previouspage";
pub const ACTION_FOLDER: &str = "opendeck.folder";

// ── WebSocket protocol strings ────────────────────────────────────────────────
pub const WS_PI_SUFFIX: &str = "|opendeck_property_inspector";
pub const WS_PI_CHILD_SUFFIX: &str = "|opendeck_property_inspector_child";
pub const WS_ALT_ELGATO: &str = "opendeck_alternative_elgato_implementation";

// ── Stream Deck SDK version advertised to plugins ────────────────────────────
pub const ESD_VERSION: &str = "7.1.0";

// ── Category mapping for built-in starterpack actions ────────────────────────
// Keyed by action UUID. Takes priority over the "Category" field in manifest.json
// so that categories always work regardless of the installed manifest version.
pub const BUILTIN_ACTION_CATEGORIES: &[(&str, &str)] = &[
    ("opendeck.builtin.runcommand",     "Automation"),
    ("opendeck.builtin.openurl",        "Automation"),
    ("opendeck.builtin.inputsimulation","Automation"),
    ("opendeck.builtin.switchprofile",  "Productivity"),
    ("opendeck.builtin.pomodoro",       "Productivity"),
    ("opendeck.builtin.brightnessup",   "System"),
    ("opendeck.builtin.brightnessdown", "System"),
    ("opendeck.builtin.lockscreen",     "System"),
    ("opendeck.builtin.sleep",          "System"),
    ("opendeck.builtin.screenshot",     "System"),
    ("opendeck.builtin.volumeup",       "Media"),
    ("opendeck.builtin.volumedown",     "Media"),
    ("opendeck.builtin.mute",           "Media"),
    ("opendeck.builtin.playpause",      "Media"),
    ("opendeck.builtin.nexttrack",      "Media"),
    ("opendeck.builtin.prevtrack",      "Media"),
];
