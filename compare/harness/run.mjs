#!/usr/bin/env node
// The comparison's orchestrator (compare/PLAN.md sections 5-6, compare/shared/PROTOCOL.md).
//
//   node compare/harness/run.mjs check                      permissions, machine, builds, fixtures
//   node compare/harness/run.mjs calibrate <build> [size]   one calibration launch, saved to results
//   node compare/harness/run.mjs probe <build> [size] [s] [--breakdown]
//                                                           launch in bench mode, print what the
//                                                           harness sees (window, procs, memory,
//                                                           stamped lines), quit
//   node compare/harness/run.mjs scenario <build> <n> [size] [--record]
//        [--idle seconds] [--focus-ax]                      one measured run of scenario n, printed
//   node compare/harness/run.mjs run --preset quick|full [--session <name>] [--builds a,b]
//        [--sizes 1000,10000] [--scenarios 1,5] [--runs n] [--warmup n] [--config plan.json]
//        [--versions file.json] [--no-record] [--idle seconds] [--focus-ax]
//        [--env K=V,...]                                    more environment for Sepal
//        [--interleave]                                     alternate builds run by run (A/B within
//                                                           one session) instead of build by build
//   node compare/harness/run.mjs memwatch <build> [size] [--clock] [--secs 70] [--every 5]
//                                                           launch, then footprint (with categories)
//                                                           every few seconds while left alone
//   node compare/harness/run.mjs breakdown <build> [size]   scenario 9 with footprint categories and
//                                                           vmmap --summary at mounted+5 s and at the end
//
// Builds are found by bundle id in compare/builds/*.app (blitz, blitzv8, tauri, gpuix, and the
// diagnostic blitzdiag / blitzv8diag, which a session runs only if named);
// `dev` is the repo's release host on examples/tracker/dist, for developing the harness.
//
// Clock: every timestamp (`t`, ns) is mach_absolute_time in ns, the clock `drive` stamps
// input with. Lines from the app are stamped on arrival with process.hrtime.bigint() plus an
// offset measured against `drive clock` at the start of every run (see `sync_clock`). On this
// Node (libuv 1.52) hrtime is clock_gettime(CLOCK_UPTIME_RAW)-like and the offset measures
// ~0, but nothing relies on that: the offset and its uncertainty are recorded per run.

import { spawn, execFile } from 'node:child_process';
import { createInterface } from 'node:readline';
import { promisify } from 'node:util';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const exec = promisify(execFile);
const HARNESS = path.dirname(new URL(import.meta.url).pathname);
const COMPARE = path.dirname(HARNESS);
const REPO = path.dirname(COMPARE);
const DRIVE = path.join(HARNESS, 'bin/drive');
const RESULTS = path.join(COMPARE, 'results');
/** JSON for the results: paths relative to the repository, so no home directory (user name) is kept. */
const json = (v, indent) => JSON.stringify(v, null, indent).replaceAll(REPO, '<repo>');
const FIXTURES = path.join(COMPARE, 'shared/fixtures');
const DATA = path.join(os.homedir(), 'Library/Application Support');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const hr = () => Number(process.hrtime.bigint());
class Abort extends Error {}

// ---------------------------------------------------------------- plan

const PRESETS = {
	quick: { sizes: [1000], warmup: 1, runs: 3, cooldown: { long: 5, short: 5 } },
	full: { sizes: [1000, 10000], warmup: 1, runs: 10, cooldown: { long: 30, short: 5 } }
};
const DEFAULTS = {
	scenarios: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
	record: true, // one screen-recorded run per scenario: the first measured one
	attempts: 3, // an aborted run is retried this many times in all, then skipped
	settle_s: 5, // from `mounted` to a scenario's start; memory is sampled at that point
	idle_s: 60,
	mark_timeout_ms: 5000,
	content: { w: 1200, h: 800 }, // PROTOCOL.md: the window's content size
	resize_to: { w: 800, h: 600 },
	wheel: { dy: 100, hz: 60, down_sign: -1 }, // CG: negative wheel1 scrolls the content down
	first_input_mark: '^(highlight|nav:)', // what a `j` press must produce (scenario 1/2)
	// Ask the app through AX, before each `j`, whether it has a key/main window. Off in measured
	// runs: the first AX query makes the app build its accessibility tree and delays the press.
	focus_ax: false,
	show_all_mark: '^(show-all|shown)', // what clicking Show all must produce (scenario 9)
	title: 'Bench task 1', // scenario 8: 12 chars
	search: 'deploy cac'
};
const LONG = new Set([1, 2, 3, 4, 9]); // cool-down group: start-up, idle, scroll

// ---------------------------------------------------------------- builds

const COLUMNS = {
	'com.compare.tracker.blitz': { name: 'blitz', kind: 'blitz' },
	'com.compare.tracker.blitzv8': { name: 'blitzv8', kind: 'blitz' },
	'com.compare.tracker.tauri': { name: 'tauri', kind: 'tauri' },
	'com.compare.tracker.gpuix': { name: 'gpuix', kind: 'gpuix' },
	// More Sepal builds, for comparing changes to it in one session (--interleave).
	'com.compare.tracker.blitzb': { name: 'blitzb', kind: 'blitz' },
	'com.compare.tracker.blitzc': { name: 'blitzc', kind: 'blitz' },
	'com.compare.tracker.blitzd': { name: 'blitzd', kind: 'blitz' },
	'com.compare.tracker.blitze': { name: 'blitze', kind: 'blitz' },
	// Diagnostic only (blitz-dom's log-phase-times): never measured.
	'com.compare.tracker.blitzdiag': { name: 'blitzdiag', kind: 'blitz' },
	'com.compare.tracker.blitzv8diag': { name: 'blitzv8diag', kind: 'blitz' }
};

async function plist(file) {
	const { stdout } = await exec('plutil', ['-convert', 'json', '-o', '-', file]);
	return JSON.parse(stdout);
}

async function find_builds() {
	const builds = {};
	const dir = path.join(COMPARE, 'builds');
	const apps = [];
	const walk = (d, depth) => {
		for (const e of fs.existsSync(d) ? fs.readdirSync(d, { withFileTypes: true }) : []) {
			if (!e.isDirectory()) continue;
			const p = path.join(d, e.name);
			if (e.name.endsWith('.app')) apps.push(p);
			else if (depth < 3) walk(p, depth + 1);
		}
	};
	walk(dir, 0);
	for (const app of apps) {
		const info = await plist(path.join(app, 'Contents/Info.plist'));
		const column = COLUMNS[info.CFBundleIdentifier];
		if (!column) continue;
		if (builds[column.name]) throw new Error(`two builds for ${column.name}: ${builds[column.name].app} and ${app}`);
		builds[column.name] = {
			...column,
			app,
			id: info.CFBundleIdentifier,
			exe: path.join(app, 'Contents/MacOS', info.CFBundleExecutable),
			args: [],
			version: info.CFBundleShortVersionString ?? null
		};
	}
	const dist = path.join(REPO, 'examples/tracker/dist');
	const host = path.join(REPO, 'crates/host/target/release/calyx');
	if (fs.existsSync(host) && fs.existsSync(path.join(dist, 'app.json'))) {
		const id = JSON.parse(fs.readFileSync(path.join(dist, 'app.json'), 'utf8')).id;
		builds.dev = { name: 'dev', kind: 'blitz', app: null, id, exe: host, args: [dist], version: 'repo' };
	}
	for (const b of Object.values(builds)) {
		b.env = b.kind === 'blitz' ? { SEPAL_TRACE: '1' } : {};
	}
	return builds;
}

// ---------------------------------------------------------------- drive

/** One long-lived `drive serve`: commands cost a pipe round trip, not a process start. */
class Drive {
	constructor() {
		this.proc = spawn(DRIVE, ['serve'], { stdio: ['pipe', 'pipe', 'pipe'] });
		this.waiting = [];
		this.err = '';
		this.proc.stderr.on('data', (d) => (this.err += d));
		createInterface({ input: this.proc.stdout }).on('line', (line) => this.waiting.shift()?.resolve(JSON.parse(line)));
		this.proc.on('exit', (code) => {
			for (const w of this.waiting.splice(0)) w.reject(new Error(`drive exited (${code}): ${this.err.trim()}`));
		});
		this.chain = this.read(); // the `ready` line
	}
	read() {
		return new Promise((resolve, reject) => this.waiting.push({ resolve, reject }));
	}
	cmd(...args) {
		const run = () => {
			const reply = this.read();
			this.proc.stdin.write(args.map(String).join(' ') + '\n');
			return reply;
		};
		const p = this.chain.then(run, run);
		this.chain = p.catch(() => {});
		return p;
	}
	close() {
		this.proc.stdin.end('quit\n');
	}
}

let drive;
let offset = { ns: 0, err_ns: 0 }; // mach ns = hrtime ns + offset

/** The offset that turns hrtime into mach ns, from the tightest of 5 round trips. */
async function sync_clock() {
	let best = null;
	for (let i = 0; i < 5; i++) {
		const a = hr();
		const m = (await drive.cmd('clock')).t[0];
		const b = hr();
		if (!best || b - a < best.err_ns * 2) best = { ns: m - (a + b) / 2, err_ns: (b - a) / 2 };
	}
	offset = best;
	return best;
}
const now = () => hr() + offset.ns;

