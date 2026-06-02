<script lang="ts">
	import ClockClockwise from "phosphor-svelte/lib/ClockClockwise";
	import ClockCounterClockwise from "phosphor-svelte/lib/ClockCounterClockwise";
	import ArrowLeft from "phosphor-svelte/lib/ArrowLeft";
	import Gear from "phosphor-svelte/lib/Gear";
	import Scroll from "phosphor-svelte/lib/Scroll";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { appState } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { backupConfigDirectory, getBuildInfo, openConfigDirectory, openLogDirectory, restoreConfigDirectory } from "$lib/api/commands";
	import { _ } from "$lib/i18n";
	import { message } from "@tauri-apps/plugin-dialog";
	import { notifyError } from "$lib/notifications";
	import { get } from "svelte/store";

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
	let activeTab = $state<"general" | "device" | "advance">("general");
	let buildInfo = $state("");
	(async () => { buildInfo = await getBuildInfo(); })();

	const t = (key: string, values?: Record<string, unknown>) => get(_)(key, { values });

	async function backupConfig() {
		await message(
			t("settings.dialogs.backup.message"),
			{ title: t("settings.dialogs.backup.title") },
		);
		if (await backupConfigDirectory()) {
			await message(t("settings.dialogs.backup.successMessage"), { title: t("settings.dialogs.backup.successTitle") });
		}
	}

	async function restoreConfig() {
		await message(
			t("settings.dialogs.restore.message"),
			{ title: t("settings.dialogs.restore.title") },
		);
		try {
			await restoreConfigDirectory();
		} catch (error: any) {
			notifyError(error);
		}
	}
</script>

