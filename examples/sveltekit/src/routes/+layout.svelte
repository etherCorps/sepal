<script lang="ts">
	import { page } from '$app/state';

	let { children } = $props();
</script>

<nav>
	<img id="logo" src="/logo.svg" alt="" />
	<a id="home" href="/">Home</a>
	<a id="about" href="/about">About</a>
	<a id="item" href="/items/42">Item 42</a>
	<span id="path">{page.url.pathname}</span>
</nav>

{@render children()}

<style>
	nav {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px;
		background: #1e293b;
	}
	nav a,
	nav span {
		color: white;
	}
	img {
		width: 24px;
		height: 24px;
	}
</style>
