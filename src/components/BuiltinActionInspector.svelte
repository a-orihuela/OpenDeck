<script lang="ts">
	import type { ActionInstance } from "$lib/bindings";
	import { getProfiles, setInstanceSettings } from "$lib/api/commands";
	import { appState } from "$lib/settings";

	let { instance }: { instance: ActionInstance } = $props();

	type SettingsMap = Record<string, unknown>;

	let profiles: string[] = $state([]);

	// Keyboard capture for Simulate Input
	let capturingField: string | null = $state(null);
	let heldModifiers = new Set<string>();

	const MODIFIER_CODES = new Set([
		'ControlLeft', 'ControlRight', 'ShiftLeft', 'ShiftRight',
		'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight',
	]);

	const CODE_TO_ENIGO: Record<string, string> = (() => {
		const m: Record<string, string> = {
			'ControlLeft': 'ControlLeft', 'ControlRight': 'ControlRight',
			'ShiftLeft': 'ShiftLeft', 'ShiftRight': 'ShiftRight',
			'AltLeft': 'Alt', 'AltRight': 'AltGr',
			'MetaLeft': 'Meta', 'MetaRight': 'Meta',
			'Enter': 'Return', 'Escape': 'Escape', 'Backspace': 'Backspace',
			'Tab': 'Tab', 'Space': 'Space', 'Delete': 'Delete', 'Insert': 'Insert',
			'Home': 'Home', 'End': 'End', 'PageUp': 'PageUp', 'PageDown': 'PageDown',
			'ArrowUp': 'UpArrow', 'ArrowDown': 'DownArrow', 'ArrowLeft': 'LeftArrow', 'ArrowRight': 'RightArrow',
			'F1': 'F1', 'F2': 'F2', 'F3': 'F3', 'F4': 'F4', 'F5': 'F5', 'F6': 'F6',
			'F7': 'F7', 'F8': 'F8', 'F9': 'F9', 'F10': 'F10', 'F11': 'F11', 'F12': 'F12',
			'PrintScreen': 'PrintScr', 'CapsLock': 'CapsLock', 'NumLock': 'NumLock', 'ScrollLock': 'ScrollLock',
			'Minus': 'Minus', 'Equal': 'Equal', 'BracketLeft': 'LeftBracket', 'BracketRight': 'RightBracket',
			'Backslash': 'BackSlash', 'Semicolon': 'SemiColon', 'Quote': 'Quote', 'Backquote': 'Grave',
			'Comma': 'Comma', 'Period': 'Dot', 'Slash': 'Slash',
		};
		for (const c of 'ABCDEFGHIJKLMNOPQRSTUVWXYZ') m[`Key${c}`] = `Key${c}`;
		for (let i = 0; i <= 9; i++) m[`Digit${i}`] = `Num${i}`;
		return m;
	})();

	$effect(() => {
		if (!capturingField) return;
		const onKeyDown = (e: KeyboardEvent) => {
			e.preventDefault();
			e.stopPropagation();
			if (MODIFIER_CODES.has(e.code)) {
				heldModifiers.add(e.code);
				return;
			}
			const mods = Array.from(heldModifiers);
			const mainKey = CODE_TO_ENIGO[e.code] ?? e.code;
			const parts = [
				...mods.map(m => `k(${CODE_TO_ENIGO[m]},Press)`),
				`k(${mainKey},Click)`,
				...mods.map(m => `k(${CODE_TO_ENIGO[m]},Release)`),
			];
			const dsl = `[${parts.join(',')}]`;
			void updateSetting(capturingField!, dsl);
			capturingField = null;
			heldModifiers = new Set();
		};
		const onKeyUp = (e: KeyboardEvent) => { heldModifiers.delete(e.code); };
		const onBlur = () => { capturingField = null; heldModifiers = new Set(); };
		document.addEventListener('keydown', onKeyDown, true);
		document.addEventListener('keyup', onKeyUp, true);
		window.addEventListener('blur', onBlur);
		return () => {
			document.removeEventListener('keydown', onKeyDown, true);
			document.removeEventListener('keyup', onKeyUp, true);
			window.removeEventListener('blur', onBlur);
		};
	});

	function cancelCapture() {
		capturingField = null;
		heldModifiers = new Set();
	}

	function readString(settings: SettingsMap, key: string): string {
		const value = settings[key];
		return typeof value === "string" ? value : "";
	}

	function readNumber(settings: SettingsMap, key: string, fallback: number): number {
		const value = settings[key];
		if (typeof value === "number" && Number.isFinite(value)) return value;
		if (typeof value === "string") {
			const parsed = Number.parseInt(value, 10);
			if (Number.isFinite(parsed)) return parsed;
		}
		return fallback;
	}

	function readBool(settings: SettingsMap, key: string, fallback: boolean): boolean {
		const value = settings[key];
		return typeof value === "boolean" ? value : fallback;
	}

	const currentSettings = $derived(
		(instance.settings && typeof instance.settings === "object" && !Array.isArray(instance.settings)
			? instance.settings
			: {}) as SettingsMap,
	);

	const controller = $derived(instance.context.split(".")[2] ?? "Keypad");
	const isEncoder = $derived(controller == "Encoder");
	const isSpanish = $derived(appState.settings?.language === "es");
	// Device comes from the action context (index 0), not from settings
	const currentDeviceId = $derived(instance.context.split(".")[0] ?? "");

	function t(english: string, spanish: string): string {
		return isSpanish ? spanish : english;
	}

	$effect(() => {
		if (!currentDeviceId) {
			profiles = [];
			return;
		}
		getProfiles(currentDeviceId)
			.then((value) => { profiles = value; })
			.catch(() => { profiles = []; });
	});

	async function updateSetting(key: string, value: unknown) {
		const next: SettingsMap = { ...currentSettings, [key]: value };
		instance.settings = next;
		await setInstanceSettings(instance.context, next);
	}

	async function updateMany(values: SettingsMap) {
		const next: SettingsMap = { ...currentSettings, ...values };
		instance.settings = next;
		await setInstanceSettings(instance.context, next);
	}

	function cyclePreview(settings: SettingsMap): string {
		const work = readNumber(settings, "work_minutes", 25);
		const brk = readNumber(settings, "break_minutes", 5);
		const longBreak = readNumber(settings, "long_break_minutes", 15);
		const sessions = readNumber(settings, "sessions_before_long_break", 4);
		const parts: string[] = [];
		for (let i = 0; i < sessions; i++) {
			parts.push(`Work ${work}m`);
			if (i < sessions - 1) parts.push(`Break ${brk}m`);
		}
		parts.push(`Long break ${longBreak}m`);
		return `${parts.join(" -> ")} -> repeat`;
	}
