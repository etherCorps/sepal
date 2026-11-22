<script lang="ts">
	import { page } from '$app/state';

	let { children } = $props();
</script>

<nav>
	<a id="home" href="/">Home</a>
	<a id="about" href="/about">About</a>
	<a id="item" href="/items/42">Item 42</a>
	<span id="path">{page.url.pathname}</span>
</nav>

{@render children()}
