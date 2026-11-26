"""SvelteKit's split build after the DOM layer stopped walking the page for selectors that end in a tag, against one bundle and main, at 1,000 issues.

    python3 compare/results/make_preload_report.py [session] > compare/results/preload-fix.md

One interleaved session (default html-3), Stage Manager off, warm-up and aborted runs left out:
  main    Sepal on main at bae68bb (one app.js, from sepalKit())                      build blitz
  single  Sepal on html-entry at 3e3bf38, output.bundleStrategy 'single'               build blitzc
  split   Sepal on html-entry at f225c91, SvelteKit's default split build, the fix     build blitzd
Each cell is the median of the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'html-3'
COLS = [('main', 'blitz'), ('single', 'blitzc'), ('split, fixed', 'blitzd')]
SCREENS = ('board', 'insights', 'settings', 'issues')

RUNS = [json.loads(line) for line in (RESULTS / 'raw' / SESSION / 'runs.jsonl').open()]


def runs(build, n):
    return [r for r in RUNS if r['status'] == 'ok' and not r.get('warmup') and r['build'] == build and r['scenario'] == n]


def fmt(vals, d):
    vals = [v for v in vals if v is not None]
    if not vals:
        return 'n/a'
    f = lambda v: f'{v:,.{d}f}'
    lo, hi = min(vals), max(vals)
    return f'{f(st.median(vals))}<br><sub>{f(lo)}–{f(hi)}</sub>' if len(vals) > 1 and lo != hi else f(st.median(vals))


def step(label, key):
    def get(r):
        s = next((s for s in r['steps'] if s['label'] == label), None)
        return s.get(key) if s else None
    return get


def turn(label, mark):
    """The navigation's turn in the window trace: the `poll took` line after its MARK."""
    def get(r):
        lines = (RESULTS / r['log']).read_text().splitlines()
        i = next((k for k, l in enumerate(lines) if f'MARK {mark}' in l), None)
        if i is None:
            return None
        m = next((re.search(r'poll took ([\d.]+)ms', l) for l in lines[i:] if 'poll took' in l), None)
        return float(m.group(1)) if m else None
    return get


def table(title, n, rows):
    print(f'### {title}\n')
    print('| | ' + ' | '.join(c for c, _ in COLS) + ' | split − single |')
    print('|---|' + '---:|' * (len(COLS) + 1))
    for label, fn, d in rows:
        meds = [[fn(r) for r in runs(b, n)] for _, b in COLS]
        gap = st.median([v for v in meds[2] if v is not None]) - st.median([v for v in meds[1] if v is not None])
        print(f'| {label} | ' + ' | '.join(fmt(v, d) for v in meds) + f' | {gap:+.1f} |')
    print(f"\n<sub>Runs: {', '.join(f'{c}: {len(runs(b, n))}' for c, b in COLS)}.</sub>\n")


print(__doc__.split('\n\n')[0].strip('"'), '\n')
print(f'Session `{SESSION}`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off. '
      'M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from the input. '
      'Single and split run the same host; `split − single` is the difference of the medians.\n')

table('Open an issue (scenario 5)', 5, [
    ('click row → mark, ms', step('click row', 'ms_to_mark'), 1),
    ('click row → frame, ms', step('click row', 'ms_to_frame'), 1),
    ('Escape → mark, ms', step('Escape', 'ms_to_mark'), 1),
    ('Escape → frame, ms', step('Escape', 'ms_to_frame'), 1),
])
table('Switch screens (scenario 10)', 10, [
    *[(f'{to} → mark, ms', step(f'click nav-{to}', 'ms_to_mark'), 1) for to in SCREENS],
    *[(f'{to} → frame, ms', step(f'click nav-{to}', 'ms_to_frame'), 1) for to in SCREENS],
    *[(f'{to}: the turn that renders it, ms', turn(f'click nav-{to}', f'nav:/{to}'), 1) for to in SCREENS],
])

