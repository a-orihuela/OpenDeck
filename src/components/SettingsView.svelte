<script lang="ts">
	import ClockClockwise from "phosphor-svelte/lib/ClockClockwise";
	import ClockCounterClockwise from "phosphor-svelte/lib/ClockCounterClockwise";
	import Gear from "phosphor-svelte/lib/Gear";
	import Scroll from "phosphor-svelte/lib/Scroll";
	import Popup from "./Popup.svelte";
	import Tooltip from "./Tooltip.svelte";

	import { settings } from "$lib/settings";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { backupConfigDirectory, getBuildInfo, openConfigDirectory, openLogDirectory, restoreConfigDirectory } from "$lib/api/commands";
	import { onDeviceBrightness } from "$lib/api/events";
	import { message } from "@tauri-apps/plugin-dialog";
	import { notify } from "$lib/notifications";

	let showPopup: boolean;
	let buildInfo: string;
	(async () => buildInfo = await getBuildInfo())();

	onDeviceBrightness((action, amount) => {
		if (!$settings) return;
		let brightness = $settings.brightness;
		switch (action) {
			case "increase": brightness += amount; break;
			case "decrease": brightness -= amount; break;
			default:         brightness  = amount; break;
		}
		$settings.brightness = Math.max(0, Math.min(100, brightness));
	});

	async function backupConfig() {
		await message(
			"You will be prompted to select a location to save the backup to. The config directory will be compressed and saved there. This may take a while if you have many plugins or profiles.",
			{ title: "Backing up configuration" },
		);
		if (await backupConfigDirectory()) {
			await message("Successfully backed up the config directory.", { title: "Backup complete" });
		}
	}

	async function restoreConfig() {
		await message(
			"You will be prompted to select a location to restore the backup from. This may take a while if you have many plugins or profiles. The application will restart after the restoration is complete.\n\nYou may encounter issues if you attempt to restore a backup from a different operating system or architecture.",
			{ title: "Restoring configuration" },
		);
		try {
			await restoreConfigDirectory();
		} catch (error: any) {
			notify(String(error));
		}
	}
</script>

<button
	class="px-3 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
	on:click={() => showPopup = true}
>
	Settings
</button>

<svelte:window
	on:keydown={(event) => {
		if (event.key == "Escape") showPopup = false;
	}}
/>

