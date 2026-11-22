<script lang="ts">
	import { page } from '$app/state';
</script>

<h1 id="title" class="text-2xl font-semibold">Error {page.status}</h1>
<p id="error-message">{page.error?.message}</p>
