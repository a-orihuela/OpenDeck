<script lang="ts">
	import type { ActionInstance, ActionState, Context, DeviceInfo, Profile } from "$lib/bindings";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import { inspectedInstance, inspectedParentAction } from "$lib/propertyInspector";
	import { inFolderMode } from "$lib/singletons";
	import { renderImage } from "$lib/rendererHelper";

	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { notify } from "$lib/notifications";
	import { onDestroy } from "svelte";

	export let device: DeviceInfo;
	export let profile: Profile;

	export let selectedDevice: string;

	let activePage = 0;

	// Keep activePage in sync when device changes or when the hardware navigates.
	$: if (device) invoke<number>("get_active_page", { device: device.id }).then(p => activePage = p);

	const unlisten = listen<{ device: string; page: number }>("page_changed", ({ payload }) => {
		if (payload.device === device.id) activePage = payload.page;
	});
	onDestroy(() => unlisten.then(fn => fn()));

	// Folder mode state.
	let activeFolderContext: string | null = null;

	const unlistenFolderOpened = listen<{ device: string; folder_context: string }>("folder_opened", ({ payload }) => {
		if (payload.device === device.id) {
			activeFolderContext = payload.folder_context;
		}
	});
	const unlistenFolderClosed = listen<{ device: string }>("folder_closed", ({ payload }) => {
		if (payload.device === device.id) {
			activeFolderContext = null;
		}
	});
	onDestroy(() => {
		unlistenFolderOpened.then(fn => fn());
		unlistenFolderClosed.then(fn => fn());
	});

	// Sync inFolderMode store for ActionList filtering.
	$: if (selectedDevice === device.id) inFolderMode.set(activeFolderContext !== null);

	$: pageSize = device.rows * device.columns;
	$: pageStart = activePage * pageSize;
	$: touchpointStart = (profile.num_pages ?? 1) * pageSize;

	// Folder-derived state.
	$: folderFlatPos = activeFolderContext ? parseInt(activeFolderContext.split('.')[3]) : -1;
	$: folderInstance = activeFolderContext ? (profile.keys[folderFlatPos] ?? null) : null;
	$: folderSlots = (folderInstance?.folder_slots ?? []) as (ActionInstance | null)[];
	$: folderClosePosition = activeFolderContext ? (folderFlatPos % pageSize) : -1;

	// Close button canvas: renders the red X and sends it to the physical device.
	let closeCanvas: HTMLCanvasElement | null = null;

	$: if (closeCanvas && activeFolderContext) {
		renderCloseIcon(closeCanvas);
	}

	async function renderCloseIcon(canvas: HTMLCanvasElement) {
		if (!activeFolderContext || folderClosePosition < 0) return;
		const closeState: ActionState = {
			image: "opendeck/folder-close.svg",
			image_scale: 100,
			background_colour: "#000000",
			name: "", text: "", show: false,
			colour: "#FFFFFF", stroke_colour: "#000000",
			alignment: "middle", family: "Liberation Sans",
			style: "Regular", size: 16, stroke_size: 3, underline: false,
		};
		const closeCtx: Context = {
			device: device.id,
			profile: profile.id,
			controller: "Keypad",
			position: folderClosePosition,
		};
		await renderImage(canvas, closeCtx, closeState, undefined, false, false, true, true, false);
	}

	function handleDragStart({ dataTransfer }: DragEvent, controller: string, position: number) {
		if (!dataTransfer) return;
		dataTransfer.effectAllowed = "move";
		dataTransfer.setData("controller", controller);
		dataTransfer.setData("position", position.toString());
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		if (!event.dataTransfer) return;
		if (event.dataTransfer.types.includes("action")) event.dataTransfer.dropEffect = "copy";
		else if (event.dataTransfer.types.includes("controller")) event.dataTransfer.dropEffect = "move";
	}

	async function handleDrop({ dataTransfer }: DragEvent, controller: string, position: number) {
		let context = { device: device.id, profile: profile.id, controller, position };
		// In folder mode, key drops go to folder slots; the backend routes correctly.
		let array = controller == "Encoder" ? profile.sliders : (activeFolderContext ? folderSlots : profile.keys);
		try {
			if (dataTransfer?.getData("action")) {
				let action = JSON.parse(dataTransfer?.getData("action"));
				if (array[position]) return;
				const result: ActionInstance | null = await invoke("create_instance", { context, action });
				if (result) {
					array[position] = result;
					if (activeFolderContext) {
						folderSlots = [...folderSlots];
					} else {
						profile = profile;
					}
				}
			} else if (dataTransfer?.getData("controller") && !activeFolderContext) {
				let oldArray = dataTransfer?.getData("controller") == "Encoder" ? profile.sliders : profile.keys;
				let oldPosition = parseInt(dataTransfer?.getData("position"));
				let response: ActionInstance = await invoke("move_instance", {
					source: { device: device.id, profile: profile.id, controller: dataTransfer?.getData("controller"), position: oldPosition },
					destination: context,
					retain: false,
				});
				if (response) {
					array[position] = response;
					oldArray[oldPosition] = null;
					profile = profile;
				}
			}
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handlePaste(item: CopiedItem, destination: Context) {
		let array = destination.controller == "Encoder" ? profile.sliders : (activeFolderContext ? folderSlots : profile.keys);
		try {
			if (item.type == "action") {
				if (array[destination.position]) return;
				const result: ActionInstance | null = await invoke("create_instance", { context: destination, action: item.action });
				if (result) {
					array[destination.position] = result;
					if (activeFolderContext) {
						folderSlots = [...folderSlots];
					} else {
						profile = profile;
					}
				}
				return;
			}
			if (activeFolderContext) return;
			let response: ActionInstance = await invoke("move_instance", { source: item.source, destination, retain: true });
			if (response) {
				array[destination.position] = response;
				profile = profile;
			}
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handleAddPage() {
		try {
			const newCount = await invoke<number>("add_page", { device: device.id });
			profile = { ...profile, num_pages: newCount };
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handleRemoveLastPage() {
		try {
			const newCount = await invoke<number>("remove_last_page", { device: device.id });
			profile = { ...profile, num_pages: newCount };
			if (activePage >= newCount) activePage = newCount - 1;
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handleSetActivePage(page: number) {
		await invoke("set_active_page", { device: device.id, page });
		activePage = page;
	}

	async function handleExitFolder() {
		try {
			await invoke("exit_folder", { device: device.id });
		} catch (error: any) {
			notify(String(error));
		}
	}

	$: overflowsX = Math.max(device.columns, device.encoders, device.touchpoints) > 8;
	$: overflowsY = (device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1)) > 4;

	// Grid navigation: track focused cell and compute row lengths for arrow key movement.
	let focusedRow = 0;
	let focusedCol = 0;

	$: gridRowLengths = [
		...Array(device.rows).fill(device.columns),
		...(device.encoders > 0 ? [device.encoders] : []),
		...(device.touchpoints > 0 ? [device.touchpoints] : []),
	];
	$: encoderRowIndex = device.rows;
	$: touchpointRowIndex = device.rows + (device.encoders > 0 ? 1 : 0);

	function flatIndexFromRowCol(row: number, col: number): number {
		let index = 0;
		for (let r = 0; r < row; r++) index += gridRowLengths[r];
		return index + col;
	}

	function rowColFromFlatIndex(flatIndex: number): [number, number] {
		let remaining = flatIndex;
		for (let r = 0; r < gridRowLengths.length; r++) {
			if (remaining < gridRowLengths[r]) return [r, remaining];
			remaining -= gridRowLengths[r];
		}
		return [0, 0];
	}

	function handleGridKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		if (target.getAttribute("role") !== "gridcell") return;
		if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;

		event.preventDefault();
		event.stopPropagation();

		let newRow = focusedRow;
		let newCol = focusedCol;

		switch (event.key) {
			case "ArrowRight":
				newCol = Math.min(focusedCol + 1, gridRowLengths[focusedRow] - 1);
				break;
			case "ArrowLeft":
				newCol = Math.max(focusedCol - 1, 0);
				break;
			case "ArrowDown":
				newRow = Math.min(focusedRow + 1, gridRowLengths.length - 1);
				newCol = Math.min(focusedCol, gridRowLengths[newRow] - 1);
				break;
			case "ArrowUp":
				newRow = Math.max(focusedRow - 1, 0);
				newCol = Math.min(focusedCol, gridRowLengths[newRow] - 1);
				break;
			case "Home":
				newCol = 0;
				break;
			case "End":
				newCol = gridRowLengths[focusedRow] - 1;
				break;
		}

		if (newRow === focusedRow && newCol === focusedCol) return;

		focusedRow = newRow;
		focusedCol = newCol;

		const grid = event.currentTarget as HTMLElement;
		const cells = grid.querySelectorAll("[role='gridcell']");
		(cells[flatIndexFromRowCol(newRow, newCol)] as HTMLElement)?.focus();
	}

	function handleGridFocusin(event: FocusEvent) {
		const grid = event.currentTarget as HTMLElement;
		const cells = Array.from(grid.querySelectorAll("[role='gridcell']"));
		const index = cells.indexOf(event.target as Element);
		if (index === -1) return;
		[focusedRow, focusedCol] = rowColFromFlatIndex(index);
	}
</script>

<style>
	.device-fade-x {
		mask-image: linear-gradient(to right, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent);
	}
	.device-fade-y {
		mask-image: linear-gradient(to bottom, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent);
	}
	.device-fade-xy {
		mask-image:
			linear-gradient(to right, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent),
			linear-gradient(to bottom, transparent, black 7.5rem, black calc(100% - 7.5rem), transparent);
		mask-composite: intersect;
	}
</style>

{#key device}
	<span id="grid-description" class="sr-only">Use arrow keys to navigate between keys. Moving to a key will display its property inspector.</span>
	<div
		class="flex flex-col justify-center grow px-16 py-6 overflow-auto"
		class:items-center={device.columns <= 8}
		class:hidden={$inspectedParentAction || selectedDevice != device.id}
		class:device-fade-x={overflowsX && !overflowsY}
		class:device-fade-y={overflowsY && !overflowsX}
		class:device-fade-xy={overflowsX && overflowsY}
		role="grid"
		aria-label={device.name}
		aria-describedby="grid-description"
		tabindex="-1"
		on:click={() => inspectedInstance.set(null)}
		on:keyup={() => inspectedInstance.set(null)}
		on:keydown|capture={handleGridKeydown}
		on:focusin={handleGridFocusin}
	>
		{#if activeFolderContext}
			<!-- Folder grid: physical positions 0..pageSize-1, no page offset. -->
			<div class="flex flex-col" role="rowgroup">
				{#each { length: device.rows } as _, r}
					<div class="flex flex-row" role="row">
						{#each { length: device.columns } as _, c}
							{@const pos = r * device.columns + c}
							{#if pos === folderClosePosition}
								<!-- Close button: shows red X, clicking exits folder. -->
								<div
									class="relative cursor-pointer"
									style={`transform: scale(${112 / (device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144)});`}
									role="gridcell"
									aria-label="Close folder"
									tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
									on:click|stopPropagation={handleExitFolder}
									on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleExitFolder(); }}
								>
									<canvas
										bind:this={closeCanvas}
										class="relative border-3 border-red-600 rounded-3xl"
										width={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
										height={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
									/>
								</div>
							{:else}
								<Key
									context={{ device: device.id, profile: profile.id, controller: "Keypad", position: pos }}
									bind:inslot={folderSlots[pos]}
									on:dragover={handleDragOver}
									on:drop={(event) => handleDrop(event, "Keypad", pos)}
									on:dragstart={(event) => handleDragStart(event, "Keypad", pos)}
									{handlePaste}
									size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
									label="Folder Key {String.fromCharCode(65 + r)}{c + 1}"
									tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
								/>
							{/if}
						{/each}
					</div>
				{/each}
			</div>
		{:else}
			<!-- Regular page grid. -->
			<div class="flex flex-col" role="rowgroup">
				{#each { length: device.rows } as _, r}
					<div class="flex flex-row" role="row">
						{#each { length: device.columns } as _, c}
							{@const pos = pageStart + (r * device.columns) + c}
							<Key
								context={{ device: device.id, profile: profile.id, controller: "Keypad", position: pos }}
								bind:inslot={profile.keys[pos]}
								on:dragover={handleDragOver}
								on:drop={(event) => handleDrop(event, "Keypad", pos)}
								on:dragstart={(event) => handleDragStart(event, "Keypad", pos)}
								{handlePaste}
								size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
								label="Key {String.fromCharCode(65 + r)}{c + 1}"
								tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
							/>
						{/each}
					</div>
				{/each}
			</div>

			<div class="flex flex-row items-center justify-center gap-2 py-2">
				{#each { length: profile.num_pages ?? 1 } as _, i}
					<button
						class="w-2.5 h-2.5 rounded-full transition-colors {i === activePage ? 'bg-white' : 'bg-white/30'}"
						aria-label="Page {i + 1}"
						on:click={() => handleSetActivePage(i)}
					/>
				{/each}
				<button
					class="ml-2 w-5 h-5 rounded text-white/60 hover:text-white hover:bg-white/10 flex items-center justify-center text-sm leading-none"
					aria-label="Add page"
					title="Add page"
					on:click={handleAddPage}
				>+</button>
				{#if (profile.num_pages ?? 1) > 1}
					<button
						class="w-5 h-5 rounded text-white/60 hover:text-white hover:bg-white/10 flex items-center justify-center text-sm leading-none"
						aria-label="Remove last page"
						title="Remove last page"
						on:click={handleRemoveLastPage}
					>−</button>
				{/if}
			</div>
		{/if}

		<div class="flex flex-row" role="row">
			{#each { length: device.encoders } as _, i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
					bind:inslot={profile.sliders[i]}
					on:dragover={handleDragOver}
					on:drop={(event) => handleDrop(event, "Encoder", i)}
					on:dragstart={(event) => handleDragStart(event, "Encoder", i)}
					{handlePaste}
					size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
					label="Encoder {i + 1}"
					tabindex={focusedRow === encoderRowIndex && focusedCol === i ? 0 : -1}
				/>
			{/each}
		</div>

		{#if !activeFolderContext}
			<div class="flex flex-row" role="row">
				{#each { length: device.touchpoints } as _, i}
					{@const pos = touchpointStart + i}
					<Key
						context={{ device: device.id, profile: profile.id, controller: "Keypad", position: pos }}
						bind:inslot={profile.keys[pos]}
						on:dragover={handleDragOver}
						on:drop={(event) => handleDrop(event, "Keypad", pos)}
						on:dragstart={(event) => handleDragStart(event, "Keypad", pos)}
						{handlePaste}
						size={device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144}
						isTouchPoint
						label="Touch point {i + 1}"
						tabindex={focusedRow === touchpointRowIndex && focusedCol === i ? 0 : -1}
					/>
				{/each}
			</div>
		{/if}
	</div>
{/key}
