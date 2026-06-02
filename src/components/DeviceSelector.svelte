<script lang="ts">
	import type { DeviceInfo, Profile } from "$lib/bindings";

	import { profileManager } from "$lib/singletons";
	import { _ } from "$lib/i18n";
	import { get } from "svelte/store";
	import { untrack } from "svelte";

	import { getBuildInfo, getDevices, getSelectedProfile, setSelectedProfile } from "$lib/api/commands";
	import { onDevices, onSwitchProfile } from "$lib/api/events";

	let { devices = $bindable({}), value = $bindable(""), selectedProfiles = $bindable({}) }: {
		devices?: { [id: string]: DeviceInfo };
		value?: string;
		selectedProfiles?: { [id: string]: Profile };
	} = $props();

	let registered: string[] = $state([]);
	let buildInfo = $state("");
	let measure: HTMLSpanElement | undefined = $state(undefined);
	let selectWidth = $state(0);
	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

	$effect(() => {
		const keys = Object.keys(devices).sort();
		if ((!value || !devices[value]) && keys.length > 0) value = keys[0];
		for (const [id, device] of Object.entries(devices)) {
			const isRegistered = untrack(() => registered.includes(id));
			if (!isRegistered) {
				untrack(() => { registered = [...registered, id]; });
				(async () => {
					const profile: Profile = await getSelectedProfile(device.id);
					// Reassign entire object so parent $state tracks the change
					selectedProfiles = { ...untrack(() => selectedProfiles), [id]: profile };
					await setSelectedProfile(id, profile.id);
				})();
			}
		}
	});

	export function reloadProfiles() {
		registered = [];
	}

	onSwitchProfile(async (device, profile) => {
		if (device == value) {
			get(profileManager)?.setProfile(profile);
		} else {
			await setSelectedProfile(device, profile);
			const reloaded = await getSelectedProfile(device);
		selectedProfiles = { ...selectedProfiles, [device]: reloaded };
		}
	});

	(async () => { devices = await getDevices(); })();
	onDevices((payload) => { devices = payload; });

	(async () => { buildInfo = await getBuildInfo(); })();
	$effect(() => {
		if (devices[value]) {
			const effectiveCols = Math.min(Math.max(devices[value].columns, devices[value].encoders, devices[value].touchpoints), 8);
			const effectiveRows = Math.min(devices[value].rows + Math.min(devices[value].encoders, 1) + Math.min(devices[value].touchpoints, 1), 4);
			const idealWidth = (effectiveCols * 132) + 416;
			const idealHeight = (effectiveRows * 132) + 384 + (buildInfo?.split("</summary>")[0]?.includes("darwin") ? 28 : 0);
			(async () => {
				const [{ getCurrentWindow }, { LogicalSize }] = await Promise.all([
					import("@tauri-apps/api/window"),
					import("@tauri-apps/api/dpi"),
				]);
				const tauriWindow = getCurrentWindow();
				const width = Math.min(idealWidth, screen.availWidth);
				const height = Math.min(idealHeight, screen.availHeight);
				await tauriWindow.setMinSize(new LogicalSize(width, height));
				await tauriWindow.setSize(new LogicalSize(width, height));
			})();
		}
	});

	$effect(() => {
		if (value && measure && devices[value]) {
			measure.textContent = devices[value].name;
			selectWidth = measure.offsetWidth + 20;
		}
	});
</script>

{#if Object.keys(devices).length > 0}
	<div class="select-device-wrapper">
		<span bind:this={measure} class="invisible fixed whitespace-pre pointer-events-none text-xl font-semibold" aria-hidden="true"></span>
		<select bind:value style:width="{selectWidth}px" aria-label={t("deviceSelector.deviceAria")}>
			<option value="" disabled selected>{t("deviceSelector.chooseDevice")}</option>

			{#each Object.entries(devices).sort() as [id, device]}
				<option value={id}>{device.name}</option>
			{/each}
		</select>
	</div>
{/if}
