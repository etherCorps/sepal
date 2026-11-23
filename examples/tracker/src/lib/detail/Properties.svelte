<!--
	The detail pane's properties: a Select per field, and the labels with an
	Edit toggle that shows a Checkbox per label. Every change is one edit with
	an Undo toast.
-->
<script lang="ts">
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { LabelDot } from '$lib/glyphs';
	import Pick from '$lib/list/Pick.svelte';
	import { edit, estimate_items, label_items, people_items, priority_items, project_items, status_items } from '$lib/list/edit';
	import { label_by_id, type Issue, type PriorityId, type StatusId } from '$lib/seed';

	let { issue }: { issue: Issue } = $props();

	let editing_labels = $state(false);
	const assignees = $derived([{ value: 'none', label: 'Unassigned' }, ...people_items()]);

	function set_label(id: string, on: boolean) {
		const ids = on ? [...issue.labelIds, id] : issue.labelIds.filter((l) => l !== id);
		edit([issue.key], { labelIds: ids });
	}
</script>

<div class="grid grid-cols-[96px_minmax(0,1fr)] items-center gap-x-2 [&>*]:min-h-8">
	<span class="text-muted-foreground flex items-center text-xs">Status</span>
	<div class="flex items-center">
		<Pick
			testid="status-select"
			variant="ghost"
			kind="status"
			value={issue.status}
			items={status_items}
			onpick={(v) => edit([issue.key], { status: v as StatusId })}
		/>
	</div>
	<span class="text-muted-foreground flex items-center text-xs">Priority</span>
	<div class="flex items-center">
		<Pick
			testid="priority-select"
			variant="ghost"
			kind="priority"
			value={issue.priority}
			items={priority_items}
			onpick={(v) => edit([issue.key], { priority: v as PriorityId })}
		/>
	</div>
	<span class="text-muted-foreground flex items-center text-xs">Assignee</span>
	<div class="flex items-center">
		<Pick
			testid="assignee-select"
			variant="ghost"
			kind="assignee"
			value={issue.assignee ?? 'none'}
			items={assignees}
			onpick={(v) => edit([issue.key], { assignee: v === 'none' ? null : v })}
		/>
	</div>
	<span class="text-muted-foreground flex items-center text-xs">Project</span>
	<div class="flex items-center">
		<Pick
			testid="project-select"
			variant="ghost"
			kind="project"
			value={issue.project}
			items={project_items}
			onpick={(v) => edit([issue.key], { project: v })}
		/>
	</div>
	<span class="text-muted-foreground flex items-center text-xs">Estimate</span>
	<div class="flex items-center">
		<Pick
			testid="estimate-select"
			variant="ghost"
			value={issue.estimate === null ? 'none' : String(issue.estimate)}
			items={estimate_items}
			onpick={(v) => edit([issue.key], { estimate: v === 'none' ? null : Number(v) })}
		/>
	</div>
	<span class="text-muted-foreground flex items-center self-start pt-2 text-xs">Labels</span>
	<div class="flex flex-wrap items-center gap-1.5 py-1">
		{#each issue.labelIds as id (id)}
			<Badge variant="outline" class="text-ink-2 h-5 gap-1.5 rounded-md px-1.5 text-[11px] font-normal">
				<LabelDot label={id} />{label_by_id[id]?.name}
			</Badge>
		{/each}
		<Button data-testid="labels-edit" variant="ghost" size="xs" class="text-muted-foreground" onclick={() => (editing_labels = !editing_labels)}>
			{editing_labels ? 'Done' : issue.labelIds.length ? 'Edit' : 'Add labels'}
		</Button>
	</div>
</div>

{#if editing_labels}
	<div data-testid="labels" class="mt-2 grid grid-cols-3 gap-x-3 gap-y-2 rounded-lg border p-3">
		{#each label_items as label (label.value)}
			<label class="text-ink-2 flex min-w-0 items-center gap-2 text-[13px]">
				<Checkbox
					data-testid="label-{label.value}"
					bind:checked={() => issue.labelIds.includes(label.value), (on) => set_label(label.value, on)}
				/>
				<LabelDot label={label.value} />
				<span class="truncate">{label.label}</span>
			</label>
		{/each}
	</div>
{/if}
