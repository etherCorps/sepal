import { error } from '@ethercorps/svelte-blitz';
import { items } from '$lib/items';

export async function load({ params, parent, url }: { params: { id: string }; parent: () => Promise<{ app: string }>; url: { searchParams: { get(name: string): string | null } } }) {
	const item = items.find((item) => item.id === params.id);
	if (!item) error(404, `No item ${params.id}`);

	const { app } = await parent();
	return { item, from: url.searchParams.get('from'), heading: `${app}: ${item.name}` };
}
