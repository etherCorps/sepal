"""Resource report (RAM, GPU memory, CPU, GPU time, size) of one or more harness sessions.

    python3 compare/results/make_mem_report.py mem-1000 mem-10000 > compare/results/mem.md

Every number comes from compare/results/raw/<session>/runs.jsonl and session.json. Warm-up
runs and aborted runs are excluded; aborted runs are listed.
"""
import json
import statistics as st
import sys
from pathlib import Path

RAW = Path(__file__).parent / 'raw'
B = ['blitz', 'blitzv8', 'tauri', 'gpuix']
NAMES = {'blitz': 'Sepal (QuickJS)', 'blitzv8': 'Sepal (V8)', 'tauri': 'Tauri 2', 'gpuix': 'gpuix-svelte †'}
HELPERS = {'SetStoreUpdateService', 'com.apple.SafariPlatformSupport.Helper', 'SafariPlatformSupport.Helper'}
MB = 2**20

sessions = sys.argv[1:]
R, S = [], {}
for s in sessions:
    S[s] = json.loads((RAW / s / 'session.json').read_text())
    R += [dict(json.loads(line), session=s) for line in (RAW / s / 'runs.jsonl').open()]
ok = [r for r in R if r['status'] == 'ok' and not r.get('warmup')]
aborted = [r for r in R if r['status'] != 'ok']


def runs(b, size, sc):
    return [r for r in ok if r['build'] == b and r['size'] == size and r['scenario'] == sc]


def app_only(fp):
    return sum(p['footprint'] for p in fp['processes'] if p['name'] not in HELPERS)


def peak(fp):
    return sum((p.get('peak') or p['footprint']) for p in fp['processes'] if p['name'] not in HELPERS)


def cell(vals, d=0):
    vals = [v for v in vals if v is not None]
    if not vals:
        return '—'
    m = st.median(vals)
    return f'{m:,.{d}f} ({min(vals):,.{d}f}–{max(vals):,.{d}f})' if len(vals) > 1 else f'{m:,.{d}f}'


out = []
p = out.append
first = S[sessions[0]]
m = first['machine']
disp = m['drive']['displays'][0]
n_runs = max(len(runs(b, r['size'], r['scenario'])) for r in ok for b in B)
p('# Resource comparison: RAM, GPU, CPU, size')
p('')
p(f"{m['cpu']} ({m['hw_model']}), {m['drive']['os']}, {disp['pixels'][0]}x{disp['pixels'][1]} display at {disp['refresh_hz']} Hz, "
  f"{m['power'].replace('Now drawing from ', '').strip(chr(39))}. The Tracker app, window 1200x800, issues from the shared fixtures. "
  f"Sessions: {', '.join(sessions)} ({sessions and S[sessions[0]]['started'][:16]} UTC). "
  f"{len(ok)} measured runs, {len(aborted)} aborted and retried. Cells: **median (min–max) of {n_runs} runs** after 1 discarded warm-up.")
p('')
p('Sepal is at commit 9375147 (the memory work: small atlas pages, shrinking layer textures, uploads in render passes). '
  '**† gpuix-svelte is a port** (its own widgets, a CSS subset, a pre-release Svelte 5.57.2); Tauri runs the same components and CSS as Sepal.')
p('')
p('**What is measured**')
p('- **RAM**: physical footprint (`footprint`, what Activity Monitor calls Memory) summed over every process the app owns: '
  "for Tauri its WebKit WebContent, GPU and Networking processes. Two macOS helpers every app gets are left out. "
  '"Peak" is each process\'s lifetime peak, summed.')
p('- **GPU memory**: the part of RAM that is GPU memory: footprint\'s "graphics" categories (driver-owned GPU memory) and IOSurface (window surfaces), same processes. '
  'macOS reclaims most of it 1-2 s after the last frame, so it depends on whether frames are being drawn.')