// ---------------------------------------------------------------- the app

/** A launched app: its stamped output, marks, windows, processes. */
class App {
	constructor(build, child, t_exec, wall_exec) {
		Object.assign(this, { build, child, pid: child.pid, t_exec, wall_exec });
		this.lines = []; // { t, fd, s }
		this.waiters = [];
		this.exited = null;
		const take = (fd) => (s) => {
			const entry = { t: now(), fd, s };
			this.lines.push(entry);
			for (const w of [...this.waiters]) if (w.test(entry, this.lines.length - 1)) w.done(entry);
		};
		createInterface({ input: child.stdout }).on('line', take(1));
		createInterface({ input: child.stderr }).on('line', take(2));
		child.on('exit', (code, signal) => (this.exited = { code, signal, t: now() }));
	}

	/** Resolves with the first line from index `from` on that passes `test`, or null after `ms`. */
	wait(test, ms, from = 0) {
		for (let i = from; i < this.lines.length; i++) if (test(this.lines[i], i)) return Promise.resolve(this.lines[i]);
		return new Promise((resolve) => {
			const w = { test: (e, i) => i >= from && test(e, i) };
			const timer = setTimeout(() => {
				this.waiters.splice(this.waiters.indexOf(w), 1);
				resolve(null);
			}, ms);
			w.done = (e) => {
				clearTimeout(timer);
				this.waiters.splice(this.waiters.indexOf(w), 1);
				resolve(e);
			};
			this.waiters.push(w);
		});
	}

	/** Waits for a mark matching `name` (string: exact; RegExp: test) after line index `from`. */
	async mark(name, from, ms = plan.mark_timeout_ms, why = '') {
		const test = (e) => {
			const m = parse_mark(e.s);
			return m && (typeof name === 'string' ? m.name === name : name.test(m.name));
		};
		const line = await this.wait(test, ms, from);
		if (!line) throw new Abort(`mark ${name} did not arrive within ${ms} ms${why ? ' ' + why : ''}`);
		const m = parse_mark(line.s);
		return { name: m.name, app_ms: m.app_ms, t: line.t, index: this.lines.indexOf(line) };
	}

	/** The runtime's "frame presented" after a mark (three clocks, PROTOCOL.md). */
	async frame(mark, ms = 2000) {
		const test =
			this.build.kind === 'blitz'
				? (e) => /^\[trace\] \+[\d.]+s (RedrawRequested|first frame) took/.test(e.s)
				: // compare/tauri's bench-emit.js cuts the name at its first space (`search:deploy c`
					// arrives as `frame:search:deploy`): accept that form too.
					(e) => [`frame:${mark.name}`, `frame:${mark.name.split(' ')[0]}`].includes(parse_mark(e.s)?.name);
		const line = await this.wait(test, ms, mark.index + 1);
		return line ? { t: line.t, signal: this.build.kind === 'blitz' ? 'trace RedrawRequested' : `MARK frame:${mark.name}` } : null;
	}

	get index() {
		return this.lines.length;
	}
}

function parse_mark(s) {
	const m = /MARK (.+) (\S+)$/.exec(s);
	return m ? { name: m[1], app_ms: Number(m[2]) } : null;
}
function parse_rect(s) {
	const m = /RECT (.+) (-?\d+(?:\.\d+)?) (-?\d+(?:\.\d+)?) (\d+(?:\.\d+)?) (\d+(?:\.\d+)?)$/.exec(s);
	return m ? { id: m[1], x: +m[2], y: +m[3], w: +m[4], h: +m[5] } : null;
}

const alive = (pid) => {
	try {
		process.kill(pid, 0);
		return true;
	} catch {
		return false;
	}
};
const launched = new Set(); // every app pid this harness started

/** No instance of the build and no helper of an earlier instance may be running. */
async function ensure_clean(build) {
	for (let round = 0; round < 3; round++) {
		const ps = await drive.cmd('ps');
		const exe = fs.realpathSync(build.exe);
		const apps = new Set(ps.filter((p) => p.path === exe || launched.has(p.pid)).map((p) => p.pid));
		const owned = ps.filter((p) => p.pid !== process.pid && (apps.has(p.pid) || apps.has(p.responsible) || launched.has(p.responsible)));
		if (!owned.length) return [];
		for (const p of owned) {
			try {
				process.kill(p.pid, round === 0 ? 'SIGTERM' : 'SIGKILL');
			} catch {}
		}
		await sleep(1500);
		if (round === 2) throw new Abort(`leftover processes would not exit: ${owned.map((p) => `${p.pid} ${p.path}`).join(', ')}`);
	}
	return [];
}

function prepare_data(build, size, bench) {
	const dir = path.join(DATA, build.id);
	fs.mkdirSync(dir, { recursive: true });
	for (const f of ['edits.json', 'settings.json']) fs.rmSync(path.join(dir, f), { force: true });
	const fixture = path.join(FIXTURES, `issues-${size}.json`);
	if (!fs.existsSync(fixture)) throw new Error(`no fixture ${fixture}`);
	fs.copyFileSync(fixture, path.join(dir, 'fixture.json'));
	fs.writeFileSync(path.join(dir, 'bench.json'), JSON.stringify(bench));
	return dir;
}

async function launch(build, size, bench) {
	await ensure_clean(build);
	prepare_data(build, size, bench);
	await sync_clock();
	const env = { ...process.env, ...build.env };
	const wall_exec = Date.now();
	const t_exec = now();
	// `drive launch` execs the app in place: same pid, our pipes, and the app is its own
	// responsible process, so WebKit's helpers can be found by responsible pid.
	const child = spawn(DRIVE, ['launch', build.exe, ...build.args], { env, stdio: ['ignore', 'pipe', 'pipe'] });
	launched.add(child.pid);
	return new App(build, child, t_exec, wall_exec);
}

async function quit(app) {
	const helpers = (await drive.cmd('procs', app.pid).catch(() => [])).map((p) => p.pid).filter((p) => p !== app.pid);
	if (!app.exited) {
		process.kill(app.pid, 'SIGTERM');
		for (let i = 0; i < 50 && !app.exited; i++) await sleep(100);
		if (!app.exited) process.kill(app.pid, 'SIGKILL');
		for (let i = 0; i < 20 && !app.exited; i++) await sleep(100);
	}
	// WebKit's helpers leave on their own once the app is gone; give them 5 s.
	for (let i = 0; i < 50 && helpers.some(alive); i++) await sleep(100);
	for (const p of helpers.filter(alive)) process.kill(p, 'SIGKILL');
	fs.rmSync(path.join(DATA, app.build.id, 'bench.json'), { force: true });
}

/** The app's window, once it is on screen; the frontmost check and title bar come with it. */
async function window_of(app, ms = 20000) {
	const until = Date.now() + ms;
	// Stage Manager keeps a new window off screen when its app does not come forward by
	// itself (Tauri, launched from a background process): after 5 s, bring it forward once.
	const park_at = Date.now() + 5000;
	let parked = false;
	let first = null, last = null, same = 0;
	while (Date.now() < until) {
		if (app.exited) throw new Abort(`the app exited (${JSON.stringify(app.exited)}) before showing a window`);
		const wins = (await drive.cmd('windows', app.pid)).filter((w) => w.onscreen && w.w >= 300 && w.h >= 200);
		if (!wins.length && !parked && Date.now() > park_at) {
			parked = true;
			console.error(`  no window on screen after 5 s: activating pid ${app.pid} (Stage Manager)`);
			await drive.cmd('activate', app.pid);
		}
		if (wins.length) {
			const w = wins[0];
			first ??= now();
			// macOS animates a new window in: its bounds settle over ~200 ms.
			if (last && ['x', 'y', 'w', 'h'].every((k) => last[k] === w[k])) {
				if (++same >= 5) return { ...w, t: first, stable_t: now(), parked };
			} else same = 0;
			last = w;
		}
		await sleep(20);
	}
	throw new Abort(`no settled on-screen window within ${ms} ms (last seen: ${JSON.stringify(last)}; Stage Manager or another app in front?)`);
}

async function ensure_front(app) {
	let f = await drive.cmd('front', app.pid);
	const activated = !f.frontmost;
	if (activated) {
		await drive.cmd('activate', app.pid);
		await sleep(300);
		f = await drive.cmd('front', app.pid);
	}
	if (!f.frontmost) throw new Abort(`window not frontmost (front pid ${f.front_pid}, top window pid ${f.top_window_pid})`);
	return { activated };
}

// ---------------------------------------------------------------- metrics

/** CPU, wake-ups and top's power column per pid, one sample a second (delta mode). */
function top_sampler(pids) {
	const proc = spawn('top', ['-l', '0', '-s', '1', '-c', 'd', '-stats', 'pid,cpu,idlew,power', ...pids.flatMap((p) => ['-pid', String(p)])]);
	const samples = [];
	let block = -1;
	createInterface({ input: proc.stdout }).on('line', (l) => {
		if (l.startsWith('PID')) block++;
		const m = /^(\d+)\s+([\d.]+)\s+(\d+)\s+([\d.]+)/.exec(l);
		// The first block has no previous sample to be a delta of.
		if (m && block > 0) samples.push({ t: now(), block, pid: +m[1], cpu: +m[2], idlew: +m[3], power: +m[4] });
	});
	return {
		stop() {
			proc.kill();
			return samples;
		}
	};
}

