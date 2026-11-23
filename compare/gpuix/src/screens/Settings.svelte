<!--
	Settings (examples/tracker/src/routes/settings/+page.svelte): General and Data tabs, on
	gpuix-svelte-ui's Tabs, Toggle (the Tracker's Switch), Select and Dialog.
	Export CSV: this runtime has no save dialog, so the button reports that in a toast (the
	Tracker's failure path) instead of writing a file.
-->
<script lang="ts">
	import { Dialog, Select, Tabs, Toggle } from 'gpuix-svelte-ui';
	import Btn from '../components/Btn.svelte';
	import PageHeader from '../components/PageHeader.svelte';
	import { count, plural } from '../format.ts';
	import { persist_stats } from '../persist.ts';
	import { data, reset_edits, set_simulate, settings, store_settings, view, type GroupBy } from '../store.svelte.ts';
	import { push, use_modal } from '../ui.svelte.ts';

	const NAME_MAX = 40;
	const GROUPINGS: { id: GroupBy; name: string }[] = [
		{ id: 'status', name: 'Status' },
		{ id: 'assignee', name: 'Assignee' },
		{ id: 'priority', name: 'Priority' },
		{ id: 'project', name: 'Project' },
		{ id: 'none', name: 'No grouping' }
	];

	let tab = $state('general');
	let name = $derived(settings.displayName);
	function name_problem(text: string): string {
		const length = text.trim().length;
		if (length === 0) return "Name can't be empty.";
		if (length > NAME_MAX) return `Keep it to ${NAME_MAX} characters.`;
		return '';
	}
	const name_error = $derived(name_problem(name));

	async function store(message?: string) {
		await store_settings();
		if (!persist_stats.available) push({ text: 'Settings were not saved: no access to the data folder.', tone: 'error' });
		else if (message) push({ text: message });
	}

	function commit_name() {
		if (name_error) return;
		settings.displayName = name.trim();
		void store();
	}

	function set_flag(key: 'compact' | 'showLabels', on: boolean, label: string) {
		settings[key] = on;
		void store(`${label} ${on ? 'on' : 'off'}`);
	}

	function set_grouping(id: string) {
		const group = id as GroupBy;
		settings.defaultGroupBy = group;
		view.groupBy = group;
		void store(`Lists group by ${GROUPINGS.find((g) => g.id === group)?.name.toLowerCase()}`);
	}

	function set_teammates(on: boolean) {
		set_simulate(on);
		void store(on ? 'Teammates edit an issue every 2 seconds' : 'Teammates stopped');
	}

	function export_csv() {
		push({ text: 'Export failed: this runtime has no save dialog.', tone: 'error' });
	}

	let confirm_reset = $state(false);
	use_modal(() => confirm_reset);
	async function reset() {
		const edits = data.edits;
		confirm_reset = false;
		await reset_edits();
		push({ text: `Deleted ${plural(edits, 'local edit')}. Back to the sample data.` });
	}
</script>

{#snippet switch_row(id: string, title: string, description: string, checked: boolean, onchange: (on: boolean) => void)}
	<div class="line">
		<div class="text">
			<text class="title">{title}</text>
			<text class="description">{description}</text>
		</div>
		<Toggle testid={id} {checked} {onchange} />
	</div>
{/snippet}

<PageHeader title="Settings" />
<div class="scroll">
	<div class="page">
		<Tabs
			testid="settings-tab"
			items={[
				{ value: 'general', label: 'General' },
				{ value: 'data', label: 'Data' }
			]}
			value={tab}
			onchange={(v) => (tab = v)}
		/>
		{#if tab === 'general'}
			<div class="section">
				<div class="name">
					<text class="title">Display name</text>
					<input
						class="field"
						class:invalid={name_error !== ''}
						testid="settings-name"
						value={name}
						onchange={(e) => {
							name = e.value;
							if (!name_problem(e.value)) settings.displayName = e.value;
						}}
						onsubmit={commit_name}
						onblur={commit_name}
					/>
					<text class="help" class:error={name_error !== ''} testid="settings-name-help">{name_error || 'Shown on comments and activity.'}</text>
				</div>
				<div class="separator"></div>
				{@render switch_row('settings-compact', 'Compact rows', '28px rows instead of 36px, for more issues on screen.', settings.compact, (on) => set_flag('compact', on, 'Compact rows'))}
				<div class="separator"></div>
				{@render switch_row('settings-labels', 'Show labels in the list', 'Up to two labels beside each title. Compact rows hide them anyway.', settings.showLabels, (on) =>
					set_flag('showLabels', on, 'Labels in the list')
				)}
				<div class="separator"></div>
				<div class="line">
					<div class="text">
						<text class="title">Default grouping</text>
						<text class="description">How the issue lists group when the app starts.</text>
					</div>
					<Select testid="settings-grouping" options={GROUPINGS.map((g) => ({ value: g.id, label: g.name }))} value={settings.defaultGroupBy} onchange={set_grouping} />
				</div>
			</div>
		{:else}
			<div class="section">
				<text class="counts" testid="settings-counts">
					{plural(data.total, 'issue')} · {plural(data.edits, 'local edit')} · {persist_stats.available ? 'saved to Application Support' : 'not saved: no access to the data folder'}
				</text>
				<div class="separator"></div>
				{@render switch_row('settings-simulate', 'Simulate teammates', 'Edits a random issue every 2 seconds.', settings.simulate, set_teammates)}
				<div class="separator"></div>
				<div class="line">
					<div class="text">
						<text class="title">Export</text>
						<text class="description">All {count(data.total)} issues as a CSV file, with your edits.</text>
					</div>
					<Btn variant="outline" size="sm" testid="settings-export" onclick={export_csv}>Export CSV</Btn>
				</div>
				<div class="separator"></div>
				<div class="line">
					<div class="text">
						<text class="title">Sample data</text>
						<text class="description">{data.edits === 0 ? 'No local edits: this is the sample data.' : 'Drop every local edit and start again from the seed.'}</text>
					</div>
					<Btn variant="outline" size="sm" testid="settings-reset" style="color: var(--destructive)" disabled={data.edits === 0} onclick={() => (confirm_reset = true)}>Reset to sample data</Btn>
				</div>
			</div>
		{/if}
	</div>
</div>

<Dialog
	open={confirm_reset}
	onclose={() => (confirm_reset = false)}
	size="sm"
	title="Reset to sample data?"
	description="This deletes your {plural(data.edits, 'local edit')}. The {count(data.total)} sample issues stay."
	testid="reset-dialog"
>
	<div></div>
	{#snippet actions()}
		<Btn variant="ghost" testid="reset-cancel" onclick={() => (confirm_reset = false)}>Cancel</Btn>
		<Btn variant="destructive" testid="reset-confirm" onclick={reset}>Delete {plural(data.edits, 'edit')}</Btn>
	{/snippet}
</Dialog>

<style>
	.scroll { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-height: 0; overflow-y: scroll; scrollbar-color: var(--input); }
	.page { display: flex; flex-direction: column; gap: 24px; max-width: 640px; padding: 32px; }
	.section { display: flex; flex-direction: column; }
	.name { display: flex; flex-direction: column; gap: 6px; padding-bottom: 20px; }
	.field {
		max-width: 320px;
		height: 32px;
		padding: 0 10px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--input);
		font-size: 14px;
		color: var(--foreground);
	}
	.field.invalid { border-color: var(--destructive); }
	.help { min-height: 18px; font-size: 12px; color: var(--muted-foreground); }
	.help.error { color: var(--destructive); }
	.separator { height: 1px; flex-shrink: 0; background-color: var(--border); }
	.line { display: flex; flex-direction: row; align-items: center; gap: 24px; min-height: 60px; padding: 12px 0; }
	.text { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-width: 0; gap: 2px; }
	.title { font-size: 13px; font-weight: 500; color: var(--foreground); }
	.description { font-size: 12px; color: var(--muted-foreground); }
	.counts { padding-bottom: 20px; font-family: JetBrains Mono; font-size: 12px; color: var(--ink-2); }
</style>
