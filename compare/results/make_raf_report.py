"""requestAnimationFrame on real frames: Sepal before and after, and Tauri 2, at 1,000 issues.

    python3 compare/results/make_raf_report.py [session] > compare/results/raf-frames.md

One interleaved session (default raf-1), warm-up and aborted runs left out:
  before  Sepal at ed1be1d (PR #3: requestAnimationFrame a 16 ms timer)   build blitz
  after   Sepal on raf-frames (requestAnimationFrame on the host's frames) build blitzb
  tauri   Tauri 2                                                         build tauri
Each cell is the median of the runs, with the range under it.
"""
import json
import re
import statistics as st
import sys
from pathlib import Path

RESULTS = Path(__file__).parent
SESSION = sys.argv[1] if len(sys.argv) > 1 else 'raf-1'
COLS = [('Sepal, before', 'blitz'), ('Sepal, after', 'blitzb'), ('Tauri 2', 'tauri')]

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


def table(title, n, rows):
    print(f'### {title}\n')
    print('| | ' + ' | '.join(c for c, _ in COLS) + ' |')
    print('|---|' + '---:|' * len(COLS))
    for label, fn, d in rows:
        print(f'| {label} | ' + ' | '.join(cell([fn(r) for r in runs(b, n)], d) for _, b in COLS) + ' |')
    counts = ', '.join(f'{c}: {len(runs(b, n))}' for c, b in COLS)
    print(f'\n<sub>Runs: {counts}.</sub>\n')


def step(label, key):
    def get(r):
        s = next((s for s in r['steps'] if s['label'] == label), None)
        return s.get(key) if s else None
    return get


def presented(r):
    d = r.get('trace_damage')
    return d['full'] + d['partial'] if d else None


def log_lines(r):
    out = []
    for line in (RESULTS / r['log']).open(errors='replace'):
        parts = line.rstrip('\n').split(' ', 2)
        if len(parts) == 3 and parts[0].replace('.', '', 1).isdigit():
            out.append((float(parts[0]), parts[2]))
    return out


DAMAGE = re.compile(r'\[trace\] damage: (Full|Rect)\b')
WINDOW_NS = 200e6


def animation_fps(r, what):
    """Frames presented in the 200 ms after each such input (the 100 ms animation and what leads up to it)."""
    if r['build'] == 'tauri':
        return None
    frames = [t for t, s in log_lines(r) if DAMAGE.search(s)]
    starts = [i['t'] for i in r['inputs'] if i['what'] == what]
    if not starts:
        return None
    counts = [sum(1 for t in frames if s <= t < s + WINDOW_NS) for s in starts]
    return st.mean(counts) / (WINDOW_NS / 1e9)


cpu = lambda r: r['usage']['cpu_ns'] / 1e6
gpu = lambda r: r['gpu_ns'] / 1e6
wake = lambda r: r['usage'].get('interrupt_wkups')
fps = lambda r: presented(r) / r['measured']['seconds'] if presented(r) is not None else None

print(__doc__.split('\n\n')[0].strip('"'), '\n')
print(f'Session `{SESSION}`, interleaved, 1 warm-up and 3 runs per build and scenario. '
      'M4 Pro, 5K display at 75 Hz: 120 Hz is untested. '
      'Frames are those presented (damage tracking drew all or part of the window); '
      'Tauri has no frame trace.\n')

table('Open an issue (scenario 5)', 5, [
    ('click row → mark, ms', step('click row', 'ms_to_mark'), 1),
    ('click row → frame, ms', step('click row', 'ms_to_frame'), 1),
    ('Escape → mark, ms', step('Escape', 'ms_to_mark'), 1),
    ('Escape → frame, ms', step('Escape', 'ms_to_frame'), 1),
])
table('Switch screens (scenario 10)', 10, [
    *[(f'{to} → mark, ms', step(f'click nav-{to}', 'ms_to_mark'), 1) for to in ('board', 'insights', 'settings', 'issues')],
    *[(f'{to} → frame, ms', step(f'click nav-{to}', 'ms_to_frame'), 1) for to in ('board', 'insights', 'settings', 'issues')],
])
table('New issue (scenario 8)', 8, [
    ('Cmd+Enter → mark, ms', step('Cmd+Enter', 'ms_to_mark'), 1),
    ('Cmd+Enter → frame, ms', step('Cmd+Enter', 'ms_to_frame'), 1),
    ('CPU time, ms', cpu, 0),
    ('GPU time, ms', gpu, 1),
    ('frames presented', presented, 0),
])
table('The New issue dialog\'s animation: open and close it ten times (scenario 12)', 12, [
    ('frames per second, 200 ms after opening', lambda r: animation_fps(r, 'click nav-new'), 0),
    ('frames per second, 200 ms after Escape', lambda r: animation_fps(r, 'key escape'), 0),
    ('frames per second, whole span', fps, 0),
    ('frames presented', presented, 0),
    ('redraws asked for', lambda r: r.get('trace_redraws'), 0),
    ('CPU time, ms', cpu, 0),
    ('GPU time, ms', gpu, 1),
    ('span, s', lambda r: r['measured']['seconds'], 2),
])
table('Idle, 20 s (scenario 3)', 3, [
    ('CPU time, ms', cpu, 1),
    ('GPU time, ms', gpu, 2),
    ('frames presented', presented, 0),
    ('interrupt wake-ups', wake, 0),
])
table('Clock, 20 s (scenario 4)', 4, [
    ('CPU time, ms', cpu, 1),
    ('GPU time, ms', gpu, 2),
    ('frames presented', presented, 0),
    ('interrupt wake-ups', wake, 0),
])

print('''### Notes

- **Links.** SvelteKit follows a link after a `requestAnimationFrame` and a
  `setTimeout(0)`. Before, that was a 16 ms timer and one more turn; now it is
  the next frame the host draws and the turn right after it. Opening an issue
  is 16 ms sooner; the screen switches gain 5-20 ms.
- **Escape and Cmd+Enter** go through `goto` and a key handler, which wait for
  no frame. In the traces the app's own turn is the same on both builds (the
  Cmd+Enter turn 6-7 ms); what varies is when the key reaches the app, 21-35 ms
  after it was posted, with no frame drawn in between. The medians differ by
  that, not by this change.
- **The dialog's animation** is 100 ms in and 100 ms out, but on both builds
  frames are drawn at the display's rate for about 280 ms after each open and
  each Escape. That was so before this change too; what it is (another
  animation or transition running on) is a job of its own. Frames, CPU and
  GPU time during it are the same before and after: requestAnimationFrame
  callbacks ride on frames the animation draws anyway.
- **Idle and the clock** draw the same frames and wake as often: with no
  callback queued, no frame is asked for.
- **120 Hz is untested.** On a ProMotion display, callbacks would run up to
  120 times a second while something waits for frames, where the 16 ms timer
  ran them at most 60 times. CSS animations already drew at the display's rate
  before this change.
''')
