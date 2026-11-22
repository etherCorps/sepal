<!--
	One button per desktop API. Each needs an entry in `permissions`
	(app.config.ts); take one out and the call is refused with a
	message that says what to add.
-->
<script lang="ts">
	import { readText, writeText } from '@ethercorps/svelte-blitz/clipboard';
	import { confirm, open as openDialog, save } from '@ethercorps/svelte-blitz/dialog';
	import { dir, readTextFile, writeTextFile } from '@ethercorps/svelte-blitz/fs';
	import { open } from '@ethercorps/svelte-blitz/shell';
	import { isMaximized, maximize, minimize, setTitle, unmaximize } from '@ethercorps/svelte-blitz/window';

	import { onMount } from 'svelte';

	let output = $state('Press a button.');
	let saves = $state(0);

	/** Run something and show what came back, or why it was refused. */
	async function run(label: string, action: () => Promise<unknown>) {
		output = `${label}…`;
		try {
			const result = await action();
			output = `${label}: ${typeof result === 'string' ? result : JSON.stringify(result)}`;
		} catch (error) {
			output = `${label} failed: ${error instanceof Error ? error.message : String(error)}`;
		}
	}

	const actions: [id: string, label: string, action: () => Promise<unknown>][] = [
		[
			'fetch',
			'fetch',
			async () => {
				const response = await fetch('https://api.github.com/zen');
				return `${response.status} ${await response.text()}`;
			}
		],
		[
			'write',
			'write a file',
			async () => {
				// 'data' is this app's own directory; it is created on first write.
				await writeTextFile('notes/count.txt', String(++saves), { dir: 'data' });
				return `saved ${saves} to ${await dir('data')}/notes/count.txt`;
			}
		],
		['read', 'read it back', () => readTextFile('notes/count.txt', { dir: 'data' })],
		[
			'open-file',
			'open a file…',
			async () => {
				const path = await openDialog({ title: 'Choose a text file', filters: [{ name: 'Text', extensions: ['txt', 'md'] }] });
				// The user chose it, so it may be read whatever `permissions.fs` says.
				return path ? (await readTextFile(path)).slice(0, 200) : 'cancelled';
			}
		],
		[
			'save-file',
			'save as…',
			async () => {
				const path = await save({ defaultName: 'hello.txt' });
				if (!path) return 'cancelled';
				await writeTextFile(path, 'Hello from Svelte on Blitz\n');
				return `wrote ${path}`;
			}
		],
		['confirm', 'ask a question', async () => ((await confirm('Do it?', { title: 'Desktop APIs' })) ? 'yes' : 'no')],
		['copy', 'copy', () => writeText('Copied from Svelte on Blitz').then(() => 'copied')],
		['paste', 'paste', () => readText()],
		['open-link', 'open a link', () => open('https://svelte.dev').then(() => 'opened')],
		['title', 'set the title', () => setTitle(`Desktop APIs (${new Date().toLocaleTimeString()})`).then(() => 'done')],
		['minimize', 'minimise', () => minimize().then(() => 'done')],
		[
			'maximize',
			'maximise / restore',
			async () => {
				const maximized = await isMaximized();
				await (maximized ? unmaximize() : maximize());
				return maximized ? 'restored' : 'maximised';
			}
		]
	];

	// Built with VITE_AUTO=<button id> the app presses that button by itself at
	// launch. scripts and manual checks use it to exercise a real window.
	onMount(() => {
		const auto = actions.find(([id]) => id === import.meta.env.VITE_AUTO);
		if (auto) run(auto[1], auto[2]);
	});
</script>

<main class="flex h-screen flex-col gap-4 bg-slate-50 p-6 font-sans text-slate-900">
	<h1 class="text-2xl font-semibold">Desktop APIs</h1>

	<div class="flex flex-wrap gap-2">
		{#each actions as [id, label, action] (id)}
			<button {id} class="rounded-lg border border-slate-300 bg-white px-3 py-1.5 hover:bg-slate-100" onclick={() => run(label, action)}>
				{label}
			</button>
		{/each}
	</div>

	<p id="output" class="min-h-0 flex-1 rounded-lg border border-slate-200 bg-white p-4 text-sm">{output}</p>
</main>
