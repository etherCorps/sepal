import { error } from '@sveltejs/kit';

// Every address no other route takes. SvelteKit's own answer to an unknown
// address is a page load from the server, and an app has neither: without
// this route, `goto` rejects and a link does nothing. Here the root error
// page shows, in the shell.
export function load({ url }: { url: URL }) {
	error(404, `Not found: ${url.pathname}`);
}
