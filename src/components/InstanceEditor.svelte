<script lang="ts">
	import type { ActionInstance } from "$lib/bindings";

	import { renderImage, resizeImage } from "$lib/rendererHelper";
	import { _ } from "$lib/i18n";

	import { getFonts, setState } from "$lib/api/commands";

	let { instance = $bindable(), showEditor = $bindable(), inline = false }: {
		instance: ActionInstance;
		showEditor: boolean;
		inline?: boolean;
	} = $props();

	let stateIndex = $state(0);
	let bold = $state(false);
	let italic = $state(false);

	let fonts: string[] = $state([]);
	$effect(() => {
		getFonts().then(f => { fonts = f; });
	});

	let fileInput: HTMLInputElement | undefined = $state(undefined);
	let solidColourInput: HTMLInputElement | undefined = $state(undefined);
	let backgroundColourInput: HTMLInputElement | undefined = $state(undefined);

	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

	function adjustImageScale(delta: number) {
		const next = (instance.states[stateIndex].image_scale || 100) + delta;
		instance.states[stateIndex].image_scale = Math.max(10, Math.min(200, next));
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();
		const file = event.dataTransfer?.files?.[0];
		if (!file || !file.type.startsWith("image/")) return;
		const reader = new FileReader();
		reader.onload = async () => {
			let result = reader.result?.toString();
			if (result) {
				let resized = await resizeImage(result);
				if (resized) instance.states[stateIndex].image = resized;
				else instance.states[stateIndex].image = result;
			}
		};
		reader.readAsDataURL(file);
	}

	function updateBoldItalic(inst: ActionInstance) {
		bold = inst.states[stateIndex].style.includes("Bold");
		italic = inst.states[stateIndex].style.includes("Italic");
	}

	$effect(() => { updateBoldItalic(instance); });
	$effect(() => { setState(instance.context, stateIndex, instance.states[stateIndex]); });
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key == "Escape") showEditor = false;
	}}
/>