p('- **CPU**: CPU time of the same processes during the scenario, from the kernel (`proc_pid_rusage`).')
p('- **GPU time**: time the GPU spent on the same processes\' Metal command queues during the scenario: the driver\'s per-client '
  '`accumulatedGPUTime` (`ioreg`, AGXDeviceUserClient), which is what Activity Monitor\'s GPU column reads.')
p('- Scenarios start 5 s after the app reports its list mounted. Idle and clock windows are 20 s here (CPU and GPU are also given per minute).')
p('')

# ------------------------------------------------------------------ size
p('## App size')
p('')
p('| | ' + ' | '.join(NAMES[b] for b in B) + ' |')
p('|---|' + '---|' * len(B))
sizes = {b: first['builds'][b]['sizes'] for b in B if b in first['builds']}
p('| `.app` on disk | ' + ' | '.join(f"{sizes[b]['du_kb'] / 1024:.1f} MB" if b in sizes else '—' for b in B) + ' |')
p('| zipped (`ditto -c -k`) | ' + ' | '.join(f"{sizes[b]['zip_bytes'] / MB:.1f} MB" if b in sizes else '—' for b in B) + ' |')
p('')
p('Tauri is small because the web engine is the system\'s WKWebView, not in the app; Sepal V8 carries V8 (~40 MB); gpuix carries the Bun runtime.')
p('')

SC = {3: 'idle (nothing on screen changes)', 4: 'a clock ticking once a second', 8: 'create an issue (New issue dialog)',
      9: 'Show all, then wheel-scroll the whole list', 10: 'switch views: Board, Insights, Settings, Issues'}
sizes_run = sorted({r['size'] for r in ok})
for size in sizes_run:
    p(f'## {size:,} issues')
    p('')
    for sc in sorted({r['scenario'] for r in ok if r['size'] == size}):
        seconds = st.median([r['measured']['seconds'] for b in B for r in runs(b, size, sc)] or [0])
        p(f'### {SC.get(sc, sc)} ({seconds:.0f} s)')
        p('')
        p('| | ' + ' | '.join(NAMES[b] for b in B) + ' |')
        p('|---|' + '---|' * len(B))
        rows = [
            ('RAM 5 s after load, MB', lambda r: app_only(r['footprint_mounted5']) / MB),
            ('RAM at the end, MB', lambda r: app_only(r['footprint']) / MB),
            ('RAM peak, MB', lambda r: peak(r['footprint']) / MB),
            ('… of it GPU memory at the end, MB', lambda r: r['footprint'].get('gpu', 0) / MB),
            ('CPU time, ms', lambda r: r['usage']['cpu_ns'] / 1e6),
            ('CPU, % of one core (average)', lambda r: r['usage']['cpu_ns'] / 1e7 / r['measured']['seconds']),
            ('GPU time, ms', lambda r: r.get('gpu_ns', 0) / 1e6),
        ]
        if sc in (3, 4):
            rows.append(('wake-ups per minute', lambda r: (r['usage']['pkg_idle_wkups'] + r['usage']['interrupt_wkups']) * 60 / r['measured']['seconds']))
        for label, fn in rows:
            d = 1 if 'CPU, %' in label else 0
            p(f'| {label} | ' + ' | '.join(cell([fn(r) for r in runs(b, size, sc)], d) for b in B) + ' |')
        p('')

# ------------------------------------------------------------------ per process
p('## Where the RAM is (clock ticking, 1,000 issues, one run, MB at the end; GPU memory in brackets)')
p('')
for b in B:
    rs = runs(b, 1000, 4)
    if not rs:
        continue
    fp = rs[0]['footprint']
    procs = ', '.join(f"{q['name']} {q['footprint'] / MB:.0f} ({q.get('gpu', 0) / MB:.0f})" for q in sorted(fp['processes'], key=lambda q: -q['footprint']) if q['name'] not in HELPERS)
    p(f'- **{NAMES[b]}**: {procs}')
p('')

p('## Aborted runs (all retried)')
p('')
if aborted:
    for r in aborted:
        p(f"- {r['key']} attempt {r.get('attempt')}: {r.get('reason', '')[:140]}")
else:
    p('None.')
p('')
print('\n'.join(out))
