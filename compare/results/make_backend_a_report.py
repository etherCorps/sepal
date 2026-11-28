"""Async context and the web module (backend PR A), against main, at 1,000 issues.

    python3 compare/results/make_backend_a_report.py [session] > compare/results/backend-a.md

One interleaved session (default be-1), Stage Manager off, under `caffeinate -di`,
warm-up and aborted runs left out:
  main      Sepal on main at 960a41c                                                build blitzb
  branch    Sepal on backend at 9e55cb6: the value carried through promise jobs      build blitzc
            (QuickJS patch 2), AsyncLocalStorage, the web module (web.js) in every
            app, fetch streaming its body
Each bench build was bundled by its own dist-profile host. Each cell is the
median of the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'be-1'
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



def stamps(r):
    """The run's log: (ms from exec, line). Lines are stamped in ns on arrival;
    exec is placed from the `start` mark, whose time from exec the harness knows."""
    lines = (RESULTS / r['log']).read_text().splitlines()
    start = next((l for l in lines if 'MARK start' in l), None)
    at = mark('start')(r)
    if start is None or at is None:
        return []
    exec_ns = float(start.split()[0]) - at * 1e6
    return [((float(l.split()[0]) - exec_ns) / 1e6, l) for l in lines if l[:1].isdigit()]


def first_frame(r):
    """The first frame presented: the first full repaint from 10 ms before the
    window is on screen. Earlier repaints are drawn while macOS has not shown
    the window yet, and dropped (the surface is occluded); the harness sees a
    window a few ms after its first frame is presented."""
    window = startup('window')(r)
    if window is None:
        return None
    return next((t for t, l in stamps(r) if 'damage: full repaint' in l and t >= window - 10), None)


def with_issues(r):
    """1 when the first frame painted came after `mounted`: the issues were in it."""
    f, m = first_frame(r), mark('mounted')(r)
    return None if f is None or m is None else (1 if f >= m else 0)


def yes_count(scenario):
    def text(build):
        rs = runs(build, scenario)
        n = sum(with_issues(r) or 0 for r in rs)
        return f'{n} of {len(rs)}'
    return text

print(__doc__.split('\n\n')[0].strip('"'), '\n')
print(f'Session `{SESSION}`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, '
      'no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app.\n')

STARTUP = [
    ('the page started (host trace), ms', page_started, 1),
    ('first JavaScript (`start` mark), ms', mark('start'), 1),
    ('window on screen, ms', startup('window'), 1),
    ('first frame presented, ms', first_frame, 1),
    ('fixture read (`read` mark), ms', mark('read'), 1),
    ('fixture read, after the first frame presented, ms', lambda r: None if first_frame(r) is None or mark('read')(r) is None else mark('read')(r) - first_frame(r), 1),
    ('issues on the page (`mounted` mark), ms', mark('mounted'), 1),
    ('first frame after `mounted`, ms', frame_after_mounted, 1),
    ('first key press answered (`j`), ms', first_input, 1),
    ('RAM 5 s after load, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
    ('CPU time, launch to settled, ms', cpu, 0),
]
for title, n in [('Start-up: first launch after quit (scenario 1)', 1), ('Start-up: warm start (scenario 2)', 2)]:
    table(title, [n], STARTUP)
    print(f"The first frame presented had the issues in it: main {yes_count(n)('blitzb')}, branch {yes_count(n)('blitzc')}.\n")
table('Idle, 20 s (scenario 3)', [3], [
    ('CPU time, ms', cpu, 1),
    ('GPU time, ms', gpu, 2),
    ('frames presented', presented, 0),
    ('interrupt wake-ups', wake, 0),
    ('RAM 5 s after load, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
    ('RAM at the end, MB', lambda r: ram(r.get('footprint')), 1),
])

print("""### Notes

- **Idle is the same**: no frames, 33 against 34 wake-ups in 20 s, CPU 8.8
  against 11.7 ms, RAM the same.
- **Start-up is 5 ms later to the issues and 11-12 ms later to the first
  frame.** The page starts as fast (27-28 ms) and the first JavaScript runs
  at the same time, so loading the web module's bytecode is not it (~0.4 ms
  measured on its own). The first difference is the shell's first redraw,
  after the window is made: 162 against 157 ms (medians over both
  scenarios), everything after it moving with it. Both builds have the same
  one slow turn after the first paint (12-14 ms, the rows drawn). Not traced
  further; the bigger heap the web module brings (one more automatic
  collection while the app starts) is the likeliest cause, and making the
  streams and FormData load on first use would be the fix to try.
- **RAM** 5 s after load is 3 MB lower in scenarios 1 and 2 and the same
  idle: within what a launch varies (one drawable is 15 MB).
- **The first frame had the issues** in 2 of 3 first launches on the branch
  against 3 of 3, and 3 of 3 on both warm.

### Sizes

| | main | branch |
|---|---:|---:|
| host, packaged (dist, `screenshot`), bytes | 18,784,576 | 18,801,168 (+16,592) |
| counter's JavaScript, bytes | 66,190 | 156,605 (+90,415) |
| Tracker's JavaScript, bytes | 543,081 | 620,063 (+76,982) |
| Tracker's bytecode, bytes | 905,060 | 1,000,807 (+95,747) |

The app's growth is the web module: web-streams-polyfill (68 KB of
JavaScript, 82 KB of bytecode), formdata-node (6 KB), and the Fetch API
grown to its full Body. The counter grows by all of it; the Tracker
already had URL, TextEncoder and AbortController from the DOM layer.
""")
