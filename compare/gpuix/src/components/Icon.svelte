<!-- A Lucide icon (icons.ts) at `size` px, tinted with its `color`. -->
<script lang="ts">
	import { icon, type IconName } from '../icons.ts';

	let { name, size = 16, color = 'var(--muted-foreground)' }: { name: IconName; size?: number; color?: string } = $props();
</script>

<svg source={icon(name)} style="width: {size}px; height: {size}px; flex-shrink: 0; color: {color}"></svg>
