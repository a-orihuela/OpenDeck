<script lang="ts">
	import type { Snippet } from "svelte";
	import Question from "phosphor-svelte/lib/Question";

	let { children }: { children?: Snippet } = $props();

	let show = $state(false);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="inline-block cursor-help"
	onmouseenter={() => show = true}
	onmouseleave={() => show = false}
>
	<Question size="24" class="text-neutral-500" />
	{#if show}
		<div class="absolute mt-2 p-2 w-96 text-xs text-neutral-300 bg-neutral-700 border border-neutral-600 rounded-lg">
			{@render children?.()}
		</div>
	{/if}
</div>