/** phys_footprint over every pid in one `footprint` call. */
/** footprint categories that are GPU memory: driver-owned graphics memory and window surfaces. */
const GPU_CATEGORY = /\(graphics\)|^IOSurface$|^IOAccelerator/;

async function footprint(pids) {
	const file = path.join(os.tmpdir(), `footprint-${process.pid}.json`);
	await exec('footprint', ['-j', file, '-f', 'bytes', ...pids.flatMap((p) => ['-p', String(p)])], { maxBuffer: 1 << 26 }).catch((e) => e);
	try {
		const j = JSON.parse(fs.readFileSync(file, 'utf8'));
		const gpu = (p) =>
			Object.entries(p.categories ?? {})
				.filter(([name]) => GPU_CATEGORY.test(name))
				.reduce((sum, [, c]) => sum + c.dirty + c.swapped, 0);
		return {
			t: now(),
			total: j['total footprint'],
			gpu: j.processes.reduce((sum, p) => sum + gpu(p), 0),
			processes: j.processes.map((p) => ({ pid: p.pid, name: p.name, footprint: p.footprint, gpu: gpu(p), peak: p.auxiliary?.phys_footprint_peak })),
			errors: j.errors
		};
	} catch (e) {
		return { t: now(), error: String(e) };
	}
}

/**
 * Where the memory is: `footprint` with its categories (JSON) and `vmmap --summary` of each
 * process, saved as `<dir>/<tag>.*`. Slow (vmmap walks the address space): never inside a
 * measured interval. Returns the categories per process, largest first.
 */
async function breakdown(procs, dir, tag) {
	fs.mkdirSync(dir, { recursive: true });
	const pids = procs.map((p) => p.pid);
	const file = path.join(dir, `${tag}.footprint.json`);
	await exec('footprint', ['-j', file, '-f', 'bytes', ...pids.flatMap((p) => ['-p', String(p)])], { maxBuffer: 1 << 26 }).catch((e) => e);
	for (const p of procs) {
		const r = await exec('vmmap', ['--summary', String(p.pid)], { maxBuffer: 1 << 26 }).catch((e) => e);
		fs.writeFileSync(path.join(dir, `${tag}.vmmap-${p.pid}-${path.basename(p.path || 'unknown')}.txt`), (r.stdout ?? '') + (r.stderr ? `\n# stderr\n${r.stderr}` : ''));
	}
	return categories(file);
}

/** footprint's categories per process, largest first, in bytes. */
function categories(file) {
	try {
		const j = JSON.parse(fs.readFileSync(file, 'utf8'));
		return j.processes.map((p) => ({
			pid: p.pid,
			name: p.name,
			footprint: p.footprint,
			categories: Object.entries(p.categories ?? {})
				.map(([name, c]) => ({ name, dirty: c.dirty, swapped: c.swapped, clean: c.clean, reclaimable: c.reclaimable, regions: c.regions }))
				.filter((c) => c.dirty + c.swapped > 0)
				.sort((a, b) => b.dirty + b.swapped - (a.dirty + a.swapped))
		}));
	} catch (e) {
		return { error: String(e) };
	}
}

async function usage(pids) {
	return drive.cmd('usage', ...pids);
}

/**
 * GPU time (ns) the GPU spent on these processes' Metal queues so far: the driver's
 * per-client `accumulatedGPUTime` (what Activity Monitor's GPU column reads), summed over
 * every client a process opened. A delta of two calls is the GPU time in between.
 */
