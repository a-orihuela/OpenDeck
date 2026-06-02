import { _, addMessages, init, locale } from "svelte-i18n";

import en from "./locales/en";
import es from "./locales/es";

const SUPPORTED_LOCALES = new Set(["en", "es"]);
let initialized = false;

function normalizeLocale(value: string | null | undefined): string {
	if (!value) return "en";
	return SUPPORTED_LOCALES.has(value) ? value : "en";
}

export function initializeI18n(initialLocale?: string) {
	if (initialized) return;

	addMessages("en", en);
	addMessages("es", es);
	init({
		fallbackLocale: "en",
		initialLocale: normalizeLocale(initialLocale),
	});

	initialized = true;
}

export function setAppLocale(nextLocale: string) {
	initializeI18n();
	locale.set(normalizeLocale(nextLocale));
}

initializeI18n("en");

export { _ };
