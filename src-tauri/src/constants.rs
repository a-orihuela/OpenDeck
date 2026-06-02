// ── App identity ─────────────────────────────────────────────────────────────
pub const APP_ID: &str = "com.omegadeck.app";
pub const PRODUCT_NAME: &str = include_str!("../../product_name.txt").trim_ascii();
/// Tauri app identifier (tauri.conf.json → "identifier"). Controls data paths.
/// Note: this differs from APP_ID which is used for D-Bus/Flatpak.
pub const BINARY_NAME: &str = "omegadeck";
pub const TRAY_ID: &str = "omegadeck";

// ── Networking ────────────────────────────────────────────────────────────────
pub const PORT_BASE: u16 = 57116;
pub const PORT_WEBSERVER_OFFSET: u16 = 2;

// ── Config directory / file names ─────────────────────────────────────────────
pub const DIR_PLUGINS: &str = "plugins";
pub const DIR_PROFILES: &str = "profiles";
pub const DIR_SETTINGS: &str = "settings";
pub const FILE_SHEET_TEMPLATES: &str = "sheet_templates";
pub const FILE_PORTS_LOCK: &str = "ports.json";

// ── Built-in plugin identifier ────────────────────────────────────────────────
pub const BUILTIN_PLUGIN: &str = "omegadeck";

// ── Built-in action UUIDs ─────────────────────────────────────────────────────
pub const ACTION_MULTIACTION: &str = "omegadeck.multiaction";
pub const ACTION_TOGGLEACTION: &str = "omegadeck.toggleaction";
pub const ACTION_NEXTPAGE: &str = "omegadeck.nextpage";
pub const ACTION_PREVIOUSPAGE: &str = "omegadeck.previouspage";
pub const ACTION_FOLDER: &str = "omegadeck.folder";

// ── WebSocket protocol strings ────────────────────────────────────────────────
pub const WS_PI_SUFFIX: &str = "|omegadeck_property_inspector";
pub const WS_PI_CHILD_SUFFIX: &str = "|omegadeck_property_inspector_child";
pub const WS_ALT_ELGATO: &str = "omegadeck_alternative_elgato_implementation";

// ── Stream Deck SDK version advertised to plugins ────────────────────────────
pub const ESD_VERSION: &str = "7.1.0";