async function gpu_time(pids) {
	const r = await exec('ioreg', ['-r', '-c', 'AGXDeviceUserClient', '-w', '0', '-d', '1'], { maxBuffer: 1 << 26 });
	const want = new Set(pids);
	let ns = 0;
	// One `+-o AGXDeviceUserClient` entry per client; `AppUsage` comes before its creator.
	for (const entry of r.stdout.split('+-o ')) {
		const creator = entry.match(/"IOUserClientCreator" = "pid (\d+),/);
		if (!creator || !want.has(Number(creator[1]))) continue;
		for (const m of entry.matchAll(/"accumulatedGPUTime"=(\d+)/g)) ns += Number(m[1]);
	}
	return ns;
}
/** WindowServer's pid: it composites every window, so it does GPU work for every app on screen. */
let windowserver = null;
async function windowserver_pid() {
	if (!windowserver) {
		const { stdout } = await exec('pgrep', ['-x', 'WindowServer']);
		windowserver = Number(stdout.trim().split('\n')[0]);
	}
	return windowserver;
}

function usage_delta(a, b) {
	const out = {};
	for (const e of b) {
		const s = a.find((x) => x.pid === e.pid) ?? {};
		if (e.error) continue;
		// energy_nj: the process's own energy (rusage V6); billed/serviced_energy: energy billed to
		// it by other processes through vouchers, and the reverse (drive.swift, `usage`).
		for (const k of ['user_ns', 'system_ns', 'pkg_idle_wkups', 'interrupt_wkups', 'energy_nj', 'penergy_nj', 'billed_energy', 'serviced_energy', 'instructions', 'cycles']) {
			out[k] = (out[k] ?? 0) + e[k] - (s[k] ?? 0);
		}
	}
	out.cpu_ns = (out.user_ns ?? 0) + (out.system_ns ?? 0);
	return out;
}

async function owned(app) {
	return (await drive.cmd('procs', app.pid)).map((p) => ({ pid: p.pid, path: p.path, responsible: p.responsible, ppid: p.ppid }));
}

/** Screen capture of the app's window (`drive record`), stopped by SIGTERM. */
async function recorder(win, file, seconds) {
	const proc = spawn(DRIVE, ['record', String(win.id), String(seconds), file, '0.25'], { stdio: ['ignore', 'pipe', 'pipe'] });
	let err = '';
	proc.stderr.on('data', (d) => (err += d));
	const lines = createInterface({ input: proc.stdout });
	const started = await new Promise((resolve) => {
		lines.once('line', (l) => resolve(JSON.parse(l)));
		proc.once('exit', () => resolve(null));
	});
	if (!started) throw new Abort(`screen capture did not start: ${err.trim()}`);
	return {
		started,
		async stop() {
			if (proc.exitCode === null) proc.kill('SIGTERM');
			await new Promise((r) => (proc.exitCode !== null ? r() : proc.once('exit', r)));
			return frames(file);
		}
	};
}

/** Per-frame lines from `drive record`: display_ns arrival_ns status mad hash. */
function frames(file) {
	return fs
		.readFileSync(file, 'utf8')
		.split('\n')
		.filter((l) => l && !l.startsWith('#'))
		.map((l) => {
			const [display, arrival, status, mad, hash] = l.split(' ');
			return { t: +display || +arrival, arrival: +arrival, status, mad: +mad, hash };
		});
}

/** Frames whose content changed in [from, to), and the first one after each input. */
function frame_stats(list, from, to, inputs) {
	const changed = list.filter((f) => f.status === 'complete' && f.mad !== 0 && f.t >= from && f.t < to);
	return {
		complete: list.filter((f) => f.status === 'complete' && f.t >= from && f.t < to).length,
		changed: changed.length,
		seconds: (to - from) / 1e9,
		first_change_after: inputs.map(({ what, t }) => {
			const f = changed.find((x) => x.t > t);
			return { what, t, frame_t: f?.t ?? null, ms: f ? (f.t - t) / 1e6 : null };
		})
	};
}

// ---------------------------------------------------------------- run context

/** What a scenario sees: the app, its window, positions, and input that aborts the run when refused. */
class Run {
	constructor(app, calib) {
		Object.assign(this, { app, calib, inputs: [], marks: {}, steps: [] });
	}

	async ready(win) {
		this.win = win;
		this.titlebar = this.calib?.titlebar ?? win.h - plan.content.h;
		this.origin = { x: win.x, y: win.y + this.titlebar };
	}

	/** The window must stay where it was: a move or resize ends the run. */
	async check_window(expect = this.win) {
		const wins = await drive.cmd('windows', this.app.pid);
		const w = wins.find((x) => x.id === expect.id);
		if (!w) throw new Abort('window gone or off screen');
		for (const k of ['x', 'y', 'w', 'h']) {
			if (Math.abs(w[k] - expect[k]) > 1) throw new Abort(`window moved or resized: ${JSON.stringify(w)} vs ${JSON.stringify(expect)}`);
		}
	}

	rect(id, state = 'start') {
		const r = this.calib?.states?.[state]?.[id] ?? this.calib?.states?.start?.[id];
		if (!r) throw new Abort(`no calibrated position for ${id} (state ${state})`);
		return r;
	}

	async post(what, ...args) {
		const r = await drive.cmd('as', this.app.pid, ...args);
		if (r.refused) throw new Abort(`input refused (${what}): ${r.refused}`);
		this.inputs.push({ what, t: r.down ?? r.t[0], move: r.move, down: r.down, up: r.up, all: r.t });
		return r;
	}

	/**
	 * Which event of a press makes `control` act: 'down' or 'up', from the calibration's probe
	 * (`probe_trigger`). Without one (an old calibration) it is 'down' and says so.
	 */
	trigger(control) {
		const t = this.calib?.triggers?.[control]?.trigger;
		return t ? { trigger: t, trigger_from: 'calibration' } : { trigger: 'down', trigger_from: 'default (not calibrated)' };
	}

	/** An input for `expect`: `t` is the event that triggers the control, the others are kept. */
	input(control, res, index) {
		const tr = this.trigger(control);
		return { control, ...tr, t: tr.trigger === 'up' ? res.up : res.down, move: res.move ?? null, down: res.down, up: res.up, index };
	}

	async click(id, state, hold) {
		await this.check_window();
		const r = this.rect(id, state);
		const x = this.origin.x + r.x + r.w / 2;
		const y = this.origin.y + r.y + r.h / 2;
		const res = await this.post(`click ${id}`, 'click', x.toFixed(1), y.toFixed(1), ...(hold ? [`hold=${hold}`] : []));
		return this.input(control_of(id), res, this.app.index);
	}

	async key(name, mods, hold) {
		await this.check_window();
		const index = this.app.index;
		const res = await this.post(`key ${mods ? mods + '+' : ''}${name}`, 'key', name, ...(mods ? [mods] : []), ...(hold ? [`hold=${hold}`] : []));
		return this.input(key_control(name, mods), res, index);
	}

	/**
	 * Expect a mark after an input; record latency to the mark and to the next frame, measured
	 * from the event that triggers the control (`input.t`: down or up, see `trigger`). The
	 * press's hold (down -> up) is recorded apart, and the latency from each event too.
	 */
	async expect(label, input, name, ms) {
		const m = await this.app.mark(name, input.index, ms, `after ${label}`);
		const f = await this.app.frame(m);
		const from = (t) => (t == null ? null : (m.t - t) / 1e6);
		const step = {
			label,
			control: input.control ?? null,
			trigger: input.trigger ?? null,
			trigger_from: input.trigger_from ?? null,
			input_t: input.t,
			move_t: input.move ?? null,
			down_t: input.down ?? null,
			up_t: input.up ?? null,
			hold_ms: input.down != null && input.up != null ? (input.up - input.down) / 1e6 : null,
			mark: m.name,
			mark_t: m.t,
			app_ms: m.app_ms,
			ms_to_mark: (m.t - input.t) / 1e6,
			ms_from_down: from(input.down),
			ms_from_up: from(input.up),
			frame_t: f?.t ?? null,
			frame_signal: f?.signal ?? null,
			ms_to_frame: f ? (f.t - input.t) / 1e6 : null
		};
		this.steps.push(step);
		return m;
	}
}

/** The calibration's name for what a click hits: every row is one control. */
const control_of = (id) => (id.startsWith('row:') ? 'row' : id);
const key_control = (name, mods) => `key:${mods ? mods.replace(/,/g, '+') + '+' : ''}${name}`;
/** Typing into a text field (scenario 6): one control for every character. */
const TYPE = 'key:type';

// ---------------------------------------------------------------- scenarios

/** Rows from calibration, in screen order, that sit fully inside the list. */
function visible_rows(run) {
	const list = run.rect('list');
	return Object.entries(run.calib.states.start)
		.filter(([id, r]) => id.startsWith('row:') && r.y >= list.y && r.y + r.h <= list.y + list.h)
		.sort((a, b) => a[1].y - b[1].y)
		.map(([id, r]) => ({ key: id.slice(4), r }));
}

/**
 * Scroll the list to the end of its first page and click Show all (scenario 9, and the
 * calibration's probe of it). Returns where the wheel events go and how many reach the end.
 */
async function show_all(run, hold) {
	const list = run.rect('list');
	const rows = visible_rows(run);
	const row_h = rows[0]?.r.h ?? 36;
	const show_all = run.rect('show-all');
	const { dy, hz, down_sign } = plan.wheel;
	const x = run.origin.x + list.x + list.w / 2;
	const y = run.origin.y + list.y + list.h / 2;
	// To the end of the first page (200 rows), where Show all is, quickly.
	const page = Math.ceil((200 * row_h + 400) / (dy * 4)) + 5;
	await run.check_window();
	await run.post('wheel to page end', 'wheel', x, y, down_sign * dy * 4, page, hz);
	await sleep(800);
	// At the bottom, the bar holding Show all is the last 36 px of the list.
	const at = { x: run.origin.x + show_all.x + show_all.w / 2, y: run.origin.y + list.y + list.h - 18 };
	const index = run.app.index;
	const c = await run.post('click show-all', 'click', at.x.toFixed(1), at.y.toFixed(1), ...(hold ? [`hold=${hold}`] : []));
	const input = run.input('show-all', c, index);
	await run.expect('click show-all', input, new RegExp(plan.show_all_mark));
	return { x, y, page, input };
}

/** Start-up marks, in order (PROTOCOL.md). */
const STARTUP_MARKS = ['start', 'read', 'parsed', 'loaded', 'mounted'];

/** Scenarios 1 and 2 share this: what start-up looks like from outside. */
async function startup(run) {
	const { app } = run;
	const from = 0;
	const marks = {};
	const win = await window_of(app);
	await run.ready(win);
	marks.window = (win.t - app.t_exec) / 1e6;
	marks.window_settled = (win.stable_t - app.t_exec) / 1e6;
	for (const name of STARTUP_MARKS) {
		const m = await app.mark(name, from, 20000);
		marks[name] = { ms_from_exec: (m.t - app.t_exec) / 1e6, app_ms: m.app_ms, wall_ms_from_exec: m.app_ms - app.wall_exec };
		if (name === 'mounted') {
			run.mounted_t = m.t;
			const f = await app.frame(m);
			marks.first_frame_after_mounted = f ? { ms_from_exec: (f.t - app.t_exec) / 1e6, signal: f.signal } : null;
		}
	}
	if (app.build.kind === 'blitz') {
		const first = await app.wait((e) => /^\[trace\] \+[\d.]+s first frame/.test(e.s), 1);
		marks.first_frame = first ? (first.t - app.t_exec) / 1e6 : null;
	}
	const front_began = now();
	const front = await ensure_front(app);
	marks.ensure_front = { ms_from_exec: (front_began - app.t_exec) / 1e6, took_ms: (now() - front_began) / 1e6, activated: front.activated };
	// First accepted input: `j` every 20 ms until one produces a mark. Before each press, where
	// the focus is (`drive focus`), so a press that was dropped can be explained, or not.
	const want = new RegExp(plan.first_input_mark);
	const tr = run.trigger(key_control('j'));
	const index = app.index;
	const at = (t) => (t == null ? null : (t - app.t_exec) / 1e6);
	let got = null;
	const presses = [];
	const until = Date.now() + 10000;
	while (!got && Date.now() < until) {
		const began = now();
		const focus = await drive.cmd('focus', app.pid, ...(plan.focus_ax ? ['ax'] : []));
		const r = await drive.cmd('as', app.pid, 'key', 'j');
		presses.push({
			refused: !!r.refused,
			down: r.down ?? null,
			up: r.up ?? null,
			down_ms_from_exec: at(r.down),
			up_ms_from_exec: at(r.up),
			focus: { ...focus, ms_from_exec: at(focus.t) }
		});
		// 20 ms of quiet after each press (the key is held 8 ms of that) before the next one's
		// checks: those copy the window list from WindowServer, and doing that right after a
		// post delayed the key's delivery by ~15-20 ms in every build (measured 23 Nov).
		got = await app.wait((e) => want.test(parse_mark(e.s)?.name ?? ''), Math.max(1, 20 - (now() - (r.down ?? began)) / 1e6), index);
	}
	const sent = presses.filter((p) => !p.refused);
	if (!got) throw new Abort(`no mark matching /${plan.first_input_mark}/ after ${sent.length} j presses (the app may not emit one)`);
	run.inputs.push(...sent.map((p) => ({ what: 'key j', t: p.down, down: p.down, up: p.up })));
	const ev = (p) => (tr.trigger === 'up' ? p.up : p.down);
	const before = sent.filter((p) => ev(p) < got.t);
	// Presses go out every 20 ms, so an app slower than that to mark one gets several before
	// the first mark is seen. Each accepted press marks (highlight moves a row): as many marks
	// as presses means none was dropped, and the first one was the one accepted.
	await sleep(300);
	const accepted = app.lines.slice(index).filter((e) => want.test(parse_mark(e.s)?.name ?? '')).length;
	marks.first_input = {
		mark: parse_mark(got.s).name,
		ms_from_exec: (got.t - app.t_exec) / 1e6,
		presses: sent.length,
		presses_before_mark: before.length,
		accepted,
		first_press_accepted: accepted >= sent.length,
		refused: presses.length - sent.length,
		...tr,
		first_press_ms_from_exec: at(ev(sent[0])),
		ms_from_first_press: (got.t - ev(sent[0])) / 1e6,
		ms_from_last_press_before: before.length ? (got.t - ev(before[before.length - 1])) / 1e6 : null,
		press_log: presses
	};
	return marks;
}

/** Each returns extra numbers for the record; `run.steps` holds per-input latencies. */
const SCENARIOS = {
	1: { name: 'first launch after quit', startup: true, run: startup },
	2: { name: 'warm start', startup: true, warm: true, run: startup },
	3: { name: 'idle 60 s, no clock', idle: true, async run(run) {
		await sleep(plan.idle_s * 1000);
	} },
	4: { name: 'idle 60 s, clock', clock: true, idle: true, async run(run) {
		const from = run.app.index;
		await sleep(plan.idle_s * 1000);
		return { clock_marks: run.app.lines.slice(from).filter((e) => parse_mark(e.s)?.name === 'clock').length };
	} },
	5: { name: 'open and close an issue', async run(run) {
		const [row] = visible_rows(run);
		if (!row) throw new Abort('no calibrated row in view');
		await run.expect('click row', await run.click(`row:${row.key}`), `nav:/issues/${row.key}`);
		await sleep(1000);
		await run.expect('Escape', await run.key('escape'), 'nav:/issues');
	} },
	6: { name: 'search', async run(run) {
		await run.click('search');
		await sleep(300);
		await run.check_window();
		const index = run.app.index;
		const text = plan.search;
		const r = await run.post('type search', 'type', encodeURIComponent(text), 100);
		for (let i = 0; i < text.length; i++) {
			const q = text.slice(0, i + 1);
			// A search box may trim or skip a prefix: only the full query is required.
			const required = i === text.length - 1;
			const tr = run.trigger(TYPE);
			const input = { control: TYPE, ...tr, t: tr.trigger === 'up' ? r.up[i] : r.down[i], down: r.down[i], up: r.up[i], index };
			try {
				await run.expect(`key ${JSON.stringify(text[i])}`, input, `search:${q}`, required ? plan.mark_timeout_ms : 1);
			} catch (e) {
				if (required) throw e;
				run.steps.push({ label: `key ${JSON.stringify(text[i])}`, input_t: r.t[i], mark: `search:${q}`, missing: true });
			}
		}
	} },
	7: { name: 'change status', async run(run) {
		const [row] = visible_rows(run);
		if (!row) throw new Abort('no calibrated row in view');
		const key = run.calib.issue_key ?? row.key;
		await run.expect('click row', await run.click(`row:${key}`), `nav:/issues/${key}`);
		await sleep(500);
		await run.click('status-select', 'issue');
		await sleep(500); // the Select opens; no mark for that
		await run.expect('click done', await run.click('status-option-done', 'select'), new RegExp(`^status:${key}:done$`));
	} },
	8: { name: 'create an issue', async run(run) {
		await run.click('nav-new');
		await sleep(500); // the dialog opens with the title focused; no mark for that
		await run.check_window();
		await run.post('type title', 'type', encodeURIComponent(plan.title), 50);
		await sleep(200);
		await run.expect('Cmd+Enter', await run.key('return', 'cmd'), /^created:/);
	} },
	9: { name: 'show all, then scroll top to bottom', scroll: true, async run(run, size) {
		const list = run.rect('list');
		const rows = visible_rows(run);
		const row_h = rows[0]?.r.h ?? 36;
		const { dy, hz, down_sign } = plan.wheel;
		const { x, y, page } = await show_all(run);
		await run.post('wheel to top', 'wheel', x, y, -down_sign * dy * 4, page + 10, hz);
		await sleep(1000);
		// The measured part: the whole list, top to bottom, at a fixed rate.
		const count = Math.ceil((size * row_h + 20 * 40 - list.h) / dy) + 30;
		await run.check_window();
		const from = now();
		const w = await run.post('wheel down', 'wheel', x, y, down_sign * dy, count, hz);
		const gaps = w.t.slice(2).map((t, i) => (t - w.t[i + 1]) / 1e6);
		return { wheel: { events: count, dy, hz, from, to: now(), interval_ms: summary(gaps) } };
	} },
	10: { name: 'navigate four screens', async run(run) {
		for (const [id, to] of [['nav-board', '/board'], ['nav-insights', '/insights'], ['nav-settings', '/settings'], ['nav-issues', '/issues']]) {
			const input = await run.click(id);
			await run.expect(`click ${id}`, input, `nav:${to}`);
			await sleep(Math.max(0, 2000 - (now() - input.t) / 1e6));
		}
	} },
	11: { name: 'resize', async run(run) {
		const original = run.win;
		const { w, h } = plan.resize_to;
		for (const [cw, ch] of [[w, h], [original.w, original.h - run.titlebar]]) {
			await run.check_window(run.current ?? original);
			const index = run.app.index;
			const r = await drive.cmd('resize', run.app.pid, cw, ch + run.titlebar);
			if (r.error) throw new Abort(`AX resize failed: ${r.error}`);
			run.inputs.push({ what: `resize ${cw}x${ch}`, t: r.t[0] });
			await run.expect(`resize ${cw}x${ch}`, { t: r.t[0], index }, `resize:${cw}x${ch}`);
			// The window list can lag the AX change by a frame or two.
			let win;
			for (let i = 0; i < 50; i++) {
				win = (await drive.cmd('windows', run.app.pid)).find((x) => x.id === original.id);
				if (win && Math.abs(win.w - cw) <= 1 && Math.abs(win.h - (ch + run.titlebar)) <= 1) break;
				await sleep(20);
			}
			if (!win || Math.abs(win.x - original.x) > 1 || Math.abs(win.y - original.y) > 1) throw new Abort(`window moved while resizing: ${JSON.stringify(win)}`);
			if (Math.abs(win.w - cw) > 1 || Math.abs(win.h - (ch + run.titlebar)) > 1) throw new Abort(`window is ${win.w}x${win.h}, wanted ${cw}x${ch + run.titlebar}`);
			run.current = win;
			await sleep(1000);
		}
		run.win = run.current; // back to the original size; check_window compares against this
	} }
};

function summary(xs) {
	if (!xs.length) return null;
	const s = [...xs].sort((a, b) => a - b);
	const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
	return { n: s.length, min: s[0], median: q(0.5), p95: q(0.95), max: s[s.length - 1] };
}

// ---------------------------------------------------------------- one run

let plan = { ...DEFAULTS };

async function run_one(build, size, n, { calib, record_dir, key, log_dir, breakdown_dir, activate_parked }) {
	const sc = SCENARIOS[n];
	const rec = { key, build: build.name, size, scenario: n, scenario_name: sc.name, wall_start: new Date().toISOString() };
	let app;
	let sampler;
	let capture;
	try {
		if (sc.warm) {
			// The previous launch of the same build, quit right before the measured one.
			const prev = await launch(build, size, { clock: false, rects: false });
			await prev.mark('mounted', 0, 20000);
			await quit(prev);
		}
		app = await launch(build, size, { clock: !!sc.clock, rects: false });
		rec.pid = app.pid;
		// mach ns and wall ms of the exec: together they turn any `t` here into wall time (powermetrics).
		rec.t_exec = app.t_exec;
		rec.wall_exec = app.wall_exec;
		rec.clock_offset = offset;
		const run = new Run(app, calib);
		let result;
		if (sc.startup) {
			sampler = top_sampler([app.pid]);
			const startup_from = app.t_exec;
			if (record_dir) {
				// Capture can only start once the window exists: the earliest frames are missed.
				const win = await window_of(app);
				capture = await recorder(win, path.join(record_dir, `${key}.frames`), 30);
			}
			result = await sc.run(run, size);
			rec.startup = result;
			await sleep(1000);
			rec.procs = await owned(app);
			rec.usage = usage_delta([], await usage(rec.procs.map((p) => p.pid))); // since exec
			rec.footprint = await footprint(rec.procs.map((p) => p.pid));
			rec.measured = { from: startup_from, to: now() };
			// The fixed point every scenario has: 5 s after `mounted`.
			await sleep(Math.max(0, run.mounted_t + plan.settle_s * 1e9 - now()) / 1e6);
			rec.footprint_mounted5 = await footprint((await owned(app)).map((p) => p.pid));
			rec.footprint_mounted5.ms_after_mounted = (rec.footprint_mounted5.t - run.mounted_t) / 1e6;
		} else {
			// `activate_parked` (the breakdown command only, never a measured run): a window Stage
			// Manager parked because the app launched behind another one is brought forward.
			const win = activate_parked
				? await window_of(app, 5000).catch(async () => (await drive.cmd('activate', app.pid), window_of(app)))
				: await window_of(app);
			await run.ready(win);
			const mounted = await app.mark('mounted', 0, 20000);
			await sleep(Math.max(0, mounted.t + plan.settle_s * 1e9 - now()) / 1e6);
			rec.procs = await owned(app);
			const pids = rec.procs.map((p) => p.pid);
			// 5 s after `mounted`, before the scenario: the same point in every scenario.
			rec.footprint_mounted5 = await footprint(pids);
			rec.footprint_mounted5.ms_after_mounted = (rec.footprint_mounted5.t - mounted.t) / 1e6;
			if (breakdown_dir) rec.breakdown_start = await breakdown(rec.procs, breakdown_dir, `${key}-mounted5`);
			await ensure_front(app);
			await run.check_window();
			if (record_dir) capture = await recorder(win, path.join(record_dir, `${key}.frames`), sc.idle ? plan.idle_s + 10 : 600);
			sampler = top_sampler(pids);
			const u0 = await usage(pids);
			const g0 = await gpu_time(pids);
			const ws0 = await gpu_time([await windowserver_pid()]);
			const from = now();
			const index = app.index;
			result = await sc.run(run, size);
			const to = now();
			const u1 = await usage(pids);
			// Processes started during the scenario (a WebKit helper) count from zero.
			rec.gpu_ns = (await gpu_time((await owned(app)).map((p) => p.pid))) - g0;
			// WindowServer's GPU time over the same span: composing this window (and
			// anything else on screen), work the app causes but is not billed for.
			rec.windowserver_gpu_ns = (await gpu_time([await windowserver_pid()])) - ws0;
			rec.measured = { from, to, seconds: (to - from) / 1e9 };
			rec.usage = usage_delta(u0, u1);
			rec.extra = result ?? null;
			await run.check_window();
			rec.procs_after = await owned(app);
			rec.footprint = await footprint(rec.procs_after.map((p) => p.pid));
			if (breakdown_dir) rec.breakdown_end = await breakdown(rec.procs_after, breakdown_dir, `${key}-end`);
			if (build.kind === 'blitz') {
				const lines = app.lines.slice(index).filter((e) => e.t <= to);
				rec.trace_redraws = lines.filter((e) => /RedrawRequested|first frame/.test(e.s)).length;
				// Damage tracking's verdict on each frame it drew: all of it, part of it, or
				// nothing (nothing changed on screen; nothing presented). None with it off.
				const kinds = { Full: 0, Rect: 0, None: 0 };
				for (const e of lines) {
					const m = /\[trace\] damage: (Full|Rect|None)\b/.exec(e.s);
					if (m) kinds[m[1]]++;
				}
				rec.trace_damage = { full: kinds.Full, partial: kinds.Rect, none: kinds.None };
			} else {
				rec.frame_marks = app.lines.slice(index).filter((e) => e.t <= to && parse_mark(e.s)?.name.startsWith('frame:')).length;
			}
		}
		rec.steps = run.steps;
		// Every posted event: `t` (what latency is measured from, for most), `move`/`down`/`up` of a press.
		rec.inputs = run.inputs;
		rec.top = sampler.stop();
		sampler = null;
		if (capture) {
			const list = await capture.stop();
			capture = null;
			rec.frames = { file: path.relative(RESULTS, path.join(record_dir, `${key}.frames`)), ...frame_stats(list, rec.measured.from, rec.measured.to, rec.inputs) };
		}
		rec.status = 'ok';
	} catch (e) {
		if (!(e instanceof Abort)) throw e;
		rec.status = 'aborted';
		rec.reason = e.message;
	} finally {
		sampler?.stop();
		await capture?.stop().catch(() => {});
		if (app) {
			await quit(app);
			rec.exit = app.exited;
			if (log_dir) {
				fs.writeFileSync(path.join(log_dir, `${key}.log`), app.lines.map((e) => `${e.t} ${e.fd} ${e.s}`).join('\n') + '\n');
				rec.log = path.relative(RESULTS, path.join(log_dir, `${key}.log`));
			}
		}
		rec.wall_end = new Date().toISOString();
	}
	return rec;
}

// ---------------------------------------------------------------- memory over time

/**
 * Launch in bench mode, then leave the app alone: footprint with categories every `every`
 * seconds for `secs` seconds after `mounted` (no input, no activation). What shrinks, and when.
 */
async function memwatch(build, size, { clock, secs, every }) {
	const app = await launch(build, size, { clock, rects: false });
	const out = { build: build.name, size, clock, secs, every, wall: new Date().toISOString(), samples: [] };
	const dir = path.join(RESULTS, 'raw', 'memory');
	fs.mkdirSync(dir, { recursive: true });
	const file = path.join(dir, `memwatch-${build.name}-${size}-${clock ? 'clock' : 'noclock'}.json`);
	const tmp = path.join(os.tmpdir(), `memwatch-${process.pid}.json`);
	try {
		await window_of(app);
		const m = await app.mark('mounted', 0, 60000);
		out.mounted_t = m.t;
		for (let at = 0; at <= secs; at += every) {
			await sleep(Math.max(0, m.t + at * 1e9 - now()) / 1e6);
			const procs = await owned(app);
			await exec('footprint', ['-j', tmp, '-f', 'bytes', ...procs.flatMap((p) => ['-p', String(p.pid)])], { maxBuffer: 1 << 26 }).catch((e) => e);
			const per = categories(tmp);
			const front = await drive.cmd('front', app.pid);
			out.samples.push({ s: (now() - m.t) / 1e9, frontmost: front.frontmost, total: Array.isArray(per) ? per.reduce((a, p) => a + p.footprint, 0) : null, processes: per });
			const mb = (c) => ((c.dirty + c.swapped) / 2 ** 20).toFixed(0);
			const line = Array.isArray(per) ? per.map((p) => `${p.name} ${(p.footprint / 2 ** 20).toFixed(0)} [${p.categories.slice(0, 5).map((c) => `${c.name} ${mb(c)}`).join(', ')}]`).join(' | ') : JSON.stringify(per);
			console.error(`${build.name} ${clock ? 'clock' : 'idle'} +${at}s front=${front.frontmost} ${line}`);
		}
	} catch (e) {
		if (!(e instanceof Abort)) throw e;
		out.aborted = e.message;
	} finally {
		await quit(app);
		// The app's own lines (Sepal: every repaint), to line memory up with frames.
		out.log = app.lines.map((e) => ({ s: (e.t - (out.mounted_t ?? app.t_exec)) / 1e9, line: e.s }));
		fs.writeFileSync(file, json(out, '\t'));
		console.log(file);
	}
}

// ---------------------------------------------------------------- calibration

/** A launch with rects: true that walks through every screen state the scenarios click in. */
async function calibrate(build, size) {
	const app = await launch(build, size, { clock: false, rects: true });
	const states = {};
	const missing = [];
	let latest = {};
	const collect = async (state, from, settle = 1200) => {
		await sleep(settle);
		const rects = {};
		for (const e of app.lines.slice(from)) {
			const r = parse_rect(e.s);
			if (r) rects[r.id] = { x: r.x, y: r.y, w: r.w, h: r.h };
		}
		states[state] = rects;
		latest = rects;
		return rects;
	};
	const calib = { build: build.name, size, wall: new Date().toISOString(), states, missing };
	try {
		const win = await window_of(app);
		const m = await app.mark('mounted', 0, 20000);
		calib.window = win;
		calib.titlebar = win.h - plan.content.h;
		calib.titlebar_standard = (await drive.cmd('machine')).titlebar;
		await ensure_front(app);
		await collect('start', m.index, 1500);
		const run = new Run(app, calib);
		await run.ready(win);
		// Which event makes each control act: pressed and held for HOLD ms, a control that acts
		// on the press marks before the release. Scenarios measure latency from that event.
		const HOLD = 300;
		const triggers = (calib.triggers = {});
		const record_trigger = (control, input, m) => {
			const from_up = (m.t - input.up) / 1e6;
			triggers[control] = {
				trigger: m.t < input.up ? 'down' : 'up',
				hold_ms: (input.up - input.down) / 1e6,
				ms_from_down: (m.t - input.down) / 1e6,
				ms_from_up: from_up,
				mark: m.name,
				// A control that acts on the press but takes longer than the hold looks like 'up'.
				...(from_up > 100 ? { uncertain: `mark ${from_up.toFixed(0)} ms after the release` } : {})
			};
		};
		const probe = async (control, input, name) => {
			const m = await app.mark(name, input.index, plan.mark_timeout_ms, `after probing ${control}`);
			record_trigger(control, input, m);
			return m;
		};
		const step = async (what, fn) => {
			try {
				await fn();
			} catch (e) {
				if (!(e instanceof Abort)) throw e;
				// Lost focus or a moved window spoils every later position: start over.
				if (/window|frontmost|refused/.test(e.message)) throw e;
				missing.push(`${what}: ${e.message}`);
			}
		};
		await step('key:j', async () => {
			await probe(key_control('j'), await run.key('j', null, HOLD), new RegExp(plan.first_input_mark));
			await sleep(300);
		});
		await step('issue', async () => {
			const [row] = visible_rows(run);
			if (!row) throw new Abort('no row in view');
			calib.issue_key = row.key;
			const i = await run.click(`row:${row.key}`, 'start', HOLD);
			await probe('row', i, `nav:/issues/${row.key}`);
			await collect('issue', i.index);
		});
		await step('select', async () => {
			const i = await run.click('status-select', 'issue');
			await collect('select', i.index);
			if (!states.select['status-option-done']) {
				await run.key('escape');
				await sleep(500);
				throw new Abort('no RECT status-option-done');
			}
			// The calibration's own change: later states are not affected (the list is collected at start).
			const d = await run.click('status-option-done', 'select', HOLD);
			await probe('status-option-done', d, new RegExp(`^status:${calib.issue_key}:done$`));
			await sleep(500);
		});
		await step('back', async () => {
			const i = await run.key('escape', null, HOLD);
			await probe(key_control('escape'), i, 'nav:/issues');
			await collect('issues', i.index);
		});
		await step('new', async () => {
			const i = await run.click('nav-new');
			await collect('new', i.index);
			try {
				await run.post('type title', 'type', encodeURIComponent(plan.title), 50);
				await sleep(200);
				await probe(key_control('return', 'cmd'), await run.key('return', 'cmd', HOLD), /^created:/);
			} catch (e) {
				await run.key('escape');
				throw e;
			} finally {
				await sleep(500);
			}
		});
		for (const [id, to] of [['nav-board', '/board'], ['nav-insights', '/insights'], ['nav-settings', '/settings'], ['nav-issues', '/issues']]) {
			await step(to, async () => {
				const i = await run.click(id, 'start', HOLD);
				await probe(id, i, `nav:${to}`);
				await collect(to, i.index);
			});
		}
		await step('type', async () => {
			await run.click('search');
			await sleep(300);
			await probe(TYPE, await run.key('d', null, HOLD), 'search:d');
			// Posted keys do not auto-repeat, but a stray repeat would leave the list filtered.
			for (let k = 0; k < 3; k++) await run.key('delete');
			await sleep(500);
		});
		if (!states.start?.['show-all']) missing.push('show-all: no RECT at start (scenario 9 needs its x)');
		else
			await step('show-all', async () => {
				const { input } = await show_all(run, HOLD);
				const s = run.steps.at(-1);
				record_trigger('show-all', input, { t: s.mark_t, name: s.mark });
			});
		const wins = await drive.cmd('windows', app.pid);
		calib.window_after = wins[0] ?? null;
	} catch (e) {
		if (!(e instanceof Abort)) throw e;
		calib.aborted = e.message;
	} finally {
		await quit(app);
		calib.log = app.lines.map((e) => `${e.t} ${e.fd} ${e.s}`);
	}
	return calib;
}

// ---------------------------------------------------------------- session

async function machine_info(versions_file) {
	const sh = async (cmd, args) => (await exec(cmd, args).catch((e) => ({ stdout: `error: ${e.message}` }))).stdout.trim();
	return {
		hw_model: await sh('sysctl', ['-n', 'hw.model']),
		cpu: await sh('sysctl', ['-n', 'machdep.cpu.brand_string']),
		ncpu: await sh('sysctl', ['-n', 'hw.ncpu']),
		memsize: await sh('sysctl', ['-n', 'hw.memsize']),
		sw_vers: await sh('sw_vers', []),
		drive: await drive.cmd('machine'),
		power: await sh('pmset', ['-g', 'batt']),
		low_power_mode: (await sh('pmset', ['-g'])).split('\n').find((l) => /lowpowermode/.test(l))?.trim() ?? null,
		stage_manager: await sh('defaults', ['read', 'com.apple.WindowManager', 'GloballyEnabled']),
		node: process.version,
		uv: process.versions.uv,
		versions: versions_file && fs.existsSync(versions_file) ? JSON.parse(fs.readFileSync(versions_file, 'utf8')) : null,
		versions_file: versions_file ?? null
	};
}

async function sizes_of(build) {
	if (!build.app) return null;
	const { stdout } = await exec('du', ['-sk', build.app]);
	const zip = path.join(os.tmpdir(), `${build.name}-${process.pid}.zip`);
	await exec('ditto', ['-c', '-k', '--keepParent', build.app, zip]);
	const zipped = fs.statSync(zip).size;
	fs.rmSync(zip);
	return { du_kb: Number(stdout.split('\t')[0]), zip_bytes: zipped };
}

async function check_permissions(need_screen) {
	const { stdout } = await exec(DRIVE, ['perm']).catch((e) => e);
	const perm = JSON.parse(stdout);
	const missing = [];
	if (!perm.ax) missing.push('Accessibility (CGEvent posting and AX resize)');
	if (need_screen && !perm.screen) missing.push('Screen Recording (frame capture; --no-record skips it)');
	if (missing.length) {
		console.error(`\nPERMISSION MISSING for the terminal running this: ${missing.join(', ')}.\nSystem Settings > Privacy & Security; then restart the terminal.\n`);
		process.exit(2);
	}
	return perm;
}

/** `--env K=V,K2=V2`: more environment for the Sepal builds (SEPAL_DAMAGE=0). */
function apply_env(builds, spec) {
	if (!spec) return;
	const extra = Object.fromEntries(String(spec).split(',').map((kv) => kv.split('=')));
	for (const b of Object.values(builds)) if (b.kind === 'blitz') Object.assign(b.env, extra);
}

function args_of(argv) {
	const out = { _: [] };
	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		if (a === '--no-record') out.record = false;
		else if (a === '--record') out.record = true;
		else if (a === '--clock') out.clock = true;
		else if (a === '--focus-ax') out.focus_ax = true;
		else if (a === '--breakdown') out.breakdown = true;
		else if (a === '--interleave') out.interleave = true;
		else if (a.startsWith('--')) out[a.slice(2)] = argv[++i];
		else out._.push(a);
	}
	return out;
}
const list_of = (s, f = Number) => (s == null ? undefined : String(s).split(',').map(f));

