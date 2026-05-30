<script lang="ts">
	import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
	import DownloadSimple from "phosphor-svelte/lib/DownloadSimple";
	import Popup from "./Popup.svelte";

	import "$lib/shims.ts";

	import { openUrl } from "$lib/api/commands";
	import { fetchPluginReadme, fetchTotalDownloadCount } from "$lib/services/pluginService";
	import DOMPurify from "dompurify";
	import { marked } from "marked";
	import markedAlert from "marked-alert";
	import { baseUrl } from "marked-base-url";

	let { id, details, install, close }: {
		id: string;
		details: { repository: string; name: string; author: string; download_url: string | undefined };
		install: () => void;
		close: () => void;
	} = $props();

	let readme = $state("<strong>Loading plugin details...</strong>");
	let downloadCount = $state(0);

	function handleReadmeClick(event: MouseEvent | KeyboardEvent) {
		const link = (event.target as HTMLElement).closest("a");
		if (link && link.href) {
			event.preventDefault();
			window.open(link.href);
		}
	}

	$effect(() => {
		const repo = details.repository.split("/")[3] + "/" + details.repository.split("/")[4];

		(async () => {
			const renderer = new marked.Renderer();
			renderer.link = function (token) {
				return marked.Renderer.prototype.link.call(this, token).replace("<a", `<a target="_blank" `);
			};
			marked.use({ renderer });

			const result = await fetchPluginReadme(repo);
			if (result) {
				marked.use(markedAlert());
				marked.use(baseUrl(result.baseUrl));
				readme = await marked.parse(DOMPurify.sanitize(result.markdown).replace(/<a/g, '<a target="_blank" '));
			} else {
				readme = await marked.parse(`**Plugin README file not found**\n\n[View plugin on GitHub](https://github.com/${repo})`);
			}

			downloadCount = await fetchTotalDownloadCount(repo);
		})();
	});
</script>

<Popup show label="{details.name} plugin details">
	{#snippet children()}
	<button class="mr-2 my-1 float-right text-xl text-neutral-300" onclick={close} aria-label="Close">✕</button>
	<div class="flex flex-row items-start">
		<img
			src={"https://openactionapi.github.io/plugins/icons/" + id + ".png"}
			alt={details.name}
			class="size-48 rounded-2xl"
		/>
		<div class="flex flex-col justify-center h-48 ml-8">
			<div class="text-3xl text-neutral-200">{details.name}</div>
			<div class="flex items-center mt-2 text-lg text-neutral-400">
				<span class="mr-2">by</span>
				<img
					src={"https://avatars.githubusercontent.com/" + details.repository.split("/")[3]}
					alt="Author avatar"
					class="size-7 mr-1.5 rounded-full"
				/>
				<a
					target="_blank"
					href={"https://github.com/" + details.repository.split("/")[3]}
					onclick={() => window.open("https://github.com/" + details.repository.split("/")[3])}
					class="underline"
				>
					{details.author}
					{#if details.repository.split("/")[3] != details.author}
						({details.repository.split("/")[3]})
					{/if}
				</a>
			</div>

			<div class="flex flex-row items-center mt-6">
				<button
					onclick={install}
					class="px-8 py-3 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 hover:bg-indigo-500 transition-colors border border-indigo-500 rounded-l-lg"
				>
					Install
				</button>

				<button
					onclick={() => openUrl(details.download_url ?? details.repository + "/releases/latest")}
					class="ml-1 p-3.5 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 hover:bg-indigo-500 transition-colors border border-indigo-500 rounded-r-lg"
					aria-label="Download latest release from GitHub"
				>
					<ArrowSquareOut size={24} />
				</button>

				{#if downloadCount}
					<div class="flex flex-row ml-6 text-neutral-300">
						<span class="mr-1 text-lg">{downloadCount}</span>
						<DownloadSimple size={28} />
					</div>
				{/if}
			</div>
		</div>
	</div>

	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="mt-4 p-6 plugin-readme text-neutral-300 border-4 border-neutral-600 rounded-xl"
		onclick={handleReadmeClick}
		onkeyup={handleReadmeClick}
		role="region"
	>
		{@html readme}
	</div>
	{/snippet}
</Popup>
