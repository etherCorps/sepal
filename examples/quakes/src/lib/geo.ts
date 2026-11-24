// The world as SVG path strings, drawn once. d3-geo projects, topojson-client
// unpacks world-atlas's countries at 1:110m (108 KB, bundled; the app loads
// nothing from the network but the feeds).

import { geoDistance, geoGraticule10, geoNaturalEarth1, geoOrthographic, geoPath } from 'd3-geo';
import { feature, mesh } from 'topojson-client';
import type { GeometryCollection, Topology } from 'topojson-specification';
import countries from 'world-atlas/countries-110m.json';

/**
 * Colours for SVG attributes. They are app.css's tokens, converted: Blitz draws
 * an inline SVG on its own, where `fill="var(--land)"` means nothing and the
 * land would come out black.
 */
export const PALETTE = {
	ocean: '#080c10',
	land: '#22272c',
	coast: '#383e44',
	grid: '#14191e',
	mag: ['', '#48b7bd', '#cacc56', '#f5ae39', '#f6722b', '#f1383e']
};

const world = countries as unknown as Topology<{ countries: GeometryCollection; land: GeometryCollection }>;
const land = feature(world, world.objects.land);
const borders = mesh(world, world.objects.countries, (a, b) => a !== b);
const sphere = { type: 'Sphere' } as const;

/** The flat map: Natural Earth, which keeps both shapes and areas near enough. */
const flat = geoNaturalEarth1().fitWidth(960, sphere);
const flat_path = geoPath(flat);
const [[x0, y0], [x1, y1]] = flat_path.bounds(sphere);

export const MAP = {
	width: x1 - x0,
	height: y1 - y0,
	viewBox: `${x0} ${y0} ${x1 - x0} ${y1 - y0}`,
	sphere: flat_path(sphere) ?? '',
	graticule: flat_path(geoGraticule10()) ?? '',
	land: flat_path(land) ?? '',
	borders: flat_path(borders) ?? ''
};

/** Where a point falls on the flat map, as fractions of its width and height. */
export function place(lon: number, lat: number): { x: number; y: number } {
	const [x, y] = flat([lon, lat]) ?? [0, 0];
	return { x: (x - x0) / MAP.width, y: (y - y0) / MAP.height };
}

/** A globe turned to face a point, `size` pixels across. */
export function globe(lon: number, lat: number, size: number) {
	const projection = geoOrthographic()
		.rotate([-lon, -lat])
		.fitSize([size, size], sphere)
		.clipAngle(90);
	const path = geoPath(projection);
	const [cx, cy] = projection([lon, lat]) ?? [size / 2, size / 2];
	return {
		sphere: path(sphere) ?? '',
		graticule: path(geoGraticule10()) ?? '',
		land: path(land) ?? '',
		borders: path(borders) ?? '',
		point: { x: cx, y: cy }
	};
}

/** Great-circle distance in km. */
export function distance(a: { lon: number; lat: number }, b: { lon: number; lat: number }): number {
	return geoDistance([a.lon, a.lat], [b.lon, b.lat]) * 6371;
}
