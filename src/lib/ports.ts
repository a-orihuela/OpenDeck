import { getPortBase } from "./api/commands.ts";
import { PORT_BASE_DEFAULT } from "./constants.ts";

let portBase = PORT_BASE_DEFAULT;
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
