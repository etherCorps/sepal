<!--
	The toast stack, bottom-right (docs/tracker-design.md, Toast). `push()` in
	ui.svelte.ts keeps at most 3 and gives each a one-shot 5 s timeout, so no
	timer outlives its toast. The enter animation runs once (tw-animate-css);
	there is no exit animation.
-->
<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';
	import InfoIcon from '@lucide/svelte/icons/info';
	import CircleAlertIcon from '@lucide/svelte/icons/circle-alert';
	import { StatusIcon } from '$lib/glyphs';
	import { act, dismiss, toasts, ui } from '$lib/ui.svelte';
</script>

<!-- Above the bulk bar while it is up (bottom 16px, 44px tall), so it never sits under a toast. -->
<div class={['pointer-events-none fixed right-4 z-50 flex w-[360px] flex-col gap-2', ui.bulk_bar ? 'bottom-[68px]' : 'bottom-4']}>
	{#each toasts as toast (toast.id)}
		<div
			data-testid="toast"
			data-tone={toast.tone ?? 'default'}
			class="bg-popover text-foreground animate-in fade-in-0 slide-in-from-bottom-2 pointer-events-auto flex min-h-11 items-center gap-2.5 rounded-lg border py-2.5 pr-2 pl-3 text-[13px] shadow-sm duration-[160ms] ease-out"
		>
			{#if toast.status}
				<StatusIcon status={toast.status} size={16} />
			{:else if toast.tone === 'error'}
				<CircleAlertIcon class="text-destructive size-4 shrink-0" strokeWidth={1.75} />
			{:else}
				<InfoIcon class="text-muted-foreground size-4 shrink-0" strokeWidth={1.75} />
			{/if}
			<span data-testid="toast-text" class="min-w-0 flex-1">{toast.text}</span>
			{#if toast.action}
				<button
					data-testid="toast-action"
					class="text-primary hover:bg-accent h-7 rounded-md px-2 font-medium transition-colors"
					onclick={() => act(toast.id)}>{toast.action.label}</button
				>
			{/if}
			<button
				data-testid="toast-close"
				class="text-muted-foreground hover:bg-accent hover:text-foreground flex size-7 items-center justify-center rounded-md transition-colors"
				aria-label="Dismiss"
				onclick={() => dismiss(toast.id)}
			>
				<XIcon class="size-3.5" strokeWidth={1.75} />
			</button>
		</div>
	{/each}
</div>
