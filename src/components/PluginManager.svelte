<script lang="ts">
	import ArrowClockwise from "phosphor-svelte/lib/ArrowClockwise";
	import ArrowLeft from "phosphor-svelte/lib/ArrowLeft";
	import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
	import CloudArrowDown from "phosphor-svelte/lib/CloudArrowDown";
	import FileArrowUp from "phosphor-svelte/lib/FileArrowUp";
	import Gear from "phosphor-svelte/lib/Gear";
	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
	import Trash from "phosphor-svelte/lib/Trash";
	import WarningCircle from "phosphor-svelte/lib/WarningCircle";
	import ListedPlugin from "./ListedPlugin.svelte";
	import PluginDetails from "./PluginDetails.svelte";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";
	import { _ } from "$lib/i18n";

	import { getWebserverUrl } from "$lib/ports";
	import { appState } from "$lib/settings";
	import { actionList, deviceSelector, PRODUCT_NAME } from "$lib/singletons";
	import { formatError } from "$lib/notifications";
	import { get } from "svelte/store";
	import { untrack } from "svelte";

	import { installPlugin as apiInstallPlugin, listPlugins, openLogDirectory, reloadPlugin, removePlugin as apiRemovePlugin, showSettingsInterface } from "$lib/api/commands";
	import { onPluginInstallProgress } from "$lib/api/events";
	import {
		type GitHubPlugin,
		checkUpdateAvailable,
		fetchElgatoArchive,
		fetchGitHubReleases,
		fetchOpenSourceCatalogue,
		filterInstallableAssets,
		parseInstallPluginUrl,
	} from "$lib/services/pluginService";
	import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
	import { ask, message, open } from "@tauri-apps/plugin-dialog";

	let {
		fullPage = false,
		onOpen,
		onBack = () => {},
	}: {
		fullPage?: boolean;
		onOpen?: (() => void) | undefined;
		onBack?: () => void;
	} = $props();

	let showPopup = $state(false);
	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

	$effect(() => {
		const interval = setInterval(async () => {
			if (showPopup || fullPage) installed = await listPlugins();
		}, 1e3);
		return () => clearInterval(interval);
	});

	let installProgress: { downloaded: number; total: number | null } | null = $state(null);
	onPluginInstallProgress((downloaded, total) => {
		installProgress = { downloaded, total };
	});

	async function installPlugin(name: string, url: string | null, file: string | null, fallback_id: string | null) {
		if (!file && !await ask(t("plugins.dialogs.installPrompt"), { title: t("plugins.dialogs.installConfirmTitle", { name }) })) return;
		installProgress = url ? { downloaded: 0, total: null } : null;
		try {
			await apiInstallPlugin(url, file, fallback_id);
			message(t("plugins.dialogs.installSuccessMessage", { name }), { title: t("plugins.dialogs.installSuccessTitle", { name }) });
			get(actionList)?.reload();
			installed = await listPlugins();
		} catch (error: any) {
			message(formatError(error), { title: t("plugins.dialogs.installFailureTitle", { name }) });
		} finally {
			installProgress = null;
		}
	}

	let choices: any[] | undefined = $state(undefined);
	let choice = $state(0);
	let finishChoice = $state((_: unknown) => {});
	let cancelChoice = $state(() => {});
	async function chooseAsset(assets: any[]): Promise<any> {
		choices = assets;
		try {
			await new Promise((resolve, reject) => {
				finishChoice = resolve;
				cancelChoice = reject;
			});
		} catch (e) {
			throw e;
		} finally {
			choices = undefined;
			finishChoice = (_: unknown) => {};
			cancelChoice = () => {};
		}
		return assets[choice];
	}

	let openDetailsView: string | null = $state(null);

	async function installPluginGitHub(id: string, plugin: GitHubPlugin) {
		if (plugin.download_url) {
			await installPlugin(plugin.name, plugin.download_url, null, id);
			return;
		}
		let releases;
		try {
			releases = await fetchGitHubReleases(plugin.repository);
		} catch (error: any) {
			message(formatError(error), { title: t("plugins.dialogs.installFailureTitle", { name: plugin.name }) });
			return;
		}
		const assets = filterInstallableAssets(releases);
		let selected;
		if (assets.length === 1) selected = assets[0];
		else {
			try { selected = await chooseAsset(assets); }
			catch { return; }
		}
		await installPlugin(plugin.name, selected.browser_download_url, null, id);
	}

	async function installPluginElgato(plugin: any) {
		await installPlugin(plugin.name, `https://plugins.amankhanna.me/rezipped/${plugin.id}.zip`, null, plugin.id);
	}

	async function installPluginFile() {
		const path = await open({ multiple: false, directory: false });
		if (!path) return;
		await installPlugin(path.split(/[\/\\]/).at(-1) ?? path, null, path, null);
	}

	async function confirmRemovePlugin(plugin: any) {
		if (!await ask(t("plugins.dialogs.removeConfirmMessage", { name: plugin.name }), { title: t("plugins.dialogs.removeConfirmTitle", { name: plugin.name }) })) return;
		try {
			await apiRemovePlugin(plugin.id);
			message(t("plugins.dialogs.removeSuccessMessage", { name: plugin.name }), { title: t("plugins.dialogs.removeSuccessTitle", { name: plugin.name }) });
			get(actionList)?.reload();
			get(deviceSelector)?.reloadProfiles();
			installed = await listPlugins();
		} catch (error: any) {
			message(formatError(error), { title: t("plugins.dialogs.removeFailureTitle", { name: plugin.name }) });
		}
	}

	let installed: any[] = $state([]);
	(async () => { installed = await listPlugins(); })();

	let plugins: Record<string, GitHubPlugin> | null = $state(null);
	(async () => { plugins = await fetchOpenSourceCatalogue(); })();

	let showArchive = $state(false);
	let archivePlugins: any[] | null = $state(null);

	let availableUpdates: { [id: string]: string | false } = $state({});
	let checkedPlugins = $state(new Set<string>());

	$effect(() => {
		if (showPopup || fullPage) {
			for (const plugin of installed) {
				if (!untrack(() => checkedPlugins.has(plugin.id))) {
					untrack(() => {
						checkedPlugins.add(plugin.id);
						checkedPlugins = new Set(checkedPlugins);
					});
					checkUpdateAvailable(plugin, untrack(() => plugins) ?? {})
						.then((version) => { untrack(() => { availableUpdates = { ...availableUpdates, [plugin.id]: version }; }); });
				}
			}
		}
	});

	$effect(() => {
		for (const plugin of installed) {
			const pv = untrack(() => pluginVersions[plugin.id]);
			if (pv != plugin.version) {
				untrack(() => {
					checkedPlugins.delete(plugin.id);
					checkedPlugins = new Set(checkedPlugins);
					delete availableUpdates[plugin.id];
					availableUpdates = { ...availableUpdates };
					pluginVersions[plugin.id] = plugin.version;
				});
			}
		}
	});

	let pluginVersions: { [id: string]: string } = $state({});
	let query = $state("");

	onOpenUrl((urls: string[]) => {
		const id = parseInstallPluginUrl(urls[0]);
		if (!id || !plugins?.[id]) return;
		installPluginGitHub(id, plugins[id]);
	});
