import { redirect } from '@sveltejs/kit';

// The app opens on the list.
export function load() {
	redirect(307, '/issues');
}
