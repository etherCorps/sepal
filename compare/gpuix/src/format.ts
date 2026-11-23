// Dates and numbers by hand: QuickJS has no Intl. All dates are UTC, so a run
// prints the same everywhere.

import { DAY, EPOCH } from './seed.ts';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const pad = (n: number) => (n < 10 ? '0' + n : String(n));

/** `Nov 21`: dates in rows and cards. */
export function short_date(ms: number): string {
	const d = new Date(ms);
	return `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}`;
}

/** `21 Nov 2026, 14:05`: the detail pane. */
export function long_date(ms: number): string {
	const d = new Date(ms);
	return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}, ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
}

/** `5,000` */
export function count(n: number): string {
	const sign = n < 0 ? '-' : '';
	const digits = String(Math.abs(Math.round(n)));
	let out = '';
	for (let i = 0; i < digits.length; i++) {
		if (i > 0 && (digits.length - i) % 3 === 0) out += ',';
		out += digits[i];
	}
	return sign + out;
}

/** Whole days from `ms` to the app's today. */
export function age_days(ms: number, today: number = EPOCH): number {
	return Math.max(0, Math.floor((today - ms) / DAY));
}

/** `3d`, `5w`: ages in tiles and the oldest list. */
export function age(ms: number, today: number = EPOCH): string {
	const days = age_days(ms, today);
	return days < 14 ? `${days}d` : `${Math.floor(days / 7)}w`;
}

/** `+12` / `-3` / `0`: the delta lines under the Insights tiles. */
export function delta(n: number): string {
	return n > 0 ? `+${count(n)}` : count(n);
}

/** `1 issue`, `12 issues` */
export function plural(n: number, one: string, many: string = one + 's'): string {
	return `${count(n)} ${n === 1 ? one : many}`;
}
