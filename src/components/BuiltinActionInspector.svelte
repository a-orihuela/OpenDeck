<script lang="ts">
	import type { ActionInstance } from "$lib/bindings";
	import { getProfiles, setInstanceSettings } from "$lib/api/commands";
	import { _ } from "$lib/i18n";

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

	const CODE_TO_ENIGO: Record<string, string> = {
		'ControlLeft': 'LControl', 'ControlRight': 'RControl',
		'ShiftLeft': 'LShift', 'ShiftRight': 'RShift',
		'AltLeft': 'Alt', 'AltRight': 'Alt',
		'MetaLeft': 'Meta', 'MetaRight': 'Meta',
		'Enter': 'Return', 'Escape': 'Escape', 'Backspace': 'Backspace',
		'Tab': 'Tab', 'Space': 'Space', 'Delete': 'Delete', 'Insert': 'Insert',
		'Home': 'Home', 'End': 'End', 'PageUp': 'PageUp', 'PageDown': 'PageDown',
		'ArrowUp': 'UpArrow', 'ArrowDown': 'DownArrow', 'ArrowLeft': 'LeftArrow', 'ArrowRight': 'RightArrow',
		'F1': 'F1', 'F2': 'F2', 'F3': 'F3', 'F4': 'F4', 'F5': 'F5', 'F6': 'F6',
		'F7': 'F7', 'F8': 'F8', 'F9': 'F9', 'F10': 'F10', 'F11': 'F11', 'F12': 'F12',
		'PrintScreen': 'PrintScr', 'CapsLock': 'CapsLock', 'NumLock': 'Numlock', 'ScrollLock': 'ScrollLock',
	};

	function toEnigoKey(code: string, key: string): string | null {
		const mapped = CODE_TO_ENIGO[code];
		if (mapped) return mapped;

		if (/^Key[A-Z]$/.test(code)) {
			return `Unicode('${code.slice(3).toLowerCase()}')`;
		}
		if (/^Digit[0-9]$/.test(code)) {
			return `Unicode('${code.slice(5)}')`;
		}
		if (key.length === 1) {
			const escaped = key.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
			return `Unicode('${escaped}')`;
		}

		return null;
	}

	$effect(() => {
		if (!capturingField) return;
		const onKeyDown = (e: KeyboardEvent) => {
			e.preventDefault();
			e.stopPropagation();
			if (MODIFIER_CODES.has(e.code)) {
				heldModifiers.add(e.code);
				return;
			}
			const mods = Array.from(heldModifiers)
				.map((code) => toEnigoKey(code, ""))
				.filter((key): key is string => key !== null);
			const mainKey = toEnigoKey(e.code, e.key);
			if (!mainKey) {
				capturingField = null;
				heldModifiers = new Set();
				return;
			}
			const parts = [
				...mods.map(m => `k(${m},Press)`),
				`k(${mainKey},Click)`,
				...mods.map(m => `k(${m},Release)`),
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
	// Device comes from the action context (index 0), not from settings
	const currentDeviceId = $derived(instance.context.split(".")[0] ?? "");
	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

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
			parts.push(t("builtinInspector.pomodoro.workLabel", { minutes: work }));
			if (i < sessions - 1) parts.push(t("builtinInspector.pomodoro.breakLabel", { minutes: brk }));
		}
		parts.push(t("builtinInspector.pomodoro.longBreakLabel", { minutes: longBreak }));
		return `${parts.join(" -> ")} -> ${t("builtinInspector.pomodoro.repeat")}`;
	}
</script>

<div class="w-full h-full overflow-auto p-3 text-neutral-200">
	{#if instance.action.uuid === "omegadeck.builtin.runcommand"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">{t("builtinInspector.runCommand.title")}</h3>
			<p class="text-xs text-neutral-400">{@html t("builtinInspector.runCommand.description")}</p>

			<label class="block text-xs text-neutral-400" for="builtin-run-down">{isEncoder ? t("builtinInspector.runCommand.dialPress") : t("builtinInspector.runCommand.onPress")}</label>
			<input
				id="builtin-run-down"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "down")}
				placeholder={t("builtinInspector.runCommand.downPlaceholder")}
				onchange={(e) => updateSetting("down", e.currentTarget.value)}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-run-up">{isEncoder ? t("builtinInspector.runCommand.dialRelease") : t("builtinInspector.runCommand.onRelease")}</label>
			<input
				id="builtin-run-up"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "up")}
				onchange={(e) => updateSetting("up", e.currentTarget.value)}
			/>

			{#if isEncoder}
				<label class="block text-xs text-neutral-400" for="builtin-run-rotate">{t("builtinInspector.runCommand.dialRotate")}</label>
				<input
					id="builtin-run-rotate"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "rotate")}
					placeholder={t("builtinInspector.runCommand.rotatePlaceholder")}
					onchange={(e) => updateSetting("rotate", e.currentTarget.value)}
				/>
			{/if}

			<hr class="border-neutral-700 my-2" />
			<label class="block text-xs text-neutral-400" for="builtin-run-file">{t("builtinInspector.runCommand.saveOutputTo")}</label>
			<input
				id="builtin-run-file"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "file")}
				placeholder={t("builtinInspector.runCommand.optionalFilePath")}
				onchange={(e) => updateSetting("file", e.currentTarget.value)}
			/>
			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-run-show">
				<input
					id="builtin-run-show"
					type="checkbox"
					checked={readBool(currentSettings, "show", false)}
					onchange={(e) => updateSetting("show", e.currentTarget.checked)}
				/>
				{t("builtinInspector.runCommand.showOutputOnKey")}
			</label>
		</div>
	{:else if instance.action.uuid === "omegadeck.builtin.openurl"}
		<div class="space-y-2 max-w-3xl">
			<h3 class="text-sm font-semibold text-neutral-100">{t("builtinInspector.openUrl.title")}</h3>

			<label class="block text-xs text-neutral-400" for="builtin-url-down">{isEncoder ? t("builtinInspector.switchProfile.dialPress") : t("builtinInspector.runCommand.onPress")}</label>
			<input
				id="builtin-url-down"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "down")}
				onchange={(e) => updateSetting("down", e.currentTarget.value)}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-url-up">{isEncoder ? t("builtinInspector.runCommand.dialRelease") : t("builtinInspector.runCommand.onRelease")}</label>
			<input
				id="builtin-url-up"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "up")}
				onchange={(e) => updateSetting("up", e.currentTarget.value)}
			/>

			{#if isEncoder}
				<label class="block text-xs text-neutral-400" for="builtin-url-anticlockwise">{t("builtinInspector.openUrl.rotateLeft")}</label>
				<input
					id="builtin-url-anticlockwise"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "anticlockwise")}
					onchange={(e) => updateSetting("anticlockwise", e.currentTarget.value)}
				/>

				<label class="block text-xs text-neutral-400" for="builtin-url-clockwise">{t("builtinInspector.openUrl.rotateRight")}</label>
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
			<h3 class="text-sm font-semibold text-neutral-100">{t("builtinInspector.inputSimulation.title")}</h3>
			<p class="text-xs text-neutral-400">{@html t("builtinInspector.inputSimulation.description")}</p>

			{#if !isEncoder}
				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-down">{t("builtinInspector.inputSimulation.onPress")}</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'down' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'down') cancelCapture(); else capturingField = 'down'; }}
					>{capturingField === 'down' ? t("builtinInspector.inputSimulation.listening") : t("builtinInspector.inputSimulation.capture")}</button>
				</div>
				<textarea
					id="builtin-input-down"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("down", e.currentTarget.value)}
				>{readString(currentSettings, "down")}</textarea>

				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-up">{t("builtinInspector.inputSimulation.onReleaseOptional")}</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'up' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'up') cancelCapture(); else capturingField = 'up'; }}
					>{capturingField === 'up' ? t("builtinInspector.inputSimulation.listening") : t("builtinInspector.inputSimulation.capture")}</button>
				</div>
				<textarea
					id="builtin-input-up"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("up", e.currentTarget.value)}
				>{readString(currentSettings, "up")}</textarea>
			{:else}
				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-anticlockwise">{t("builtinInspector.inputSimulation.rotateLeft")}</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'anticlockwise' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'anticlockwise') cancelCapture(); else capturingField = 'anticlockwise'; }}
					>{capturingField === 'anticlockwise' ? t("builtinInspector.inputSimulation.listening") : t("builtinInspector.inputSimulation.capture")}</button>
				</div>
				<textarea
					id="builtin-input-anticlockwise"
					rows="2"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					onchange={(e) => updateSetting("anticlockwise", e.currentTarget.value)}
				>{readString(currentSettings, "anticlockwise")}</textarea>

				<div class="flex items-center justify-between">
					<label class="block text-xs text-neutral-400" for="builtin-input-clockwise">{t("builtinInspector.inputSimulation.rotateRight")}</label>
					<button
						class="text-xs px-2 py-0.5 rounded {capturingField === 'clockwise' ? 'bg-amber-600 text-white animate-pulse' : 'bg-neutral-600 hover:bg-neutral-500 text-neutral-200'}"
						onclick={() => { if (capturingField === 'clockwise') cancelCapture(); else capturingField = 'clockwise'; }}
					>{capturingField === 'clockwise' ? t("builtinInspector.inputSimulation.listening") : t("builtinInspector.inputSimulation.capture")}</button>
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
			<h3 class="text-sm font-semibold text-neutral-100">{t("builtinInspector.screenshot.title")}</h3>
			<label class="block text-xs text-neutral-400" for="builtin-screenshot-mode">{t("builtinInspector.screenshot.method")}</label>
			<select
				id="builtin-screenshot-mode"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readString(currentSettings, "mode") || "system_shortcut"}
				onchange={(e) => updateSetting("mode", e.currentTarget.value)}
			>
				<option value="system_shortcut">{t("builtinInspector.screenshot.systemShortcut")}</option>
				<option value="command">{t("builtinInspector.screenshot.customCommand")}</option>
			</select>

			{#if (readString(currentSettings, "mode") || "system_shortcut") == "command"}
				<label class="block text-xs text-neutral-400" for="builtin-screenshot-command">{t("builtinInspector.screenshot.command")}</label>
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
			<h3 class="text-sm font-semibold text-neutral-100">{t("builtinInspector.switchProfile.title")}</h3>
			<p class="text-xs text-neutral-400">{t("builtinInspector.switchProfile.description")}</p>

			<label class="block text-xs text-neutral-400" for="builtin-switch-profile">{isEncoder ? t("builtinInspector.switchProfile.dialPress") : t("builtinInspector.switchProfile.profile")}</label>
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
				<label class="block text-xs text-neutral-400" for="builtin-switch-anticlockwise">{t("builtinInspector.switchProfile.rotateLeft")}</label>
				<input
					id="builtin-switch-anticlockwise"
					list="builtin-profiles"
					class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
					value={readString(currentSettings, "anticlockwise") || "Default"}
					onchange={(e) => updateSetting("anticlockwise", e.currentTarget.value)}
				/>

				<label class="block text-xs text-neutral-400" for="builtin-switch-clockwise">{t("builtinInspector.switchProfile.rotateRight")}</label>
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
			<h3 class="text-sm font-semibold text-neutral-100">{instance.action.uuid.endsWith("up") ? t("builtinInspector.brightness.upTitle") : t("builtinInspector.brightness.downTitle")}</h3>
			<p class="text-xs text-neutral-400">{t("builtinInspector.brightness.description")}</p>
			<label class="block text-xs text-neutral-400" for="builtin-brightness-step">{t("builtinInspector.brightness.step", { value: readNumber(currentSettings, "step", 10) })}</label>
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
			<h3 class="text-sm font-semibold text-neutral-100">{t("builtinInspector.pomodoro.title")}</h3>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-work">{t("builtinInspector.pomodoro.work")}</label>
			<input
				id="builtin-pomodoro-work"
				type="number"
				min="1"
				max="120"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "work_minutes", 25)}
				onchange={(e) => updateMany({ work_minutes: Number.parseInt(e.currentTarget.value, 10) || 25 })}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-break">{t("builtinInspector.pomodoro.break")}</label>
			<input
				id="builtin-pomodoro-break"
				type="number"
				min="1"
				max="60"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "break_minutes", 5)}
				onchange={(e) => updateMany({ break_minutes: Number.parseInt(e.currentTarget.value, 10) || 5 })}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-longbreak">{t("builtinInspector.pomodoro.longBreak")}</label>
			<input
				id="builtin-pomodoro-longbreak"
				type="number"
				min="1"
				max="120"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "long_break_minutes", 15)}
				onchange={(e) => updateMany({ long_break_minutes: Number.parseInt(e.currentTarget.value, 10) || 15 })}
			/>

			<label class="block text-xs text-neutral-400" for="builtin-pomodoro-sessions">{t("builtinInspector.pomodoro.sessionsUntilLongBreak")}</label>
			<input
				id="builtin-pomodoro-sessions"
				type="number"
				min="1"
				max="10"
				class="w-full px-2 py-1 text-sm text-neutral-100 bg-neutral-700 border border-neutral-600 rounded"
				value={readNumber(currentSettings, "sessions_before_long_break", 4)}
				onchange={(e) => updateMany({ sessions_before_long_break: Number.parseInt(e.currentTarget.value, 10) || 4 })}
			/>

			<p class="text-xs text-neutral-400">{t("builtinInspector.pomodoro.cyclePreview")}</p>
			<p class="text-xs text-neutral-300">{cyclePreview(currentSettings)}</p>

			<hr class="border-neutral-700 my-2" />
			<p class="text-xs font-semibold text-neutral-300">{t("builtinInspector.pomodoro.alertsTitle")}</p>

			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-pomodoro-notify-system">
				<input
					id="builtin-pomodoro-notify-system"
					type="checkbox"
					checked={readBool(currentSettings, "notify_system", true)}
					onchange={(e) => updateSetting("notify_system", e.currentTarget.checked)}
				/>
				{t("builtinInspector.pomodoro.notifySystem")}
			</label>

			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-pomodoro-notify-sound">
				<input
					id="builtin-pomodoro-notify-sound"
					type="checkbox"
					checked={readBool(currentSettings, "notify_sound", true)}
					onchange={(e) => updateSetting("notify_sound", e.currentTarget.checked)}
				/>
				{t("builtinInspector.pomodoro.notifySound")}
			</label>

			<label class="inline-flex items-center gap-2 text-xs text-neutral-300" for="builtin-pomodoro-show-on-key">
				<input
					id="builtin-pomodoro-show-on-key"
					type="checkbox"
					checked={readBool(currentSettings, "show_on_key", true)}
					onchange={(e) => updateSetting("show_on_key", e.currentTarget.checked)}
				/>
				{t("builtinInspector.pomodoro.showOnKey")}
			</label>
		</div>
	{:else}
		<div class="text-sm text-neutral-400 p-2">
			{t("builtinInspector.unsupported")}
		</div>
	{/if}
</div>
