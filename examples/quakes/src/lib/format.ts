// Dates and numbers by hand: QuickJS has no Intl. Times are UTC, as USGS gives them.

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const pad = (n: number) => (n < 10 ? '0' + n : String(n));

/** `14:05:09 UTC` */
export function clock(ms: number): string {
	const d = new Date(ms);
	return `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())} UTC`;
}

/** `24 Nov 2026, 14:05 UTC` */
export function datetime(ms: number): string {
	const d = new Date(ms);
	return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}, ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())} UTC`;
}

/** `Nov 24` */
export function day(ms: number): string {
	const d = new Date(ms);
	return `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}`;
}

/** `just now`, `12 min ago`, `3 h ago`, `2 d ago` */
export function ago(ms: number, now: number): string {
	const minutes = Math.floor((now - ms) / 60_000);
	if (minutes < 1) return 'just now';
	if (minutes < 60) return `${minutes} min ago`;
	const hours = Math.floor(minutes / 60);
	if (hours < 48) return `${hours} h ago`;
	return `${Math.floor(hours / 24)} d ago`;
}

/** `4.6` */
export const mag = (m: number) => m.toFixed(1);

/** `10.0 km` */
export const km = (n: number) => `${n.toFixed(1)} km`;

/** `17.70° S, 65.70° E` */
export function coords(lat: number, lon: number): string {
	return `${Math.abs(lat).toFixed(2)}° ${lat < 0 ? 'S' : 'N'}, ${Math.abs(lon).toFixed(2)}° ${lon < 0 ? 'W' : 'E'}`;
}

/** `1,234` */
export function count(n: number): string {
	const digits = String(Math.round(Math.abs(n)));
	let out = '';
	for (let i = 0; i < digits.length; i++) {
		if (i > 0 && (digits.length - i) % 3 === 0) out += ',';
		out += digits[i];
	}
	return (n < 0 ? '-' : '') + out;
}

/** `4.1e19 N·m` */
export function moment(nm: number): string {
	const exponent = Math.floor(Math.log10(nm));
	return `${(nm / 10 ** exponent).toFixed(1)}e${exponent} N·m`;
}
