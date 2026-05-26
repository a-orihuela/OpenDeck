export type { Settings } from "./bindings.ts";
import type { Settings } from "./bindings.ts";

import { invoke } from "@tauri-apps/api/core";
import { type Writable, writable } from "svelte/store";
import { notify } from "./notifications.ts";

export const settings: Writable<Settings | null> = writable(null);
(async () => settings.set(await invoke("get_settings")))();
export const localisations: Writable<{ [plugin: string]: any } | null> = writable(null);
settings.subscribe(async (value) => {
	if (value) {
		try {
			await invoke("set_settings", { settings: value });
			localisations.set(await invoke("get_localisations", { locale: value.language }));
		} catch (error: any) {
			notify(String(error));
		}
	}
});