async function session(opts, builds) {
	const preset = PRESETS[opts.preset];
	if (!preset) throw new Error(`--preset quick|full`);
	const config = opts.config ? JSON.parse(fs.readFileSync(opts.config, 'utf8')) : {};
	plan = { ...DEFAULTS, ...preset, ...config };
	for (const k of ['runs', 'warmup', 'attempts']) if (opts[k] != null) plan[k] = Number(opts[k]);
	if (opts.idle) plan.idle_s = Number(opts.idle);
	if (opts.focus_ax) plan.focus_ax = true;
	if (opts.sizes) plan.sizes = list_of(opts.sizes);
	if (opts.scenarios) plan.scenarios = list_of(opts.scenarios);
	if (opts.record === false) plan.record = false;
	if (opts.env) plan.env = opts.env;
	if (opts.interleave) plan.interleave = true;
	plan.builds = list_of(opts.builds, String) ?? plan.builds ?? ['blitz', 'blitzv8', 'tauri', 'gpuix'].filter((b) => builds[b]);
	for (const b of plan.builds) if (!builds[b]) throw new Error(`no build ${b} (found: ${Object.keys(builds).join(', ')})`);
	for (const s of plan.sizes) if (!fs.existsSync(path.join(FIXTURES, `issues-${s}.json`))) throw new Error(`no fixture for ${s}`);

	const name = opts.session ?? `${new Date().toISOString().slice(0, 10)}-${opts.preset}`;
	const dir = path.join(RESULTS, 'raw', name);
	const logs = path.join(dir, 'logs');
	const caps = path.join(dir, 'frames');
	for (const d of [dir, logs, caps]) fs.mkdirSync(d, { recursive: true });
	const runs_file = path.join(dir, 'runs.jsonl');
	const done = new Map();
	if (fs.existsSync(runs_file)) {
		for (const l of fs.readFileSync(runs_file, 'utf8').split('\n').filter(Boolean)) {
			const r = JSON.parse(l);
			const d = done.get(r.key) ?? { ok: false, attempts: 0 };
			d.attempts++;
			d.ok ||= r.status === 'ok';
			done.set(r.key, d);
		}
	}
	const session_file = path.join(dir, 'session.json');
	const info = fs.existsSync(session_file) ? JSON.parse(fs.readFileSync(session_file, 'utf8')) : { started: new Date().toISOString(), resumed: [] };
	if (info.machine) info.resumed.push({ at: new Date().toISOString(), machine: await machine_info(opts.versions) });
	else info.machine = await machine_info(opts.versions ?? path.join(COMPARE, 'shared/versions.json'));
	info.plan = plan;
	info.builds ??= {};
	for (const b of plan.builds) info.builds[b] ??= { ...builds[b], sizes: await sizes_of(builds[b]) };
	fs.writeFileSync(session_file, json(info, '\t'));
	if (info.machine.stage_manager === '1') console.error('WARNING: Stage Manager is on: it moves and shrinks windows that lose focus. Runs abort if that happens.');

	const total = plan.builds.length * plan.sizes.length * plan.scenarios.length * (plan.warmup + plan.runs);
	let count = 0;
	// Build by build, or (--interleave) every build in turn for each run of each
	// scenario, so that a slow drift of the machine (clocks, heat) falls on all
	// builds alike.
	const order = [];
	for (const size of plan.sizes) {
		if (plan.interleave) {
			for (let i = 0; i < plan.warmup + plan.runs; i++)
				for (const n of plan.scenarios) for (const b of plan.builds) order.push({ b, size, n, i });
		} else {
			for (const b of plan.builds)
				for (const n of plan.scenarios) for (let i = 0; i < plan.warmup + plan.runs; i++) order.push({ b, size, n, i });
		}
	}
	if (!plan.interleave) order.sort((x, y) => plan.builds.indexOf(x.b) - plan.builds.indexOf(y.b));
	const calibs = new Map();
	for (const { b, size, n, i } of order) {
		const build = builds[b];
		if (!calibs.has(`${b}-${size}`)) {
			const calib_file = path.join(dir, `calibration-${b}-${size}.json`);
			let calib = fs.existsSync(calib_file) ? JSON.parse(fs.readFileSync(calib_file, 'utf8')) : null;
			for (let attempt = 1; (!calib || calib.aborted) && attempt <= plan.attempts; attempt++) {
				console.error(`calibrating ${b} ${size}${attempt > 1 ? ` attempt ${attempt}` : ''}`);
				calib = await calibrate(build, size);
				fs.writeFileSync(calib_file, json(calib, '\t'));
				if (calib.aborted) console.error(`  calibration aborted: ${calib.aborted}`);
				for (const m of calib.missing) console.error(`  missing: ${m}`);
				await sleep(plan.cooldown.short * 1000);
			}
			if (calib.aborted) console.error(`SKIPPING ${b} ${size}: no calibration (${calib.aborted})`);
			calibs.set(`${b}-${size}`, calib);
		}
		const calib = calibs.get(`${b}-${size}`);
		count++;
		if (calib.aborted) continue;
		const key = `${b}-${size}-s${n}-r${i}`;
		for (;;) {
			const d = done.get(key) ?? { ok: false, attempts: 0 };
			if (d.ok || d.attempts >= plan.attempts) break;
			const record = plan.record && i === plan.warmup;
			console.error(`[${count}/${total}] ${key}${i < plan.warmup ? ' (warm-up)' : ''}${record ? ' (recorded)' : ''}${d.attempts ? ` attempt ${d.attempts + 1}` : ''}`);
			const r = await run_one(build, size, n, { calib, key, log_dir: logs, record_dir: record ? caps : null });
			r.run = i;
			r.warmup = i < plan.warmup;
			r.recorded = record;
			r.attempt = d.attempts + 1;
			fs.appendFileSync(runs_file, json(r) + '\n');
			d.attempts++;
			d.ok = r.status === 'ok';
			done.set(key, d);
			if (r.status !== 'ok') console.error(`  ABORTED: ${r.reason}`);
			await sleep((LONG.has(n) ? plan.cooldown.long : plan.cooldown.short) * 1000);
		}
	}
	info.finished = new Date().toISOString();
	fs.writeFileSync(session_file, json(info, '\t'));
	console.error(`done: ${runs_file}`);
}