<Popup show={showPopup} label="Settings">
	<button class="mr-2 my-1 float-right text-xl text-neutral-300" on:click={() => showPopup = false} aria-label="Close">✕</button>
	<h2 class="m-2 font-semibold text-xl text-neutral-300">Settings</h2>

	{#if $settings}

		<!-- ── General ──────────────────────────────────────────── -->
		<h3 class="mx-2 mt-4 mb-1 text-xs font-semibold uppercase tracking-wider text-neutral-500">General</h3>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-language" class="text-neutral-400">Language:</label>
			<div class="select-wrapper">
				<select bind:value={$settings.language} class="w-32" id="settings-language">
					<option value="en">English</option>
					<option value="es">Español</option>
					<option value="zh_CN">中文</option>
					<option value="fr">Français</option>
					<option value="de">Deutsch</option>
					<option value="ja">日本語</option>
					<option value="ko">韓国語</option>
				</select>
			</div>
			<Tooltip>
				{PRODUCT_NAME} itself is not yet translated. Changing this setting will translate the text from installed plugins into your language for those that support it.
			</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-background" class="text-neutral-400">Run in background:</label>
			<input type="checkbox" bind:checked={$settings.background} id="settings-background" />
			<Tooltip>If this option is enabled, {PRODUCT_NAME} will minimise to the tray and run in the background.</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-autolaunch" class="text-neutral-400">Start at login:</label>
			<input type="checkbox" bind:checked={$settings.autolaunch} id="settings-autolaunch" />
			<Tooltip>
				If this option is enabled, {PRODUCT_NAME} will automatically start at login.
				{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
					<br />
					If you used Flatpak to install {PRODUCT_NAME}, this option may not function as intended.
				{/if}
			</Tooltip>
		</div>

		<!-- ── Device ───────────────────────────────────────────── -->
		<h3 class="mx-2 mt-4 mb-1 text-xs font-semibold uppercase tracking-wider text-neutral-500">Device</h3>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-brightness" class="text-neutral-400">Brightness:</label>
			<input type="range" min="0" max="100" bind:value={$settings.brightness} id="settings-brightness" />
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-sleep_timeout_minutes" class="text-neutral-400">Sleep after inactivity:</label>
			<input
				type="number"
				min="0"
				bind:value={$settings.sleep_timeout_minutes}
				class="w-12 px-1 text-neutral-300 border border-neutral-600 rounded-lg"
				id="settings-sleep_timeout_minutes"
			/>
			<span class="text-neutral-400">minutes</span>
			<Tooltip>How many minutes of inactivity will cause devices to enter sleep mode. Set to 0 to disable auto-sleep.</Tooltip>
		</div>

		<div class="flex flex-row items-center m-2 space-x-2">
			<label for="settings-rotation" class="text-neutral-400">Image rotation:</label>
			<div class="select-wrapper">
				<select bind:value={$settings.rotation} id="settings-rotation">
					<option value={0}>0°</option>
					<option value={90}>90°</option>
					<option value={180}>180°</option>
					<option value={270}>270°</option>
				</select>
			</div>
		</div>

		<!-- ── Advanced (collapsible) ────────────────────────────── -->
		<details class="mx-2 mt-4 group">
			<summary class="cursor-pointer text-xs font-semibold uppercase tracking-wider text-neutral-500 select-none list-none flex items-center gap-1">
				<span class="transition-transform group-open:rotate-90">▶</span>
				Advanced
			</summary>

			<div class="mt-2 space-y-0">
				<div class="flex flex-row items-center m-2 space-x-2">
					<label for="settings-developer" class="text-neutral-400">Developer mode:</label>
					<input type="checkbox" bind:checked={$settings.developer} id="settings-developer" />
					<Tooltip>
						Enables features that make plugin development and debugging easier. Also exposes all file paths on your device on the local webserver to allow symbolic linking of plugins — disable when not in use.
					</Tooltip>
				</div>

				<div class="flex flex-row items-center m-2 space-x-2">
					<label for="settings-disableelgato" class="text-neutral-400">Disable Elgato device discovery:</label>
					<input type="checkbox" bind:checked={$settings.disableelgato} id="settings-disableelgato" />
					<Tooltip>Disables discovery of Elgato devices so that they can be managed by other software.</Tooltip>
				</div>
			</div>
		</details>

	{/if}

	<!-- ── Footer ───────────────────────────────────────────────── -->
	<div class="ml-2 mt-4">
		<div class="flex flex-row flex-wrap gap-2 my-3">
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => backupConfig()}
			>
				<ClockCounterClockwise class="mr-1" />
				Back up config
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => restoreConfig()}
			>
				<ClockClockwise class="mr-1" />
				Restore config
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => openConfigDirectory()}
			>
				<Gear class="mr-1" />
				Open config
			</button>
			<button
				class="flex flex-row items-center px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
				on:click={() => openLogDirectory()}
			>
				<Scroll class="mr-1" />
				Open logs
			</button>
		</div>

		<span class="text-xs text-neutral-400">
			{@html buildInfo}
		</span>

		<div class="mt-3 text-xs text-neutral-500">
			<a
				href="https://github.com/a-orihuela/OpenDeck"
				class="underline hover:text-neutral-400 transition-colors"
				on:click|preventDefault={() => { /* opened via openUrl in shims */ window.open("https://github.com/a-orihuela/OpenDeck"); }}
			>
				View on GitHub
			</a>
		</div>
	</div>
</Popup>
