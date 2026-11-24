// The USGS earthquake feeds: https://earthquake.usgs.gov/earthquakes/feed/v1.0/
// GeoJSON, no key, updated every minute. sepal.config.ts lets the app reach this
// origin and nothing else.

const ORIGIN = 'https://earthquake.usgs.gov';

/** The feeds offered, smallest first. USGS publishes a file per magnitude and period. */
export const PRESETS = [
	{ id: 'all_hour', label: 'All', period: 'past hour' },
	{ id: '2.5_day', label: 'M2.5+', period: 'past day' },
	{ id: 'significant_month', label: 'Significant', period: 'past month' },
	{ id: '4.5_week', label: 'M4.5+', period: 'past week' },
	{ id: '1.0_day', label: 'M1.0+', period: 'past day' },
	{ id: 'all_day', label: 'All', period: 'past day' },
	{ id: '2.5_week', label: 'M2.5+', period: 'past week' },
	{ id: '4.5_month', label: 'M4.5+', period: 'past month' }
] as const;

export type PresetId = (typeof PRESETS)[number]['id'];
export const DEFAULT_PRESET: PresetId = '2.5_week';

export interface Quake {
	id: string;
	mag: number;
	magType: string;
	place: string;
	time: number;
	updated: number;
	lon: number;
	lat: number;
	depth: number;
	url: string;
	alert: 'green' | 'yellow' | 'orange' | 'red' | null;
	tsunami: boolean;
	felt: number | null;
	sig: number;
	status: string;
}

interface Feature {
	id: string;
	geometry: { coordinates: [number, number, number] };
	properties: Record<string, any>;
}

function quake(feature: Feature): Quake {
	const p = feature.properties;
	const [lon, lat, depth] = feature.geometry.coordinates;
	return {
		id: feature.id,
		mag: p.mag ?? 0,
		magType: p.magType ?? '',
		place: p.place ?? 'Unknown place',
		time: p.time,
		updated: p.updated,
		lon,
		lat,
		depth,
		url: p.url,
		alert: p.alert ?? null,
		tsunami: p.tsunami === 1,
		felt: p.felt ?? null,
		sig: p.sig ?? 0,
		status: p.status ?? ''
	};
}

async function get(path: string, fetcher: typeof fetch) {
	const response = await fetcher(ORIGIN + path);
	if (!response.ok) throw new Error(`USGS answered ${response.status} for ${path}`);
	return response.json();
}

/** One feed, newest first. */
export async function feed(preset: PresetId, fetcher: typeof fetch = fetch) {
	const json = await get(`/earthquakes/feed/v1.0/summary/${preset}.geojson`, fetcher);
	const quakes = (json.features as Feature[]).map(quake).sort((a, b) => b.time - a.time);
	return { title: json.metadata.title as string, generated: json.metadata.generated as number, quakes };
}

export interface Detail {
	quake: Quake;
	/** PAGER's estimate of losses, when there is one. */
	pager: { alert: string; maxmmi: number | null } | null;
	/** The fault planes the moment tensor allows, as strike / dip / rake. */
	planes: { strike: number; dip: number; rake: number }[];
	scalarMoment: number | null;
	stations: number | null;
	horizontalError: number | null;
	depthType: string | null;
	products: string[];
}

const num = (value: unknown) => {
	const n = Number(value);
	return value == null || value === '' || Number.isNaN(n) ? null : n;
};

/** One event, with the products USGS has made for it so far. */
export async function detail(id: string, fetcher: typeof fetch = fetch): Promise<Detail> {
	const json = await get(`/earthquakes/feed/v1.0/detail/${encodeURIComponent(id)}.geojson`, fetcher);
	const products = json.properties.products ?? {};
	const first = (name: string) => products[name]?.[0]?.properties ?? null;
	const pager = first('losspager');
	const tensor = first('moment-tensor');
	const origin = first('origin');
	const planes = tensor
		? [1, 2].map((n) => ({
				strike: Number(tensor[`nodal-plane-${n}-strike`]),
				dip: Number(tensor[`nodal-plane-${n}-dip`]),
				rake: Number(tensor[`nodal-plane-${n}-rake`])
			}))
		: [];
	return {
		quake: quake(json),
		pager: pager ? { alert: pager.alertlevel, maxmmi: num(pager.maxmmi) } : null,
		planes: planes.filter((plane) => !Number.isNaN(plane.strike)),
		scalarMoment: num(tensor?.['scalar-moment']),
		stations: num(origin?.['num-stations-used']),
		horizontalError: num(origin?.['horizontal-error']),
		depthType: origin?.['depth-type'] ?? null,
		products: Object.keys(products).sort()
	};
}

/** `5 km SSW of Volcano, Hawaii` -> `Hawaii`: the part after the last comma. */
export function region(place: string): string {
	const comma = place.lastIndexOf(',');
	return (comma < 0 ? place.replace(/^.* of /, '') : place.slice(comma + 1)).trim().replace(/^the /, '');
}

/** 1 to 5, for the colour ramp. */
export function band(mag: number): 1 | 2 | 3 | 4 | 5 {
	if (mag >= 6) return 5;
	if (mag >= 4.5) return 4;
	if (mag >= 3) return 3;
	if (mag >= 1.5) return 2;
	return 1;
}

export function depthClass(km: number): string {
	if (km < 70) return 'shallow';
	if (km < 300) return 'intermediate';
	return 'deep';
}
