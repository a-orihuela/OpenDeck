<script lang="ts">
	import type { Snippet } from "svelte";
	import { tick } from "svelte";

	let { show = false, label = "", children }: { show?: boolean; label?: string; children?: Snippet } = $props();

	let popupEl: HTMLDivElement | undefined = $state(undefined);
	let previousFocus: HTMLElement | null = $state(null);

	$effect(() => {
		if (show) {
			previousFocus = document.activeElement as HTMLElement | null;
			tick().then(() => popupEl?.focus());
		} else if (previousFocus) {
			previousFocus.focus();
			previousFocus = null;
		}
		return () => { previousFocus?.focus(); };
	});
</script>

{#if show}
	<!-- Backdrop -->
	<div class="fixed inset-0 bg-black/40 z-40"></div>
	<div
		bind:this={popupEl}
		class="fixed top-4 left-4 right-4 bottom-4 m-auto p-4 max-w-3xl max-h-[90vh] bg-neutral-800 border border-neutral-700 rounded-lg overflow-auto z-50"
		role="dialog"
		tabindex="-1"
		aria-label={label}
	>
		{@render children?.()}
	</div>
{/if}
