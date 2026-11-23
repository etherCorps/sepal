<!--
	`?`: every shortcut, from SHORTCUTS in ui.svelte.ts (docs/tracker-design.md, Shortcuts).
	Open state is `ui.shortcuts`.
-->
<script lang="ts">
	import { Dialog, DialogContent, DialogTitle } from '$lib/components/ui/dialog/index.js';
	import Kbd from '$lib/components/Kbd.svelte';
	import { SHORTCUTS, key_label, ui } from '$lib/ui.svelte';


	const columns = [
		['Global', 'Navigation'],
		['List', 'Issue']
	] as const;
</script>

<Dialog bind:open={ui.shortcuts}>
	<DialogContent data-testid="shortcuts" class="gap-5 rounded-lg p-5 shadow-sm sm:max-w-[480px]">
		<DialogTitle class="font-display text-foreground text-base font-medium tracking-[-0.01em]">Keyboard shortcuts</DialogTitle>
		<div class="grid grid-cols-2 gap-x-6">
			{#each columns as groups, column (column)}
				<div class="flex flex-col gap-4">
					{#each groups as group (group)}
						<section class="flex flex-col">
							<h3 class="text-muted-foreground mb-1 font-mono text-[11px] font-medium tracking-[0.06em] uppercase">{group}</h3>
							{#each SHORTCUTS.filter((s) => s.group === group) as shortcut (shortcut.keys)}
								<div class="flex h-7 items-center justify-between gap-3">
									<span class="text-ink-2 truncate text-[13px]">{shortcut.label}</span>
									<Kbd class="shrink-0">{key_label(shortcut.keys)}</Kbd>
								</div>
							{/each}
						</section>
					{/each}
				</div>
			{/each}
		</div>
	</DialogContent>
</Dialog>
