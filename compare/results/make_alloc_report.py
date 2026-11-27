"""QuickJS-ng's Map and Set hash patched, against main, at 1,000 issues.

    python3 compare/results/make_alloc_report.py [session] > compare/results/alloc.md

One interleaved session (default alloc-1), Stage Manager off, warm-up and aborted runs left out:
  main      Sepal on main at 21df621 (PR #13): QuickJS-ng 0.16.2 as on crates.io   build blitzb
  branch    Sepal on alloc at 6050f37: map_hash_key mixes every non-string key   build blitzc
The branch changes the host only. One bench build of the app, bundled
twice, each time by its own dist-profile host: main's was built from the
vendored rquickjs-sys with quickjs.c as on crates.io, the branch's with the
patch. The session ran before PR #13 was merged, at 43e5fb7, which has
main's tree. Each cell is the median of the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'alloc-1'
COLS = [('main', 'blitzb'), ('branch', 'blitzc')]
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
    print('| | ' + ' | '.join(c for c, _ in COLS) + ' | branch − main |')
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


def turn(mark_name, line='poll took'):
    """The navigation's turn in the window trace: the first `line` after its MARK.
    A link waits for a frame and navigates from a timer (`poll`); Escape navigates
    inside the key press (`KeyboardInput`)."""
    def get(r):
        lines = (RESULTS / r['log']).read_text().splitlines()
        i = next((k for k, l in enumerate(lines) if f'MARK {mark_name}' in l), None)
        if i is None:
            return None
        m = next((re.search(rf'{line} ([\d.]+)ms', l) for l in lines[i:] if line in l), None)
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
    ('fixture loaded → issues on the page, ms', lambda r: mark('mounted')(r) - mark('loaded')(r), 1),
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
    ('click row: the turn that renders it, ms', turn('nav:/issues/'), 1),
    ('Escape: the key press that renders it, ms', turn('nav:/issues ', 'KeyboardInput took'), 1),
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

- **Screens drawn for the first time are where it shows.** The turn that
  renders the Board is 21 ms shorter (100 to 79 ms), Insights' 6 ms and
  Issues' 5 ms; to the mark and to the frame the Board is 22 ms sooner and
  Insights 14. Settings has few nodes and moves by less than 1 ms. Those
  turns make thousands of nodes, and the renderer keys its node maps
  (`elements`, `by_tag`) by node id: ids that only go up, which QuickJS-ng
  0.16.2's hash sent to a few buckets (fixed upstream in #1931, not yet
  released).
- **Opening an issue** is 2.5 ms sooner to its mark and its frame, and its
  render turn 1 ms shorter. Escape's key press is within its range.
- **Start-up is the same.** The JavaScript that renders the list, from the
  fixture's `loaded` mark to `mounted`, is 7.3-7.5 ms on the branch against
  7.7-8.4 on main. On a first launch `loaded` comes at either ~220 or ~245
  ms, on both builds (the fixture is read on a thread): two of the
  branch's three runs and one of main's came late, which is all the
  +20 ms on `mounted` is. Warm starts are 7 ms sooner, within the ranges.
- **Idle** is the same: no frames, 33 and 34 wake-ups in 20 s.
- **Memory and size.** RAM 5 s after load is within 1 MB over the nine
  start-up and idle runs. The host is the same size: 18,751,376 bytes
  packaged (dist, `screenshot`), 18,387,184 as measured here (dist), on
  both.

### Headless, the same change

`drive`'s `eval:` on the release host, median of three launches. One
round: a new Map that sets, gets, tests and deletes 10,000 keys, then a
new Set that adds and tests them. ms a round:

| keys | before | after |
|---|---:|---:|
| ints 1 to 10,000 | 91.5 | 3.35 |
| small BigInts | 94.2 | 3.30 |
| floats (i + 0.5) | 19.9 | 3.45 |
| objects | 21.8 | 3.25 |
| symbols | 14.1 | 3.25 |
| ints from 1,000,000 | 2.55 | 3.40 |
| strings (unchanged) | 3.10 | 3.20 |

Ints from a million were spread well by luck (their doubles' high words
change with every key) and now cost what strings do.

The bench Tracker's Board at 1,000 issues, 1200x800, five launches each,
a cycle collection before each timed step:

| ms | first visit | repeat |
|---|---:|---:|
| before | 137-147 | 79 |
| after | 113-123 | 73-76 |

### What remains of a first visit

The Board's first visit still costs ~40 ms over a repeat. With counters
added to QuickJS for the purpose (a throwaway build), the first visit and
the repeat do the same work within 9%: 85k against 78k objects, 401k
against 369k mallocs, 17.8k against 16.7k shapes. QuickJS's other hash
tables are not the cause: shapes and property tables take their bucket
from the top bits of a multiplied hash, atoms hash every character into
the low bits, and each averages about one probe a lookup (Maps, patched,
1.3). The first visit runs two automatic collections (8-11 ms inside
them) where the repeat runs none, but with automatic collection off it is
only 3-6 ms quicker. The rest is the same operations costing more the
first time, which these counters do not explain; it is left for its own
job.
''')