{#if !fullPage}
	<button
		class="px-3 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
		onclick={() => {
			if (onOpen) onOpen();
			else showPopup = true;
		}}
	>
		{$_("settings.title")}
	</button>
{/if}

<svelte:window
	onkeydown={(event) => {
		if (event.key == "Escape") {
			if (fullPage) onBack();
			else showPopup = false;
		}
	}}
/>

{#snippet settingsContent()}
	{#if appState.settings}
		<div class="mx-2 mt-4 mb-4 border-b border-neutral-700">
			<div class="flex flex-row items-center gap-2">
				<button
					class="px-3 py-1.5 text-sm border-b-2 transition-colors {activeTab === 'general' ? 'text-neutral-200 border-blue-500' : 'text-neutral-400 border-transparent hover:text-neutral-200'}"
					onclick={() => { activeTab = "general"; }}
				>
					{$_("settings.tabs.general")}
				</button>
				<button
					class="px-3 py-1.5 text-sm border-b-2 transition-colors {activeTab === 'device' ? 'text-neutral-200 border-blue-500' : 'text-neutral-400 border-transparent hover:text-neutral-200'}"
					onclick={() => { activeTab = "device"; }}
				>
					{$_("settings.tabs.device")}
				</button>
				<button
					class="px-3 py-1.5 text-sm border-b-2 transition-colors {activeTab === 'advance' ? 'text-neutral-200 border-blue-500' : 'text-neutral-400 border-transparent hover:text-neutral-200'}"
					onclick={() => { activeTab = "advance"; }}
				>
					{$_("settings.tabs.advance")}
				</button>
			</div>
		</div>

		{#if activeTab === "general"}

			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-language" class="text-neutral-400">{$_("settings.general.language.label")}</label>
				<div class="select-wrapper">
					<select bind:value={appState.settings!.language} class="w-32" id="settings-language">
						<option value="en">{$_("settings.general.language.english")}</option>
						<option value="es">{$_("settings.general.language.spanish")}</option>
					</select>
				</div>
				{#snippet tooltipContent()}
					{$_("settings.general.language.tooltip")}
				{/snippet}
				<Tooltip>{@render tooltipContent()}</Tooltip>
			</div>

			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-background" class="text-neutral-400">{$_("settings.general.background.label")}</label>
				<input type="checkbox" bind:checked={appState.settings!.background} id="settings-background" />
				{#snippet tooltipBg()}{$_("settings.general.background.tooltip", { values: { product: PRODUCT_NAME } })}{/snippet}
				<Tooltip>{@render tooltipBg()}</Tooltip>
			</div>

			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-autolaunch" class="text-neutral-400">{$_("settings.general.autolaunch.label")}</label>
				<input type="checkbox" bind:checked={appState.settings!.autolaunch} id="settings-autolaunch" />
				{#snippet tooltipAuto()}
					{$_("settings.general.autolaunch.tooltip", { values: { product: PRODUCT_NAME } })}
					{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
						<br />
						{$_("settings.general.autolaunch.flatpak", { values: { product: PRODUCT_NAME } })}
					{/if}
				{/snippet}
				<Tooltip>{@render tooltipAuto()}</Tooltip>
			</div>
		{/if}

		{#if activeTab === "device"}

			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-sleep_timeout_minutes" class="text-neutral-400">{$_("settings.device.sleepTimeout.label")}</label>
				<input
					type="number"
					min="0"
					bind:value={appState.settings!.sleep_timeout_minutes}
					class="w-12 px-1 text-neutral-300 border border-neutral-600 rounded-lg"
					id="settings-sleep_timeout_minutes"
				/>
				<span class="text-neutral-400">{$_("common.minutes")}</span>
				{#snippet tooltipSleep()}{$_("settings.device.sleepTimeout.tooltip")}{/snippet}
				<Tooltip>{@render tooltipSleep()}</Tooltip>
			</div>

			<div class="flex flex-row items-center m-2 space-x-2">
				<label for="settings-rotation" class="text-neutral-400">{$_("settings.device.rotation.label")}</label>
				<div class="select-wrapper">
					<select bind:value={appState.settings!.rotation} id="settings-rotation">
						<option value={0}>0°</option>
						<option value={90}>90°</option>
						<option value={180}>180°</option>
						<option value={270}>270°</option>
					</select>
				</div>
			</div>
		{/if}

		{#if activeTab === "advance"}
			<div class="mt-2 space-y-0">
				<div class="flex flex-row items-center m-2 space-x-2">
					<label for="settings-developer" class="text-neutral-400">{$_("settings.advance.developer.label")}</label>
					<input type="checkbox" bind:checked={appState.settings!.developer} id="settings-developer" />
					{#snippet tooltipDev()}
						{$_("settings.advance.developer.tooltip")}
					{/snippet}
					<Tooltip>{@render tooltipDev()}</Tooltip>
				</div>

				<div class="flex flex-row items-center m-2 space-x-2">
					<label for="settings-disableelgato" class="text-neutral-400">{$_("settings.advance.disableElgato.label")}</label>
					<input type="checkbox" bind:checked={appState.settings!.disableelgato} id="settings-disableelgato" />
					{#snippet tooltipElgato()}{$_("settings.advance.disableElgato.tooltip")}{/snippet}
					<Tooltip>{@render tooltipElgato()}</Tooltip>
				</div>
			</div>
		{/if}

	{/if}

	<!-- ── Footer ───────────────────────────────────────────────── -->
	<div class="ml-2 mt-4">
		<div class="flex flex-row flex-wrap gap-2 my-3">
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				onclick={() => backupConfig()}
			>
				<ClockCounterClockwise class="mr-1" />
				{$_("settings.actions.backupConfig")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				onclick={() => restoreConfig()}
			>
				<ClockClockwise class="mr-1" />
				{$_("settings.actions.restoreConfig")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				onclick={() => openConfigDirectory()}
			>
				<Gear class="mr-1" />
				{$_("settings.actions.openConfig")}
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				onclick={() => openLogDirectory()}
			>
				<Scroll class="mr-1" />
				{$_("settings.actions.openLogs")}
			</button>
		</div>

		<span class="text-xs text-neutral-400">
			{@html buildInfo}
		</span>

		<div class="mt-3 text-xs text-neutral-500">
			<a
				href="https://github.com/a-orihuela/OpenDeck"
				class="underline hover:text-neutral-400 transition-colors"
				onclick={(e) => { e.preventDefault(); window.open("https://github.com/a-orihuela/OpenDeck"); }}
			>
				{$_("settings.actions.viewGithub")}
			</a>
		</div>
	</div>
{/snippet}

{#if fullPage}
	<div class="flex flex-col h-full min-h-0">
		<div class="flex items-center gap-2 px-3 py-2 border-b border-neutral-700 shrink-0">
			<button
				class="p-1 text-neutral-300 hover:text-white hover:bg-neutral-700 rounded-md transition-colors"
				onclick={() => onBack()}
				aria-label={$_("common.back")}
			>
				<ArrowLeft size="18" />
			</button>
			<h2 class="text-lg font-semibold text-neutral-300">{$_("settings.title")}</h2>
		</div>
		<div class="grow min-h-0 overflow-auto pb-6">
			{@render settingsContent()}
		</div>
	</div>
{:else}
	<Popup show={showPopup} label={$_("settings.title")}>
		{#snippet children()}
			<button class="mr-2 my-1 float-right text-xl text-neutral-300" onclick={() => { showPopup = false; }} aria-label={$_("common.close")}>✕</button>
			<h2 class="m-2 font-semibold text-xl text-neutral-300">{$_("settings.title")}</h2>
			{@render settingsContent()}
		{/snippet}
	</Popup>
{/if}
