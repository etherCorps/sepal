import { redirect } from '@ethercorps/svelte-blitz-vite/router';

// The app opens on the list.
export function load() {
	redirect(307, '/issues');
}
