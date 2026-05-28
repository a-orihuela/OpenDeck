import { listen } from "@tauri-apps/api/event";
import type { ActionInstance, Context, DeviceInfo } from "../bindings.ts";

// ── Devices ──────────────────────────────────────────────────────────────────

export const onDevices = (cb: (devices: Record<string, DeviceInfo>) => void) =>
	listen<Record<string, DeviceInfo>>("devices", ({ payload }) => cb(payload));

export const onSwitchProfile = (cb: (device: string, profile: string) => void) =>
	listen<{ device: string; profile: string }>("switch_profile", ({ payload }) =>
		cb(payload.device, payload.profile));

// ── Pages & Folders ──────────────────────────────────────────────────────────

export const onPageChanged = (cb: (device: string, page: number) => void) =>
	listen<{ device: string; page: number }>("page_changed", ({ payload }) =>
		cb(payload.device, payload.page));

export const onFolderOpened = (cb: (device: string, folderContext: string) => void) =>
	listen<{ device: string; folder_context: string }>("folder_opened", ({ payload }) =>
		cb(payload.device, payload.folder_context));

export const onFolderClosed = (cb: (device: string) => void) =>
	listen<{ device: string }>("folder_closed", ({ payload }) =>
		cb(payload.device));

// ── Key state ────────────────────────────────────────────────────────────────

export const onUpdateState = (cb: (context: string, contents: ActionInstance | null) => void) =>
	listen<{ context: string; contents: ActionInstance | null }>("update_state", ({ payload }) =>
		cb(payload.context, payload.contents));

export const onKeyMoved = (cb: (context: Context, pressed: boolean) => void) =>
	listen<{ context: Context; pressed: boolean }>("key_moved", ({ payload }) =>
		cb(payload.context, payload.pressed));

export const onShowAlert = (cb: (context: string) => void) =>
	listen<string>("show_alert", ({ payload }) => cb(payload));

export const onShowOk = (cb: (context: string) => void) =>
	listen<string>("show_ok", ({ payload }) => cb(payload));

// ── Profiles ─────────────────────────────────────────────────────────────────

export const onRerenderImages = (cb: () => void) =>
	listen<void>("rerender_images", () => cb());

export const onApplications = (cb: (apps: string[]) => void) =>
	listen<string[]>("applications", ({ payload }) => cb(payload));

// ── Plugins ──────────────────────────────────────────────────────────────────

export const onPluginStatusChanged = (cb: (uuid: string, connected: boolean) => void) =>
	listen<{ uuid: string; connected: boolean }>("plugin_status_changed", ({ payload }) =>
		cb(payload.uuid, payload.connected));

export const onPluginInstallProgress = (cb: (downloaded: number, total: number | null) => void) =>
	listen<{ downloaded: number; total: number | null }>("plugin_install_progress", ({ payload }) =>
		cb(payload.downloaded, payload.total));

export const onPluginReloaded = (cb: (pluginId: string) => void) =>
	listen<string>("plugin_reloaded", ({ payload }) => cb(payload));

// ── Settings ─────────────────────────────────────────────────────────────────

export const onDeviceBrightness = (cb: (action: string, value: number) => void) =>
	listen<{ action: string; value: number }>("device_brightness", ({ payload }) =>
		cb(payload.action, payload.value));
