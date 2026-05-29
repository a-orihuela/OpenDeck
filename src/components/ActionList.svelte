<script lang="ts">
	import type { Action } from "$lib/bindings";

	import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";

	import { ACTION_FOLDER, ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE } from "$lib/constants";
	import { getWebserverUrl } from "$lib/ports";
	import { copiedItem } from "$lib/propertyInspector";
	import { localisations } from "$lib/settings";

	import { getCategories } from "$lib/api/commands";

	import { inFolderMode } from "$lib/singletons";

	const FOLDER_FORBIDDEN_ACTIONS = new Set([ACTION_NEXTPAGE, ACTION_PREVIOUSPAGE, ACTION_FOLDER]);

	let categories: { [name: string]: { icon?: string; actions: Action[] } } = {};
	export async function reload() {
		categories = await getCategories();
	}
	reload();

	let query: string = "";
	let filteredCategories: [string, { icon?: string; actions: Action[] }][] = [];
	$: {
		let lowerCaseQuery = query.toLowerCase().trim();
		filteredCategories = Object.entries(categories)
			.sort((a, b) => a[0].localeCompare(b[0]))
			.map(([categoryName, { icon, actions }]): [string, { icon?: string; actions: Action[] }] => {
				if ($inFolderMode) {
					actions = actions.filter((action) => !FOLDER_FORBIDDEN_ACTIONS.has(action.uuid));
				}
				if (!categoryName.toLowerCase().includes(lowerCaseQuery)) {
					actions = actions.filter((action) => action.name.toLowerCase().includes(lowerCaseQuery));
				}
				return [categoryName, { icon, actions }];
			})
			.filter(([_, { actions }]) => actions.length > 0);
	}

	function handleListKeydown(event: KeyboardEvent) {
		if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
		const list = event.currentTarget as HTMLElement;
		const items = Array.from(list.querySelectorAll("[role='option']"));
		const currentIndex = items.indexOf(event.target as Element);
		if (currentIndex == -1) return;

		event.preventDefault();

		let newIndex = currentIndex;
		switch (event.key) {
			case "ArrowDown":
				newIndex = Math.min(currentIndex + 1, items.length - 1);
				break;
			case "ArrowUp":
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

<div class="flex flex-col w-[18rem] h-full bg-neutral-900 border-l border-neutral-700">
	<div class="flex flex-row items-center m-2 bg-neutral-700 border border-neutral-600 rounded-lg">
		<MagnifyingGlass size="13" class="ml-2 mr-1 text-neutral-300" />
		<input
			bind:value={query}
			class="w-full p-1 text-sm text-neutral-300"
			placeholder="Search actions"
			type="search"
			spellcheck="false"
		/>
	</div>

	<span id="action-list-hint" class="sr-only">Use arrow keys to navigate between actions within a category.</span>
	<div class="grow overflow-auto select-none divide-y divide-neutral-800!">
		{#each filteredCategories as [name, { icon, actions }]}
			<details open>
				<summary class="pl-4 py-3 text-lg font-semibold text-neutral-300 hover:bg-neutral-800 transition-colors cursor-pointer">
					{#if icon}
						<img
							src={!icon.startsWith("omegadeck/") ? getWebserverUrl(icon) : icon.replace("omegadeck", "")}
							alt={name}
							class="w-5 h-5 rounded-xs ml-1 -mt-1 inline"
						/>
					{/if}
					<span class="ml-1">{name}</span>
				</summary>
				<div
					role="listbox"
					aria-label={name}
					aria-describedby="action-list-hint"
					tabindex="-1"
					on:keydown={handleListKeydown}
					on:focusin={handleListFocusin}
				>
					{#each actions as action, i}
						<div
							class="flex flex-row items-center p-2 pl-6 bg-neutral-950 hover:bg-neutral-900 transition-colors border-t border-neutral-800 cursor-grab active:cursor-grabbing"
							draggable="true"
							title={$localisations?.[action.plugin]?.[action.uuid]?.Tooltip ?? action.tooltip}
							role="option"
							aria-selected="false"
							tabindex={i == 0 ? 0 : -1}
							aria-label={$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}
							on:dragstart={(event) => {
								if (!event.dataTransfer) return;
								event.dataTransfer.effectAllowed = "copy";
								event.dataTransfer.setData("action", JSON.stringify(action));
							}}
							on:keydown={(event) => {
								if ((event.ctrlKey || event.metaKey) && event.key == "c") {
									copiedItem.set({ type: "action", action });
								}
							}}
						>
							<img
								src={!action.icon.startsWith("omegadeck/") ? getWebserverUrl(action.icon) : action.icon.replace("omegadeck", "")}
								alt=""
								class="m-0.5 mr-3 w-11 h-11 rounded-lg border border-neutral-700 pointer-events-none"
							/>
							<span class="text-neutral-400">{$localisations?.[action.plugin]?.[action.uuid]?.Name ?? action.name}</span>
						</div>
					{/each}
				</div>
			</details>
		{/each}
	</div>
</div>
