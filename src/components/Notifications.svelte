<script lang="ts">
	import { fly } from "svelte/transition";
	import { appState, dismiss } from "$lib/notifications";
</script>

{#if appState.notifications.length > 0}
	<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm pointer-events-none">
		{#each appState.notifications as n (n.id)}
			<div
				transition:fly={{ y: 20, duration: 200 }}
				class="flex items-start gap-2 rounded-lg px-4 py-3 text-sm shadow-lg pointer-events-auto
					{n.level === 'error' ? 'bg-red-700 text-white' : n.level === 'warning' ? 'bg-yellow-600 text-white' : 'bg-gray-700 text-white'}"
			>
				<span class="flex-1 break-words">{n.message}</span>
				<button class="ml-2 opacity-70 hover:opacity-100" onclick={() => dismiss(n.id)}>✕</button>
			</div>
		{/each}
	</div>
{/if}
