import type { Action, Context, Settings } from "../bindings.ts";
import { getLocalisations, getSettings, setSettings, switchPropertyInspector } from "../api/commands.ts";
import { onPluginStatusChanged } from "../api/events.ts";

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

export function dismissNotification(id: string) {
	appState.notifications = appState.notifications.filter((item) => item.id !== id);
}

(async () => { appState.settings = await getSettings(); })();

$effect.root(() => {
	let prevSettings: Settings | null = null;
	$effect(() => {
		const s = appState.settings;
		if (!s || s === prevSettings) return;
		prevSettings = s;
		setSettings(s)
			.then(() => getLocalisations(s!.language))
			.then(loc => { appState.localisations = loc; })
			.catch(e => notify(String(e), "warning"));
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
