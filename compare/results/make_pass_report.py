"""Markdown report of one harness pass (quick or full): median / p95 / worst over measured runs.

    python3 compare/results/make_pass_report.py quick-1 > compare/results/quick-1.md

Every number comes from compare/results/raw/<session>/runs.jsonl and session.json. Warm-up runs
and aborted runs are excluded from the statistics; aborted runs are listed.
"""
import json
import math
import statistics as st
import sys
from pathlib import Path

session = sys.argv[1]
raw = Path(__file__).parent / 'raw' / session
S = json.loads((raw / 'session.json').read_text())
R = [json.loads(line) for line in (raw / 'runs.jsonl').open()]
B = ['blitz', 'blitzv8', 'tauri', 'gpuix']
NAMES = {'blitz': 'svelte-blitz QuickJS', 'blitzv8': 'svelte-blitz V8', 'tauri': 'Tauri 2', 'gpuix': 'gpuix † (Svelte 5.57.2)'}
HELPERS = {'SetStoreUpdateService', 'SafariPlatformSupport.Helper'}  # attributed to every app by macOS
SIZE = 1000

runs = {}  # (build, scenario) -> measured ok runs
for r in R:
    # The recorded run only feeds the screen check: capture starts before the harness polls for
    # the window (start-up's `window` and first input +~280 ms) and adds ~10-20 MB and a few % CPU.
    if r['status'] == 'ok' and not r.get('warmup') and not r.get('recorded') and r['size'] == SIZE:
        runs.setdefault((r['build'], r['scenario']), []).append(r)
aborted = [r for r in R if r['status'] != 'ok']
N = max(len(v) for v in runs.values())


def pct(xs, q):
    """Nearest rank: with 3 runs p95 is the worst run."""
    xs = sorted(xs)
    return xs[max(0, math.ceil(q * len(xs)) - 1)]


