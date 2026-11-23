<!--
	The toast stack, bottom-right (examples/tracker/src/lib/overlays/Toasts.svelte), in a Portal so
	it paints over everything. Same rules (push() in ui.svelte.ts: at most 3, 5 s each). The enter
	animation is a 160 ms fade through GPUI's `motion`; the Tracker's 8 px slide and its shadow
	have no equivalent here.
-->
<script lang="ts">
	import Portal from 'gpuix-svelte/components/Portal.svelte';
	import Icon from '../components/Icon.svelte';
	import StatusIcon from '../components/StatusIcon.svelte';
	import { act, dismiss, toasts, ui } from '../ui.svelte.ts';

	const FADE = { initial: { opacity: 0 }, animate: { opacity: 1 }, transition: { duration: 0.16, ease: 'easeOut' } } as const;
</script>

{#if toasts.length > 0}
	<Portal>
		<div class="stack" style="bottom: {ui.bulk_bar ? 68 : 16}px">
			{#each toasts as toast (toast.id)}
				<div class="toast" testid="toast" motion={FADE}>
					{#if toast.status}
						<StatusIcon status={toast.status} size={16} />
					{:else if toast.tone === 'error'}
						<Icon name="circle-alert" color="var(--destructive)" />
					{:else}
						<Icon name="info" />
					{/if}
					<text class="text" testid="toast-text">{toast.text}</text>
					{#if toast.action}
						<button class="action" testid="toast-action" hitbox="self" onclick={() => act(toast.id)}>{toast.action.label}</button>
					{/if}
					<button class="close" testid="toast-close" hitbox="self" onclick={() => dismiss(toast.id)}><Icon name="x" size={14} /></button>
				</div>
			{/each}
		</div>
	</Portal>
{/if}

<style>
	.stack { position: absolute; right: 16px; display: flex; flex-direction: column; gap: 8px; width: 360px; }
	.toast {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 10px;
		min-height: 44px;
		padding: 10px 8px 10px 12px;
		border-radius: 8px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--popover);
		font-size: 13px;
		color: var(--foreground);
		pointer-events: auto;
	}
	.text { flex-grow: 1; flex-basis: 0; min-width: 0; }
	.action { display: flex; align-items: center; height: 28px; padding: 0 8px; border-radius: 6px; font-weight: 500; color: var(--primary); cursor: pointer; }
	.action:hover { background-color: var(--accent); }
	.close { display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; border-radius: 6px; cursor: pointer; }
	.close:hover { background-color: var(--accent); }
</style>
