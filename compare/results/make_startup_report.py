"""Start-up in parallel (startup-parallel), against main, at 1,000 issues.

    python3 compare/results/make_startup_report.py [session] > compare/results/startup-parallel.md

One interleaved session (default startup-1), Stage Manager off, under `caffeinate -di`,
warm-up and aborted runs left out:
  main      Sepal on main at 75b21db                                                  build blitzb
  branch    Sepal on startup-parallel at 47a3622: fonts and the GPU on threads, the   build blitzc
            Han fallback list asked for early, the first poll without ResumeReady.
            The branch's last commit (74ae246) replaced that first poll's first
            form; both measured the same in alternating launches (see the notes)
The branch changes the host only. One bench build of the app, bundled
twice, each time by its own dist-profile host. Each cell is the median of
the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'startup-1'
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

- **The window is on screen 16-17 ms sooner, its first frame 21-23 ms
  sooner, and that frame has the issues**: 3 of 3 runs in each scenario
  against 0 of 3 on main. The issues are on the page 70-72 ms sooner,
  before the window is shown; the first key press is answered 19-25 ms
  sooner.
- **"The page started" no longer compares like for like.** It is timed
  from the document's creation to the page settled, and on the branch the
  system's fonts are listed before that, on a thread, while the event loop
  is made: 28 against 64 ms says where the fonts went, not that the page
  starts 36 ms sooner. The first JavaScript (`start`) runs 2-7 ms later,
  as the event loop is now made before the page.
- **First frame presented** is the first full repaint from 10 ms before
  the window is on screen. Repaints before that are drawn while macOS has
  not shown the window and are dropped (the surface is occluded). The
  branch measured here (the first form of its last change) painted two of
  those, ~150 and ~163 ms from exec; the final form paints no more of them
  than main (below).
- **CPU from launch to settled** is 16 ms higher in scenario 1 and 2 ms
  in scenario 2. With the probe launcher, the process's CPU over its first
  3 s was 225.7 ms on main and 229.9 on the final branch (10 launches
  each), and the same for both forms of change 4. Not broken down.
- **Idle** is the same: no frames, 31-34 wake-ups in 20 s.
- **RAM** 5 s after load is the same in scenarios 1 and 2. The idle runs
  show 16 MB less on the branch, but main's idle RAM was 105-107 MB in
  `nav-1` and `alloc-1`: this session's 122 MB on main is the odd one. 15-16
  MB is about one drawable (2400 x 1600 x 4 bytes), which the surface keeps
  or does not from launch to launch. The branch does not lower RAM.
- **Host size:** 18,784,576 bytes packaged (dist, `screenshot`), 33,200
  more than main.

### Each change alone

Headless timings cannot see a window, so each change was measured with
start-up probes in a dist host (`[probe]` lines: wall time and the UI
thread's CPU time) and the bench Tracker bundled, 5 or 10 launches each,
alternating with the change before it. ms from exec, medians:

| change | window visible | first frame | issues on the page | also |
|---|---:|---:|---:|---|
| 1. fonts listed on a thread while the event loop is made | 253 -> 228 | 260 -> 236 | | |
| 2. the Han fallback list asked for on that thread | 225 -> 222 | | | window and first layout 56.8 -> 52.7 |
| 3. GPU device and pipelines on a thread | 217 -> 197 | 226 -> 204 | 211 -> 192 | window and first layout 51.4 -> 35.1 |
| 4. the first poll without ResumeReady | 200 -> 204 | 209 -> 212 | 194 -> 165 | issues 37 ms before the window, not 5 |

Change 4's two forms, alternating, 10 launches each: the first, which
called blitz-shell's `complete_resume` early, rendered 3 (2-3) frames
before the window was visible, all dropped; the final one, which polls the
document with a waker of its own and leaves the first paint to
blitz-shell, renders 2 (1-2), and in a batch against main 1 (0-2), as main
does (1, 0-2). Window, first frame, issues and CPU were the same for both
forms. The final branch against main in that batch: window visible 242.6
-> 205.5, first frame 253.6 -> 214.0, issues 233.1 -> 166.1, the first
frame with the issues 7 of 10 -> 10 of 10.

### The first layout

The first style and layout, made when the window is, took 20-24 ms:
style 1.1-1.6, layout 0.2-0.3, and building the layout's boxes and text
(`construct`) 18-22. Of that, font work was ~5 ms: the Han fallback list
(4.3 ms, now asked for on the font thread), 390 family lookups and 391 font
files (~0.1 ms), 2,261 font matches (~0.9 ms). The 192 inline layouts took
under 0.12 ms each. The other ~14 ms of `construct` is not font work and
was not traced.
""")
