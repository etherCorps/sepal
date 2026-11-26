"""SvelteKit's split build against one bundle, both loaded from index.html, and main, at 1,000 issues.

    python3 compare/results/make_single_report.py [session] > compare/results/single-bundle.md

One interleaved session (default html-2), Stage Manager off, warm-up and aborted runs left out:
  main    Sepal on main at bae68bb (one app.js, from sepalKit())                 build blitz
  split   Sepal on html-entry, SvelteKit's default split build                    build blitzb
  single  Sepal on html-entry, the same app with output.bundleStrategy 'single'   build blitzc
Each cell is the median of the runs, with the range under it.
"""
import json
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'html-2'
COLS = [('main', 'blitz'), ('split', 'blitzb'), ('single', 'blitzc')]
MB = 2**20
HELP = {'SetStoreUpdateService', 'com.apple.SafariPlatformSupport.Helper', 'SafariPlatformSupport.Helper'}

RUNS = [json.loads(line) for line in (RESULTS / 'raw' / SESSION / 'runs.jsonl').open()]


def runs(build, n):
    return [r for r in RUNS if r['status'] == 'ok' and not r.get('warmup') and r['build'] == build and r['scenario'] == n]


def cell(vals, d):
    vals = [v for v in vals if v is not None]
    if not vals:
        return 'n/a'
    f = lambda v: f'{v:,.{d}f}'
    lo, hi = min(vals), max(vals)
    return f'{f(st.median(vals))}<br><sub>{f(lo)}–{f(hi)}</sub>' if len(vals) > 1 and lo != hi else f(st.median(vals))


def table(title, scenarios, rows):
    print(f'### {title}\n')
    print('| | ' + ' | '.join(c for c, _ in COLS) + ' |')
    print('|---|' + '---:|' * len(COLS))
    for label, fn, d in rows:
        print(f'| {label} | ' + ' | '.join(cell([fn(r) for n in scenarios for r in runs(b, n)], d) for _, b in COLS) + ' |')
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


def presented(r):
    d = r.get('trace_damage')
    return d['full'] + d['partial'] if d else None


ram = lambda fp: sum(p['footprint'] for p in fp['processes'] if p['name'] not in HELP) / MB if fp else None
gpu_mem = lambda fp: fp.get('gpu', 0) / MB if fp else None
cpu = lambda r: r['usage']['cpu_ns'] / 1e6
gpu = lambda r: r['gpu_ns'] / 1e6 if 'gpu_ns' in r else None
wake = lambda r: r['usage'].get('interrupt_wkups')

print(__doc__.split('\n\n')[0].strip('"'), '\n')
print(f'Session `{SESSION}`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off. '
      'M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app. '
      'Frames are those presented (damage tracking drew all or part of the window). '
      'All three run the same app; split and single run the same host.\n')

