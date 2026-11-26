"""The Tracker's modules loaded from QuickJS bytecode, against main, at 1,000 issues.

    python3 compare/results/make_bytecode_report.py [session] > compare/results/bytecode.md

One interleaved session (default bc-1), Stage Manager off, warm-up and aborted runs left out:
  main      Sepal on main at 8c81c90: every module compiled from source       build blitzb
  bytecode  Sepal on bytecode at 8779185: `x.js.qbc` next to every module     build blitzc
Each bench build was bundled by its own dist-profile host; the bytecode
bundle compiled its modules again in the bundle (`--bundle`). The commits
after 8779185 change how a build calls the compiler, not what the host
loads. Each cell is the median of the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'bc-1'
COLS = [('main', 'blitzb'), ('bytecode', 'blitzc')]
HELP = ('WebKit', 'com.apple.WebKit')
MB = 1024 * 1024

RUNS = [json.loads(line) for line in (RESULTS / 'raw' / SESSION / 'runs.jsonl').open()]


def runs(build, n):
    return [r for r in RUNS if r['status'] == 'ok' and not r.get('warmup') and r['build'] == build and r['scenario'] == n]


def median(vals):
    vals = [v for v in vals if v is not None]
    return st.median(vals) if vals else None


def cell(vals, d):
    vals = [v for v in vals if v is not None]
    if not vals:
        return 'n/a'
    f = lambda v: f'{v:,.{d}f}'
    lo, hi = min(vals), max(vals)
    return f'{f(st.median(vals))}<br><sub>{f(lo)}–{f(hi)}</sub>' if len(vals) > 1 and lo != hi else f(st.median(vals))


def table(title, scenarios, rows):
    print(f'### {title}\n')
    print('| | ' + ' | '.join(c for c, _ in COLS) + ' | bytecode − main |')
    print('|---|' + '---:|' * (len(COLS) + 1))
    for label, fn, d in rows:
        vals = [[fn(r) for n in scenarios for r in runs(b, n)] for _, b in COLS]
        a, b = median(vals[0]), median(vals[1])
        gap = f'{b - a:+,.{d}f}' if a is not None and b is not None else 'n/a'
        print(f'| {label} | ' + ' | '.join(cell(v, d) for v in vals) + f' | {gap} |')
    counts = ', '.join(f'{c}: {sum(len(runs(b, n)) for n in scenarios)}' for c, b in COLS)
    print(f'\n<sub>Runs: {counts}.</sub>\n')


def step(label, key):
    def get(r):
        s = next((s for s in r['steps'] if s['label'] == label), None)
        return s.get(key) if s else None
    return get


def mark(name):
    return lambda r: (r.get('startup') or {}).get(name, {}).get('ms_from_exec')


def startup(key):
    return lambda r: (r.get('startup') or {}).get(key)


def first_input(r):
    return ((r.get('startup') or {}).get('first_input') or {}).get('ms_from_exec')


def frame_after_mounted(r):
    return ((r.get('startup') or {}).get('first_frame_after_mounted') or {}).get('ms_from_exec')


def page_started(r):
    """The host's own count: index.html parsed, its scripts run, settled."""
    for line in (RESULTS / r['log']).read_text().splitlines():
        m = re.search(r'the page started in ([\d.]+)ms', line)
        if m:
            return float(m.group(1))
    return None


def turn(mark_name):
    """The navigation's turn in the window trace: the `poll took` line after its MARK."""
    def get(r):
        lines = (RESULTS / r['log']).read_text().splitlines()
        i = next((k for k, l in enumerate(lines) if f'MARK {mark_name}' in l), None)
        if i is None:
            return None
        m = next((re.search(r'poll took ([\d.]+)ms', l) for l in lines[i:] if 'poll took' in l), None)
        return float(m.group(1)) if m else None
    return get


def presented(r):
    d = r.get('trace_damage')
    return d['full'] + d['partial'] if d else None


ram = lambda fp: sum(p['footprint'] for p in fp['processes'] if p['name'] not in HELP) / MB if fp else None
cpu = lambda r: r['usage']['cpu_ns'] / 1e6
gpu = lambda r: r['gpu_ns'] / 1e6 if 'gpu_ns' in r else None
wake = lambda r: r['usage'].get('interrupt_wkups')

print(__doc__.split('\n\n')[0].strip('"'), '\n')
print(f'Session `{SESSION}`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, '
      'no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app '
      '(start-up) or from the input (navigation).\n')

