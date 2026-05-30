<script lang="ts">
	import type { ActionInstance, ActionState, Context } from "$lib/bindings";
	import type { CopiedItem } from "$lib/propertyInspector";

	import Clipboard from "phosphor-svelte/lib/Clipboard";
	import Copy from "phosphor-svelte/lib/Copy";
	import Pencil from "phosphor-svelte/lib/Pencil";
	import Trash from "phosphor-svelte/lib/Trash";
	import InstanceEditor from "./InstanceEditor.svelte";

	import { appState } from "$lib/propertyInspector";
	import { CanvasLock, renderImage } from "$lib/rendererHelper";
	import { notify } from "$lib/notifications";

	import { ACTION_FOLDER, ACTION_MULTIACTION, ACTION_TOGGLEACTION, BUILTIN_PLUGIN } from "$lib/constants";
	import { enterFolder, removeInstance, triggerVirtualPress as apiTriggerVirtualPress, updateImage } from "$lib/api/commands";
	import { onKeyMoved, onShowAlert, onShowOk, onUpdateState } from "$lib/api/events";
	import { tick } from "svelte";

	let {
		context,
		label = "",
		tabindex = 0,
		role = "gridcell",
		inslot = $bindable(null as ActionInstance | null),
		active = true,
		scale = 1,
		isTouchPoint = false,
		handlePaste = undefined,
		size = 144,
		ondragstart,
		ondragover,
		ondrop,
	}: {
		context: Context | null;
		label?: string;
		tabindex?: number;
		role?: string;
		inslot: ActionInstance | null;
		active?: boolean;
		scale?: number;
		isTouchPoint?: boolean;
		handlePaste?: ((item: CopiedItem, destination: Context) => Promise<void>) | undefined;
		size?: number;
		ondragstart?: (e: DragEvent) => void;
		ondragover?: (e: DragEvent) => void;
		ondrop?: (e: DragEvent) => void;
	} = $props();

	let slot: ActionInstance | null = $state(null);
	const updateSlot = (val: ActionInstance | null) => {
		if (val && context && val.context.split(".")[0] != context.device) return;
		slot = val;
	};
	$effect(() => { updateSlot(inslot); });

	let pressed = $state(false);
	let showAlert = $state(false);
	let showOk = $state(false);
	let showEditor = $state(false);

	const currentState: ActionState | undefined = $derived(slot != null ? (slot as ActionInstance).states[(slot as ActionInstance).current_state] : undefined);
	const pluginOffline = $derived(slot != null && (slot as ActionInstance).action.plugin !== BUILTIN_PLUGIN && !appState.connectedPlugins.has((slot as ActionInstance).action.plugin));
	const accessibleLabel = $derived(label + (slot != null ? ": " + (slot as ActionInstance).action.name + (currentState?.show && currentState?.text ? " - " + currentState.text : "") : ""));

	onUpdateState((ctx, contents) => {
		if (ctx == slot?.context) slot = contents;
	});

	onKeyMoved((ctx, isPressed) => {
		if (JSON.stringify(context) == JSON.stringify(ctx)) pressed = isPressed;
	});

	function handleSelect(event: MouseEvent | KeyboardEvent) {
		if (event instanceof MouseEvent && event.ctrlKey) return;
		appState.openContextMenu = null;
		if (!slot) {
			appState.inspectedInstance = context;
			return;
		}
		if (slot.action.uuid == ACTION_FOLDER && context) {
			enterFolder(context.device, slot.context);
			return;
		}
		if (slot.action.uuid == ACTION_MULTIACTION || slot.action.uuid == ACTION_TOGGLEACTION) {
			appState.inspectedParentAction = context;
		} else {
			appState.inspectedInstance = slot.context;
		}
	}

	function handleFocus() {
		appState.openContextMenu = null;
		if (!slot) {
			appState.inspectedInstance = context;
			return;
		}
		if (slot.action.uuid != ACTION_MULTIACTION && slot.action.uuid != ACTION_TOGGLEACTION) {
			appState.inspectedInstance = slot.context;
		} else {
			appState.inspectedInstance = context;
		}
	}

	let contextMenuEl: HTMLDivElement | undefined = $state(undefined);
	async function handleContextMenu(event: MouseEvent | KeyboardEvent) {
		event.preventDefault();
		if (!active || !context) return;
		const rect = canvas?.getBoundingClientRect();
		let x = (event instanceof MouseEvent && event.x) ? event.x : rect?.left ?? 0;
		let y = (event instanceof MouseEvent && event.y) ? event.y : rect?.bottom ?? 0;
		appState.openContextMenu = { context, x, y };
		await tick();
		contextMenuEl?.querySelector("button")?.focus();
	}

	function edit() {
		appState.openContextMenu = null;
		showEditor = true;
	}

	function copy() {
		appState.openContextMenu = null;
		if (!context || !slot) return;
		appState.copiedItem = { type: "instance", source: context };
	}

	async function paste() {
		appState.openContextMenu = null;
		if (!appState.copiedItem || !context || !handlePaste) return;
		await handlePaste(appState.copiedItem, context);
		await tick();
		appState.inspectedInstance = `${context.device}.${context.profile}.${context.controller}.${context.position}.0`;
	}

	async function clear() {
		appState.openContextMenu = null;
		if (!slot) return;
		try {
			await removeInstance(slot.context);
		} catch (error: any) {
			notify(String(error));
			return;
		}
		showEditor = false;
		slot = null;
		inslot = slot;
		await tick();
		appState.inspectedInstance = context;
	}

	let timeouts: number[] = [];
	onShowAlert((ctx) => {
		if (!slot || ctx != slot.context) return;
		timeouts.forEach(clearTimeout);
		showOk = false;
		showAlert = true;
		timeouts.push(setTimeout(() => { showAlert = false; }, 1.5e3));
	});
	onShowOk((ctx) => {
		if (!slot || ctx != slot.context) return;
		timeouts.forEach(clearTimeout);
		showAlert = false;
		showOk = true;
		timeouts.push(setTimeout(() => { showOk = false; }, 1.5e3));
	});

	let canvas: HTMLCanvasElement | undefined = $state(undefined);
	let lock = new CanvasLock();

	$effect(() => {
		const sl = $state.snapshot(slot);
		const rotation = appState.settings?.rotation;
		const st = currentState;
		const sa = showAlert;
		const so = showOk;
		const po = pluginOffline;
		const ac = active;
		const pr = pressed;

		(async () => {
			if (!sl) {
				const unlock = await lock.lock();
				try {
					const ctx = canvas?.getContext("2d");
					if (ctx) ctx.clearRect(0, 0, canvas!.width, canvas!.height);
					if (ac) await updateImage(context as any, null);
				} finally {
					unlock();
				}
			} else {
				const unlock = await lock.lock();
				try {
					let fallback = sl.action.states[sl.current_state]?.image ?? sl.action.icon;
					if (st) await renderImage(canvas!, context, st, fallback, so, sa || po, true, ac, pr, rotation);
				} finally {
					unlock();
				}
			}
		})();
	});

	$effect(() => {
		if (appState.settings?.rotation != undefined) {
			canvas?.getContext("2d")?.clearRect(0, 0, canvas.width, canvas.height);
			slot = slot;
		}
	});

	async function triggerVirtualPress() {
		if (!active || !context || !slot) return;
		await apiTriggerVirtualPress(context as any);
	}
