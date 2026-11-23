const T: Record<string, [number, number, number]> = {
	background: [0.985, 0.004, 250], foreground: [0.24, 0.02, 258], 'ink-2': [0.34, 0.018, 257], card: [0.995, 0.003, 250], popover: [0.995, 0.003, 250],
	primary: [0.58, 0.2, 256], 'primary-foreground': [0.985, 0.004, 250], secondary: [0.955, 0.006, 252], muted: [0.965, 0.005, 252], 'muted-foreground': [0.48, 0.018, 257],
	accent: [0.945, 0.01, 255], destructive: [0.56, 0.2, 25], border: [0.9, 0.008, 255], input: [0.87, 0.01, 255], sidebar: [0.97, 0.005, 252], 'sidebar-accent': [0.93, 0.01, 255],
	graphite: [0.22, 0.016, 260], 'graphite-2': [0.29, 0.018, 260], 'graphite-foreground': [0.94, 0.006, 255], 'graphite-muted': [0.68, 0.015, 257],
	'row-hover': [0.955, 0.008, 255], 'row-active': [0.94, 0.03, 256], 'row-checked': [0.96, 0.02, 256],
	'status-review': [0.58, 0.15, 300], 'status-done': [0.58, 0.13, 155], 'chart-1': [0.58, 0.2, 256], 'chart-2': [0.8, 0.07, 256], 'chart-3': [0.6, 0.015, 257], 'chart-4': [0.45, 0.018, 257], 'chart-5': [0.32, 0.018, 257]
};
const HUES = [256, 155, 60, 300, 20, 200, 100, 340, 230, 130, 30, 280];
const lab = ([l, c, h]: number[]) => [l, c * Math.cos((h * Math.PI) / 180), c * Math.sin((h * Math.PI) / 180)];
function lab_to_rgb([L, a, b]: number[]) {
	const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3, m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3, s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
	const lin = [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s];
	return lin.map((v) => { v = Math.min(1, Math.max(0, v)); return Math.round(255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055)); });
}
const hex = (rgb: number[]) => '#' + rgb.map((v) => v.toString(16).padStart(2, '0')).join('');
// color-mix(in oklch, A p%, B): interpolate L, C and hue (shorter arc); an achromatic side takes the other's hue.
function mix(A: number[], B: number[], p: number) {
	let h1 = A[2], h2 = B[2];
	if (B[1] === 0) h2 = h1;
	let d = h2 - h1; if (d > 180) d -= 360; if (d < -180) d += 360;
	return [A[0] * p + B[0] * (1 - p), A[1] * p + B[1] * (1 - p), h1 + d * (1 - p)];
}
for (const [k, v] of Object.entries(T)) console.log(`\t\t--${k}: ${hex(lab_to_rgb(lab(v)))};`);
HUES.forEach((h, i) => console.log(`\t\t--hue-${i + 1}: ${hex(lab_to_rgb(lab([0.62, 0.13, h])))};`));
const bg = T.background;
console.log('AVATAR_BG', JSON.stringify(HUES.map((h) => hex(lab_to_rgb(lab(mix([0.62, 0.13, h], bg, 0.18)))))));
console.log('AVATAR_FG', JSON.stringify(HUES.map((h) => hex(lab_to_rgb(lab(mix([0.62, 0.13, h], [0, 0, 0], 0.7)))))));
