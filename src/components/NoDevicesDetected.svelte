<script lang="ts">
	import { _ } from "$lib/i18n";
	import { PRODUCT_NAME } from "$lib/singletons";

	import { getBuildInfo, restart } from "$lib/api/commands";

	let buildInfo = $state("");
	(async () => { buildInfo = await getBuildInfo(); })();
</script>

<div class="flex flex-col justify-center items-center w-full h-full text-center text-neutral-300">
	<div class="w-80 text-sm">
		<h2 class="text-lg font-bold mb-2">{$_("noDevices.title")}</h2>
		<p class="mb-2">{$_("noDevices.description")}</p>
		{#if buildInfo?.split("</summary>")[0]?.includes("linux")}
			<p class="mb-2">{$_("noDevices.linuxHint")}</p>
		{/if}
		<p class="mb-4">{$_("noDevices.pluginHint")}</p>
		<button
			class="px-2 py-1 text-sm text-neutral-300 bg-neutral-700 hover:bg-neutral-600 transition-colors border border-neutral-600 rounded-lg"
			onclick={() => restart()}
		>
			{$_("noDevices.restart", { values: { product: PRODUCT_NAME } })}
		</button>
	</div>
</div>
