<script lang="ts">
	import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
	import DownloadSimple from "phosphor-svelte/lib/DownloadSimple";
	import Popup from "./Popup.svelte";
	import { _ } from "$lib/i18n";

	import "$lib/shims.ts";

	import { openUrl } from "$lib/api/commands";
	import { fetchPluginReadme, fetchTotalDownloadCount } from "$lib/services/pluginService";

	let { id, details, install, close }: {
		id: string;
		details: { repository: string; name: string; author: string; download_url: string | undefined };
		install: () => void;
		close: () => void;
	} = $props();

	const translate = $derived($_);
	const t = (key: string, values?: Record<string, unknown>) => translate(key, { values });

	let readme = $state(`<strong>${t("pluginDetails.loading")}</strong>`);
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
			const [domPurifyModule, markedModule, markedAlertModule, markedBaseUrlModule] = await Promise.all([
				import("dompurify"),
				import("marked"),
				import("marked-alert"),
				import("marked-base-url"),
			]);
			const DOMPurify = domPurifyModule.default;
			const { marked } = markedModule;
			const markedAlert = markedAlertModule.default;
			const { baseUrl } = markedBaseUrlModule;
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
				readme = await marked.parse(`**${t("pluginDetails.readmeMissing")}**\n\n[${t("pluginDetails.viewOnGithub")}](https://github.com/${repo})`);
			}

			downloadCount = await fetchTotalDownloadCount(repo);
		})();
	});
</script>

<Popup show label={t("pluginDetails.dialogLabel", { name: details.name })}>
	{#snippet children()}
	<button class="mr-2 my-1 float-right text-xl text-neutral-300" onclick={close} aria-label={t("pluginDetails.closeAria")}>✕</button>
	<div class="flex flex-row items-start">
		<img
			src={"https://openactionapi.github.io/plugins/icons/" + id + ".png"}
			alt={details.name}
			class="size-48 rounded-2xl"
		/>
		<div class="flex flex-col justify-center h-48 ml-8">
			<div class="text-3xl text-neutral-200">{details.name}</div>
			<div class="flex items-center mt-2 text-lg text-neutral-400">
				<span class="mr-2">{t("pluginDetails.by")}</span>
				<img
					src={"https://avatars.githubusercontent.com/" + details.repository.split("/")[3]}
					alt={t("pluginDetails.authorAvatarAlt")}
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
					{t("pluginDetails.install")}
				</button>

				<button
					onclick={() => openUrl(details.download_url ?? details.repository + "/releases/latest")}
					class="ml-1 p-3.5 active:translate-y-0.5 text-lg text-neutral-100 bg-indigo-600 hover:bg-indigo-500 transition-colors border border-indigo-500 rounded-r-lg"
					aria-label={t("pluginDetails.downloadLatestAria")}
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