</script>

<div class="w-full h-full overflow-auto p-3 text-neutral-200">
	{#if instance.action.uuid === "omegadeck.builtin.runcommand"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">{t("Run Command", "Ejecutar comando")}</h3>
			<p class="text-xs text-neutral-400">{@html t(
				'Write the shell command that OmegaDeck should execute for each gesture. Example: in <strong>On press</strong> you can put <code class="bg-neutral-800 px-1 rounded">notify-send &quot;OmegaDeck&quot; &quot;Action triggered&quot;</code>. If the action is on an encoder, the <strong>Dial rotate</strong> field runs on each turn, and <code class="bg-neutral-800 px-1 rounded">%d</code> is replaced with the number of ticks moved: <code class="bg-neutral-800 px-1 rounded">1</code>, <code class="bg-neutral-800 px-1 rounded">2</code>, <code class="bg-neutral-800 px-1 rounded">-1</code>...',
				'Escribe el comando de shell que OmegaDeck debe ejecutar para cada gesto. Por ejemplo, en <strong>Al pulsar</strong> puedes poner <code class="bg-neutral-800 px-1 rounded">notify-send &quot;OmegaDeck&quot; &quot;Acción ejecutada&quot;</code>. Si la acción está en un encoder, el campo <strong>Al girar</strong> se ejecuta en cada giro, y <code class="bg-neutral-800 px-1 rounded">%d</code> se sustituye por el número de pasos detectados: <code class="bg-neutral-800 px-1 rounded">1</code>, <code class="bg-neutral-800 px-1 rounded">2</code>, <code class="bg-neutral-800 px-1 rounded">-1</code>...'
			)}</p>

			<label class="block text-xs text-neutral-400" for="builtin-run-down">{isEncoder ? t("Dial press", "Pulsación del dial") : t("On press", "Al pulsar")}</label>
			<input
				id="builtin-run-down"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "down")}
				placeholder={t('notify-send "OmegaDeck" "Action triggered!"', 'notify-send "OmegaDeck" "¡Acción ejecutada!"')}
				onchange={(e) => updateSetting("down", e.currentTarget.value)}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-run-up">{isEncoder ? t("Dial release", "Soltar el dial") : t("On release", "Al soltar")}</label>
			<input
				id="builtin-run-up"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "up")}
				onchange={(e) => updateSetting("up", e.currentTarget.value)}
			/>

			{#if isEncoder}
				<label class="block text-xs text-neutral-400" for="builtin-run-rotate">{t("Dial rotate", "Al girar")}</label>
				<input
					id="builtin-run-rotate"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "rotate")}
					placeholder={t('Example: notify-send "OmegaDeck" "Rotated %d ticks"', 'Ejemplo: notify-send "OmegaDeck" "Girado %d pasos"')}
					onchange={(e) => updateSetting("rotate", e.currentTarget.value)}
				/>
			{/if}

			<hr class="border-neutral-700 my-2" />
			<label class="block text-xs text-neutral-400" for="builtin-run-file">{t("Save output to", "Guardar salida en")}</label>
			<input
				id="builtin-run-file"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "file")}
				placeholder={t("Optional file path", "Ruta de archivo opcional")}
				onchange={(e) => updateSetting("file", e.currentTarget.value)}
			/>
			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-run-show">
				<input
					id="builtin-run-show"
					type="checkbox"
					checked={readBool(currentSettings, "show", false)}
					onchange={(e) => updateSetting("show", e.currentTarget.checked)}
				/>
				{t("Show output on key", "Mostrar salida en la tecla")}
			</label>
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.openurl"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">Open URL</h3>

			<label class="block text-xs text-neutral-400" for="builtin-url-down">{isEncoder ? "Dial press" : "On press"}</label>
			<input
				id="builtin-url-down"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "down")}
				onchange={(e) => updateSetting("down", e.currentTarget.value)}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-url-up">{isEncoder ? "Dial release" : "On release"}</label>
			<input
				id="builtin-url-up"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "up")}
				onchange={(e) => updateSetting("up", e.currentTarget.value)}
			/>

			{#if isEncoder}
				<label class="block text-xs text-neutral-400" for="builtin-url-anticlockwise">Rotate left</label>
				<input
					id="builtin-url-anticlockwise"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "anticlockwise")}
					onchange={(e) => updateSetting("anticlockwise", e.currentTarget.value)}
				/>

				<label class="block text-xs text-neutral-400" for="builtin-url-clockwise">Rotate right</label>
				<input
					id="builtin-url-clockwise"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "clockwise")}
					onchange={(e) => updateSetting("clockwise", e.currentTarget.value)}
				/>
			{/if}
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.inputsimulation"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">Simulate Input</h3>
			<p class="text-xs text-neutral-400">Enigo DSL format: <code class="bg-neutral-800 px-1 rounded">[k(ControlLeft,Press),k(KeyC,Click),k(ControlLeft,Release)]</code>. Press <strong>Capture</strong> to record a shortcut automatically.</p>

			{#if !isEncoder}
				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-down">On press</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'down' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'down') cancelCapture(); else capturingField = 'down'; }}
					>{capturingField === 'down' ? '⌨ Listening…' : '⌨ Capture'}</button>
				</div>
				<textarea
					id="builtin-input-down"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("down", e.currentTarget.value)}
				>{readString(currentSettings, "down")}</textarea>

				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-up">On release (optional)</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'up' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'up') cancelCapture(); else capturingField = 'up'; }}
					>{capturingField === 'up' ? '⌨ Listening…' : '⌨ Capture'}</button>
				</div>
				<textarea
					id="builtin-input-up"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("up", e.currentTarget.value)}
				>{readString(currentSettings, "up")}</textarea>
			{:else}
				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-anticlockwise">Rotate left</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'anticlockwise' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'anticlockwise') cancelCapture(); else capturingField = 'anticlockwise'; }}
					>{capturingField === 'anticlockwise' ? '⌨ Listening…' : '⌨ Capture'}</button>
				</div>
				<textarea
					id="builtin-input-anticlockwise"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("anticlockwise", e.currentTarget.value)}
				>{readString(currentSettings, "anticlockwise")}</textarea>

				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-clockwise">Rotate right</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'clockwise' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'clockwise') cancelCapture(); else capturingField = 'clockwise'; }}
					>{capturingField === 'clockwise' ? '⌨ Listening…' : '⌨ Capture'}</button>
				</div>
				<textarea
					id="builtin-input-clockwise"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("clockwise", e.currentTarget.value)}
				>{readString(currentSettings, "clockwise")}</textarea>
			{/if}
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.screenshot"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">Screenshot</h3>
			<label class="block text-xs text-neutral-400" for="builtin-screenshot-mode">Method</label>
			<select
				id="builtin-screenshot-mode"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "mode") || "system_shortcut"}
				onchange={(e) => updateSetting("mode", e.currentTarget.value)}
			>
				<option value="system_shortcut">System shortcut</option>
				<option value="command">Custom command</option>
			</select>

			{#if (readString(currentSettings, "mode") || "system_shortcut") == "command"}
				<label class="block text-xs text-neutral-400" for="builtin-screenshot-command">Command</label>
				<input
					id="builtin-screenshot-command"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "command")}
					onchange={(e) => updateSetting("command", e.currentTarget.value)}
				/>
			{/if}
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.switchprofile"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">Switch Profile</h3>
			<p class="text-xs text-neutral-400">Switches the profile on the device that contains this button.</p>

			<label class="block text-xs text-neutral-400" for="builtin-switch-profile">{isEncoder ? "Dial press" : "Profile"}</label>
			<input
				id="builtin-switch-profile"
				list="builtin-profiles"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "profile") || "Default"}
				onchange={(e) => updateSetting("profile", e.currentTarget.value)}
			/>
			<datalist id="builtin-profiles">
				{#each profiles as id}
					<option value={id}></option>
				{/each}
			</datalist>

			{#if isEncoder}
				<label class="block text-xs text-neutral-400" for="builtin-switch-anticlockwise">Rotate left</label>
				<input
					id="builtin-switch-anticlockwise"
					list="builtin-profiles"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "anticlockwise") || "Default"}
					onchange={(e) => updateSetting("anticlockwise", e.currentTarget.value)}
				/>

				<label class="block text-xs text-neutral-400" for="builtin-switch-clockwise">Rotate right</label>
				<input
					id="builtin-switch-clockwise"
					list="builtin-profiles"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "clockwise") || "Default"}
					onchange={(e) => updateSetting("clockwise", e.currentTarget.value)}
				/>
			{/if}
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.brightnessup" || instance.action.uuid === "omegadeck.builtin.brightnessdown"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">{instance.action.uuid.endsWith("up") ? "Brightness Up" : "Brightness Down"}</h3>
			<p class="text-xs text-neutral-400">Controls Stream Deck brightness. Safety limits: minimum 5%, maximum 100%.</p>
			<label class="block text-xs text-neutral-400" for="builtin-brightness-step">Step ({readNumber(currentSettings, "step", 10)}%)</label>
			<input
				id="builtin-brightness-step"
				type="range"
				min="1"
				max="50"
				value={readNumber(currentSettings, "step", 10)}
				oninput={(e) => updateSetting("step", Number.parseInt(e.currentTarget.value, 10) || 10)}
			/>
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.pomodoro"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">Pomodoro Timer</h3>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-work">Work (min)</label>
			<input
				id="builtin-pomodoro-work"
				type="number"
				min="1"
				max="120"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "work_minutes", 25)}
				onchange={(e) => updateMany({ work_minutes: Number.parseInt(e.currentTarget.value, 10) || 25 })}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-break">Break (min)</label>
			<input
				id="builtin-pomodoro-break"
				type="number"
				min="1"
				max="60"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "break_minutes", 5)}
				onchange={(e) => updateMany({ break_minutes: Number.parseInt(e.currentTarget.value, 10) || 5 })}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-longbreak">Long break (min)</label>
			<input
				id="builtin-pomodoro-longbreak"
				type="number"
				min="1"
				max="120"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "long_break_minutes", 15)}
				onchange={(e) => updateMany({ long_break_minutes: Number.parseInt(e.currentTarget.value, 10) || 15 })}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-sessions">Sessions until long break</label>
			<input
				id="builtin-pomodoro-sessions"
				type="number"
				min="1"
				max="10"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "sessions_before_long_break", 4)}
				onchange={(e) => updateMany({ sessions_before_long_break: Number.parseInt(e.currentTarget.value, 10) || 4 })}
			/>

			<p class="text-xs text-neutral-400">Cycle preview</p>
			<p class="text-xs text-neutral-300">{cyclePreview(currentSettings)}</p>

			<hr class="border-neutral-700 my-2" />
			<p class="text-xs font-semibold text-neutral-300">Alerts</p>

			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-pomodoro-notify-system">
				<input
					id="builtin-pomodoro-notify-system"
					type="checkbox"
					checked={readBool(currentSettings, "notify_system", true)}
					onchange={(e) => updateSetting("notify_system", e.currentTarget.checked)}
				/>
				System notification on phase change
			</label>

			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-pomodoro-notify-sound">
				<input
					id="builtin-pomodoro-notify-sound"
					type="checkbox"
					checked={readBool(currentSettings, "notify_sound", true)}
					onchange={(e) => updateSetting("notify_sound", e.currentTarget.checked)}
				/>
				Play sound on phase change
			</label>

			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-pomodoro-show-on-key">
				<input
					id="builtin-pomodoro-show-on-key"
					type="checkbox"
					checked={readBool(currentSettings, "show_on_key", true)}
					onchange={(e) => updateSetting("show_on_key", e.currentTarget.checked)}
				/>
				Show countdown on Stream Deck key
			</label>
		</div>
	{:else}
		<div class="text-sm text-neutral-400 p-2">
			No property inspector is available for this built-in action yet.
		</div>
	{/if}
</div>
