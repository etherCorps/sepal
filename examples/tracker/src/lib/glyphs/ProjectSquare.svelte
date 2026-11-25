<!-- A project's 10px colour square. -->
<script lang="ts">
	import { cn } from '#lib/utils.ts';
	import { project_by_key } from '#lib/seed.ts';

	let { project, class: className = '' }: { project: string; class?: string } = $props();
</script>

<span
	class={cn('inline-block size-2.5 shrink-0 rounded-[3px]', className)}
	style="background: var(--hue-{project_by_key[project]?.hue ?? 1});"
></span>
