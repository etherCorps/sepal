import { redirect } from '@ethercorps/svelte-blitz';

export function load() {
	redirect(307, '/about');
}
