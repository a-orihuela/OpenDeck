import text from "../../product_name.txt?raw" with { type: "text" };
export const PRODUCT_NAME = text.trim();

// Component instance refs — inherently Svelte-specific, cannot be framework-agnostic.
import { type Writable, writable } from "svelte/store";

import type ActionList from "../components/ActionList.svelte";
import type DeviceSelector from "../components/DeviceSelector.svelte";
import type ProfileManager from "../components/ProfileManager.svelte";

export const actionList: Writable<ActionList | null> = writable(null);
export const deviceSelector: Writable<DeviceSelector | null> = writable(null);
export const profileManager: Writable<ProfileManager | null> = writable(null);

// inFolderMode lives in appState (framework-agnostic) — re-exported here for convenience.
export { inFolderMode } from "./state/appState.ts";
