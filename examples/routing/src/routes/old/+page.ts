import { redirect } from 'sepal/router';

export function load() {
	redirect(307, '/about');
}
