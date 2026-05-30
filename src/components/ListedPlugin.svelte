<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		icon,
		name,
		subtitle,
		hidden = false,
		disconnected = false,
		action,
		actionLabel = "",
		secondaryAction = undefined,
		secondaryActionLabel = "",
		children,
		secondary,
		subtitleSnippet,
	}: {
		icon: string;
		name: string;
		subtitle: string;
		hidden?: boolean;
		disconnected?: boolean;
		action: () => void;
		actionLabel?: string;
		secondaryAction?: (() => void) | undefined;
		secondaryActionLabel?: string;
		children?: Snippet;
		secondary?: Snippet;
		subtitleSnippet?: Snippet;
	} = $props();
</script>

<div
	class="flex flex-row items-center m-2 p-2 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
	class:hidden
>
	<img src={icon} class="w-24 h-24 rounded-lg" class:opacity-75={disconnected} alt={name} loading="lazy" />
	<div class="ml-4 mr-2 text-neutral-300 wrap-anywhere" class:opacity-75={disconnected}>
		<p class="font-semibold">{name}</p>
		{#if subtitleSnippet}{@render subtitleSnippet()}{:else}{subtitle}{/if}
	</div>

	<div class="flex flex-col ml-auto mr-4">
		{#if secondaryAction}
			<button onclick={secondaryAction} aria-label={secondaryActionLabel}>
				{@render secondary?.()}
			</button>
		{/if}
		<button onclick={action} aria-label={actionLabel}>
			{@render children?.()}
		</button>
	</div>
</div>
