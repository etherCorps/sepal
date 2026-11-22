<!-- Shown when a route fails and the app has no +error.svelte for it. -->
<!-- Without a rune in the file Svelte would compile it in legacy mode, where a
     component tag is resolved once and never switches. -->
<svelte:options runes={true} />

<script>
	import { page } from './routing.svelte.js';
</script>

<h1>{page.status}</h1>
<p>{page.error?.message}</p>
