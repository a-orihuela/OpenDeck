<script lang="ts">
	import type { Snippet } from "svelte";
	import "../app.css";
	import Notifications from "../components/Notifications.svelte";

	let { children }: { children: Snippet } = $props();
</script>

{@render children()}
<Notifications />
