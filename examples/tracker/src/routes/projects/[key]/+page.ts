import { error } from 'sepal/router';
import { project_by_key } from '$lib/seed';

export function load({ params }: { params: { key: string } }) {
	const project = project_by_key[params.key];
	if (!project) error(404, `No project ${params.key}`);
	return { project };
}
