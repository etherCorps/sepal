import { redirect } from '@ethercorps/svelte-blitz-vite/router';

export function load() {
	redirect(307, '/about');
}
