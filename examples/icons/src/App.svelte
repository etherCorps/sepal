<!--
	Icon rendering probe. Render it with:

	  pnpm --filter icons build
	  cargo run --manifest-path crates/host/Cargo.toml --features screenshot -- \
	    --screenshot docs/icons-probe.png examples/icons/dist 640x480

	`crates/host/tests/icons.rs` asserts on the pixels of every row.
-->
<script lang="ts">
	import { Camera, Heart, Settings } from '@lucide/svelte';

	// Vite inlines an imported asset as a data: URL.
	import square from './square.svg';

	const circle = 'M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z';

	let hot = $state(false);
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

	<button
		id="recolour"
		class={['flex items-center gap-4 text-left', hot ? 'text-rose-600' : 'text-indigo-600']}
		onclick={() => (hot = !hot)}
	>
		<span class="w-40">6. click to recolour</span>
		<svg class="fill" width="32" height="32" viewBox="0 0 24 24" fill="currentColor"><path d={circle} /></svg>
		<Heart size={32} />
	</button>

	<section id="image-file" class="flex items-center gap-4">
		<span class="w-40">7. img, file in dist</span>
		<img width="32" height="32" alt="" src="dot.svg" />
	</section>

	<section id="image-import" class="flex items-center gap-4">
		<span class="w-40">8. img, imported</span>
		<img width="32" height="32" alt="" src={square} />
	</section>
</main>
