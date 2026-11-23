<!-- A key hint: `⌘K`, `C`, `/`. Mono 11, bordered, on muted. -->
<script lang="ts">
	let { text }: { text: string } = $props();
</script>

<div class="kbd">{text}</div>

<style>
	.kbd {
		flex-shrink: 0;
		padding: 0 6px;
		border-radius: 4px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--muted);
		color: var(--muted-foreground);
		font-family: JetBrains Mono;
		font-size: 11px;
		font-weight: 500;
		line-height: 18px;
	}
</style>
