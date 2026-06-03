<script lang="ts">
	import type { Action } from "$lib/bindings";

	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
	import Check from "phosphor-svelte/lib/Check";
	import Copy from "phosphor-svelte/lib/Copy";
	import FilePlus from "phosphor-svelte/lib/FilePlus";
	import FloppyDisk from "phosphor-svelte/lib/FloppyDisk";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";

	import { ACTION_FOLDER, ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE, BUILTIN_PLUGIN } from "$lib/constants";
	import { getWebserverUrl } from "$lib/ports";
	import { _ } from "$lib/i18n";
	import { appState } from "$lib/propertyInspector";
	import { profileManager } from "$lib/singletons";
	import { notifyError, notify } from "$lib/notifications";
	import { get } from "svelte/store";
	

	import {
		applySheetTemplate,
		duplicateSheetTemplate,
		deleteSheetTemplate,
		insertSheetTemplateAsNewPage,
		listSheetTemplates,
		renameSheetTemplate,
		saveCurrentPageAsSheetTemplate,
		type SheetTemplateMeta,
	} from "$lib/api/commands";
	import { getCategories } from "$lib/api/commands";

	

	const FOLDER_FORBIDDEN_ACTIONS = new Set([ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE, ACTION_FOLDER]);
	let templates: SheetTemplateMeta[] = $state([]);
	let selectedTemplateId = $state("");
	let newTemplateName = $state("");
	let renamingTemplateId = $state<string | null>(null);
	let renameTemplateName = $state("");

	let categories: { [name: string]: { icon?: string | null; actions: Action[] } } = $state({});
	export async function reload() {
		categories = await getCategories();
	}
	reload();

	let query = $state("");

	async function reloadTemplates() {
		try {
			templates = await listSheetTemplates();
			if (templates.length === 0) {
				selectedTemplateId = "";
			} else if (!templates.some((v) => v.id === selectedTemplateId)) {
				selectedTemplateId = templates[0].id;
			}
		} catch (error: any) {
			notifyError(error, "warning");
		}
	}
	reloadTemplates();

	async function saveCurrentPageTemplate() {
		if (!appState.selectedDevice || !newTemplateName.trim()) return;
		try {
			await saveCurrentPageAsSheetTemplate(appState.selectedDevice, newTemplateName.trim());
			newTemplateName = "";
			await reloadTemplates();
			notify(t("sheets.saved"), "info");
		} catch (error: any) {
			notifyError(error);
		}
	}

	async function applyTemplateToCurrentPage() {
		if (!appState.selectedDevice || !selectedTemplateId) return;
		try {
			await applySheetTemplate(appState.selectedDevice, selectedTemplateId);
			if (appState.selectedProfile) {
				await get(profileManager)?.setProfile(appState.selectedProfile);
			}
			notify(t("sheets.applied"), "info");
		} catch (error: any) {
			notifyError(error);
		}
	}

	async function insertTemplateAsPage() {
		if (!appState.selectedDevice || !selectedTemplateId) return;
		try {
			await insertSheetTemplateAsNewPage(appState.selectedDevice, selectedTemplateId);
			if (appState.selectedProfile) {
				await get(profileManager)?.setProfile(appState.selectedProfile);
			}
			notify(t("sheets.inserted"), "info");
		} catch (error: any) {
			notifyError(error);
		}
	}

	async function removeTemplate(id: string) {
		if (!id) return;
		try {
			await deleteSheetTemplate(id);
			await reloadTemplates();
		} catch (error: any) {
			notifyError(error);
		}
	}

	async function duplicateTemplate(id: string) {
		if (!id) return;
		try {
			const created = await duplicateSheetTemplate(id);
			await reloadTemplates();
			selectedTemplateId = created.id;
			notify(t("sheets.duplicated"), "info");
		} catch (error: any) {
			notifyError(error);
		}
	}

	function startRenameTemplate(template: SheetTemplateMeta) {
		renamingTemplateId = template.id;
		renameTemplateName = template.name;
	}

	async function saveRenameTemplate(id: string) {
		if (!renameTemplateName.trim()) return;
		try {
			const renamed = await renameSheetTemplate(id, renameTemplateName.trim());
			renamingTemplateId = null;
			renameTemplateName = "";
			await reloadTemplates();
			selectedTemplateId = renamed.id;
			notify(t("sheets.renamed"), "info");
		} catch (error: any) {
			notifyError(error);
		}
	}

	const filteredCategories = $derived.by(() => {
		const lowerCaseQuery = query.toLowerCase().trim();
		return Object.entries(categories)
			.sort((a, b) => a[0].localeCompare(b[0]))
			.map(([categoryName, { icon, actions }]): [string, { icon?: string | null; actions: Action[] }] => {
				const displayCategoryName = localizeCategory(categoryName);
				if (appState.inFolderMode) {
					actions = actions.filter((action) => !FOLDER_FORBIDDEN_ACTIONS.has(action.uuid));
				}
				if (!displayCategoryName.toLowerCase().includes(lowerCaseQuery)) {
					actions = actions.filter((action) => actionLabel(action).toLowerCase().includes(lowerCaseQuery));
				}
				return [categoryName, { icon, actions }];
			})
			.filter(([_, { actions }]) => actions.length > 0);
	});

	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

	function localizeCategory(name: string): string {
		const normalized = name.startsWith("builtin.") ? name.slice("builtin.".length) : name.toLowerCase();
		const key = `builtinCategories.${normalized}`;
		const value = t(key);
		return value === key ? name : value;
	}

	function actionLabel(action: Action): string {
		if (action.plugin === BUILTIN_PLUGIN) {
			const key = `builtinActions.${action.uuid}.name`;
			const value = t(key);
			if (value !== key) return value;
		}
		return appState.localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name;
	}

	function actionTooltip(action: Action): string {
		if (action.plugin === BUILTIN_PLUGIN) {
			const key = `builtinActions.${action.uuid}.tooltip`;
			const value = t(key);
			if (value !== key) return value;
		}
		return appState.localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip;
	}

	function handleListKeydown(event: KeyboardEvent) {
		if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
		const list = event.currentTarget as HTMLElement;
		const items = Array.from(list.querySelectorAll("[role='option']"));
		const currentIndex = items.indexOf(event.target as Element);
		if (currentIndex == -1) return;

		event.preventDefault();

		let newIndex = currentIndex;
		switch (event.key) {
			case "ArrowDown":
			case "ArrowRight":
				newIndex = Math.min(currentIndex + 1, items.length - 1);
				break;
			case "ArrowUp":
			case "ArrowLeft":
				newIndex = Math.max(currentIndex - 1, 0);
				break;
			case "Home":
				newIndex = 0;
				break;
			case "End":
				newIndex = items.length - 1;
				break;
		}

		if (newIndex == currentIndex) return;
		(items[currentIndex] as HTMLElement).tabIndex = -1;
		(items[newIndex] as HTMLElement).tabIndex = 0;
		(items[newIndex] as HTMLElement).focus();
	}

	function handleListFocusin(event: FocusEvent) {
		const list = event.currentTarget as HTMLElement;
		const items = Array.from(list.querySelectorAll("[role='option']"));
		const index = items.indexOf(event.target as Element);
		if (index == -1) return;
		for (let i = 0; i < items.length; i++) {
			(items[i] as HTMLElement).tabIndex = i == index ? 0 : -1;
		}
	}
