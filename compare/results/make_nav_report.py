"""Navigation after the DOM layer's three fixes, against main, at 1,000 issues.

    python3 compare/results/make_nav_report.py [session] > compare/results/nav-cost.md

One interleaved session (default nav-1), Stage Manager off, warm-up and aborted runs left out:
  main      Sepal on main at 51d4598                                          build blitzb
  branch    Sepal on nav-cost at 6fec75a: the href cache, selector tests      build blitzc
            built once, url.js's lazy searchParams
Both bench builds were bundled by the same dist-profile host, built at
6fec75a (the branch changes no Rust the host runs): the two apps differ in
one chunk, the entry, which carries the DOM layer and url.js. Each cell is
the median of the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'nav-1'
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
      'no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from the input.\n')

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


print('''### Notes

- **The render turns are the cleaner number.** The Board's is 5.5 ms
  shorter and Issues' 3.2; Settings and Insights are within 1 ms. The
  marks and frames add the wait before that turn: SvelteKit follows a
  link from a `requestAnimationFrame` callback, which waits for the next
  vsync, and while the nav item's hover transition draws, frames are
  coming every 13 ms. Where the click lands in that cycle moves a mark by
  up to a frame. Insights shows it: the same render turn on both builds,
  and its mark 7.6 ms later on the branch. Headless, where there are no
  frames, its first visit is 1 ms quicker.
- **Idle** is the same: no frames, 34 and 32 wake-ups in 20 s.

### Where SvelteKit's link scan went

After every navigation SvelteKit takes `querySelectorAll('a')` and, for
each link, its `href`, a `new URL` of it, and five `getAttribute`s on every
ancestor up to `<html>`, to set up preloading. Timed piece by piece on the
Board (316 links, 8.9 ancestors each, 3,529 elements), headless, µs a link:

| | before | after |
|---|---:|---:|
| `a.href` | 4.9 | 0.47 |
| `new URL(a.href, document.baseURI)` | 4.27 | 0.87 |
| the ancestor walk (`getAttribute` and `parentNode`) | 4.35 | 4.11 |
| the loop, whole | 16.3 | 7.28 |
| `querySelectorAll('a')`, per call | 1,775 | 800 |
| `querySelector('[autofocus]')`, per call | 2,475 | 750 |

One commit for each, with a cycle collection before each timed step
(`drive`'s `gc:`): the Board's repeat visit went from 76 to 73 ms with the
`href` cache, to 70 with the selector tests, and to 69 with url.js. The
ancestor walk is SvelteKit's own five calls a level, which no change on
this side makes fewer.

The 4.7 ms the Board trailed the old main by (`preload-fix.md`, without the
scan) was `[autofocus]`, the other walk of the whole page: with both
walks taken out, the Board's repeat visit was 72 ms, the old main's
(`bae68bb`) 72 ms.

### First visits

A screen's first visit is not slower in the split build for any reason of
its own: headless, the split build trails a single bundle by 1-1.5 ms on
Insights, Settings and opening an issue (the route's chunk read and Vite's
preload helper adding a `modulepreload` link), and not at all on the
Board. The route's chunk is read in the turn that renders it, no turn waits
for a stylesheet, and the turns are the same: the click's frame, then the
timer that navigates and renders.

The Board's first visit costs ~50 ms over a repeat (136 against 76 ms
here), on the old main too. It is spread evenly over the 300 cards it
draws (210-320 µs a card, against 130-145), and it is neither `cn()` (0.55
ms), nor Svelte's templates (a custom renderer gets new nodes every time),
nor Bits UI, the app's data, QuickJS's collector or its warm-up. Making and
dropping 20,000 of the renderer's nodes before the first visit takes all of
it away: the process makes nodes in numbers it has not before. Which part
of that growth costs, JavaScript's or the allocator's, is the next job.

| headless, ms | first visit | repeat | first, 20,000 nodes made first | repeat, same |
|---|---:|---:|---:|---:|
| before | 134-136 | 76-77 | 76-78 | 70-72 |
| after | 132-136 | 72-73 | 73 | 67-68 |
''')
