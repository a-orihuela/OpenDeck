<script lang="ts">
	import { notifications, dismiss } from "$lib/notifications";
</script>

{#if $notifications.length > 0}
	<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm pointer-events-none">
		{#each $notifications as n (n.id)}
			<div
				class="flex items-start gap-2 rounded-lg px-4 py-3 text-sm shadow-lg pointer-events-auto
					{n.level === 'error' ? 'bg-red-700 text-white' : n.level === 'warning' ? 'bg-yellow-600 text-white' : 'bg-gray-700 text-white'}"
			>
				<span class="flex-1 break-words">{n.message}</span>
				<button class="ml-2 opacity-70 hover:opacity-100" on:click={() => dismiss(n.id)}>✕</button>
			</div>
		{/each}
	</div>
{/if}
