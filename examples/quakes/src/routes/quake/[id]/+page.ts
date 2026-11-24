import { error } from '@sveltejs/kit';
import { detail } from '#lib/usgs.ts';
import type { PageLoad } from './$types';

// Each quake's detail feed: its products (moment tensor, PAGER, origin) as
// USGS has made them so far. Fetched again on every visit; they fill in over
// the first hours.
export const load: PageLoad = async ({ params, fetch }) => {
	try {
		return { detail: await detail(params.id, fetch) };
	} catch (cause) {
		error(502, `Could not load ${params.id} from USGS: ${(cause as Error).message}`);
	}
};
