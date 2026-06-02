<script lang="ts">
	import type { ActionInstance, Context, DeviceInfo, Profile } from "$lib/bindings";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Key from "./Key.svelte";

	import { appState } from "$lib/propertyInspector";
	import { _ } from "$lib/i18n";
	
	import { renderImage } from "$lib/rendererHelper";

	import { addPage, exitFolder, getActivePage, removeLastPage, setActivePage } from "$lib/api/commands";
	import { onFolderClosed, onFolderOpened, onPageChanged } from "$lib/api/events";
	import { computeGridRowLengths, dropMoveInstance, dropNewAction, flatIndexFromRowCol, pasteItem, rowColFromFlatIndex } from "$lib/services/deviceService";
	import { notify } from "$lib/notifications";

	let { device = $bindable(), profile = $bindable(), selectedDevice = $bindable("") }: {
		device: DeviceInfo;
		profile: Profile;
		selectedDevice: string;
	} = $props();

	let activePage = $state(0);

	$effect(() => {
		if (device) getActivePage(device.id).then(p => { activePage = p; });
	});

	$effect(() => {
		const unlistenPage = onPageChanged((dev, page) => {
			if (dev === device.id) activePage = page;
		});
		const unlistenOpened = onFolderOpened((dev, folderContext) => {
			if (dev === device.id) activeFolderContext = folderContext;
		});
		const unlistenClosed = onFolderClosed((dev) => {
			if (dev === device.id) activeFolderContext = null;
		});
		return () => {
			unlistenPage.then(fn => fn());
			unlistenOpened.then(fn => fn());
			unlistenClosed.then(fn => fn());
		};
	});

	let activeFolderContext: string | null = $state(null);

	$effect(() => {
		if (selectedDevice === device.id) appState.inFolderMode = activeFolderContext !== null;
	});

	const gridRowLengths = $derived(computeGridRowLengths(device));
	const pageSize = $derived(device.rows * device.columns);
	const pageStart = $derived(activePage * pageSize);
	const touchpointStart = $derived((profile.num_pages ?? 1) * pageSize);

	const folderFlatPos = $derived(activeFolderContext ? parseInt((activeFolderContext as string).split('.')[3]) : -1);
	const folderSlots = $derived((activeFolderContext ? (profile.keys[folderFlatPos]?.folder_slots ?? []) : []) as (ActionInstance | null)[]);
	const folderClosePosition = $derived(activeFolderContext ? (folderFlatPos % pageSize) : -1);

	let closeCanvas: HTMLCanvasElement | null = $state(null);

	$effect(() => {
		if (closeCanvas && activeFolderContext) {
			renderCloseIcon(closeCanvas);
		}
	});

	const keySize = $derived(device.id.startsWith("sd-") && device.rows == 4 && device.columns == 8 ? 192 : 144);

	async function renderCloseIcon(canvas: HTMLCanvasElement) {
		if (!activeFolderContext || folderClosePosition < 0) return;
		const closeCtx: Context = {
			device: device.id,
			profile: profile.id,
			controller: "Keypad",
			position: folderClosePosition,
		};
		await renderImage(canvas, closeCtx, {
				image: "omegadeck/builtin/folder-close.svg",
			image_scale: 100,
			background_colour: "#000000",
			name: "", text: "", show: false,
			colour: "#FFFFFF", stroke_colour: "#000000",
			alignment: "middle", family: "Liberation Sans",
			style: "Regular", size: 16, stroke_size: 3, underline: false,
		}, undefined, false, false, true, true, false);
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
		const context: Context = { device: device.id, profile: profile.id, controller, position };
		const array = controller == "Encoder" ? profile.sliders : (activeFolderContext ? folderSlots : profile.keys);
		try {
			if (dataTransfer?.getData("action")) {
				const result = await dropNewAction(context, dataTransfer.getData("action"), array[position]);
				if (result) {
					array[position] = result;
					profile = { ...profile };
				}
			} else if (dataTransfer?.getData("controller") && !activeFolderContext) {
				const oldController = dataTransfer.getData("controller");
				const oldPosition = parseInt(dataTransfer.getData("position"));
				const oldArray = oldController == "Encoder" ? profile.sliders : profile.keys;
				const { instance } = await dropMoveInstance(device, profile, oldController, oldPosition, context);
				if (instance) {
					array[position] = instance;
					oldArray[oldPosition] = null;
					profile = { ...profile };
				}
			}
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handlePaste(item: CopiedItem, destination: Context) {
		const array = destination.controller == "Encoder" ? profile.sliders : (activeFolderContext ? folderSlots : profile.keys);
		try {
			const result = await pasteItem(item, destination, array[destination.position], activeFolderContext);
			if (result) {
				array[destination.position] = result;
				profile = { ...profile };
			}
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handleAddPage() {
		try {
			const newCount = await addPage(device.id);
			profile = { ...profile, num_pages: newCount };
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handleRemoveLastPage() {
		try {
			const newCount = await removeLastPage(device.id);
			profile = { ...profile, num_pages: newCount };
			if (activePage >= newCount) activePage = newCount - 1;
		} catch (error: any) {
			notify(String(error));
		}
	}

	async function handleSetActivePage(page: number) {
		await setActivePage(device.id, page);
		activePage = page;
	}

	async function handleExitFolder() {
		try {
			await exitFolder(device.id);
		} catch (error: any) {
			notify(String(error));
		}
	}

	const overflowsX = $derived(Math.max(device.columns, device.encoders, device.touchpoints) > 8);
	const overflowsY = $derived((device.rows + Math.min(device.encoders, 1) + Math.min(device.touchpoints, 1)) > 4);

	let focusedRow = $state(0);
	let focusedCol = $state(0);
	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

	const encoderRowIndex = $derived(device.rows);
	const touchpointRowIndex = $derived(device.rows + (device.encoders > 0 ? 1 : 0));

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
		(cells[flatIndexFromRowCol(gridRowLengths, newRow, newCol)] as HTMLElement)?.focus();
	}

	function handleGridFocusin(event: FocusEvent) {
		const grid = event.currentTarget as HTMLElement;
		const cells = Array.from(grid.querySelectorAll("[role='gridcell']"));
		const index = cells.indexOf(event.target as Element);
		if (index === -1) return;
		[focusedRow, focusedCol] = rowColFromFlatIndex(gridRowLengths, index);
	}
</script>

{#key device}
	<span id="grid-description" class="sr-only">{t("deviceView.gridDescription")}</span>
	<div
		class="flex flex-col justify-center grow px-16 py-6 overflow-auto"
		class:items-center={device.columns <= 8}
		class:hidden={appState.inspectedParentAction || selectedDevice != device.id}
		class:device-fade-x={overflowsX && !overflowsY}
		class:device-fade-y={overflowsY && !overflowsX}
		class:device-fade-xy={overflowsX && overflowsY}
		role="grid"
		aria-label={device.name}
		aria-describedby="grid-description"
		tabindex="-1"
		onclick={() => { appState.inspectedInstance = null; }}
		onkeyup={() => { appState.inspectedInstance = null; }}
		onkeydown={handleGridKeydown}
		onfocusin={handleGridFocusin}
	>
		{#if activeFolderContext}
			<!-- Folder grid -->
			<div class="flex flex-col" role="rowgroup">
				{#each { length: device.rows } as _, r}
					<div class="flex flex-row" role="row">
						{#each { length: device.columns } as _, c}
							{@const pos = r * device.columns + c}
							{#if pos === folderClosePosition}
								{@const closeSize = keySize}
								<div
									class="relative cursor-pointer"
									style={`transform: scale(${112 / closeSize});`}
									role="gridcell"
									aria-label={t("deviceView.closeFolder")}
									tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
									onclick={(e) => { e.stopPropagation(); handleExitFolder(); }}
									onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleExitFolder(); }}
								>
									<canvas
										bind:this={closeCanvas}
										class="relative border-3 border-red-600 rounded-3xl"
										style={`margin: ${-((closeSize + 6 - 132) / 2)}px;`}
										width={closeSize}
										height={closeSize}
									></canvas>
								</div>
							{:else}
								<Key
									context={{ device: device.id, profile: profile.id, controller: "Keypad", position: pos }}
									bind:inslot={folderSlots[pos]}
									ondragover={handleDragOver}
									ondrop={(event) => handleDrop(event, "Keypad", pos)}
									ondragstart={(event) => handleDragStart(event, "Keypad", pos)}
									{handlePaste}
									size={keySize}
									label={t("deviceView.folderKeyLabel", { row: String.fromCharCode(65 + r), column: c + 1 })}
									tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
								/>
							{/if}
						{/each}
					</div>
				{/each}
			</div>
		{:else}
			<!-- Regular page grid -->
			<div class="flex flex-col" role="rowgroup">
				{#each { length: device.rows } as _, r}
					<div class="flex flex-row" role="row">
						{#each { length: device.columns } as _, c}
							{@const pos = pageStart + (r * device.columns) + c}
							<Key
								context={{ device: device.id, profile: profile.id, controller: "Keypad", position: pos }}
								bind:inslot={profile.keys[pos]}
								ondragover={handleDragOver}
								ondrop={(event) => handleDrop(event, "Keypad", pos)}
								ondragstart={(event) => handleDragStart(event, "Keypad", pos)}
								{handlePaste}
								size={keySize}
								label={t("deviceView.keyLabel", { row: String.fromCharCode(65 + r), column: c + 1 })}
								tabindex={focusedRow === r && focusedCol === c ? 0 : -1}
							/>
						{/each}
					</div>
				{/each}
			</div>
		{/if}

		<!-- Page navigation -->
		<div
			class="flex flex-row items-center justify-center gap-2 py-2"
			class:invisible={!!activeFolderContext}
			class:pointer-events-none={!!activeFolderContext}
		>
			{#each { length: profile.num_pages ?? 1 } as _, i}
				<button
					class="w-2.5 h-2.5 rounded-full transition-colors {i === activePage ? 'bg-white' : 'bg-white/30'}"
					aria-label={t("deviceView.pageAria", { index: i + 1 })}
					onclick={() => handleSetActivePage(i)}
				></button>
			{/each}
			<button
				class="ml-2 w-5 h-5 rounded text-white/60 hover:text-white hover:bg-white/10 flex items-center justify-center text-sm leading-none"
				aria-label={t("deviceView.addPage")}
				title={t("deviceView.addPage")}
				onclick={handleAddPage}
			>+</button>
			{#if (profile.num_pages ?? 1) > 1}
				<button
					class="w-5 h-5 rounded text-white/60 hover:text-white hover:bg-white/10 flex items-center justify-center text-sm leading-none"
					aria-label={t("deviceView.removeLastPage")}
					title={t("deviceView.removeLastPage")}
					onclick={handleRemoveLastPage}
				>−</button>
			{/if}
		</div>

		<div class="flex flex-row" role="row">
			{#each { length: device.encoders } as _, i}
				<Key
					context={{ device: device.id, profile: profile.id, controller: "Encoder", position: i }}
					bind:inslot={profile.sliders[i]}
					ondragover={handleDragOver}
					ondrop={(event) => handleDrop(event, "Encoder", i)}
					ondragstart={(event) => handleDragStart(event, "Encoder", i)}
					{handlePaste}
					size={keySize}
					label={t("deviceView.encoderLabel", { index: i + 1 })}
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
						ondragover={handleDragOver}
						ondrop={(event) => handleDrop(event, "Keypad", pos)}
						ondragstart={(event) => handleDragStart(event, "Keypad", pos)}
						{handlePaste}
						size={keySize}
						isTouchPoint
						label={t("deviceView.touchPointLabel", { index: i + 1 })}
						tabindex={focusedRow === touchpointRowIndex && focusedCol === i ? 0 : -1}
					/>
				{/each}
			</div>
		{/if}
	</div>
{/key}
