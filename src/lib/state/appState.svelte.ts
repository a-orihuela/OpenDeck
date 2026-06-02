import type { Action, Context, Settings } from "../bindings.ts";
import { getLocalisations, getSettings, setSettings, switchPropertyInspector } from "../api/commands.ts";
import { onPluginStatusChanged } from "../api/events.ts";
import { _, setAppLocale } from "../i18n";
import { get } from "svelte/store";

export type CopiedItem =
	| { type: "instance"; source: Context }
	| { type: "action"; action: Action };

export type NotificationLevel = "error" | "warning" | "info";

export interface Notification {
	id: string;
	message: string;
	level: NotificationLevel;
}

class AppStateClass {
	settings = $state<Settings | null>(null);
	localisations = $state<{ [plugin: string]: any } | null>(null);
	connectedPlugins = $state(new Set<string>());
	inspectedInstance = $state<string | Context | null>(null);
	inspectedParentAction = $state<Context | null>(null);
	openContextMenu = $state<{ context: Context; x: number; y: number } | null>(null);
	copiedItem = $state<CopiedItem | null>(null);
	notifications = $state<Notification[]>([]);
	inFolderMode = $state(false);
}

export const appState = new AppStateClass();

export function notify(message: string, level: NotificationLevel = "error") {
	const id = Math.random().toString(36).slice(2);
	appState.notifications = [...appState.notifications, { id, message, level }];
	setTimeout(() => dismissNotification(id), 8000);
}

function extractErrorMessage(error: unknown): string {
	if (typeof error === "string") return error;
	if (error instanceof Error) return error.message || String(error);
	if (error && typeof error === "object") {
		const candidate = error as { description?: unknown; message?: unknown; error?: unknown };
		if (typeof candidate.description === "string") return candidate.description;
		if (typeof candidate.message === "string") return candidate.message;
		if (typeof candidate.error === "string") return candidate.error;
	}
	return String(error ?? "");
}

function localizeErrorMessage(raw: string): string {
	const clean = raw.replace(/^Error:\s*/i, "").trim();
	if (!clean || clean === "[object Object]") return get(_)("errors.generic");

	const value = clean.toLowerCase();
	if (value.includes("github_api_error")) return get(_)("errors.network");
	if (value.includes("permission denied")) return get(_)("errors.permissionDenied");
	if (value.includes("no such file") || value.includes("not found")) return get(_)("errors.notFound");
	if (value.includes("timeout") || value.includes("timed out")) return get(_)("errors.timeout");
	if (value.includes("failed to fetch") || value.includes("connection refused") || value.includes("network")) return get(_)("errors.network");
	if (value.includes("already exists")) return get(_)("errors.alreadyExists");
	return clean;
}

export function formatError(error: unknown): string {
	return localizeErrorMessage(extractErrorMessage(error));
}

export function notifyError(error: unknown, level: NotificationLevel = "error") {
	notify(formatError(error), level);
}

export function dismissNotification(id: string) {
	appState.notifications = appState.notifications.filter((item) => item.id !== id);
}

(async () => {
	const settings = await getSettings();
	appState.settings = settings;
	setAppLocale(settings.language);
})();

$effect.root(() => {
	let prevSettings: Settings | null = null;
	$effect(() => {
		const s = appState.settings;
		if (!s || s === prevSettings) return;
		prevSettings = s;
		setAppLocale(s.language);
		setSettings(s)
			.then(() => getLocalisations(s!.language))
			.then(loc => { appState.localisations = loc; })
			.catch(e => notifyError(e, "warning"));
	});

	onPluginStatusChanged((uuid, connected) => {
		const next = new Set(appState.connectedPlugins);
		connected ? next.add(uuid) : next.delete(uuid);
		appState.connectedPlugins = next;
	});

	let oldInspected: string | Context | null = null;
	$effect(() => {
		const value = appState.inspectedInstance;
		if (value === oldInspected) return;
		switchPropertyInspector(
			typeof oldInspected === "string" ? oldInspected : null,
			typeof value === "string" ? value : null,
		);
		oldInspected = value;
	});

	document.addEventListener("click", () => { appState.openContextMenu = null; });
	document.addEventListener("keydown", (e) => { if (e.key === "Escape") appState.openContextMenu = null; });
	globalThis.addEventListener("blur", () => { appState.openContextMenu = null; });
});
