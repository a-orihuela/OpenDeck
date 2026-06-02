<script lang="ts">
	import type { DeviceInfo, Profile } from "$lib/bindings";

	import Browsers from "phosphor-svelte/lib/Browsers";
	import Copy from "phosphor-svelte/lib/Copy";
	import FloppyDisk from "phosphor-svelte/lib/FloppyDisk";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import Popup from "./Popup.svelte";

	import { appState } from "$lib/propertyInspector";
	import { formatError, notifyError } from "$lib/notifications";

	import {
		deleteProfile as apiDeleteProfile,
		getApplicationProfiles,
		getApplications,
		getProfiles as apiGetProfiles,
		getSelectedProfile as apiGetSelectedProfile,
		renameProfile as apiRenameProfile,
		setApplicationProfiles,
		setSelectedProfile as apiSetSelectedProfile,
	} from "$lib/api/commands";
	import { onApplications, onRerenderImages } from "$lib/api/events";
	import {
		flatProfileList,
		generateDuplicateName,
		organizeIntoFolders,
		removeFolderEntry,
		updateFoldersAfterRename,
	} from "$lib/services/profileService";
	import { _ } from "$lib/i18n";
	import { message } from "@tauri-apps/plugin-dialog";
	import { get } from "svelte/store";
	import { untrack } from "svelte";

	let { device = $bindable(), profile = $bindable() }: { device: DeviceInfo; profile: Profile } = $props();

	let folders: { [name: string]: string[] } = $state({});
	let value = $state("");
	let oldValue = $state("");

	const t = (key: string, values?: Record<string, unknown>) => get(_)(key, { values });

	async function getProfiles(dev: DeviceInfo) {
		try {
			const profiles = await apiGetProfiles(dev.id);
			folders = organizeIntoFolders(profiles);
			if (Object.keys(folders).length === 0) {
				folders = { "": ["Default"] };
			}
			profile = await apiGetSelectedProfile(dev.id);
			value = profile?.id || profiles[0] || "Default";
			oldValue = value;
		} catch (error: any) {
			notifyError(error, "warning");
			folders = { "": ["Default"] };
			value = "Default";
			oldValue = "Default";
		}
	}

	$effect(() => {
		if (device?.id) {
			getProfiles(device);
		}
	});

	export async function setProfile(id: string) {
		if (!device || !id) return;
		if (value != id) {
			value = id;
			return;
		}
		await apiSetSelectedProfile(device.id, id);
		profile = await apiGetSelectedProfile(device.id);

		let folder = id.includes("/") ? id.split("/")[0] : "";
		if (folders[folder]) {
			if (!folders[folder].includes(id)) folders[folder].push(id);
		} else folders[folder] = [id];
		folders = { ...folders };

		appState.inspectedInstance = null;
	}

	onRerenderImages(async () => {
		try {
			profile = await apiGetSelectedProfile(device.id);
		} catch (e: any) {
			notifyError(e, "warning");
		}
	});

	async function deleteProfile(id: string) {
		for (const devices of Object.values(applicationProfiles)) {
			if (devices[device.id] == id) {
				delete devices[device.id];
				applicationProfiles = { ...applicationProfiles };
			}
		}
		await apiDeleteProfile(device.id, id);
		folders = removeFolderEntry(folders, id);
	}

	let renamingProfile: string | null = $state(null);
	let renameInput: HTMLInputElement | undefined = $state(undefined);
	let newId = $state("");

	$effect(() => {
		if (renameInput) renameInput.focus();
	});

	async function saveRenamedProfile(oldId: string) {
		if (!renameInput?.checkValidity() || !newId) return;
		if (newId == oldId) { renamingProfile = null; return; }

		if (flatProfileList(folders).includes(newId)) {
			message(t("profiles.dialogs.duplicateId", { id: newId }), { title: t("profiles.dialogs.renameFailed") });
			return;
		}

		try {
			await apiRenameProfile(device.id, oldId, newId, false);
		} catch (error: any) {
			message(formatError(error), { title: t("profiles.dialogs.renameFailed") });
			console.error(error);
			return;
		}

		for (const devices of Object.values(applicationProfiles)) {
			if (devices[device.id] == oldId) devices[device.id] = newId;
		}
		applicationProfiles = { ...applicationProfiles };
		folders = updateFoldersAfterRename(folders, oldId, newId);
		renamingProfile = null;
	}

	async function duplicateProfile(id: string) {
		const dupId = generateDuplicateName(id, flatProfileList(folders));
		await apiRenameProfile(device.id, id, dupId, true);
		await getProfiles(device);
	}

	$effect(() => {
		const v = value;
		if (v == "omegadeck_edit_profiles") {
			const prev = untrack(() => oldValue);
			showPopup = true;
			untrack(() => { value = prev || profile?.id || "Default"; });
		} else {
			const prev = untrack(() => oldValue);
			const p = untrack(() => profile);
			if (v && v != prev && (!p || p.id != v)) {
				setProfile(v);
				untrack(() => { oldValue = v; });
			}
		}
	});

	let showPopup = $state(false);
	let nameInput: HTMLInputElement | undefined = $state(undefined);

	let showApplicationManager = $state(false);
	let applications: string[] = $state([]);
	let applicationProfiles: { [appName: string]: { [device: string]: string } } = $state({});
	(async () => {
		applications = await getApplications();
		applicationProfiles = await getApplicationProfiles();
	})();
	onApplications((apps) => { applications = apps; });

	let applicationsAddAppName = $state("omegadeck_select_application");
	let applicationsAddProfile = $state("omegadeck_select_profile");

	$effect(() => {
		const appName = applicationsAddAppName;
		const profId = applicationsAddProfile;
		if (appName != "omegadeck_select_application" && profId != "omegadeck_select_profile") {
			untrack(() => {
				applicationProfiles[appName] ||= {};
				applicationProfiles[appName][device.id] = profId;
				applicationsAddAppName = "omegadeck_select_application";
				applicationsAddProfile = "omegadeck_select_profile";
			});
		}
	});

	$effect(() => {
		const profs = applicationProfiles;
		if (profs && Object.keys(profs).length) {
			const filtered = Object.fromEntries(
				Object.entries(profs).filter(([_, devices]) => Object.values(devices).filter((v) => v).length != 0)
			);
			untrack(() => {
				applicationProfiles = filtered;
				setApplicationProfiles(filtered);
			});
		}
	});

	let measure: HTMLSpanElement | undefined = $state(undefined);
	let selectWidth = $state(0);
	$effect(() => {
		if (value && measure) {
			measure.textContent = value.includes("/") ? value.split("/")[1] : value;
			selectWidth = measure.offsetWidth + 18;
		}
	});