STARTUP = [
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
table('Open an issue (scenario 5)', [5], [
    ('click row → mark, ms', step('click row', 'ms_to_mark'), 1),
    ('click row → frame, ms', step('click row', 'ms_to_frame'), 1),
    ('Escape → mark, ms', step('Escape', 'ms_to_mark'), 1),
    ('Escape → frame, ms', step('Escape', 'ms_to_frame'), 1),
])
table('Switch screens (scenario 10)', [10], [
    *[(f'{to} → mark, ms', step(f'click nav-{to}', 'ms_to_mark'), 1) for to in ('board', 'insights', 'settings', 'issues')],
    *[(f'{to} → frame, ms', step(f'click nav-{to}', 'ms_to_frame'), 1) for to in ('board', 'insights', 'settings', 'issues')],
])
table('RAM 5 s after load, every run (scenarios 1, 2, 5 and 10)', [1, 2, 5, 10], [
    ('RAM, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
])
SESSION_NOTES = '''- **The start-up rows are valid.** With Stage Manager off, every window
  was on screen at 214-271 ms, and none waited for the harness to
  activate it.
- **Start-up.** Single starts its JavaScript 15 ms after split (one
  570 KB module is parsed before its first line runs; split parses the
  entry and its imports module by module) and 5 ms after main. On a
  first launch the issues are on the page 18 ms after split and main; on
  a warm start the three are within each other's ranges. CPU time from
  launch to settled is the same within 10 ms.
- **Memory.** Split holds 5 MB less 5 s after load (111 against 116.5);
  single holds what main holds. Most likely that is the code of the
  routes not opened yet, which one bundle loads and compiles at start-up.
- **Navigation.** Single takes back most of what split loses: opening an
  issue is 23.3 ms to its mark (split 28.7, main 19.0), Escape 38.8 (50.9,
  33.3), and Insights, Settings and the way back to Issues come out as
  main's, give or take 4 ms.
- **The Board's mark row is misleading.** Single's Board → mark (99.7)
  equals split's, but the turn that renders the Board, read from the
  window trace, is 96.4 ms for single, 103.9 for split and 87.1 for main
  (the same order headless: 112, 119, 104). Single sets its mark later
  inside that turn (73 ms in, against 58 on main), and the mark row
  counts that as more time. What is left between single and main is
  roughly in proportion to the work: 1.5-2.5 ms on a small navigation,
  8-9 ms on the Board, whose turn is 20,000 ops. Not looked into.'''

print('''### Notes

SESSION_NOTES

### Escape back to the list: the 325 ops

In session `html-1` the Escape turn had 1,417 ops on the branch against
1,092 on main. Logged headless (`drive`, the same steps, every batch the
host applied), the two turns differ in one op kind only: 350 `DROP`
against 25. Every other kind, `CREATE_*`, `SET_*`, `INSERT`, `REMOVE`,
`LISTEN`, comes to the same count; the only difference in targets is that
the list goes back into the parsed mount node (`page#61`) instead of
main's root (`#1`). The 325 extra `DROP`s are the first list's nodes
(141 text nodes, 87 comments, 76 spans, the icons) freed by a QuickJS
cycle collection that the branch's allocations bring forward into that
turn; on main it falls in a later one (headless, in the second open). It
costs that turn about 5 ms: with a collection forced before each step,
Escape is 19.6 ms instead of 24.5. A shift in where the collection lands,
not more work.

### Opening the same issue twice, headless

`drive` on the release host, 1200x800, 1,000 issues: open the first row,
Escape, three times; each step timed whole (the click, the frame and the
timer SvelteKit waits for, settled). Median of 7 rounds, with a cycle
collection forced before each step so that it does not land in one of
them. ms:

| | open 1 | Escape 1 | open 2 | Escape 2 | open 3 | Escape 3 |
|---|---:|---:|---:|---:|---:|---:|
| main | 29.4 | 13.4 | 18.3 | 12.1 | 17.6 | 11.9 |
| split | 41.6 | 19.6 | 25.9 | 19.0 | 24.9 | 18.7 |
| single | 33.9 | 15.1 | 20.6 | 13.7 | 19.6 | 13.6 |
| split, nonce lookup taken out | 35.7 | 16.2 | 22.9 | 15.6 | 22.1 | 15.4 |
| split, and the URL parses cached | 37.4 | 14.7 | 20.6 | 14.0 | 19.8 | 13.8 |

- **Compiling the route is 2-4 ms of the first visit.** The first open
  costs split 15.7 ms more than the second, and single 13.3 ms more
  (main 11.1: the first render of the issue page, whatever the build).
  The difference, 2.4 ms, is reading and compiling the route's chunks;
  against the last row (split, helper patched) it is 3.5 ms.
- **The rest of the gap is paid on every visit, the second too:** split
  is about 5 ms slower than single on each navigation. It is Vite's module
  preload helper (`__vitePreload`), which SvelteKit's split build calls
  for every route node on every navigation, loaded or not: it looks up
  `meta[property=csp-nonce]` with `document.querySelector` (a walk of the
  whole tree in the DOM layer: 0.39 ms a call on the issues page), and parses two `URL`s per dependency (8 to 25 per node)
  before its cache of what it has seen. In a copy of the split build with
  the nonce lookup taken out, a navigation is about 3 ms faster; with the
  URL parses cached as well, it is as fast as single. Bytecode would not
  remove this; a faster `querySelector` for that selector in the DOM
  layer, or `sepal()` replacing the helper, would.
- **Single is still 1.5-2.5 ms slower than main per navigation.** Not
  looked into.

### The app and the host

| | main | split | single |
|---|---:|---:|---:|
| app folder (the bench Tracker): files | 13 | 60 | 15 |
| app folder: bytes in the files | 871,912 | 845,442 | 873,261 |
| app folder: on disk (`du -sk`) | 880 KB | 972 KB | 892 KB |
| files the app reads to show its first screen | 9 | 46 | 10 |

Single: `index.html`, `app.json`, the icon, then from the page one
JavaScript module (`bundle.*.js`, 570 KB), one stylesheet and the same 5
fonts. Counted with `drive … files:` after the first layout.

Release host (`calyx`, `--features screenshot`, thin LTO, stripped),
bytes, from the release workflow: the published 0.2.0-alpha.0, and the
dry run on html-entry. Split and single use the same host.

| Platform | 0.2.0-alpha.0 (main) | html-entry | |
|---|---:|---:|---|
| darwin-arm64 | 25,493,936 | 26,007,952 | +514,016 (+2.0 %) |
| darwin-x64 | 26,499,360 | 27,303,208 | +803,848 (+3.0 %) |
| linux-arm64-gnu | 29,766,944 | 30,496,032 | +729,088 (+2.4 %), now over 30,000,000 (29.1 MiB) |
| linux-x64-gnu | 32,608,272 | 33,530,736 | +922,464 (+2.8 %), over before this branch |
'''.replace('SESSION_NOTES', SESSION_NOTES))
