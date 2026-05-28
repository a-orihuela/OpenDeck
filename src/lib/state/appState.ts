import type { Action, Context, Settings } from "../bindings.ts";
import { getLocalisations, getSettings, setSettings, switchPropertyInspector } from "../api/commands.ts";
import { onPluginStatusChanged } from "../api/events.ts";

// ── Minimal observable that matches the Svelte store contract ────────────────
// Implements subscribe/set/update without importing svelte/store.
// Svelte components can use the $store shorthand; React wraps it in a hook.

type Subscriber<T> = (value: T) => void;
type Unsubscriber = () => void;

export class Observable<T> {
	private _value: T;
	private _subs = new Set<Subscriber<T>>();

	constructor(initial: T) {
		this._value = initial;
	}

	get(): T {
		return this._value;
	}

	set(value: T): void {
		this._value = value;
		for (const sub of this._subs) sub(value);
	}

	update(fn: (value: T) => T): void {
		this.set(fn(this._value));
	}

	// Svelte store contract: calls subscriber immediately, returns unsubscriber.
	subscribe(run: Subscriber<T>, _invalidate?: () => void): Unsubscriber {
		this._subs.add(run);
		run(this._value);
		return () => this._subs.delete(run);
	}
}

// ── Settings ─────────────────────────────────────────────────────────────────

export const settings = new Observable<Settings | null>(null);
export const localisations = new Observable<{ [plugin: string]: any } | null>(null);

(async () => settings.set(await getSettings()))();

let _prevSettings: Settings | null = null;
settings.subscribe(async (value) => {
	if (!value || value === _prevSettings) return;
	_prevSettings = value;
	try {
		await setSettings(value);
		localisations.set(await getLocalisations(value.language));
	} catch {}
});

// ── Plugin connectivity ───────────────────────────────────────────────────────

export const connectedPlugins = new Observable<Set<string>>(new Set());

onPluginStatusChanged((uuid, connected) => {
	connectedPlugins.update((set) => {
		const next = new Set(set);
		if (connected) next.add(uuid);
		else next.delete(uuid);
		return next;
	});
});

// ── Property inspector / UI state ────────────────────────────────────────────

export const inspectedInstance = new Observable<string | Context | null>(null);
export const inspectedParentAction = new Observable<Context | null>(null);
export const openContextMenu = new Observable<{ context: Context; x: number; y: number } | null>(null);

export type CopiedItem =
	| { type: "instance"; source: Context }
	| { type: "action"; action: Action };
export const copiedItem = new Observable<CopiedItem | null>(null);

let _oldInspected: string | Context | null = null;
inspectedInstance.subscribe(async (value) => {
	if (value === _oldInspected) return;
	await switchPropertyInspector(
		typeof _oldInspected == "string" ? _oldInspected : null,
		typeof value == "string" ? value : null,
	);
	_oldInspected = value;
});

document.addEventListener("click", () => openContextMenu.set(null));
document.addEventListener("keydown", (e) => { if (e.key === "Escape") openContextMenu.set(null); });
globalThis.addEventListener("blur", () => openContextMenu.set(null));

// ── Notifications ─────────────────────────────────────────────────────────────

export type NotificationLevel = "error" | "warning" | "info";

export interface Notification {
	id: string;
	message: string;
	level: NotificationLevel;
}

export const notifications = new Observable<Notification[]>([]);

export function notify(message: string, level: NotificationLevel = "error") {
	const id = Math.random().toString(36).slice(2);
	notifications.update((n) => [...n, { id, message, level }]);
	setTimeout(() => dismissNotification(id), 8000);
}

export function dismissNotification(id: string) {
	notifications.update((n) => n.filter((item) => item.id !== id));
}

// ── App-level UI state ────────────────────────────────────────────────────────

export const inFolderMode = new Observable<boolean>(false);
