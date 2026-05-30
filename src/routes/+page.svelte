<script lang="ts">
	import type { DeviceInfo, Profile } from "$lib/bindings";

	import { initPortBase } from "$lib/ports";
	import { appState } from "$lib/propertyInspector";
	import { actionList, deviceSelector, profileManager } from "$lib/singletons";

	import ActionList from "../components/ActionList.svelte";
	import DeviceSelector from "../components/DeviceSelector.svelte";
	import DeviceView from "../components/DeviceView.svelte";
	import NoDevicesDetected from "../components/NoDevicesDetected.svelte";
	import ParentActionView from "../components/ParentActionView.svelte";
	import PluginManager from "../components/PluginManager.svelte";
	import ProfileManager from "../components/ProfileManager.svelte";
	import PropertyInspectorView from "../components/PropertyInspectorView.svelte";
	import SettingsView from "../components/SettingsView.svelte";

	let devices = $state<{ [id: string]: DeviceInfo }>({});
	let selectedDevice = $state("");
	let selectedProfiles = $state<{ [id: string]: Profile }>({});

	let actionListRef: ActionList | null = $state(null);
	let deviceSelectorRef: DeviceSelector | null = $state(null);
	let profileManagerRef: ProfileManager | null = $state(null);

	$effect(() => { actionList.set(actionListRef); });
	$effect(() => { deviceSelector.set(deviceSelectorRef); });
	$effect(() => { profileManager.set(profileManagerRef); });

	initPortBase();
</script>

<svelte:window ondragover={(event) => event.preventDefault()} ondrop={(event) => event.preventDefault()} />

<div class="flex flex-row h-screen">
	<!-- Left sidebar: ActionList -->
	<ActionList bind:this={actionListRef} />

	<!-- Main column -->
	<div class="flex flex-col grow min-w-0">

		<!-- Navbar -->
		<nav class="flex flex-row items-center px-3 py-2 border-b border-neutral-700 shrink-0">
			<DeviceSelector
				bind:devices
				bind:value={selectedDevice}
				bind:selectedProfiles
				bind:this={deviceSelectorRef}
			/>
			{#key selectedDevice}
				{#if selectedDevice && devices[selectedDevice]}
					<div class="ml-4">
						<ProfileManager
							bind:device={devices[selectedDevice]}
							bind:profile={selectedProfiles[selectedDevice]}
							bind:this={profileManagerRef}
						/>
					</div>
				{/if}
			{/key}
			<div class="ml-auto flex flex-row items-center gap-2">
				<PluginManager />
				<SettingsView />
			</div>
		</nav>

		<!-- Content area -->
		{#if Object.keys(devices).length > 0 && selectedProfiles}
			<div class="flex flex-col grow overflow-hidden">

				<!-- Device panel -->
				<div class="overflow-auto flex-1 min-h-0">
					{#if appState.inspectedParentAction}
						<ParentActionView bind:profile={selectedProfiles[selectedDevice]} />
					{:else}
						{#each Object.keys(devices) as id}
							{#if devices[id] && selectedProfiles[id]}
								<DeviceView bind:device={devices[id]} bind:profile={selectedProfiles[id]} bind:selectedDevice />
							{/if}
						{/each}
					{/if}
				</div>

				<!-- Property inspector panel -->
				{#if selectedProfiles[selectedDevice]}
					<div class="border-t border-neutral-700" style="flex: 0 0 16rem; max-height: 40%">
						<PropertyInspectorView device={devices[selectedDevice]} profile={selectedProfiles[selectedDevice]} />
					</div>
				{/if}
			</div>
		{:else}
			<NoDevicesDetected />
		{/if}

	</div>
</div>