def stats(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return None
    return st.median(xs), pct(xs, 0.95), max(xs)


def cell(xs, d=1):
    s = stats(xs)
    if s is None:
        return '—'
    m, p, w = s
    if N <= 3:
        return f'{m:,.{d}f} / {w:,.{d}f}'
    return f'{m:,.{d}f} / {p:,.{d}f} / {w:,.{d}f}'


def table(title, rows, note=''):
    out.append(f'| {title} | ' + ' | '.join(NAMES[b] for b in B) + ' |')
    out.append('|---|' + '---|' * len(B))
    for label, fn, d in rows:
        out.append(f'| {label} | ' + ' | '.join(cell(fn(b), d) for b in B) + ' |')
    out.append('')
    out.append(STAT + (' ' + note if note else ''))
    out.append('')


def per_run(b, sc, fn):
    vals = []
    for r in runs.get((b, sc), []):
        try:
            vals.append(fn(r))
        except (KeyError, TypeError, IndexError, StopIteration):
            vals.append(None)
    return vals


def step(label, key='ms_to_mark'):
    return lambda r: next(s[key] for s in r['steps'] if s['label'] == label)


def mb(x):
    return x / 2**20


def app_only(fp):
    return sum(p['footprint'] for p in fp['processes'] if p['name'] not in HELPERS)


def cpu_ms(r):
    return r['usage']['cpu_ns'] / 1e6


def wakeups(r):
    return r['usage']['pkg_idle_wkups'] + r['usage']['interrupt_wkups']


def top_power(r):
    """Mean over top's sampling blocks of the summed power column of the app's processes."""
    blocks = {}
    for s in r['top']:
        blocks[s['block']] = blocks.get(s['block'], 0) + (s['power'] or 0)
    return st.mean(blocks.values()) if blocks else None


def recorded(b, sc):
    return next((r for r in R if r['build'] == b and r['scenario'] == sc and r.get('recorded') and r['status'] == 'ok'), None)


out = []
p = out.append
m = S['machine']
disp = m['drive']['displays'][0]
STAT = (f'*Cells: median / worst of {N} measured runs (1 warm-up and the screen-recorded run excluded)' + ('; with this few runs p95 (nearest rank) is the worst run, so it is not shown separately.*' if N <= 3 else '; median / p95 / worst.*') +
        ' *† gpuix is a port running a pre-release Svelte 5.57.2 (others 5.59.1), its own widgets and a CSS subset.*')

p(f'# Runtime comparison: {session}')
p('')
p(f"{len([r for r in R if r['status'] == 'ok'])} runs ok, {len(aborted)} aborted (all retried; see Flaky). {SIZE:,} issues. "
  f"{m['hw_model']} ({m['cpu']}), {m['drive']['os']}, display {disp['pixels'][0]}x{disp['pixels'][1]} at {disp['refresh_hz']} Hz, "
  f"{m['power'].replace('Now drawing from ', '').strip(chr(39))}, Stage Manager {'on' if m['stage_manager'] == '1' else 'off'}. Window content 1200x800.")
p('')
p('**Clocks.** Input events and every app line are stamped by the harness with `mach_absolute_time` (0.04 µs), '
  'app lines on arrival over stderr. App-side clocks are 1 ms. Treat latency differences under 2 ms as noise. '
  '"→ frame" uses each runtime\'s own signal (svelte-blitz: host repaint trace; Tauri: double rAF, mark over async IPC; '
  'gpuix: `next_frame()`), three different things. "Screen" is the independent check: first changed window frame in a 75 Hz '
  'ScreenCaptureKit recording, one recorded run per scenario, so ±13.3 ms (one frame) and a single value, not a median.')
p('')
p('**Energy.** powermetrics was not running during this pass (its log ends 16:55, the pass ran 17:16-18:30), so energy is '
  '**top\'s `power` column** (Apple\'s unitless "energy impact" score, summed over the app\'s processes, mean over 1 s samples). '
  'It is a relative score, not joules.')
p('')

# ------------------------------------------------------------------ start-up
p('## 1-2. Start-up (ms from exec)')
p('')
for sc, name in ((1, 'Scenario 1: first launch after quit (no reboot or cache purge, so not a cold start)'), (2, 'Scenario 2: warm start (relaunch right after quitting)')):
    p(f'### {name}')
    p('')
    rows = [(f'`{k}` mark', (lambda k: lambda b: per_run(b, sc, lambda r: r['startup'][k]['ms_from_exec']))(k), 1) for k in ('start', 'read', 'parsed', 'loaded', 'mounted')]
    rows.insert(0, ('window on screen', lambda b: per_run(b, sc, lambda r: r['startup']['window']), 1))
    rows.append(('first frame after `mounted`', lambda b: per_run(b, sc, lambda r: r['startup']['first_frame_after_mounted']['ms_from_exec']), 1))
    rows.append(('first `j` accepted (ms from exec)', lambda b: per_run(b, sc, lambda r: r['startup']['first_input']['ms_from_exec']), 1))
    rows.append(('… latency of that press (ms)', lambda b: per_run(b, sc, lambda r: r['startup']['first_input']['ms_from_last_press_before']), 1))
    rows.append(('… presses refused/ignored before it', lambda b: per_run(b, sc, lambda r: r['startup']['first_input']['presses_before_mark'] - 1), 0))
    rows.append(('memory 5 s after `mounted` (MB, app only)', lambda b: per_run(b, sc, lambda r: mb(app_only(r['footprint_mounted5']))), 0))
    table('ms', rows)

# ------------------------------------------------------------------ interactions
p('## 5-8, 10, 11. Interactions (ms from the input event that makes the control act)')
p('')
p('Clicks act on the mouse **release**, keys on the key **press**, in all four builds (calibrated per build and control).')
p('')
SC_STEPS = {
    5: ('Scenario 5: open an issue, then go back', ['click row', 'Escape']),
    7: ('Scenario 7: change an issue\'s status', ['click row', 'click done']),
    8: ('Scenario 8: create an issue (Cmd+Enter submits)', ['Cmd+Enter']),
    10: ('Scenario 10: switch views, 2 s apart', ['click nav-board', 'click nav-insights', 'click nav-settings', 'click nav-issues']),
    11: ('Scenario 11: resize through the Accessibility API (from the AX call)', ['resize 800x600', 'resize 1200x800']),
}


def changed_frames(r):
    """(display ns, mean abs difference to the previous frame) of captured frames that changed."""
    f = raw.parent.parent / r['frames']['file']
    out = []
    for line in f.read_text().splitlines():
        if line and not line.startswith('#'):
            d, a, status, mad, h = line.split(' ')
            if status == 'complete' and float(mad) != 0:
                out.append((int(d) or int(a), float(mad)))
    return out


def screen_ms(b, sc, label):
    """First changed captured frame after the event that makes the control act (release for clicks)."""
    r = recorded(b, sc)
    if not r or 'frames' not in r:
        return '—'
    i = next((i for i, s in enumerate(r['steps']) if s['label'] == label), None)
    if i is None:
        return '—'
    trig = lambda s: s.get('up_t') if s.get('trigger') == 'up' else (s.get('down_t') or s['input_t'])
    t = trig(r['steps'][i])
    # First change, and the biggest within 500 ms: a press or hover restyle changes a few pixels,
    # the result (a new page, a pane) most of them.
    fr = [x for x in changed_frames(r) if t < x[0] < t + 500e6]
    if not fr:
        return 'no change'
    big = max(fr, key=lambda x: x[1])
    first, most = (fr[0][0] - t) / 1e6, (big[0] - t) / 1e6
    return f'{first:,.0f}' if big is fr[0] else f'{first:,.0f} / {most:,.0f}'


for sc, (name, labels) in SC_STEPS.items():
    p(f'### {name}')
    p('')
    rows = []
    for lab in labels:
        rows.append((f'{lab} → mark', (lambda lab: lambda b: per_run(b, sc, step(lab)))(lab), 1))
        rows.append((f'{lab} → frame', (lambda lab: lambda b: per_run(b, sc, step(lab, 'ms_to_frame')))(lab), 1))
    table('ms', rows)
    p('| screen: first change / biggest change in 500 ms (1 recorded run, ±13.3 ms) | ' + ' | '.join(' · '.join(screen_ms(b, sc, lab) for lab in labels) for b in B) + ' |')
    p('|---|' + '---|' * len(B))
    p('')

p('### Scenario 6: type `deploy cac` into search, one key every 100 ms (all 10 keys of every run pooled)')
p('')
keys = lambda b, key: [s[key] for r in runs.get((b, 6), []) for s in r['steps'] if s['label'].startswith('key ')]
table('ms', [('key → `search:` mark', lambda b: keys(b, 'ms_to_mark'), 1), ('key → frame', lambda b: keys(b, 'ms_to_frame'), 1)],
      f'*Pooled: {N} runs × 10 keys = {N * 10} values per cell.*')

# ------------------------------------------------------------------ idle, scroll
p('## 3, 4, 9. Idle and scrolling')
p('')
for sc, name in ((3, 'Scenario 3: idle 60 s, nothing animating'), (4, 'Scenario 4: idle 60 s, a clock ticking once a second')):
    p(f'### {name}')
    p('')
    table('', [
        ('CPU time (ms per 60 s, all processes)', lambda b: per_run(b, sc, cpu_ms), 1),
        ('wake-ups (idle + interrupt, 60 s)', lambda b: per_run(b, sc, wakeups), 0),
        ('top power score (mean, unitless)', lambda b: per_run(b, sc, top_power), 2),
        ('memory at end (MB, app only)', lambda b: per_run(b, sc, lambda r: mb(app_only(r['footprint']))), 0),
    ])
    p('| frames that changed on screen (recorded run, of 75 Hz) | ' + ' | '.join(
        (f"{recorded(b, sc)['frames']['changed']} of {recorded(b, sc)['frames']['complete']}" if recorded(b, sc) and recorded(b, sc).get('frames') else '—') for b in B) + ' |')
    p('|---|' + '---|' * len(B))
    p('')

p('### Scenario 9: Show all, then wheel-scroll the whole list (391 wheel events at 60 Hz, 100 px each)')
p('')
table('', [
    ('Show all → `shown:all` (ms)', lambda b: per_run(b, 9, step('click show-all')), 1),
    ('Show all → frame (ms)', lambda b: per_run(b, 9, step('click show-all', 'ms_to_frame')), 1),
    ('CPU time, whole scenario (ms)', lambda b: per_run(b, 9, cpu_ms), 0),
    ('top power score (mean)', lambda b: per_run(b, 9, top_power), 1),
    ('memory at end (MB, app only)', lambda b: per_run(b, 9, lambda r: mb(app_only(r['footprint']))), 0),
])


def scroll_frames(b):
    r = recorded(b, 9)
    if not r or 'frames' not in r:
        return '—'
    w = r['extra']['wheel']
    fr = [x[0] for x in changed_frames(r) if w['from'] <= x[0] <= w['to'] + 100e6]
    gaps = [(b - a) / 1e6 for a, b in zip(fr, fr[1:])]
    first = (fr[0] - w['from']) / 1e6 if fr else None
    return (f"{len(fr)} frames changed in {(w['to'] - w['from']) / 1e9:.1f} s ({len(fr) / ((w['to'] - w['from']) / 1e9 + 0.1):.0f}/s); "
            f"longest gap {max(gaps):.0f} ms; first wheel → screen {first:.0f} ms") if fr else 'no change'


p('| recorded run | ' + ' | '.join(scroll_frames(b) for b in B) + ' |')
p('|---|' + '---|' * len(B))
p('')
p('Counted from the first wheel event to 100 ms after the last. 60 wheel events/s on a 75 Hz display: a renderer that draws every event changes ~60 frames/s; the longest gap is the worst stall on screen.')
p('')

# ------------------------------------------------------------------ memory, CPU
p('## Memory (MB, app only, median)')
p('')
p('Physical footprint over every process the app owns, minus two macOS helpers every app gets. GPU memory is reclaimed ~1-2 s after the last frame, '
  'so "at end" depends on how long ago the last frame was (scenario 3: a minute; 4: a second).')
p('')
p('| scenario | ' + ' | '.join(NAMES[b] for b in B) + ' |')
p('|---|' + '---|' * len(B))
for sc in range(1, 12):
    name = next((r['scenario_name'] for r in R if r['scenario'] == sc), str(sc))
    vals = []
    for b in B:
        a = stats(per_run(b, sc, lambda r: mb(app_only(r['footprint_mounted5'])) if 'footprint_mounted5' in r else None))
        e = stats(per_run(b, sc, lambda r: mb(app_only(r['footprint']))))
        vals.append(f"{a[0]:,.0f} → {e[0]:,.0f}" if a and e else '—')
    p(f'| {sc}. {name} (5 s after mounted → end) | ' + ' | '.join(vals) + ' |')
p('')

p('## CPU time per scenario (ms, all processes, median)')
p('')
p('| scenario | ' + ' | '.join(NAMES[b] for b in B) + ' |')
p('|---|' + '---|' * len(B))
for sc in range(1, 12):
    name = next((r['scenario_name'] for r in R if r['scenario'] == sc), str(sc))
    p(f'| {sc}. {name} | ' + ' | '.join(cell(per_run(b, sc, cpu_ms), 0) for b in B) + ' |')
p('')
p(STAT)
p('')

# ------------------------------------------------------------------ flaky
p('## Flaky and aborted')
p('')
p('| run | attempt | reason |')
p('|---|---|---|')
for r in aborted:
    p(f"| {r['key']}{' (recorded)' if r.get('recorded') else ''} | {r.get('attempt')} | {r.get('reason', '')[:160]} |")
p('')
spread = []
for (b, sc), rs in sorted(runs.items()):
    for i, s0 in enumerate(rs[0]['steps'] if rs else []):
        lab = s0['label']
        xs = [x for x in per_run(b, sc, lambda r: r['steps'][i]['ms_to_mark']) if x is not None]
        if len(xs) >= 2 and st.median(xs) > 5 and max(xs) / st.median(xs) > 1.5:
            spread.append(f"{NAMES[b]} s{sc} `{lab}`: median {st.median(xs):.1f}, worst {max(xs):.1f} ms")
p('**Latencies whose worst run is over 1.5× the median** (medians over 5 ms):')
p('')
for s in spread or ['none']:
    p(f'- {s}')
p('')
NOTES = Path(__file__).parent / 'raw' / session / 'NOTES.md'
if NOTES.exists():
    p(NOTES.read_text())
print('\n'.join(out))
