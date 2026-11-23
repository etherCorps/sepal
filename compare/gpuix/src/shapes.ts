// The glyphs as path data: one <svg> holds at most two <path>s and no
// {#if}, so a glyph costs 2-3 host nodes. Rows and cards (thousands of them)
// write the <svg> inline from these tables; the components use them too.

import type { PriorityId, StatusId } from './seed.ts';

/** A circle around (7, 7) as the path usvg makes of `<circle>`: four quarters clockwise from the right, so dashes fall where they did. */
const circle = (r: number) =>
	`M${7 + r} 7A${r} ${r} 0 0 1 7 ${7 + r}A${r} ${r} 0 0 1 ${7 - r} 7A${r} ${r} 0 0 1 7 ${7 - r}A${r} ${r} 0 0 1 ${7 + r} 7Z`;
/** The 1.5 px stroke of a 5.5 circle, as a fill (evenodd), so a filled part can share its path. */
const ring = circle(6.25) + circle(4.75);

export interface StatusGlyph {
	class: string;
	d: string;
	/** filled (evenodd) or stroked 1.5 */
	fill: boolean;
	dash?: string;
}

export const STATUS_GLYPH: Record<StatusId, StatusGlyph> = {
	backlog: { class: 'text-status-backlog', d: circle(5.5), fill: false, dash: '2.2 1.9' },
	todo: { class: 'text-status-todo', d: circle(5.5), fill: false },
	progress: { class: 'text-status-progress', d: ring + 'M7 4A3 3 0 0 1 7 10Z', fill: true },
	review: { class: 'text-status-review', d: ring + 'M7 4A3 3 0 1 1 4 7L7 7Z', fill: true },
	// the check is a hole, so it shows whatever is behind
	done: {
		class: 'text-status-done',
		d: 'M1 7A6 6 0 1 0 13 7A6 6 0 1 0 1 7ZM3.67 7.73L6.2 10.25L10.35 5.92L9.25 4.88L6.2 8.1L4.73 6.67Z',
		fill: true
	},
	canceled: { class: 'text-status-canceled', d: circle(5.5) + 'M5 5L9 9M9 5L5 9', fill: false }
};

const rounded = (x: number, y: number, w: number, h: number, r: number) =>
	`M${x + r} ${y}H${x + w - r}A${r} ${r} 0 0 1 ${x + w} ${y + r}V${y + h - r}A${r} ${r} 0 0 1 ${x + w - r} ${y + h}H${x + r}A${r} ${r} 0 0 1 ${x} ${y + h - r}V${y + r}A${r} ${r} 0 0 1 ${x + r} ${y}Z`;
const bars = [rounded(1.5, 8.5, 3, 4, 0.75), rounded(5.5, 5.5, 3, 7, 0.75), rounded(9.5, 2.5, 3, 10, 0.75)];

export interface PriorityGlyph {
	class: string;
	/** drawn in full */
	on: string;
	/** drawn at 30%: the unfilled signal bars; none for the rest (a <path> without `d` draws nothing) */
	off?: string;
}

export const PRIORITY_GLYPH: Record<PriorityId, PriorityGlyph> = {
	none: { class: 'text-muted-foreground', on: rounded(1.5, 6.25, 2.5, 1.5, 0.5) + rounded(5.75, 6.25, 2.5, 1.5, 0.5) + rounded(10, 6.25, 2.5, 1.5, 0.5) },
	low: { class: 'text-ink-2', on: bars[0], off: bars[1] + bars[2] },
	medium: { class: 'text-ink-2', on: bars[0] + bars[1], off: bars[2] },
	high: { class: 'text-ink-2', on: bars.join('') },
	urgent: {
		class: 'text-priority-urgent',
		on: 'M4 1H10A3 3 0 0 1 13 4V10A3 3 0 0 1 10 13H4A3 3 0 0 1 1 10V4A3 3 0 0 1 4 1ZM6.25 3.5H7.75V8H6.25ZM6.25 9.25H7.75V10.75H6.25Z'
	}
};
