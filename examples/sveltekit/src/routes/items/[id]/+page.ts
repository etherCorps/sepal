import type { PageLoad } from './$types';

export const load: PageLoad = ({ params }) => ({ id: params.id, double: Number(params.id) * 2 });
