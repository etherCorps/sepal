<!--
	The whole update flow an app has to offer: look for an update, say what was
	found, download it, install it.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { check, type Update } from '@ethercorps/svelte-blitz/updater';

	type Status = 'idle' | 'checking' | 'up to date' | 'available' | 'downloading' | 'ready' | 'installing' | 'error';

	let status = $state<Status>('idle');
	let message = $state('');
	let update = $state.raw<Update | null>(null);

	function failed(error: unknown) {
		status = 'error';
		message = error instanceof Error ? error.message : String(error);
	}

	async function look() {
		status = 'checking';
		message = '';
		try {
			update = await check();
			status = update ? 'available' : 'up to date';
		} catch (error) {
			failed(error);
		}
	}

	async function download() {
		if (!update) return;
		status = 'downloading';
		try {
			// Fetched and its signature verified. Nothing is replaced yet.
			await update.download();
			status = 'ready';
		} catch (error) {
			failed(error);
		}
	}

	async function install() {
		if (!update) return;
		status = 'installing';
		try {
			// Replaces the app and starts the new version; this never returns on success.
			await update.install();
		} catch (error) {
			failed(error);
		}
	}

	// Built with VITE_AUTO_UPDATE=1 the app updates itself at launch without
	// asking: the "silent updater" pattern, written in app code.
	onMount(async () => {
		if (import.meta.env.VITE_AUTO_UPDATE !== '1') return;
		await look();
		if (update) await install();
	});
</script>

<main class="flex h-screen flex-col gap-4 bg-slate-50 p-6 font-sans text-slate-900">
	<h1 class="text-2xl font-semibold">Updater demo</h1>
	<p>Version <strong id="version">{__APP_VERSION__}</strong></p>

	<div class="flex gap-3">
		<button id="check" class="rounded-lg bg-indigo-600 px-4 py-2 font-medium text-white" onclick={look}>
			check for updates
		</button>
		{#if update}
			<button id="download" class="rounded-lg border border-slate-300 px-4 py-2" onclick={download}>download</button>
			<button id="install" class="rounded-lg border border-slate-300 px-4 py-2" onclick={install}>
				install and restart
			</button>
		{/if}
	</div>

	<p>Status: <span id="status">{status}</span></p>
	{#if update}
		<p id="found">Version {update.version} is available. {update.notes ?? ''}</p>
	{/if}
	{#if message}
		<p id="message" class="text-rose-600">{message}</p>
	{/if}
</main>
