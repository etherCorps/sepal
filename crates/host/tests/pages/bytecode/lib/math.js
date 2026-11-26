export function add(a, b) {
	return a + b;
}
export const where = () => import.meta.url;

export function thrower() {
	throw new Error('thrown on line 7');
}
