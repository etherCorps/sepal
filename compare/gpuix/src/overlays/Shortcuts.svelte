<!-- `?`: every shortcut (examples/tracker/src/lib/overlays/Shortcuts.svelte), in gpuix-svelte-ui's Dialog. -->
<script lang="ts">
	import { Dialog } from 'gpuix-svelte-ui';
	import Kbd from '../components/Kbd.svelte';
	import { SHORTCUTS, key_label, ui } from '../ui.svelte.ts';

	const columns = [
		['Global', 'Navigation'],
		['List', 'Issue']
	] as const;
</script>

<Dialog open={ui.shortcuts} onclose={() => (ui.shortcuts = false)} title="Keyboard shortcuts" testid="shortcuts">
	<div class="columns">
		{#each columns as groups, column (column)}
			<div class="column">
				{#each groups as group (group)}
					<div class="group">
						<text class="heading">{group.toUpperCase()}</text>
						{#each SHORTCUTS.filter((s) => s.group === group) as shortcut (shortcut.keys)}
							<div class="line">
								<text class="label">{shortcut.label}</text>
								<Kbd text={key_label(shortcut.keys)} />
							</div>
						{/each}
					</div>
				{/each}
			</div>
		{/each}
	</div>
</Dialog>

<style>
	.columns { display: flex; flex-direction: row; gap: 24px; }
	.column { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-width: 0; gap: 16px; }
	.group { display: flex; flex-direction: column; }
	.heading { margin-bottom: 4px; font-family: JetBrains Mono; font-size: 11px; font-weight: 500; color: var(--muted-foreground); }
	.line { display: flex; flex-direction: row; align-items: center; justify-content: space-between; gap: 12px; height: 28px; }
	.label { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-size: 13px; color: var(--ink-2); }
</style>