<div class={`p-2 text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg ${inline ? "" : "absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-10"}`}>
	<div class="flex flex-row">
		<div class="select-wrapper m-1 w-full">
			<select class="w-full bg-neutral-600! border-neutral-500!" bind:value={stateIndex} aria-label={t("instanceEditor.stateAria")}>
				{#each instance.states as _, i}
					<option value={i}>{t("instanceEditor.stateOption", { index: i + 1 })}</option>
				{/each}
			</select>
		</div>
		<button class="ml-2 mr-1 float-right text-xl text-neutral-300" onclick={() => showEditor = false} aria-label={t("instanceEditor.closeAria")}>✕</button>
	</div>
	<div class="flex flex-row mx-1">
		<div class="flex flex-col justify-center items-center mt-2 mb-1">
			<button
				onclick={(event) => {
					if (event.ctrlKey) return;
					fileInput?.click();
				}}
				ondragover={(event) => {
					event.preventDefault();
					if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
				}}
				ondrop={handleDrop}
				oncontextmenu={(event) => {
					event.preventDefault();
					const defaultImage = instance.action.states[stateIndex]?.image;
					instance.states[stateIndex].image = defaultImage == "actionDefaultImage"
						? instance.action.icon
						: (defaultImage ?? instance.action.icon);
				}}
				title={t("instanceEditor.imageSelectHint")}
				aria-label={t("instanceEditor.imageSelectHint")}
			>
				{#await renderImage(null, null, instance.states[stateIndex], instance.action.states[stateIndex]?.image ?? instance.action.icon, false, false, true, false, false, 0, true)}
					<div class="w-32 min-w-32 h-32 bg-neutral-800 animate-pulse border border-neutral-600 rounded-xl"></div>
				{:then resolvedSrc}
					<img
						src={resolvedSrc}
						class="my-auto w-32 min-w-32 h-min aspect-square bg-black border border-neutral-600 rounded-xl cursor-pointer"
						alt={t("instanceEditor.imageAlt", { index: stateIndex + 1 })}
					/>
				{/await}
			</button>
			<div class="flex flex-row items-center justify-center mt-1 space-x-1 text-neutral-300">
				<button
					onclick={() => adjustImageScale(-10)}
					class="w-6 h-6 text-sm bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-md"
					title={t("instanceEditor.scaleDown")}
					aria-label={t("instanceEditor.scaleDown")}
				>
					-
				</button>
				<span class="min-w-12 text-center text-xs tabular-nums">
					{instance.states[stateIndex].image_scale || 100}%
				</span>
				<button
					onclick={() => adjustImageScale(10)}
					class="w-6 h-6 text-sm bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-md"
					title={t("instanceEditor.scaleUp")}
					aria-label={t("instanceEditor.scaleUp")}
				>
					+
				</button>
			</div>
			<button
				onclick={() => backgroundColourInput?.click()}
				onfocus={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput && (backgroundColourInput.className = "");
				}}
				onmouseover={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput && (backgroundColourInput.className = "");
				}}
				onblur={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput && (backgroundColourInput.className = "absolute invisible w-0 h-0");
				}}
				onmouseleave={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) backgroundColourInput && (backgroundColourInput.className = "absolute invisible w-0 h-0");
				}}
				class="mt-1 px-0.5 text-sm text-neutral-300 bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-lg"
			>
				{t("instanceEditor.setBackground")}
				<input
					bind:this={backgroundColourInput}
					type="color"
					bind:value={instance.states[stateIndex].background_colour}
					class="absolute invisible w-0 h-0"
				/>
			</button>
			<button
				onclick={() => solidColourInput?.click()}
				onfocus={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput && (solidColourInput.className = "");
				}}
				onmouseover={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput && (solidColourInput.className = "");
				}}
				onblur={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput && (solidColourInput.className = "absolute invisible w-0 h-0");
				}}
				onmouseleave={() => {
					if (navigator.userAgent.toLowerCase().includes("mac")) solidColourInput && (solidColourInput.className = "absolute invisible w-0 h-0");
				}}
				class="mt-1 px-0.5 text-sm text-neutral-300 bg-neutral-600 hover:bg-neutral-500 transition-colors border border-neutral-500 rounded-lg"
			>
				{t("instanceEditor.useSolidColor")}
				<input
					bind:this={solidColourInput}
					type="color"
					class="absolute invisible w-0 h-0"
					value="#FFFFFE"
					onchange={() => {
						if (!solidColourInput) return;
						const canvas = document.createElement("canvas");
						canvas.width = 1;
						canvas.height = 1;
						const context = canvas.getContext("2d");
						if (!context) return;
						context.fillStyle = solidColourInput.value;
						context.fillRect(0, 0, canvas.width, canvas.height);
						instance.states[stateIndex].image = canvas.toDataURL("image/png");
					}}
				/>
			</button>
		</div>
		<input
			bind:this={fileInput}
			type="file"
			class="hidden"
			accept="image/*"
			onchange={async () => {
				if (!fileInput?.files || fileInput.files.length == 0) return;
				const reader = new FileReader();
				reader.onload = async () => {
					let result = reader.result?.toString();
					if (result) {
						let resized = await resizeImage(result);
						if (resized) instance.states[stateIndex].image = resized;
						else instance.states[stateIndex].image = result;
					}
				};
				reader.readAsDataURL(fileInput.files[0]);
			}}
		/>

		<div class="flex flex-col justify-center pl-4 pr-2 pt-4 pb-2 space-y-2">
			<div class="flex flex-row items-center space-x-2">
				<label for="editor-text">{t("instanceEditor.text")}</label>
				<textarea
					bind:value={instance.states[stateIndex].text}
					placeholder={instance.action.states[stateIndex]?.text || instance.action.name}
					rows="1"
					class="w-full px-1 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg resize-none"
					id="editor-text"
				></textarea>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-colour" class="mr-2">{t("instanceEditor.colour")}</label>
				<input
					type="color"
					bind:value={instance.states[stateIndex].colour}
					class="mr-2 px-0.5 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-colour"
				/>
				<label for="editor-show" class="mr-2">{t("instanceEditor.show")}</label>
				<input
					type="checkbox"
					bind:checked={instance.states[stateIndex].show}
					class="mr-4 mt-1 scale-125"
					id="editor-show"
				/>
				<select
					bind:value={instance.states[stateIndex].alignment}
					class="px-1! py-0.5!"
					aria-label={t("instanceEditor.alignmentAria")}
				>
					<option value="top">{t("instanceEditor.alignmentTop")}</option>
					<option value="middle">{t("instanceEditor.alignmentMiddle")}</option>
					<option value="bottom">{t("instanceEditor.alignmentBottom")}</option>
				</select>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-stroke" class="mr-2">{t("instanceEditor.stroke")}</label>
				<input
					type="color"
					bind:value={instance.states[stateIndex].stroke_colour}
					class="mr-2 px-0.5 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-stroke"
				/>
				<label for="editor-outline" class="mr-2">{t("instanceEditor.outline")}</label>
				<input
					type="number"
					bind:value={instance.states[stateIndex].stroke_size}
					class="px-0.5 w-14 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-outline"
				/>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-font" class="mr-2">{t("instanceEditor.font")}</label>
				<input
					list="families"
					bind:value={instance.states[stateIndex].family}
					placeholder={t("instanceEditor.fontFamilyPlaceholder")}
					class="w-full px-1 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-font"
				/>
				<datalist id="families">
					<option value="Liberation Sans">Liberation Sans</option>
					<option value="Archivo Black">Archivo Black</option>
					<option value="Comic Neue">Comic Neue</option>
					<option value="Courier Prime">Courier Prime</option>
					<option value="Tinos">Tinos</option>
					<option value="Anton">Anton</option>
					<option value="Liberation Serif">Liberation Serif</option>
					<option value="Open Sans">Open Sans</option>
					<option value="Fira Sans">Fira Sans</option>
					<option disabled>──────────</option>
					{#each fonts as font}
						<option value={font}>{font}</option>
					{/each}
				</datalist>
			</div>
			<div class="flex flex-row items-center">
				<label for="editor-bold" class="mr-3 font-bold" title={t("instanceEditor.bold")}>B</label>
				<input
					type="checkbox"
					bind:checked={bold}
					onchange={() => { instance.states[stateIndex].style = bold && italic ? "Bold Italic" : bold ? "Bold" : italic ? "Italic" : "Regular"; }}
					class="mr-4 mt-1 scale-125"
					id="editor-bold"
				/>
				<label for="editor-italic" class="mr-3 italic" title={t("instanceEditor.italic")}>I</label>
				<input
					type="checkbox"
					bind:checked={italic}
					onchange={() => { instance.states[stateIndex].style = bold && italic ? "Bold Italic" : bold ? "Bold" : italic ? "Italic" : "Regular"; }}
					class="mr-4 mt-1 scale-125"
					id="editor-italic"
				/>
				<label for="editor-underline" class="mr-3 underline" title={t("instanceEditor.underline")}>U</label>
				<input
					type="checkbox"
					bind:checked={instance.states[stateIndex].underline}
					class="mr-4 mt-1 scale-125"
					id="editor-underline"
				/>
				<label for="editor-size" class="mr-2">{t("instanceEditor.size")}</label>
				<input
					type="number"
					bind:value={instance.states[stateIndex].size}
					class="px-0.5 w-14 text-neutral-300 bg-neutral-600 border border-neutral-500 rounded-lg"
					id="editor-size"
				/>
			</div>
		</div>
	</div>
</div>
