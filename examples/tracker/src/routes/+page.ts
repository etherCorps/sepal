import { redirect } from 'sepal/router';

// The app opens on the list.
export function load() {
	redirect(307, '/issues');
}
