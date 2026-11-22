<script lang="ts">
	interface Props {
		row: { id: number; label: string; hits: number };
		selected: boolean;
		onselect: () => void;
	}

	let { row, selected, onselect }: Props = $props();
</script>

<li
	class={['row flex cursor-pointer justify-between px-4 py-2', selected && 'selected']}
	onclick={onselect}
>
	<span>{row.label}</span>
	<span class="text-slate-400">{row.hits}</span>
</li>

<style>
	.row {
		border-bottom: 1px solid #e2e8f0;
	}
	.row.selected {
		background: #e0e7ff;
	}
</style>
