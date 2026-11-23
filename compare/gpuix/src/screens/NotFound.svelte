<!-- examples/tracker/src/routes/+error.svelte -->
<script lang="ts">
	import { navigate } from 'gpuix-svelte-ui';
	import PageHeader from '../components/PageHeader.svelte';

	let { message = 'Not found' }: { message?: string } = $props();
</script>

<PageHeader title="Not found" />
<div class="body">
	<text class="message" testid="error-message">{message}</text>
	<button class="link" hitbox="self" onclick={() => navigate('/issues')}>Back to issues</button>
</div>

<style>
	.body { display: flex; flex-direction: column; align-items: start; gap: 8px; padding: 32px; }
	.message { font-size: 14px; font-weight: 500; color: var(--foreground); }
	.link { font-size: 13px; color: var(--primary); cursor: pointer; }
</style>
