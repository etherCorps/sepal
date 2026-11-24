"""Damage tracking on vs off for Sepal: GPU and CPU time, GPU memory, frames.

    python3 compare/results/make_damage_report.py <session> [<session with SEPAL_DAMAGE=0>]

Every number comes from compare/results/raw/<session>/runs.jsonl. Warm-up and aborted runs are
excluded; aborted runs are listed. Frames come from the host's SEPAL_TRACE lines during the
scenario: `damage: Full|Rect|None` per frame drawn with damage tracking on, `RedrawRequested` per
frame with it off (every one of those is drawn in full and presented).
"""
import json
import statistics as st
import sys
from pathlib import Path

RAW = Path(__file__).parent / 'raw'
MB = 2**20
SC = {3: 'idle (nothing changes)', 4: 'clock ticking once a second', 5: 'type in search',
      8: 'create an issue (New issue dialog)', 9: 'Show all, then wheel-scroll the list',
      10: 'switch views: Board, Insights, Settings, Issues'}


def load(session):
    path = RAW / session
    runs = [json.loads(line) for line in (path / 'runs.jsonl').open()]
    return json.loads((path / 'session.json').read_text()), runs


def cell(vals, d=0):
    vals = [v for v in vals if v is not None]
    if not vals:
        return '—'
    m = st.median(vals)
    return f'{m:,.{d}f} ({min(vals):,.{d}f}–{max(vals):,.{d}f})' if len(vals) > 1 else f'{m:,.{d}f}'


def kinds(r):
    """Frames by damage tracking's verdict; with it off, every frame drawn is full."""
    d = r.get('trace_damage')
    if d and d['full'] + d['partial'] + d['none'] > 0:
        return d
    return {'full': r.get('trace_redraws'), 'partial': 0, 'none': 0}


def presented(r):
    d = kinds(r)
    return d['full'] + d['partial']


on_name = sys.argv[1]
off_name = sys.argv[2] if len(sys.argv) > 2 else None
title = 'Sepal: damage tracking on and off' if off_name else 'Sepal, damage tracking on'
on_s, on_runs = load(on_name)
off_s, off_runs = load(off_name) if off_name else (None, [])
cols = ['damage on', 'damage off'] if off_name else ['damage on']
ok = lambda runs, sc: [r for r in runs if r['status'] == 'ok' and not r.get('warmup') and r['scenario'] == sc]
scenarios = sorted({r['scenario'] for r in on_runs + off_runs})

out = []
p = out.append
m = on_s['machine']
p(f'# {title}')
p('')
p(f"{m['cpu']}, {m['drive']['os']}, the Tracker (bench build, {on_s['plan']['sizes'][0]:,} issues), window 1200x800 at 2x. "
  f"Session `{on_name}` (damage tracking on, the default)"
  + (f" and `{off_name}` (`SEPAL_DAMAGE=0`)" if off_name else '')
  + f", {on_s['plan']['warmup']} warm-up + {on_s['plan']['runs']} measured runs per scenario. Cells: median (min–max).")
p('')
p('- GPU time: the driver\'s per-process `accumulatedGPUTime` (what Activity Monitor shows) during the scenario.')
p('- CPU time: `proc_pid_rusage` during the scenario.')
p('- GPU memory: footprint\'s graphics categories and IOSurface at the end of the scenario.')
p('- Frames: drawn frames during the scenario, from the trace. Full / partial / nothing changed is damage tracking\'s verdict; '
  '"nothing changed" frames present nothing. With damage tracking off every frame is drawn in full.')
p('')
for sc in scenarios:
    a, b = ok(on_runs, sc), ok(off_runs, sc)
    secs = st.median([r['measured']['seconds'] for r in a + b] or [0])
    p(f'## {sc}: {SC.get(sc, sc)} ({secs:.0f} s)')
    p('')
    p('| | ' + ' | '.join(cols) + ' |')
    p('|---|' + '---|' * len(cols))
    rows = [
        ('GPU time, ms', lambda r: r.get('gpu_ns', 0) / 1e6, 1),
        ('CPU time, ms', lambda r: r['usage']['cpu_ns'] / 1e6, 0),
        ('GPU memory at the end, MB', lambda r: r['footprint'].get('gpu', 0) / MB, 0),
        ('RAM at the end, MB', lambda r: sum(q['footprint'] for q in r['footprint']['processes']) / MB, 0),
        ('frames presented', presented, 0),
        ('… full', lambda r: kinds(r)['full'], 0),
        ('… partial', lambda r: kinds(r)['partial'], 0),
        ('drawn but nothing changed (not presented)', lambda r: kinds(r)['none'], 0),
    ]
    for label, fn, d in rows:
        p(f'| {label} | ' + ' | '.join(cell([fn(r) for r in rs], d) for rs in [a, b][:len(cols)]) + ' |')
    p('')

aborted = [r for r in on_runs + off_runs if r['status'] != 'ok']
p('## Aborted runs')
p('')
for r in aborted:
    p(f"- {r['key']} attempt {r.get('attempt')}: {r.get('reason', '')[:160]}")
if not aborted:
    p('None.')
p('')
print('\n'.join(out))
