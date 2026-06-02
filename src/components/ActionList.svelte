<script lang="ts">
	import type { Action } from "$lib/bindings";

	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";

	import { ACTION_FOLDER, ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE, BUILTIN_PLUGIN } from "$lib/constants";
	import { getWebserverUrl } from "$lib/ports";
	import { _ } from "$lib/i18n";
	import { appState } from "$lib/propertyInspector";
	

	import { getCategories } from "$lib/api/commands";

	

	const FOLDER_FORBIDDEN_ACTIONS = new Set([ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE, ACTION_FOLDER]);

	let categories: { [name: string]: { icon?: string | null; actions: Action[] } } = $state({});
	export async function reload() {
		categories = await getCategories();
	}
	reload();

	let query = $state("");

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
					actions = actions.filter((action) => action.name.toLowerCase().includes(lowerCaseQuery));
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
							<img
								src={!action.icon.startsWith("omegadeck/") ? getWebserverUrl(action.icon) : action.icon.replace("omegadeck", "")}
								alt=""
								class="w-full h-full object-contain rounded-md pointer-events-none"
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