</script>

{#if !fullPage}
	<button
		class="px-3 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
		onclick={() => {
			if (onOpen) onOpen();
			else showPopup = true;
		}}
	>
		{t("plugins.button")}
	</button>
{/if}

<svelte:window
	onkeydown={(event) => {
		if (event.key == "Escape") {
			if (choices) cancelChoice();
			else if (openDetailsView) openDetailsView = null;
			else if (fullPage) onBack();
			else showPopup = false;
		}
	}}
/>

	{#snippet pluginContent()}
		{#if installProgress}
			<div class="mx-2 mt-4">
				<p class="text-sm text-neutral-400 mb-1">
					{#if installProgress.total}
						{t("plugins.downloadProgressWithTotal", { downloaded: Math.round(installProgress.downloaded / 1024), total: Math.round(installProgress.total / 1024) })}
					{:else}
						{t("plugins.downloadProgressNoTotal", { downloaded: Math.round(installProgress.downloaded / 1024) })}
					{/if}
				</p>
				<div class="w-full bg-neutral-700 rounded-full h-2">
					<div
						class="bg-blue-500 h-2 rounded-full transition-all"
						style="width: {installProgress.total ? Math.round((installProgress.downloaded / installProgress.total) * 100) : 100}%"
					></div>
				</div>
			</div>
		{/if}

		<h2 class="mx-2 mt-6 mb-2 text-lg text-neutral-400">{t("plugins.installedTitle")}</h2>
	<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
		<!-- deno-fmt-ignore -->
		{#each installed.sort((a, b) =>
			(a.builtin && !b.builtin) ? -1 :
			(b.builtin && !a.builtin) ? 1 :
			(a.has_settings_interface && !b.has_settings_interface) ? -1 :
			(b.has_settings_interface && !a.has_settings_interface) ? 1 :
			a.id.localeCompare(b.id)
		) as plugin}
			<ListedPlugin
				icon={getWebserverUrl(plugin.icon)}
				name={(appState.localisations?.[plugin.id]?.Name) ?? plugin.name}
				subtitle={plugin.version}
				disconnected={!plugin.registered}
				action={() => {
					if (appState.settings?.developer) reloadPlugin(plugin.id);
					else confirmRemovePlugin(plugin);
				}}
				actionLabel={appState.settings?.developer ? t("plugins.actionReload") : t("plugins.actionRemove")}
				secondaryAction={!plugin.registered ? () => openLogDirectory() : plugin.has_settings_interface ? () => showSettingsInterface(plugin.id) : undefined}
				secondaryActionLabel={!plugin.registered ? t("plugins.actionViewLogs") : t("plugins.actionSettings")}
			>
				{#snippet subtitleSnippet()}
					{plugin.version}
					{#if availableUpdates[plugin.id]}
						(<span class="text-yellow-400">
							{t("plugins.available")}
							<button
								class="font-semibold underline"
								onclick={() => { openDetailsView = plugin.id.endsWith(".sdPlugin") ? plugin.id.slice(0, -9) : plugin.id; }}
							>
								{availableUpdates[plugin.id]}
							</button></span>)
					{/if}
				{/snippet}

				{#snippet secondary()}
					{#if !plugin.registered}
						<WarningCircle size="24" class="text-yellow-500" />
					{:else if plugin.has_settings_interface}
						<Gear size="24" class="text-green-600" />
					{/if}
				{/snippet}

				{#snippet children()}
					{#if appState.settings?.developer}
						<ArrowClockwise size="24" class="mt-2 text-neutral-400" />
					{:else if !plugin.builtin}
						<Trash size="24" class="mt-2 text-neutral-400" />
					{/if}
				{/snippet}
			</ListedPlugin>
		{/each}
	</div>

	<div class="flex flex-row justify-between items-center mx-2 mt-6 mb-2">
		<h2 class="text-lg text-neutral-400">{t("plugins.storeTitle")}</h2>
		<button
			class="flex flex-row items-center mt-2 px-1 py-0.5 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			onclick={installPluginFile}
		>
			<FileArrowUp />
			<span class="ml-1">{t("plugins.installFromFile")}</span>
		</button>
	</div>

	<div class="flex flex-row items-center mx-2 my-4 p-3 space-x-2 bg-yellow-900/20 border-l-4 border-yellow-500 rounded">
		<WarningCircle size="20" class="mt-0.5 text-yellow-500" />
		<div class="text-sm text-yellow-200">
			{t("plugins.supportWarning", { product: PRODUCT_NAME })}
		</div>
	</div>

	<div class="flex flex-row items-center m-2 bg-neutral-700 border border-neutral-600 rounded-lg">
		<MagnifyingGlass size="14" class="ml-3 mr-0.5 text-neutral-300" />
		<input
			bind:value={query}
			class="w-full p-2 text-neutral-300"
			placeholder={t("plugins.searchPlaceholder")}
			aria-label={t("plugins.searchAria")}
			type="search"
			spellcheck="false"
		/>
	</div>

	{#if !plugins}
		<h2 class="mx-2 mt-6 mb-2 text-md text-neutral-400">{t("plugins.loadingOpenSource")}</h2>
	{:else}
		<div class="flex flex-row items-center ml-2 mt-6 mb-2 space-x-2">
			<h2 class="font-semibold text-md text-neutral-400">{t("plugins.openSourceTitle")}</h2>
			{#snippet tooltipOss()}{t("plugins.openSourceTooltip")}{/snippet}
			<Tooltip>{@render tooltipOss()}</Tooltip>
		</div>
		<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
			{#each Object.entries(plugins) as [id, plugin]}
				<ListedPlugin
					icon="https://openactionapi.github.io/plugins/icons/{id}.png"
					name={plugin.name}
					subtitle={plugin.author}
					hidden={!plugin.name.toLowerCase().includes(query.toLowerCase())}
					action={() => { openDetailsView = id; }}
					actionLabel={t("plugins.viewDetails")}
				>
					{#snippet children()}
						<ArrowSquareOut size="24" class="text-neutral-400" />
					{/snippet}
				</ListedPlugin>
			{/each}
		</div>
	{/if}

	<div class="flex flex-row items-center mt-6 mb-2">
		<h2 class="mx-2 font-semibold text-md text-neutral-400">{t("plugins.elgatoArchiveTitle")}</h2>
		{#snippet tooltipElgato()}{t("plugins.elgatoArchiveTooltip")}{/snippet}
		<Tooltip>{@render tooltipElgato()}</Tooltip>
	</div>
	{#if !showArchive}
		<button
			class="ml-2 mt-2 mb-2 px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			onclick={async () => {
				showArchive = true;
				archivePlugins = await fetchElgatoArchive();
			}}
		>
			{t("plugins.loadElgatoArchive")}
		</button>
	{:else if !archivePlugins}
		<h2 class="mx-2 mt-4 mb-2 text-md text-neutral-400">{t("plugins.loadingElgatoArchive")}</h2>
	{:else}
		<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
			{#each archivePlugins as plugin}
				<ListedPlugin
					icon="https://plugins.amankhanna.me/icons/{plugin.id}.png"
					name={plugin.name}
					subtitle={plugin.author}
					hidden={!plugin.name.toLowerCase().includes(query.toLowerCase())}
					action={() => installPluginElgato(plugin)}
					actionLabel={t("plugins.install")}
				>
					{#snippet children()}
						<CloudArrowDown size="24" class="text-neutral-400" />
					{/snippet}
				</ListedPlugin>
			{/each}
		</div>
	{/if}

	{#if "Tacto Connect".toLowerCase().includes(query.toLowerCase())}
		<div class="flex flex-row items-center mt-6 mb-2">
			<h2 class="mx-2 font-semibold text-md text-neutral-400">{t("plugins.tactoTitle")}</h2>
			{#snippet tooltipTacto()}{t("plugins.tactoTooltip")}{/snippet}
			<Tooltip>{@render tooltipTacto()}</Tooltip>
		</div>
		<div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
			<ListedPlugin
				icon="https://tacto.live/icon-192.png"
				name="Tacto Connect"
				subtitle="Rivulus"
				action={() => {
					installPluginGitHub("us.rivul.tacto", {
						name: "Tacto Connect",
						author: "Rivulus",
						repository: "https://github.com/RivulusLive/tacto-desktop",
						download_url: undefined,
					});
				}}
				actionLabel={t("plugins.install")}
				secondaryAction={() => window.open("https://tacto.live")}
				secondaryActionLabel={t("plugins.visitWebsite")}
			>
				{#snippet secondary()}
					<ArrowSquareOut size="24" class="text-neutral-400" />
				{/snippet}
				{#snippet children()}
					<CloudArrowDown size="24" class="mt-2 text-neutral-400" />
				{/snippet}
			</ListedPlugin>
		</div>
	{/if}
{/snippet}

{#if fullPage}
	<div class="flex flex-col h-full min-h-0">
		<div class="flex items-center gap-2 px-3 py-2 border-b border-neutral-700 shrink-0">
			<button
				class="p-1 text-neutral-300 hover:text-white hover:bg-neutral-700 rounded-md transition-colors"
				onclick={() => onBack()}
				aria-label={t("common.back")}
			>
				<ArrowLeft size="18" />
			</button>
			<h2 class="text-lg font-semibold text-neutral-300">{t("plugins.title")}</h2>
		</div>
		<div class="grow min-h-0 overflow-auto pb-6">
			{@render pluginContent()}
		</div>
	</div>
{:else}
	<Popup show={showPopup} label={t("plugins.manageTitle")}>
		{#snippet children()}
			<button class="mr-2 my-1 float-right text-xl text-neutral-300" onclick={() => { showPopup = false; }} aria-label={t("common.close")}>✕</button>
			<h2 class="m-2 font-semibold text-xl text-neutral-300">{t("plugins.manageTitle")}</h2>
			{@render pluginContent()}
		{/snippet}
	</Popup>
{/if}

{#if openDetailsView && plugins?.[openDetailsView]}
	<PluginDetails
		id={openDetailsView}
		details={plugins[openDetailsView]}
		install={() => {
			// @ts-expect-error
			installPluginGitHub(openDetailsView, plugins[openDetailsView]);
		}}
		close={() => { openDetailsView = null; }}
	/>
{/if}

{#if choices}
	<div class="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 mt-2 p-2 w-96 text-xs text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg z-40">
		<h3 class="mb-2 font-semibold text-lg text-center">{t("plugins.chooseAssetTitle")}</h3>
		<div class="select-wrapper">
			<select class="w-full bg-neutral-800!" bind:value={choice} aria-label={t("plugins.releaseAssetAria")}>
				{#each choices as c, i}
					<option value={i}>{c.name}</option>
				{/each}
			</select>
		</div>
		<button
			class="mt-2 p-1 w-full text-sm text-neutral-300 bg-neutral-800 hover:bg-neutral-900 transition-colors border border-neutral-600 rounded-lg"
			onclick={finishChoice}
		>
			{t("plugins.install")}
		</button>
	</div>
{/if}
