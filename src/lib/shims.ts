import { openUrl } from "./api/commands.ts";

globalThis.open = (url?: string | URL) => {
	if (url) openUrl(String(url));
	return null;
};
