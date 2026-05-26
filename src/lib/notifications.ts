import { writable } from "svelte/store";

export type NotificationLevel = "error" | "warning" | "info";

export interface Notification {
	id: string;
	message: string;
	level: NotificationLevel;
}

export const notifications = writable<Notification[]>([]);

export function notify(message: string, level: NotificationLevel = "error") {
	const id = Math.random().toString(36).slice(2);
	notifications.update((n) => [...n, { id, message, level }]);
	setTimeout(() => dismiss(id), 8000);
}

export function dismiss(id: string) {
	notifications.update((n) => n.filter((item) => item.id !== id));
}
