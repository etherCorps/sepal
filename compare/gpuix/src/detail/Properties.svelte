<!--
	The detail pane's properties (examples/tracker/src/lib/detail/Properties.svelte): a Select per
	field, and the labels with an Edit toggle that shows a Checkbox per label (gpuix-svelte-ui's).
-->
<script lang="ts">
	import { Checkbox } from 'gpuix-svelte-ui';
	import Btn from '../components/Btn.svelte';
	import LabelDot from '../components/LabelDot.svelte';
	import Pick from '../components/Pick.svelte';
	import { edit, estimate_items, label_items, people_items, priority_items, project_items, status_items } from '../edit.ts';
	import { label_by_id, type Issue, type PriorityId, type StatusId } from '../seed.ts';

	let { issue }: { issue: Issue } = $props();

	let editing_labels = $state(false);
	const assignees = $derived([{ value: 'none', label: 'Unassigned' }, ...people_items()]);

	function set_label(id: string, on: boolean) {
		const ids = on ? [...issue.labelIds, id] : issue.labelIds.filter((l) => l !== id);
		edit([issue.key], { labelIds: ids });
	}
</script>

<div class="props">
	<div class="line"><text class="name">Status</text><Pick testid="status-select" variant="ghost" value={issue.status} items={status_items} onpick={(v) => edit([issue.key], { status: v as StatusId })} /></div>
	<div class="line"><text class="name">Priority</text><Pick testid="priority-select" variant="ghost" value={issue.priority} items={priority_items} onpick={(v) => edit([issue.key], { priority: v as PriorityId })} /></div>
	<div class="line"><text class="name">Assignee</text><Pick testid="assignee-select" variant="ghost" value={issue.assignee ?? 'none'} items={assignees} onpick={(v) => edit([issue.key], { assignee: v === 'none' ? null : v })} /></div>
	<div class="line"><text class="name">Project</text><Pick testid="project-select" variant="ghost" value={issue.project} items={project_items} onpick={(v) => edit([issue.key], { project: v })} /></div>
	<div class="line">
		<text class="name">Estimate</text>
		<Pick testid="estimate-select" variant="ghost" value={issue.estimate === null ? 'none' : String(issue.estimate)} items={estimate_items} onpick={(v) => edit([issue.key], { estimate: v === 'none' ? null : Number(v) })} />
	</div>
	<div class="line top">
		<text class="name labels-name">Labels</text>
		<div class="badges">
			{#each issue.labelIds as id (id)}
				<div class="badge"><LabelDot label={id} /><text>{label_by_id[id]?.name}</text></div>
			{/each}
			<Btn testid="labels-edit" variant="ghost" size="xs" style="color: var(--muted-foreground)" onclick={() => (editing_labels = !editing_labels)}>
				{editing_labels ? 'Done' : issue.labelIds.length ? 'Edit' : 'Add labels'}
			</Btn>
		</div>
	</div>
</div>

{#if editing_labels}
	<div class="labels" testid="labels">
		{#each label_items as label (label.value)}
			<div class="choice">
				<Checkbox testid="label-{label.value}" checked={issue.labelIds.includes(label.value)} onchange={(on) => set_label(label.value, on)} />
				<LabelDot label={label.value} />
				<text class="choice-name">{label.label}</text>
			</div>
		{/each}
	</div>
{/if}

<style>
	.props { display: flex; flex-direction: column; }
	.line { display: flex; flex-direction: row; align-items: center; gap: 8px; min-height: 32px; }
	.line.top { align-items: start; }
	.name { width: 96px; flex-shrink: 0; font-size: 12px; color: var(--muted-foreground); }
	.labels-name { padding-top: 8px; }
	.badges { display: flex; flex-direction: row; flex-wrap: wrap; align-items: center; flex-grow: 1; flex-basis: 0; gap: 6px; padding: 4px 0; }
	.badge {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 6px;
		height: 20px;
		padding: 0 6px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		font-size: 11px;
		white-space: nowrap;
		color: var(--ink-2);
	}
	.labels {
		display: grid;
		grid-template-columns: 3;
		column-gap: 12px;
		row-gap: 8px;
		margin-top: 8px;
		padding: 12px;
		border-radius: 8px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
	}
	.choice { display: flex; flex-direction: row; align-items: center; gap: 8px; min-width: 0; font-size: 13px; color: var(--ink-2); }
	.choice-name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
</style>
