// The Tracker's glyphs (shapes.ts, unchanged) as SVG markup for <svg source>, which GPUI paints as
// a mask tinted with the element's `color`: so one colour per <svg>, as the Tracker's glyphs are
// (currentColor), and the priority's off bars keep their 30% through the mask's alpha.
// `tone` is the token shapes.ts's Tailwind class reads (`text-status-done` -> `status-done`); the
// components that draw glyphs have a `.<tone> { color: var(--<tone>) }` rule for each.

import { PRIORITY_GLYPH, STATUS_GLYPH } from './shapes.ts';
import { PRIORITIES, STATUSES, type PriorityId, type StatusId } from './seed.ts';
import { person } from './store.svelte.ts';

const tone_of = (tw: string) => tw.replace(/^text-/, '');

const svg = (body: string) => `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 14 14">${body}</svg>`;

export const STATUS_SVG = Object.fromEntries(
	STATUSES.map(({ id }) => {
		const g = STATUS_GLYPH[id];
		const paint = g.fill ? 'fill="currentColor" stroke="none"' : `fill="none" stroke="currentColor" stroke-width="1.5"${g.dash ? ` stroke-dasharray="${g.dash}"` : ''}`;
		return [id, { source: svg(`<path d="${g.d}" fill-rule="evenodd" ${paint}/>`), tone: tone_of(g.class) }];
	})
) as Record<StatusId, { source: string; tone: string }>;

export const PRIORITY_SVG = Object.fromEntries(
	PRIORITIES.map(({ id }) => {
		const g = PRIORITY_GLYPH[id];
		const off = g.off ? `<path d="${g.off}" opacity="0.3"/>` : '';
		return [id, { source: svg(`<g fill="currentColor" fill-rule="evenodd"><path d="${g.on}"/>${off}</g>`), tone: tone_of(g.class) }];
	})
) as Record<PriorityId, { source: string; tone: string }>;

/** color-mix(in oklch, var(--hue-N) 18%, var(--background)) and (… 70%, black), worked out once (scripts/colors.ts). */
const AVATAR_BG = ['#d8e6f6', '#d4e8f4', '#e1e3f6', '#dae5f7', '#dee4f7', '#d5e7f5', '#d2e9f1', '#dce4f7', '#d7e7f6', '#d3e8f3', '#dfe3f7', '#d9e6f6'];
const AVATAR_FG = ['#2d5282', '#1c5f3a', '#754312', '#59447b', '#7b3a3c', '#005f65', '#5d5100', '#713b60', '#005979', '#405a20', '#7b3b32', '#484a82'];

/** The avatar as one element's class and style, as the Tracker's `avatar()`. Reactive (the user's name). */
export function avatar(id: string | null, size: 20 | 24 = 20) {
	const who = person(id);
	const box = `width: ${size}px; height: ${size}px;`;
	if (!who) return { empty: true, style: box, title: 'Unassigned', initials: '' };
	return {
		empty: false,
		style: `${box} font-size: ${size === 24 ? 11 : 10}px; background-color: ${AVATAR_BG[who.hue - 1]}; color: ${AVATAR_FG[who.hue - 1]};`,
		title: who.name,
		initials: who.initials
	};
}
