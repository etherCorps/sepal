<!--
	Icon rendering probe. Render it with:

	  pnpm --filter icons build
	  cargo run --manifest-path crates/host/Cargo.toml --features screenshot -- \
	    --screenshot icons.png examples/icons/dist 640x360

	Rows 1 and 3 render correctly today. Rows 2 and 4 do not: see HANDOVER.md, task 2.
-->
<script lang="ts">
	import { Camera, Heart, Settings } from '@lucide/svelte';

	const circle = 'M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z';
</script>

<main class="flex flex-col gap-4 bg-white p-6 text-slate-900">
	<section id="inline-hex" class="flex items-center gap-4" style="color: #4f46e5">
		<span class="w-40">1. inline, hex colour</span>
		<svg class="stroke" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<circle cx="12" cy="12" r="10" /><path d="M8 12h8" />
		</svg>
		<svg class="fill" width="32" height="32" viewBox="0 0 24 24" fill="currentColor"><path d={circle} /></svg>
	</section>

	<section id="inline-tailwind" class="flex items-center gap-4 text-indigo-600">
		<span class="w-40">2. inline, Tailwind colour</span>
		<svg class="stroke" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<circle cx="12" cy="12" r="10" /><path d="M8 12h8" />
		</svg>
		<svg class="fill" width="32" height="32" viewBox="0 0 24 24" fill="currentColor"><path d={circle} /></svg>
	</section>

	<section id="lucide-hex" class="flex items-center gap-4" style="color: #e11d48">
		<span class="w-40">3. Lucide, hex colour</span>
		<Camera />
		<Heart size={40} />
		<Settings size={32} strokeWidth={1} />
	</section>

	<section id="lucide-tailwind" class="flex items-center gap-4 text-rose-600">
		<span class="w-40">4. Lucide, Tailwind colour</span>
		<Camera />
		<Heart size={40} />
		<Settings class="size-8" strokeWidth={1} />
	</section>

	<section id="image" class="flex items-center gap-4">
		<span class="w-40">5. img, data URI</span>
		<img
			width="32"
			height="32"
			alt=""
			src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%23f59e0b'%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3C/svg%3E"
		/>
	</section>
</main>
