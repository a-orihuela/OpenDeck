import type { PluginInfo } from "../api/commands.ts";

// @ts-expect-error — browser override for Tauri webview
const _fetch: typeof fetch = window.fetchNative ?? window.fetch;

export type GitHubPlugin = {
	name: string;
	author: string;
	repository: string;
	download_url: string | undefined;
};

// ── Catalogue fetching ────────────────────────────────────────────────────────

export async function fetchOpenSourceCatalogue(): Promise<Record<string, GitHubPlugin>> {
	return _fetch("https://openactionapi.github.io/plugins/catalogue.json").then(r => r.json());
}

export async function fetchElgatoArchive(): Promise<any[]> {
	return _fetch("https://plugins.amankhanna.me/catalogue.json").then(r => r.json());
}

// ── GitHub releases ───────────────────────────────────────────────────────────

export async function fetchGitHubReleases(repoUrl: string): Promise<any[]> {
	const endpoint = new URL(repoUrl);
	endpoint.hostname = "api." + endpoint.hostname;
	endpoint.pathname = "/repos" + endpoint.pathname + "/releases";
	const res = await _fetch(endpoint.toString());
	return res.json();
}

export async function fetchLatestGitHubRelease(repoUrl: string): Promise<any> {
	const endpoint = new URL(repoUrl);
	endpoint.hostname = "api." + endpoint.hostname;
	endpoint.pathname = "/repos" + endpoint.pathname + "/releases/latest";
	const res = await _fetch(endpoint.toString());
	if (!res.ok) throw new Error(`GitHub API error: ${res.status}`);
	return res.json();
}

/** Returns the assets from the latest release that are installable (.streamdeckplugin or .zip). */
export function filterInstallableAssets(releases: any[]): any[] {
	return (releases[0]?.assets ?? []).filter((a: any) =>
		a.name.toLowerCase().endsWith(".streamdeckplugin") ||
		a.name.toLowerCase().endsWith(".zip"),
	);
}

// ── Version checking ──────────────────────────────────────────────────────────

function normalizeVersion(v: string): string {
	return v.replace(/^v/, "").replace(/^(\d+\.\d+\.\d+)\.\d+$/, "$1");
}

/** Returns the newer version string if an update is available, false otherwise. */
export async function checkUpdateAvailable(
	plugin: Pick<PluginInfo, "id" | "version">,
	catalogue: Record<string, GitHubPlugin>,
): Promise<string | false> {
	const id = plugin.id.endsWith(".sdPlugin") ? plugin.id.slice(0, -9) : plugin.id;
	const cataloguePlugin = catalogue[id];
	if (!cataloguePlugin || cataloguePlugin.download_url) return false;

	try {
		const release = await fetchLatestGitHubRelease(cataloguePlugin.repository);
		if (normalizeVersion(release.tag_name) !== normalizeVersion(plugin.version)) {
			return release.tag_name.replace(/^v/, "");
		}
		return false;
	} catch {
		return false;
	}
}

// ── Plugin README ─────────────────────────────────────────────────────────────

/** Fetches the raw README markdown for a GitHub repo (tries main/master, README/readme). */
export async function fetchPluginReadme(repo: string): Promise<{ markdown: string; baseUrl: string } | null> {
	const candidates = [
		`https://raw.githubusercontent.com/${repo}/main/README.md`,
		`https://raw.githubusercontent.com/${repo}/main/readme.md`,
		`https://raw.githubusercontent.com/${repo}/master/README.md`,
		`https://raw.githubusercontent.com/${repo}/master/readme.md`,
	];
	for (const url of candidates) {
		const res = await _fetch(url);
		if (res.ok) return { markdown: await res.text(), baseUrl: url };
	}
	return null;
}

/** Sums download counts across all releases/assets for a GitHub repo. */
export async function fetchTotalDownloadCount(repo: string): Promise<number> {
	const res = await _fetch(`https://api.github.com/repos/${repo}/releases`);
	const releases: any[] = await res.json();
	let total = 0;
	for (const release of releases)
		for (const asset of release.assets)
			total += asset.download_count;
	return total;
}

// ── Deep-link URL parsing ─────────────────────────────────────────────────────

/** Extracts a plugin id from an omegadeck:// installPlugin deep-link URL. */
export function parseInstallPluginUrl(url: string): string | null {
	if (!url.includes("installPlugin/")) return null;
	return url.split("installPlugin/")[1] ?? null;
}
