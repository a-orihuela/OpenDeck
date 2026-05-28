import { invoke } from "@tauri-apps/api/core";
import type { Action, ActionInstance, ActionState, Category, Context, DeviceInfo, Profile, Settings } from "../bindings.ts";

export type PluginInfo = {
	id: string;
	name: string;
	author: string;
	icon: string;
	version: string;
	has_settings_interface: boolean;
	builtin: boolean;
	registered: boolean;
};

// ── Devices ──────────────────────────────────────────────────────────────────

export const getDevices = () =>
	invoke<Record<string, DeviceInfo>>("get_devices");

export const getBuildInfo = () =>
	invoke<string>("get_build_info");

// ── Profiles ─────────────────────────────────────────────────────────────────

export const getProfiles = (device: string) =>
	invoke<string[]>("get_profiles", { device });

export const getSelectedProfile = (device: string) =>
	invoke<Profile>("get_selected_profile", { device });

export const setSelectedProfile = (device: string, id: string) =>
	invoke<void>("set_selected_profile", { device, id });

export const deleteProfile = (device: string, profile: string) =>
	invoke<void>("delete_profile", { device, profile });

export const renameProfile = (device: string, oldId: string, newId: string, retain: boolean) =>
	invoke<void>("rename_profile", { device, oldId, newId, retain });

export const getApplications = () =>
	invoke<string[]>("get_applications");

export const getApplicationProfiles = () =>
	invoke<{ [appName: string]: { [device: string]: string } }>("get_application_profiles");

export const setApplicationProfiles = (value: { [appName: string]: { [device: string]: string } }) =>
	invoke<void>("set_application_profiles", { value });

// ── Pages & Folders ──────────────────────────────────────────────────────────

export const getActivePage = (device: string) =>
	invoke<number>("get_active_page", { device });

export const setActivePage = (device: string, page: number) =>
	invoke<void>("set_active_page", { device, page });

export const addPage = (device: string) =>
	invoke<number>("add_page", { device });

export const removeLastPage = (device: string) =>
	invoke<number>("remove_last_page", { device });

export const enterFolder = (device: string, folderContext: string) =>
	invoke<void>("enter_folder", { device, folderContext });

export const exitFolder = (device: string) =>
	invoke<void>("exit_folder", { device });

// ── Instances ────────────────────────────────────────────────────────────────

export const createInstance = (context: Context | string, action: string) =>
	invoke<ActionInstance | null>("create_instance", { context, action });

export const moveInstance = (source: string, destination: Context, retain: boolean) =>
	invoke<ActionInstance>("move_instance", { source, destination, retain });

export const removeInstance = (context: string) =>
	invoke<void>("remove_instance", { context });

export const setState = (context: string, index: number, state: ActionState) =>
	invoke<void>("set_state", { context, index, state });

export const updateImage = (context: string, image: string | null) =>
	invoke<void>("update_image", { context, image });

export const triggerVirtualPress = (context: string) =>
	invoke<void>("trigger_virtual_press", { context });

// ── Actions & Categories ─────────────────────────────────────────────────────

export const getCategories = () =>
	invoke<Record<string, Category>>("get_categories");

// ── Plugins ──────────────────────────────────────────────────────────────────

export const listPlugins = () =>
	invoke<PluginInfo[]>("list_plugins");

export const installPlugin = (url: string | null, file: string | null, fallback_id: string | null) =>
	invoke<void>("install_plugin", { url, file, fallback_id });

export const removePlugin = (id: string) =>
	invoke<void>("remove_plugin", { id });

export const reloadPlugin = (id: string) =>
	invoke<void>("reload_plugin", { id });

export const showSettingsInterface = (plugin: string) =>
	invoke<void>("show_settings_interface", { plugin });

// ── Property Inspector ───────────────────────────────────────────────────────

export const makeInfo = (plugin: string) =>
	invoke<object>("make_info", { plugin });

export const switchPropertyInspector = (oldContext: string | null, newContext: string | null) =>
	invoke<void>("switch_property_inspector", { old: oldContext, new: newContext });

// ── Settings ─────────────────────────────────────────────────────────────────

export const getSettings = () =>
	invoke<Settings>("get_settings");

export const setSettings = (settings: Settings) =>
	invoke<void>("set_settings", { settings });

export const getLocalisations = (locale: string) =>
	invoke<{ [plugin: string]: any }>("get_localisations", { locale });

export const getFonts = () =>
	invoke<string[]>("get_fonts");

export const getPortBase = () =>
	invoke<number>("get_port_base");

// ── Utilities ────────────────────────────────────────────────────────────────

export const openUrl = (url: string) =>
	invoke<void>("open_url", { url });

export const openConfigDirectory = () =>
	invoke<void>("open_config_directory");

export const openLogDirectory = () =>
	invoke<void>("open_log_directory");

export const backupConfigDirectory = () =>
	invoke<boolean>("backup_config_directory");

export const restoreConfigDirectory = () =>
	invoke<void>("restore_config_directory");

export const restart = () =>
	invoke<void>("restart");