STARTUP = [
    ('the page started (host trace), ms', page_started, 1),
    ('first JavaScript (`start` mark), ms', mark('start'), 1),
    ('window on screen, ms', startup('window'), 1),
    ('issues on the page (`mounted` mark), ms', mark('mounted'), 1),
    ('first frame after `mounted`, ms', frame_after_mounted, 1),
    ('first key press answered (`j`), ms', first_input, 1),
    ('RAM 5 s after load, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
    ('CPU time, launch to settled, ms', cpu, 0),
]
table('Start-up: first launch after quit (scenario 1)', [1], STARTUP)
table('Start-up: warm start (scenario 2)', [2], STARTUP)
table('Idle, 20 s (scenario 3)', [3], [
    ('CPU time, ms', cpu, 1),
    ('GPU time, ms', gpu, 2),
    ('frames presented', presented, 0),
    ('interrupt wake-ups', wake, 0),
    ('RAM 5 s after load, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
    ('RAM at the end, MB', lambda r: ram(r.get('footprint')), 1),
])
table('Open an issue (scenario 5)', [5], [
    ('click row → mark, ms', step('click row', 'ms_to_mark'), 1),
    ('click row → frame, ms', step('click row', 'ms_to_frame'), 1),
    ('Escape → mark, ms', step('Escape', 'ms_to_mark'), 1),
    ('Escape → frame, ms', step('Escape', 'ms_to_frame'), 1),
])
SCREENS = ('board', 'insights', 'settings', 'issues')
table('Switch screens (scenario 10)', [10], [
    *[(f'{to} → mark, ms', step(f'click nav-{to}', 'ms_to_mark'), 1) for to in SCREENS],
    *[(f'{to} → frame, ms', step(f'click nav-{to}', 'ms_to_frame'), 1) for to in SCREENS],
    *[(f'{to}: the turn that renders it, ms', turn(f'nav:/{to}'), 1) for to in SCREENS],
])
table('RAM 5 s after load, all start-up and idle runs (scenarios 1, 2 and 3)', [1, 2, 3], [
    ('RAM, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
])

print('''### Notes

- **Start-up is where it shows.** The host's own count from parsing
  `index.html` to the page settled is 19-20 ms shorter, as is the first
  line of JavaScript (`start`): 36 modules are loaded from bytecode instead
  of parsed and compiled. CPU from launch to settled is 23 ms lower. Later
  marks (`mounted`, the first frame, the first key press) are 5-10 ms
  sooner on a first launch; they wait for the fixture, read on a thread.
  On a warm start the window was on screen 45 ms sooner, with tight
  ranges: more than the JavaScript saved, and not explained. Take it as
  this session's until it is seen again.
- **First visits barely move**, as expected: a route's own chunk is small
  (the Board's, `nodes/6`, is 6 KB and compiles in 0.38 ms), so its first
  visit costs rendering, not compiling. Opening an issue is 3 ms sooner to
  its mark; the screens move by 1-4 ms either way, within the ranges.
- **Idle** is the same: no frames, 34 and 35 wake-ups in 20 s. CPU over
  the 20 s is 11.4 against 7.0 ms.
- **Memory.** RAM 5 s after load is 5.5 MB lower over all start-up and
  idle runs: QuickJS keeps no source text for functions loaded without it.
- No module fell back to its JavaScript in any run (`[host] … loading the
  JavaScript instead` appears in no log).

### Where start-up and first visits spent their time on main

Headless (`drive` on the release host, the bench Tracker, 1,000 issues,
1200x800), timed in the QuickJS loader of a throwaway build; medians of 5.
Parse and compile are one pass in QuickJS; a graph is linked in one pass,
so linking is per navigation, not per module.

| | modules | JavaScript | read | parse and compile | link | evaluate |
|---|---:|---:|---:|---:|---:|---:|
| start-up | 36 | 442 KB | 1.08 ms | 22.4 ms | 0.33 ms | 3.10 ms |
| open an issue | 2 | 28 KB | 0.19 | 2.05 | 0.04 | 0.12 |
| Board | 1 | 6 KB | 0.10 | 0.38 | 0.03 | 0.02 |
| Insights | 1 | 35 KB | 0.11 | 1.81 | 0.04 | 0.07 |
| Settings | 1 | 15 KB | 0.09 | 0.79 | 0.03 | 0.04 |

The page's own script (`page.js` and the page's nodes, 6.9 KB) costs
0.30 ms to compile and run at every launch, and stays JavaScript.

### Bytecode sizes, the Tracker's 45 modules

| | bytes | of the JavaScript |
|---|---:|---:|
| the minified JavaScript | 541,963 | 1.00 |
| bytecode with source text and debug info | 1,685,410 | 3.11 |
| **bytecode without source text, with debug info** | **896,939** | **1.65** |
| the same with calyx's headers: what `--compile` writes | 903,000 | 1.67 |
| bytecode without debug info (no file or line in an error's stack) | 761,655 | 1.41 |

Loading all 45 from bytecode takes 4.4 ms where compiling them takes 24.7
(a throwaway program on rquickjs 0.14, QuickJS-ng 0.16.2).

### The app folder, the build and the host

| | main | bytecode |
|---|---:|---:|
| app folder (the bench Tracker): files | 60 | 105 |
| app folder: bytes in the files | 847,618 | 1,750,620 |
| app folder: on disk (`du -sk`) | 972 KB | 1,964 KB |
| files the app reads to show its first screen | 38 | 38 |
| `pnpm --filter tracker build`, from nothing | 1.91 s | 1.97 s |
| the same after an edit to one component (`Row.svelte`) | 2.03 s | 2.09 s |
| of which `calyx --compile` (45 modules) | | 38-41 ms |
| dist host, darwin-arm64, bytes | 18,320,896 | 18,403,696 (+82,800) |

Build times are medians of 3, interleaved. The edit was timed with two
builds in a row: `vite build --watch` saw no file events from the shell
these ran in, on main as on the branch; a watch rebuild runs the same
adapter and the same compile. The `.js` files ship as well: they are what
loads when the bytecode does not fit the host, and what V8 loads.
''')
