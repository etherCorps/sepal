"""The app as a web build, loaded from index.html: Sepal before and after, and Tauri 2, at 1,000 issues.

    python3 compare/results/make_html_report.py [session] > compare/results/html-entry.md

One interleaved session (default html-1), warm-up and aborted runs left out:
  before  Sepal on main at bae68bb (one app.js, from sepalKit())        build blitz
  after   Sepal on html-entry (index.html, SvelteKit's split build)     build blitzb
  tauri   Tauri 2, built from the same SvelteKit setup                  build tauri
Each cell is the median of the runs, with the range under it.
"""
import json
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'html-1'
COLS = [('Sepal, before', 'blitz'), ('Sepal, after', 'blitzb'), ('Tauri 2', 'tauri')]
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
print(f'Session `{SESSION}`, interleaved, 1 warm-up and 3 runs per build and scenario. '
      'M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app. '
      'Frames are those presented (damage tracking drew all or part of the window); '
      'Tauri has no frame trace.\n')

STARTUP = [
    ('first JavaScript (`start` mark), ms', mark('start'), 1),
    ('window on screen, ms', startup('window'), 1),
    ('issues on the page (`mounted` mark), ms', mark('mounted'), 1),
    ('first frame after `mounted`, ms', frame_after_mounted, 1),
    ('first key press answered (`j`), ms', first_input, 1),
    ('RAM 5 s after load, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
    ('GPU memory 5 s after load, MB', lambda r: gpu_mem(r.get('footprint_mounted5')), 1),
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
table('Switch screens (scenario 10)', [10], [
    *[(f'{to} → mark, ms', step(f'click nav-{to}', 'ms_to_mark'), 1) for to in ('board', 'insights', 'settings', 'issues')],
    *[(f'{to} → frame, ms', step(f'click nav-{to}', 'ms_to_frame'), 1) for to in ('board', 'insights', 'settings', 'issues')],
])

table('RAM 5 s after load, all start-up and idle runs (scenarios 1, 2 and 3)', [1, 2, 3], [
    ('RAM, MB', lambda r: ram(r.get('footprint_mounted5')), 1),
])

print('''### Notes

- **Tauri's window rows are not valid in this session.** Stage Manager kept
  the rebuilt Tauri app's window off screen until the harness activated it
  after 5 s (12 of its 20 runs; 1 of Sepal's, outside the start-up rows).
  That is the window, first frame and first key rows; its `mounted` mark,
  memory and CPU are unaffected. In session `sk-1` its window was on
  screen at about 200 ms.
- **Start-up.** JavaScript starts 9-12 ms sooner (no 1 MB script to parse
  before the first line runs), and the issues are on the page 20 ms later:
  36 modules are read and compiled instead of one. Without bytecode
  (the next job), each is parsed by QuickJS from source.
- **Navigation pays for what is loaded late.** Opening an issue is 9 ms
  slower to its mark: its route's chunk is read and compiled on the first
  open (2 more files), where the single bundle paid for it at start-up. In
  the window trace the navigation turn is 30.5 ms against 20.4, with the same
  number of ops. The Board and Insights, opened for the first time in
  scenario 10, are 25 and 12 ms slower; Settings and the way back to
  Issues, whose code is mostly loaded already, are 2 and 5 ms slower. Escape back to the list is 18 ms slower
  at the median, with 1,417 ops against 1,092 in its turn: 325 more `DROP`s,
  from a cycle collection that lands in that turn on this branch. Most of
  the navigation gap is not compiling but Vite's preload helper, paid on
  every visit; `single-bundle.md` has both.
- **Idle** is the same: no frames, the same wake-ups.
- **Memory.** RAM 5 s after load is about 4 MB lower over all start-up and
  idle runs (one script's source and its compiled code no longer held
  whole); single scenarios scatter by 15-25 MB, depending on whether the
  kept frame and depth buffer had been let go yet (CLAUDE.md, "Where memory
  goes in a window").

### The app and the host

Measured on 25 Nov 2026, the builds above.

| | Sepal, before | Sepal, after |
|---|---:|---:|
| app folder (the bench Tracker): files | 13 | 60 |
| app folder: bytes in the files | 871,907 | 845,436 |
| app folder: on disk (`du -sk`) | 880 KB | 972 KB |
| files the app reads to show its first screen | 9 | 46 |

Before: `app.js`, `app.css`, `app.json`, the icon and 5 fonts. After:
`index.html`, `app.json`, the icon, then 43 from the page: 36 JavaScript
modules (of 45 chunks), 2 stylesheets and the same 5 fonts. Counted with
`drive … files:` after the first layout, and on main with a log line in a
throwaway build. More bytes on disk for fewer bytes of files: 60 files of
4 KB blocks.

Release host (`calyx`, `--features screenshot`, thin LTO, stripped), bytes:

| Platform | 0.2.0-alpha.0 (main) | html-entry | |
|---|---:|---:|---|
| darwin-arm64, built here | 25,437,648 | 26,102,336 | +664,688 (+2.6 %) |
| darwin-arm64, release CI | 25,493,936 | 26,007,952 | +514,016 (+2.0 %) |
| darwin-x64, release CI | 26,499,360 | 27,303,208 | +803,848 (+3.0 %) |
| linux-arm64-gnu, release CI | 29,766,944 | 30,496,032 | +729,088 (+2.4 %), now over 30,000,000 (29.1 MiB) |
| linux-x64-gnu, release CI | 32,608,272 | 33,530,736 | +922,464 (+2.8 %), over the 30 MB budget before this branch |

The release CI column for main is the published 0.2.0-alpha.0; for
html-entry, the release workflow's dry run on the pushed branch. The
difference is html5ever and the QuickJS module loader.
''')