</script>

<div class="select-profile-wrapper">
	<span bind:this={measure} class="invisible fixed whitespace-pre pointer-events-none" aria-hidden="true"></span>
	<select bind:value style:width="{selectWidth}px" aria-label={$_("profiles.aria.profile")}>
		{#if flatProfileList(folders).length === 0}
			<option value="Default">{$_("profiles.defaultProfile")}</option>
		{/if}
		{#each Object.entries(folders).sort() as [id, profiles]}
			{#if id && profiles.length}
				<optgroup label={id}>
					{#each [...profiles].sort() as prof}
						<option value={prof}>{prof.split("/")[1]}</option>
					{/each}
				</optgroup>
			{:else}
				{#each [...profiles].sort() as prof}
					<option value={prof}>{prof}</option>
				{/each}
			{/if}
		{/each}
		<option value="omegadeck_edit_profiles">{$_("profiles.editOption")}</option>
	</select>
</div>

<svelte:window
	onkeydown={(event) => {
		if (event.key == "Escape") {
			if (showApplicationManager) showApplicationManager = false;
			else if (renamingProfile) renamingProfile = null;
			else showPopup = false;
		}
	}}
/>

<Popup show={showPopup} label={$_("profiles.title", { values: { device: device.name } })}>
	{#snippet children()}
	<button class="mr-1 float-right text-xl text-neutral-300" onclick={() => { showPopup = false; }} aria-label={$_("common.close")}>✕</button>
	<h2 class="text-xl font-semibold text-neutral-300">{device.name}</h2>

	<div class="flex flex-row mt-2 mb-1">
		<input
			bind:this={nameInput}
			pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
			class="grow p-2 text-neutral-300 invalid:text-red-400 bg-neutral-700 border-l border-y border-neutral-600 rounded-l-lg"
			placeholder={$_("profiles.namePlaceholder")}
			aria-label={$_("profiles.nameAria")}
		/>
		<button
			onclick={async () => {
				if (!nameInput?.checkValidity() || !nameInput.value) return;
				await setProfile(nameInput.value);
				value = nameInput.value;
				nameInput.value = "";
				showPopup = false;
			}}
			class="px-4 text-neutral-300 bg-neutral-900 hover:bg-neutral-800 transition-colors border-r border-y border-neutral-600 rounded-r-lg"
		>
			{$_("common.create")}
		</button>
		<button
			class="ml-2 px-4 flex items-center text-neutral-300 bg-neutral-900 hover:bg-neutral-800 transition-colors border border-neutral-600 rounded-lg"
			onclick={() => { showApplicationManager = true; }}
			aria-label={$_("profiles.aria.applicationProfiles")}
		>
			<Browsers size={24} />
		</button>
	</div>

	<div class="divide-y divide-neutral-500!">
		{#each Object.entries(folders).sort() as [id, profiles]}
			{#if id && profiles.length}
				<h4 class="py-2 font-bold text-lg text-neutral-300">{id}</h4>
			{/if}
			{#each [...profiles].sort() as prof}
				<div class="flex flex-row items-center py-2 space-x-2" class:ml-6={id} class:pl-2={id}>
					<input type="radio" bind:group={value} value={prof} disabled={renamingProfile == prof} id={`profile-${encodeURIComponent(prof)}`} aria-label={id ? prof.split("/")[1] : prof} />
					{#if prof == renamingProfile}
						<input
							bind:this={renameInput}
							bind:value={newId}
							pattern="[a-zA-Z0-9_ ]+(\/[a-zA-Z0-9_ ]+)?"
							class="grow px-2 py-1 text-neutral-300 invalid:text-red-400 bg-neutral-700 rounded"
							placeholder={$_("profiles.namePlaceholder")}
							onkeydown={(e) => {
								if (e.key === "Enter") saveRenamedProfile(prof);
							}}
						/>
						<button onclick={() => saveRenamedProfile(prof)} title={$_("profiles.actions.save")} aria-label={$_("profiles.actions.save")}>
							<FloppyDisk size="20" class="text-green-500" />
						</button>
					{:else}
						<label class="grow text-neutral-400" for={`profile-${encodeURIComponent(prof)}`}>{id ? prof.split("/")[1] : prof}</label>
						<button onclick={() => duplicateProfile(prof)} title={$_("profiles.actions.duplicate")} aria-label={$_("profiles.actions.duplicate")}>
							<Copy size="20" class="text-neutral-400" />
						</button>
						{#if prof != value}
							<button onclick={() => { renamingProfile = newId = prof; }} title={$_("profiles.actions.rename")} aria-label={$_("profiles.actions.rename")}>
								<Pencil size="20" class="text-neutral-400" />
							</button>
							<button onclick={() => deleteProfile(prof)} title={$_("profiles.actions.delete")} aria-label={$_("profiles.actions.delete")}>
								<Trash size="20" class="text-neutral-400" />
							</button>
						{/if}
					{/if}
				</div>
			{/each}
		{/each}
	</div>
	{/snippet}
</Popup>

<Popup show={showApplicationManager} label={$_("profiles.applicationProfiles")}>
	{#snippet children()}
	<button class="mr-1 float-right text-xl text-neutral-300" onclick={() => { showApplicationManager = false; }} aria-label={$_("common.close")}>✕</button>
	<h2 class="text-xl font-semibold text-neutral-300">{device.name}</h2>
	<span class="text-sm text-neutral-400">{$_("profiles.help.missingApp")}</span>
	<span class="text-sm text-neutral-400">{$_("profiles.help.defaultBehavior")}</span>

	<table class="w-full text-neutral-300 divide-y divide-neutral-500!">
		<tbody>
		{#each Object.entries(applicationProfiles).sort((a, b) => a[0] == "omegadeck_default" ? -1 : b[0] == "omegadeck_default" ? 1 : a[0].localeCompare(b[0])) as [appName, devices]}
			{#if devices[device.id]}
				<tr class="h-12">
					<td>{appName == "omegadeck_default" ? $_("profiles.defaultProfile") : appName}:</td>
					<td class="select-wrapper">
						<select bind:value={applicationProfiles[appName][device.id]} class="w-full" aria-label={appName == "omegadeck_default" ? $_("profiles.aria.defaultProfileMapping") : $_("profiles.aria.applicationProfile", { values: { app: appName } })}>
							{#each Object.entries(folders) as [id, profiles]}
								{#if id && profiles.length}
									<optgroup label={id}>
										{#each profiles as prof}
											<option value={prof}>{prof.split("/")[1]}</option>
										{/each}
									</optgroup>
								{:else}
									{#each profiles as prof}
										<option value={prof}>{prof}</option>
									{/each}
								{/if}
							{/each}
							<option disabled>──────────</option>
							<option value={undefined}>{$_("profiles.removeApplication")}</option>
						</select>
					</td>
				</tr>
			{/if}
		{/each}
		<tr class="h-12">
			<td class="w-48 select-wrapper">
				<select bind:value={applicationsAddAppName} class="w-full" aria-label={$_("profiles.selectApplication")}>
					<option selected disabled value="omegadeck_select_application">{$_("profiles.selectApplicationOption")}</option>
					{#if !applicationProfiles["omegadeck_default"] || !applicationProfiles["omegadeck_default"][device.id]}
						<option value="omegadeck_default">{$_("profiles.defaultProfile")}</option>
						{#if applications.filter((appName) => !applicationProfiles[appName] || !applicationProfiles[appName][device.id]).length > 0}
							<option disabled>──────────</option>
						{/if}
					{/if}
					{#each applications as appName}
						{#if !applicationProfiles[appName] || !applicationProfiles[appName][device.id]}
							<option value={appName}>{appName}</option>
						{/if}
					{/each}
				</select>
			</td>
			<td class="w-96 select-wrapper">
				<select bind:value={applicationsAddProfile} class="w-full" aria-label={$_("profiles.selectProfile")}>
					<option selected disabled value="omegadeck_select_profile">{$_("profiles.selectProfileOption")}</option>
					{#each Object.entries(folders) as [id, profiles]}
						{#if id && profiles.length}
							<optgroup label={id}>
								{#each profiles as prof}
									<option value={prof}>{prof.split("/")[1]}</option>
								{/each}
							</optgroup>
						{:else}
							{#each profiles as prof}
								<option value={prof}>{prof}</option>
							{/each}
						{/if}
					{/each}
				</select>
			</td>
		</tr>
		</tbody>
	</table>
	{/snippet}
</Popup>