</script>

<div
	class="relative"
	style={`transform: scale(${(112 / size) * scale});`}
>
	<canvas
		bind:this={canvas}
		class="relative border-3 border-neutral-700 rounded-3xl outline-none outline-offset-2 outline-blue-500"
		style={`margin: ${-((size + 3 * 2 - 132) / 2)}px;`}
		class:outline-solid={active && ((slot && appState.inspectedInstance == slot.context) || (context && appState.inspectedInstance == context))}
		class:rounded-full!={context?.controller == "Encoder"}
		class:bg-black={slot != null}
		width={size}
		height={size}
		draggable={slot != null}
		{tabindex}
		{role}
		aria-label={accessibleLabel}
		{ondragstart}
		{ondragover}
		{ondrop}
		onclick={(e) => { e.stopPropagation(); handleSelect(e); }}
		ondblclick={(e) => { e.stopPropagation(); triggerVirtualPress(); }}
		onkeydown={(e) => {
			if (!active || !context) return;
			if (e.key == "Enter") handleSelect(e);
			else if (e.key == "F2") edit();
			else if ((e.ctrlKey || e.metaKey) && e.key == "c") copy();
			else if ((e.ctrlKey || e.metaKey) && e.key == "v") paste();
			else if (e.key == "Delete") clear();
			else if (e.key == "ContextMenu" || (e.shiftKey && e.key == "F10")) handleContextMenu(e);
		}}
		onkeyup={(e) => {
			if (!active || !context) return;
			if (e.key == " ") { e.stopPropagation(); handleSelect(e); }
		}}
		onfocus={handleFocus}
		oncontextmenu={handleContextMenu}
	></canvas>
	{#if isTouchPoint && !slot}
		<div class="absolute left-1/4 top-1/2 w-1/2 border-t-4 border-neutral-700 pointer-events-none"></div>
	{/if}
</div>

{#if appState.openContextMenu && JSON.stringify(appState.openContextMenu.context) === JSON.stringify(context)}
	<div
		bind:this={contextMenuEl}
		class="absolute w-32 font-semibold text-sm text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg divide-y divide-neutral-600! z-10"
		style={`left: ${appState.openContextMenu.x}px; top: ${appState.openContextMenu.y}px;`}
	>
		{#if !slot}
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-lg cursor-pointer"
				onclick={(e) => { e.stopPropagation(); paste(); }}
			>
				<Clipboard size="18" class="text-neutral-300" />
				<span class="ml-2"> Paste </span>
			</button>
		{:else}
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-t-lg cursor-pointer"
				onclick={(e) => { e.stopPropagation(); edit(); }}
			>
				<Pencil size="18" class="text-neutral-300" />
				<span class="ml-2"> Edit </span>
			</button>
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors cursor-pointer"
				onclick={(e) => { e.stopPropagation(); copy(); }}
			>
				<Copy size="18" class="text-neutral-300" />
				<span class="ml-2"> Copy </span>
			</button>
			<button
				class="flex flex-row items-center w-full p-2 hover:bg-neutral-600 transition-colors rounded-b-lg cursor-pointer"
				onclick={(e) => { e.stopPropagation(); clear(); }}
			>
				<Trash size="18" class="text-red-400" />
				<span class="ml-2"> Delete </span>
			</button>
		{/if}
	</div>
{/if}

{#if slot && showEditor}
	<InstanceEditor bind:instance={slot} bind:showEditor />
{/if}