print('''### Against the 2 ms mark

Not there. The fixed split build is within 2 ms of single on Settings and
Insights, and on Escape, where single's median sits on a fast run (its
range is 24-40 ms), but opening an issue is +2.9 ms to its mark and the
Board's turn +9.4 ms. Both are first visits: every navigation in these
scenarios opens a route for the first time since launch.

### Headless: first and second visits

`drive` on the release host, 1200x800, 1,000 issues, a cycle collection
forced before each step, each step timed whole. Median of 7 rounds
(issues) and 5 (screens), ms, on a quiet machine after the session.
"Split, before" is 1d4c8d0's build, "after" f225c91's.

| | open 1 | Escape 1 | open 2 | Escape 2 | open 3 | Escape 3 |
|---|---:|---:|---:|---:|---:|---:|
| main | 28.3 | 13.3 | 18.6 | 11.5 | 16.9 | 11.4 |
| split, before | 39.6 | 19.4 | 24.7 | 18.3 | 24.1 | 18.4 |
| split, after | 36.3 | 14.6 | 19.5 | 13.1 | 18.7 | 12.8 |
| single | 32.6 | 14.7 | 19.9 | 13.2 | 18.7 | 13.2 |

| | Board | Insights | Settings | Issues | Board again | Insights again | Settings again | Issues again |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| main | 103.5 | 24.1 | 13.1 | 23.9 | 74.9 | 21.5 | 11.6 | 23.8 |
| split, before | 120.1 | 41.1 | 19.1 | 29.5 | 90.2 | 34.0 | 15.2 | 29.2 |
| split, after | 122.1 | 30.0 | 15.3 | 25.8 | 89.4 | 23.8 | 12.3 | 25.7 |
| single | 112.0 | 25.6 | 13.6 | 25.9 | 86.6 | 22.9 | 12.2 | 25.8 |
| split, after, without SvelteKit's link scan | 111.1 | 28.8 | 15.0 | 24.6 | 79.5 | 22.6 | 12.1 | 24.4 |

The last row is a copy of the fixed build with one loop taken out (below),
from a second round of 5; the other rows of that round were within 1 ms
of these.

### Walks of the whole page per navigation

Document-wide `querySelector`, `querySelectorAll`, `getElementsByTagName`
and `getElementById` calls that walked the page, counted in instrumented
copies of each build (headless, the same steps):

| | open issue | Escape | Board | Insights | Settings | Issues |
|---|---:|---:|---:|---:|---:|---:|
| main | 2 | 2 | 2 | 2 | 2 | 2 |
| split, before | 17 | 15 | 13 | 14 | 11 | 18 |
| split, after | 1 | 1 | 2 | 2 | 2 | 2 |

Before: `meta[property=csp-nonce]` 5-9 times (Vite's preload helper; no
page has one, so each walked everything), `title`, `link` on first visits,
`body`, and SvelteKit's own two. After: SvelteKit's own two, `[autofocus]`
and `querySelectorAll('a')`; the second is answered from the index on the
issue pages (64 links or fewer) and walks on the others.

### Where the rest goes

- **Against single: loading a route on its first visit.** The second
  visits are level (within 1 ms; the Board within 3). The first visit
  reads the route's chunks (Board: one file, `nodes/7`, 36 KB; an issue:
  two) and QuickJS compiles and runs them: Board +10 ms, Insights +4,
  an issue +3.7, Settings +2. Single pays for all of it at start-up (its
  `start` mark is 15 ms later in `html-2`). Bytecode is what would make
  that cheap for both.
- **Against main, also: SvelteKit's link scan.** After every navigation
  SvelteKit looks at each `<a>` in `document.documentElement` (its URL,
  its `data-sveltekit-*` options up the ancestors) to set up preloading.
  On main the DOM layer's `<html>` was a stand-in with no children, so the
  scan found nothing; now `<html>` is the page, and the Board's 316 links
  cost 10 ms (32 µs a link), Issues and Insights 1.5 ms. It is what a
  browser does; the cost per link is the DOM layer's (`href`, attributes,
  `closest`) and can come down without touching the build.
- **Left over:** without the scan, second visits are within about 1 ms of main,
  and the Board 4.7 ms behind. Not looked into.
''')
