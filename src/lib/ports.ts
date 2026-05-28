import { getPortBase } from "./api/commands.ts";

let portBase = 57116;
let initialised = false;

export async function initPortBase(): Promise<void> {
	if (initialised) return;
	portBase = await getPortBase();
	initialised = true;
}

export function getWebSocketPort(): number {
	return portBase;
}

export function getWebserverUrl(path: string = ""): string {
	return `http://localhost:${portBase + 2}/${path}`;
}