// ---------------------------------------------------------------- main

async function main() {
	const opts = args_of(process.argv.slice(2));
	const [cmd, ...rest] = opts._;
	const builds = await find_builds();
	apply_env(builds, opts.env);
	const need_screen = cmd === 'run' ? opts.record !== false : cmd === 'scenario' && opts.record === true;
	const perm = await check_permissions(need_screen);
	drive = new Drive();
	await drive.chain;
	// The Mac stays awake for as long as this process lives.
	const caffeinate = spawn('caffeinate', ['-dimsu', '-w', String(process.pid)], { stdio: 'ignore' });
	try {
		if (cmd === 'check') {
			const fixtures = fs.existsSync(FIXTURES) ? fs.readdirSync(FIXTURES) : [];
			console.log(JSON.stringify({ perm, clock: await sync_clock(), machine: await machine_info(path.join(COMPARE, 'shared/versions.json')), builds, fixtures }, null, 2));
		} else if (cmd === 'calibrate') {
			const [b, size = 1000] = rest;
			const calib = await calibrate(builds[b] ?? die(`no build ${b}`), Number(size));
			const dir = path.join(RESULTS, 'raw', 'calibration');
			fs.mkdirSync(dir, { recursive: true });
			const file = path.join(dir, `calibration-${b}-${size}.json`);
			fs.writeFileSync(file, json(calib, '\t'));
			const { log, ...short } = calib;
			console.log(JSON.stringify({ ...short, states: Object.fromEntries(Object.entries(calib.states).map(([k, v]) => [k, Object.keys(v).length + ' rects'])) }, null, 2));
			console.log(file);
		} else if (cmd === 'probe') {
			const [b, size = 1000, secs = 4] = rest;
			const build = builds[b] ?? die(`no build ${b}`);
			const app = await launch(build, Number(size), { clock: false, rects: false });
			try {
			// A window Stage Manager parks in its strip (the app launched behind another one) never
			// settles at full size: activate the app and look again (probe only; scenarios abort).
			let win = await window_of(app, 5000).catch((e) => ({ error: e.message }));
			if (win.error) {
				await drive.cmd('activate', app.pid);
				win = { ...(await window_of(app).catch((e) => ({ error: e.message }))), activated_after: win.error };
			}
			const m = await app.mark('mounted', 0, 60000).catch((e) => ({ error: e.message }));
			const marks = {};
			for (const name of STARTUP_MARKS) {
				const x = await app.mark(name, 0, 1).catch(() => null);
				marks[name] = x && { ms_from_exec: +((x.t - app.t_exec) / 1e6).toFixed(1), app_ms_from_exec: +(x.app_ms - app.wall_exec).toFixed(1) };
			}
			// `secs` after `mounted`: probe 10000 5 is the 10,000-issue check's footprint at mounted+5 s.
			if (m.t) await sleep(Math.max(0, m.t + Number(secs) * 1e9 - now()) / 1e6);
			const procs = await owned(app);
			const out = {
				pid: app.pid,
				size: Number(size),
				clock_offset: offset,
				window: win,
				window_ms_from_exec: win.t ? (win.t - app.t_exec) / 1e6 : null,
				marks,
				footprint_ms_after_mounted: m.t ? (now() - m.t) / 1e6 : null,
				front: await drive.cmd('front', app.pid),
				ax: await drive.cmd('ax', app.pid),
				procs,
				usage: await usage(procs.map((p) => p.pid)),
				footprint: await footprint(procs.map((p) => p.pid)),
				lines: app.lines.slice(0, 40).map((e) => `${((e.t - app.t_exec) / 1e6).toFixed(2)}ms fd${e.fd} ${e.s}`)
			};
			if (opts.breakdown) out.breakdown = await breakdown(procs, path.join(RESULTS, 'raw', 'memory'), `probe-${b}-${size}-mounted${secs}`);
			console.log(JSON.stringify(out, null, 2));
			} finally {
				await quit(app);
			}
		} else if (cmd === 'scenario') {
			const [b, n, size = 1000] = rest;
			if (opts.idle) plan.idle_s = Number(opts.idle); // shorter idle scenarios, for trying them
			if (opts.focus_ax) plan.focus_ax = true;
			const build = builds[b] ?? die(`no build ${b}`);
			const file = path.join(RESULTS, 'raw', 'calibration', `calibration-${b}-${size}.json`);
			const calib = fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : null;
			if (!calib) console.error(`no calibration (${file}); run \`calibrate ${b} ${size}\` first for scenarios that click`);
			const dir = path.join(RESULTS, 'raw', 'scratch');
			fs.mkdirSync(dir, { recursive: true });
			const r = await run_one(build, Number(size), Number(n), { calib, key: `${b}-${size}-s${n}`, log_dir: dir, record_dir: opts.record ? dir : null });
			console.log(JSON.stringify(r, null, 2));
		} else if (cmd === 'memwatch') {
			const [b, size = 1000] = rest;
			await memwatch(builds[b] ?? die(`no build ${b}`), Number(size), { clock: !!opts.clock, secs: Number(opts.secs ?? 70), every: Number(opts.every ?? 5) });
		} else if (cmd === 'breakdown') {
			const [b, size = 1000] = rest;
			const build = builds[b] ?? die(`no build ${b}`);
			const file = path.join(RESULTS, 'raw', 'calibration', `calibration-${b}-${size}.json`);
			const calib = fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : die(`no calibration ${file}`);
			const dir = path.join(RESULTS, 'raw', 'memory');
			const key = `breakdown-${b}-${size}-s9`;
			const r = await run_one(build, Number(size), 9, { calib, key, log_dir: dir, breakdown_dir: dir, activate_parked: true });
			fs.writeFileSync(path.join(dir, `${key}.json`), json(r, '\t'));
			const top = (x) => (Array.isArray(x) ? x.map((p) => ({ pid: p.pid, name: p.name, mb: +(p.footprint / 2 ** 20).toFixed(1), top: p.categories.slice(0, 8).map((c) => `${c.name} ${((c.dirty + c.swapped) / 2 ** 20).toFixed(1)}`) })) : x);
			console.log(JSON.stringify({ status: r.status, reason: r.reason, start: top(r.breakdown_start), end: top(r.breakdown_end) }, null, 2));
		} else if (cmd === 'run') {
			await session(opts, builds);
		} else {
			die('usage: see the top of run.mjs');
		}
	} finally {
		drive.close();
		caffeinate.kill();
	}
}
function die(msg) {
	console.error(msg);
	process.exit(1);
}

main().catch((e) => {
	console.error(e);
	process.exit(1);
});
