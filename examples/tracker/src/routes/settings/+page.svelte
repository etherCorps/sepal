<!--
	Settings (docs/tracker-design.md, Settings): General and Data tabs. Every
	change writes settings.json through the store; switches say so in a toast.
	Export CSV opens the system's save dialog, so it only ever runs from a
	click, and tests never press it.
-->
<script lang="ts">
	import { save } from 'sepal/dialog';
	import { writeTextFile } from 'sepal/fs';
	import PageHeader from '#lib/components/PageHeader.svelte';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogTitle } from '#lib/components/ui/dialog/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { Label } from '#lib/components/ui/label/index.ts';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '#lib/components/ui/select/index.ts';
	import { Separator } from '#lib/components/ui/separator/index.ts';
	import { Switch } from '#lib/components/ui/switch/index.ts';
	import { Tabs, TabsContent, TabsList, TabsTrigger } from '#lib/components/ui/tabs/index.ts';
	import { count, plural } from '#lib/format.ts';
	import { persist_stats } from '#lib/persist.ts';
	import { data, reset_edits, set_simulate, settings, store_settings, to_csv, view, type GroupBy } from '#lib/store.svelte.ts';
	import { push } from '#lib/ui.svelte.ts';

	const NAME_MAX = 40;
	const GROUPINGS: { id: GroupBy; name: string }[] = [
		{ id: 'status', name: 'Status' },
		{ id: 'assignee', name: 'Assignee' },
		{ id: 'priority', name: 'Priority' },
		{ id: 'project', name: 'Project' },
		{ id: 'none', name: 'No grouping' }
	];

	// Follows settings (they arrive one turn after mount) until the user types.
	let name = $derived(settings.displayName);
	function name_problem(text: string): string {
		const length = text.trim().length;
		if (length === 0) return "Name can't be empty.";
		if (length > NAME_MAX) return `Keep it to ${NAME_MAX} characters.`;
		return '';
	}
	const name_error = $derived(name_problem(name));

	/** Save settings.json; say so if the disk refused. */
	async function store(message?: string) {
		await store_settings();
		if (!persist_stats.available) push({ text: 'Settings were not saved: no access to the data folder.', tone: 'error' });
		else if (message) push({ text: message });
	}

	function commit_name() {
		if (name_error) return;
		// Already live in `settings` while typing, so always write.
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

	let exporting = $state(false);
	async function export_csv() {
		if (exporting) return;
		exporting = true;
		try {
			const path = await save({ title: 'Export issues', defaultName: 'tracker-issues.csv', filters: [{ name: 'CSV', extensions: ['csv'] }] });
			if (!path) return;
			const list = data.issues;
			await writeTextFile(path, to_csv(list));
			push({ text: `Exported ${plural(list.length, 'issue')} to ${path.split(/[\\/]/).pop()}` });
		} catch (error) {
			push({ text: `Export failed: ${(error as Error)?.message ?? error}`, tone: 'error' });
		} finally {
			exporting = false;
		}
	}

	let confirm_reset = $state(false);
	async function reset() {
		const edits = data.edits;
		confirm_reset = false;
		await reset_edits();
		push({ text: `Deleted ${plural(edits, 'local edit')}. Back to the sample data.` });
	}
</script>

{#snippet switch_row(id: string, title: string, description: string, checked: boolean, onchange: (on: boolean) => void)}
	<div class="flex min-h-[60px] items-center gap-6 py-3">
		<div class="flex min-w-0 flex-1 flex-col gap-0.5">
			<Label for={id} class="text-foreground text-[13px] font-medium">{title}</Label>
			<span class="text-muted-foreground text-xs">{description}</span>
		</div>
		<Switch {id} data-testid={id} {checked} onCheckedChange={onchange} />
	</div>
{/snippet}

<PageHeader title="Settings" />
<div class="min-h-0 flex-1 overflow-y-scroll">
	<div class="flex max-w-[640px] flex-col p-8">
		<Tabs value="general" class="gap-6">
			<TabsList>
				<TabsTrigger value="general" data-testid="settings-tab-general">General</TabsTrigger>
				<TabsTrigger value="data" data-testid="settings-tab-data">Data</TabsTrigger>
			</TabsList>

			<TabsContent value="general" class="flex flex-col">
				<div class="flex flex-col gap-1.5 pb-5">
					<Label for="settings-name" class="text-foreground text-[13px]">Display name</Label>
					<Input
						id="settings-name"
						data-testid="settings-name"
						class="h-8 max-w-[320px]"
						aria-invalid={name_error ? 'true' : undefined}
						aria-describedby="settings-name-help"
						bind:value={name}
						oninput={(event: Event) => {
							// Live in the sidebar while valid; written on Enter or when the field loses focus.
							// The value from the event: this runs before `bind:value` has updated `name`.
							const value = (event.currentTarget as HTMLInputElement).value;
							if (!name_problem(value)) settings.displayName = value;
						}}
						onkeydown={(event: KeyboardEvent) => {
							if (event.key === 'Enter') commit_name();
						}}
						onblur={commit_name}
					/>
					<p
						id="settings-name-help"
						data-testid="settings-name-help"
						class={['min-h-[18px] text-xs', name_error ? 'text-destructive' : 'text-muted-foreground']}
					>
						{name_error || 'Shown on comments and activity.'}
					</p>
				</div>
				<Separator />
				{@render switch_row(
					'settings-compact',
					'Compact rows',
					'28px rows instead of 36px, for more issues on screen.',
					settings.compact,
					(on) => set_flag('compact', on, 'Compact rows')
				)}
				<Separator />
				{@render switch_row(
					'settings-labels',
					'Show labels in the list',
					'Up to two labels beside each title. Compact rows hide them anyway.',
					settings.showLabels,
					(on) => set_flag('showLabels', on, 'Labels in the list')
				)}
				<Separator />
				<div class="flex min-h-[60px] items-center gap-6 py-3">
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="text-foreground text-[13px] font-medium">Default grouping</span>
						<span class="text-muted-foreground text-xs">How the issue lists group when the app starts.</span>
					</div>
					<Select type="single" value={settings.defaultGroupBy} onValueChange={set_grouping}>
						<SelectTrigger data-testid="settings-grouping" size="sm" class="w-36" aria-label="Default grouping">
							{GROUPINGS.find((g) => g.id === settings.defaultGroupBy)?.name}
						</SelectTrigger>
						<SelectContent>
							{#each GROUPINGS as group (group.id)}
								<SelectItem value={group.id} label={group.name} />
							{/each}
						</SelectContent>
					</Select>
				</div>
			</TabsContent>

			<TabsContent value="data" class="flex flex-col">
				<p data-testid="settings-counts" class="text-ink-2 pb-5 font-mono text-xs">
					{plural(data.total, 'issue')} · {plural(data.edits, 'local edit')} · {persist_stats.available
						? 'saved to Application Support'
						: 'not saved: no access to the data folder'}
				</p>
				<Separator />
				{@render switch_row(
					'settings-simulate',
					'Simulate teammates',
					'Edits a random issue every 2 seconds.',
					settings.simulate,
					set_teammates
				)}
				<Separator />
				<div class="flex min-h-[60px] items-center gap-6 py-3">
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="text-foreground text-[13px] font-medium">Export</span>
						<span class="text-muted-foreground text-xs">All {count(data.total)} issues as a CSV file, with your edits.</span>
					</div>
					<Button variant="outline" size="sm" data-testid="settings-export" disabled={exporting} onclick={export_csv}>Export CSV</Button>
				</div>
				<Separator />
				<div class="flex min-h-[60px] items-center gap-6 py-3">
					<div class="flex min-w-0 flex-1 flex-col gap-0.5">
						<span class="text-foreground text-[13px] font-medium">Sample data</span>
						<span class="text-muted-foreground text-xs">
							{data.edits === 0 ? 'No local edits: this is the sample data.' : 'Drop every local edit and start again from the seed.'}
						</span>
					</div>
					<Button
						variant="outline"
						size="sm"
						data-testid="settings-reset"
						class="text-destructive hover:text-destructive"
						disabled={data.edits === 0}
						onclick={() => (confirm_reset = true)}>Reset to sample data</Button
					>
				</div>
			</TabsContent>
		</Tabs>
	</div>
</div>

<Dialog bind:open={confirm_reset}>
	<DialogContent data-testid="reset-dialog" class="gap-4 rounded-lg p-5 shadow-sm sm:max-w-[420px]" showCloseButton={false}>
		<div class="flex flex-col gap-1.5">
			<DialogTitle class="font-display text-foreground text-base font-medium tracking-[-0.01em]">Reset to sample data?</DialogTitle>
			<DialogDescription class="text-ink-2 text-[13px]">
				This deletes your {plural(data.edits, 'local edit')}. The {count(data.total)} sample issues stay.
			</DialogDescription>
		</div>
		<DialogFooter class="gap-2">
			<Button variant="ghost" data-testid="reset-cancel" onclick={() => (confirm_reset = false)}>Cancel</Button>
			<Button variant="destructive" data-testid="reset-confirm" onclick={reset}>Delete {plural(data.edits, 'edit')}</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>
