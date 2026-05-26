import { writable } from "svelte/store";
import { listen } from "@tauri-apps/api/event";

export const connectedPlugins = writable<Set<string>>(new Set());

listen("plugin_status_changed", ({ payload }: { payload: { uuid: string; connected: boolean } }) => {
	connectedPlugins.update((set) => {
		const next = new Set(set);
		if (payload.connected) next.add(payload.uuid);
		else next.delete(payload.uuid);
		return next;
	});
});