</script>

<div class="flex flex-col w-[18rem] h-full bg-neutral-900 border-r border-neutral-700">
	<div class="flex flex-row items-center m-2 bg-neutral-700 border border-neutral-600 rounded-lg">
		<MagnifyingGlass size="13" class="ml-2 mr-1 text-neutral-300" />
		<input
			bind:value={query}
			class="w-full p-1 text-sm text-neutral-300"
			placeholder={$_("actions.searchPlaceholder")}
			type="search"
			spellcheck="false"
		/>
	</div>

	<span id="action-list-hint" class="sr-only">{$_("actions.keyboardHint")}</span>
	<div class="grow overflow-auto select-none divide-y divide-neutral-800!">
		<details open>
			<summary class="pl-4 py-3 text-lg font-semibold text-neutral-300 hover:bg-neutral-800 transition-colors cursor-pointer">
				{$_("sheets.title")}
			</summary>
			<div class="px-3 pb-2 space-y-1.5">
				<input
					bind:value={newTemplateName}
					class="w-full px-2 py-1.5 text-xs text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg"
					placeholder={$_("sheets.namePlaceholder")}
					aria-label={$_("sheets.namePlaceholder")}
				/>
				<div class="grid grid-cols-3 gap-1.5">
					<button
						class="flex items-center justify-center px-2 py-1.5 text-neutral-300 bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded-lg"
						title={$_("sheets.saveCurrent")}
						aria-label={$_("sheets.saveCurrent")}
						onclick={saveCurrentPageTemplate}
						disabled={!appState.selectedDevice || !newTemplateName.trim()}
					>
						<FloppyDisk size={13} />
					</button>
					<button
						class="flex items-center justify-center px-2 py-1.5 text-neutral-300 bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded-lg"
						title={$_("sheets.applyCurrent")}
						aria-label={$_("sheets.applyCurrent")}
						onclick={applyTemplateToCurrentPage}
						disabled={!appState.selectedDevice || !selectedTemplateId}
					>
						<Check size={13} />
					</button>
					<button
						class="flex items-center justify-center px-2 py-1.5 text-neutral-300 bg-neutral-800 hover:bg-neutral-700 border border-neutral-600 rounded-lg"
						title={$_("sheets.insertAsNewPage")}
						aria-label={$_("sheets.insertAsNewPage")}
						onclick={insertTemplateAsPage}
						disabled={!appState.selectedDevice || !selectedTemplateId}
					>
						<FilePlus size={13} />
					</button>
				</div>
				<div class="max-h-48 overflow-auto space-y-1.5" role="list" aria-label={t("sheets.choose")}>
					{#if templates.length === 0}
						<p class="text-xs text-neutral-400">{$_("sheets.empty")}</p>
					{:else}
						{#each templates as template}
							<div class={`p-1.5 rounded-md border border-neutral-700 ${selectedTemplateId === template.id ? "bg-neutral-800/70" : ""}`}>
								<div class="flex items-start gap-2">
									<button
										class="min-w-0 grow text-left"
										onclick={() => { selectedTemplateId = template.id; }}
										aria-label={template.name}
									>
										<p class="text-xs text-neutral-200 truncate leading-tight">{template.name}</p>
										<p class="text-[10px] text-neutral-400 leading-tight">{template.rows}x{template.columns}</p>
									</button>
									{#if renamingTemplateId === template.id}
										<div class="flex items-center gap-1 shrink-0">
											<input
												bind:value={renameTemplateName}
												class="w-28 p-1 text-xs text-neutral-300 bg-neutral-700 border border-neutral-600 rounded"
												aria-label={t("sheets.rename")}
												onkeydown={(e) => { if (e.key === "Enter") saveRenameTemplate(template.id); }}
											/>
											<button class="p-1" onclick={() => saveRenameTemplate(template.id)} aria-label={t("common.save")}><FloppyDisk size={12} class="text-green-400" /></button>
											<button class="p-1 text-xs" onclick={() => { renamingTemplateId = null; }} aria-label={t("common.close")}>✕</button>
										</div>
									{:else}
										<div class="flex items-center gap-1 shrink-0">
											<button class="p-1" onclick={() => duplicateTemplate(template.id)} aria-label={t("sheets.duplicate")}><Copy size={12} class="text-neutral-300" /></button>
											<button class="p-1" onclick={() => startRenameTemplate(template)} aria-label={t("sheets.rename")}><Pencil size={12} class="text-neutral-300" /></button>
											<button class="p-1" onclick={() => removeTemplate(template.id)} aria-label={t("sheets.delete")}><Trash size={12} class="text-red-300" /></button>
										</div>
									{/if}
								</div>
							</div>
						{/each}
					{/if}
				</div>
			</div>
		</details>

		{#each filteredCategories as [name, { icon, actions }]}
			<details open>
				<summary class="pl-4 py-3 text-lg font-semibold text-neutral-300 hover:bg-neutral-800 transition-colors cursor-pointer">
					{#if icon}
						<img
							src={!icon.startsWith("omegadeck/") ? getWebserverUrl(icon) : icon.replace("omegadeck", "")}
							alt={localizeCategory(name)}
							class="w-5 h-5 rounded-xs ml-1 -mt-1 inline"
						/>
					{/if}
					<span class="ml-1">{localizeCategory(name)}</span>
				</summary>
				<div
					class="grid grid-cols-3 gap-1.5 px-3 py-2"
					role="listbox"
					aria-label={localizeCategory(name)}
					aria-describedby="action-list-hint"
					tabindex="-1"
					onkeydown={handleListKeydown}
					onfocusin={handleListFocusin}
				>
					{#each actions as action, i}
						<div
							class="group relative flex items-center justify-center aspect-square p-1.5 bg-neutral-950 hover:bg-neutral-900 transition-colors rounded-lg border border-neutral-800 cursor-grab active:cursor-grabbing"
							draggable="true"
							title={actionTooltip(action)}
							role="option"
							aria-selected="false"
							tabindex={i == 0 ? 0 : -1}
							aria-label={actionLabel(action)}
							ondragstart={(event) => {
								if (!event.dataTransfer) return;
								event.dataTransfer.effectAllowed = "copy";
								event.dataTransfer.setData("action", JSON.stringify(action));
							}}
							onkeydown={(event) => {
								if ((event.ctrlKey || event.metaKey) && event.key == "c") {
									appState.copiedItem = { type: "action", action };
								}
							}}
						>
							<div class="absolute inset-2 rounded-md bg-neutral-900/70 border border-neutral-700/60 pointer-events-none"></div>
							<img
								src={!action.icon.startsWith("omegadeck/") ? getWebserverUrl(action.icon) : action.icon.replace("omegadeck", "")}
								alt=""
								class="relative z-10 w-[88%] h-[88%] object-contain rounded-md pointer-events-none"
							/>
							<div class="absolute inset-x-0 bottom-0 rounded-b-lg bg-neutral-900/95 px-1 py-0.5 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
								<span class="block text-xs text-center text-neutral-200 truncate leading-tight">
									{actionLabel(action)}
								</span>
							</div>
						</div>
					{/each}
				</div>
			</details>
		{/each}
	</div>
</div>
