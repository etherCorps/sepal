"""Write a shareable Markdown report of one harness session (dry run).

    python3 compare/results/make_dryrun_report.py dryrun-1 > compare/results/dryrun-1.md

Every number comes from compare/results/raw/<session>/runs.jsonl and session.json.
"""
import json
import statistics as st
import sys
from pathlib import Path

session = sys.argv[1]
raw = Path(__file__).parent / 'raw' / session
S = json.loads((raw / 'session.json').read_text())
records = [json.loads(line) for line in (raw / 'runs.jsonl').open()]
ok = {}
attempts = {}
for r in records:
    k = (r['build'], int(r['scenario']))
    attempts.setdefault(k, []).append(r)
    if r['status'] == 'ok':
        ok[k] = r

B = ['blitz', 'blitzv8', 'tauri', 'gpuix']

# Which event makes each control act, from the calibration launches made after the dry run
# (run.mjs `calibrate`: one press held 300 ms). The dry run measured every click from its
# press (mouse down) and every key from its key-down.
CALIB = Path(__file__).parent / 'raw' / 'calibration'
TRIG = {b: (json.loads((CALIB / f'calibration-{b}-1000.json').read_text()).get('triggers') or {}) for b in B
        if (CALIB / f'calibration-{b}-1000.json').exists()}
# The dry run did not stamp the release; its hold was a 30 ms sleep. What that sleep measured
# once stamped (scenario 9's Show all click, default hold, raw/memory/breakdown-*.json):
HOLDS = []
for f in (Path(__file__).parent / 'raw' / 'memory').glob('breakdown-*-s9.json'):
    HOLDS += [st_['hold_ms'] for st_ in json.loads(f.read_text()).get('steps', []) if st_.get('hold_ms') and st_['hold_ms'] < 100]
HOLD = st.median(HOLDS) if HOLDS else 30.0


def control(label):
    if label == 'click row':
        return 'row'
    if label == 'click done':
        return 'status-option-done'
    if label.startswith('click '):
        return label[6:]
    return {'Escape': 'key:escape', 'Cmd+Enter': 'key:cmd+return'}.get(label, 'key:type' if label.startswith('key ') else None)


def trigger(b, label):
    t = TRIG.get(b, {}).get(control(label) or '')
    return t['trigger'] if t else 'down'
NAMES = {'blitz': 'svelte-blitz QuickJS', 'blitzv8': 'svelte-blitz V8', 'tauri': 'Tauri 2', 'gpuix': 'gpuix-svelte †'}
HEAD = '| ' + ' | '.join(NAMES[b] for b in B) + ' |'


def f(x, d=1):
    return '—' if x is None else f'{x:,.{d}f}'


def row(label, values, unit=''):
    return f'| {label} | ' + ' | '.join(values) + ' |'


def table(title_cols):
    return f'| {title_cols} ' + HEAD + '\n|' + '---|' * (len(B) + 1)


def missing(b, s):
    a = attempts.get((b, s), [])
    if not a:
        return 'not run'
    return f'aborted ×{len(a)}'


out = []
p = out.append
m = S['machine']
v = m['versions']
disp = m['drive']['displays'][0]

p(f'# Runtime comparison: dry run ({session})')
p('')
p('**Status: dry run, not results.** One run per scenario, no warm-up, 1,000 issues only, no screen recording. '
  'Its purpose was to prove the harness end to end. Single runs carry no statistics: do not draw conclusions '
  'from differences of a few ms, and do not treat any of this as the comparison.')
p('')
p('## Setup')
p('')
p('| | |')
p('|---|---|')
p(f"| Machine | {m['hw_model']}, {m['cpu']}, {m['ncpu']} cores, {int(m['memsize']) // 2**30} GB |")
p(f"| macOS | {m['drive']['os']} |")
p(f"| Display | {disp['pixels'][0]}x{disp['pixels'][1]} px at {disp['points'][0]}x{disp['points'][1]} pt (2x), {disp['refresh_hz']} Hz |")
p(f"| Power | {m['power'].replace('Now drawing from ', '').strip(chr(39))}; Low Power Mode off; Stage Manager **on** (see failures) |")
p(f"| Started | {S['started']} (UTC) |")
p('| App | the Tracker (Linear-style issue tracker), 1,000 issues from a shared fixture file, window content 1200x800 |')
p('')
p('### Builds')
p('')
p('| Build | Runtime | JS engine | Svelte | Versions | .app on disk | zipped |')
p('|---|---|---|---|---|---|---|')
sizes = {b: S['builds'][b]['sizes'] for b in B}
p(f"| svelte-blitz QuickJS | Blitz (Rust HTML/CSS engine), native window | QuickJS {v['svelte-blitz']['rquickjs']} (rquickjs, interpreter) | {v['app']['svelte']} | blitz-dom {v['svelte-blitz']['blitz-dom']} | {sizes['blitz']['du_kb']/1024:.1f} MB | {sizes['blitz']['zip_bytes']/2**20:.1f} MB |")
p(f"| svelte-blitz V8 | same host, built with `engine-v8` | V8 (crate {v['svelte-blitz']['v8']}, JIT) | {v['app']['svelte']} | same | {sizes['blitzv8']['du_kb']/1024:.1f} MB | {sizes['blitzv8']['zip_bytes']/2**20:.1f} MB |")
p(f"| Tauri 2 | system WKWebView | JavaScriptCore (JIT), in WebKit's WebContent process | {v['app']['svelte']} | tauri {v['tauri']['tauri']}, wry {v['tauri']['wry']}, tao {v['tauri']['tao']} | {sizes['tauri']['du_kb']/1024:.1f} MB | {sizes['tauri']['zip_bytes']/2**20:.1f} MB |")
p(f"| gpuix-svelte † | GPUI (Zed's UI framework) via @gpuix/native {v['gpuix']['@gpuix/native']} | Bun {v['gpuix']['bun']} (JavaScriptCore, JIT) | **{v['gpuix']['bundled_svelte']}** | gpuix-svelte {v['gpuix']['gpuix-svelte']} | {sizes['gpuix']['du_kb']/1024:.1f} MB | {sizes['gpuix']['zip_bytes']/2**20:.1f} MB |")
p('')
p('svelte-blitz and Tauri run the **same Svelte components and the same Tailwind CSS** (byte-identical app.css); only four '
  'desktop calls and the router\'s DOM hook differ. **† gpuix-svelte is a port**: same screens, data, store logic and '
  'list-windowing algorithm, but its own widgets (no bits-ui/shadcn), a CSS subset instead of Tailwind, icons as SVG '
  'strings, and a pre-release Svelte 5.57.2. Every gpuix number carries that caveat.')
p('')
p('## How it was measured')
p('')
p('- **Input**: real OS-level events (CGEvent: mouse, keys, pixel wheel) from a Swift tool, posted only while the app is frontmost; window resize through the Accessibility API.')
p('- **Clock**: one monotonic clock (`mach_absolute_time`, ~0.04 µs) stamps the input events, every line the app writes, and (later) captured frames. '
  'App marks (`MARK <name>`) are stamped on arrival on stderr.')
p('- **Latency** = OS input event → the app\'s mark after its DOM/state update ("→ mark"), and → the runtime\'s own "frame" signal ("→ frame"), '
  '**from the event that makes the control act**. Calibration launches made after the dry run (each control pressed once and held 300 ms) found the same in all four builds: '
  'every clicked control acts on the **release** (mouse up), every key on the **press** (key down). '
  f'The dry run measured clicks from the press, which added the click\'s hold (a 30 ms sleep, {HOLD:.1f} ms when stamped later) to every click; it did not stamp the release. '
  'Click rows below are therefore **estimates from the release: as measured minus the median hold (≈, ±3 ms: stamped holds ranged 30-35 ms)**, with the as-measured value from the press in brackets. Key rows are unchanged. '
  'The three frame signals are **different clocks of different things**: svelte-blitz = host repaint trace (`RedrawRequested`); '
  'Tauri = double `requestAnimationFrame` (mark sent over async IPC, ~1.1-1.5 ms later; rAF ≤ 75 fps here); '
  'gpuix = `next_frame()` (GPUI\'s next 8 ms tick, not presentation). The independent check (screen recording at 75 Hz) was **not** run in this dry run.')
p('- **App clock resolution**: 1 ms in all four (svelte-blitz `Date.now()`, WKWebView `performance.now()`); arrival stamps are ~0.04 µs. '
  'Treat differences under ~2 ms, and under one 75 Hz frame (13.3 ms) for anything frame-related, as noise.')
p('- **CPU, wake-ups**: `proc_pid_rusage` at scenario start and end, summed over every process the app owns '
  '(found by responsible PID: Tauri\'s WebKit WebContent/GPU/Networking helpers included). No energy figure: see the note under scenario 9.')
p('- **Memory**: `footprint` over the same processes in one call (physical footprint, as Activity Monitor\'s "Memory"). '
  'Totals include two macOS helpers attributed to every app (`SetStoreUpdateService`, `SafariPlatformSupport.Helper`); the app-only figure is shown separately.')
p('- "First launch after quit" (not "cold start": no reboot or file-cache purge).')
p('')

# ------------------------------------------------------------------ startup
def first_press(b, sc):
    r = ok.get((b, sc))
    if not r:
        return missing(b, sc)
    js = [i['t'] for i in r['inputs'] if i['what'] == 'key j']
    return f((js[0] - r['measured']['from']) / 1e6) if js else '—'


def presses(b, sc):
    r = ok.get((b, sc))
    if not r:
        return missing(b, sc)
    log = Path(__file__).parent / r['log']
    marks = sum(1 for l in log.open() if 'MARK highlight:' in l) if log.exists() else None
    return f"{r['startup']['first_input']['presses']} / {marks if marks is not None else '—'}"



p('## 1-2. Start-up (ms from exec, one run each)')
p('')
for sc, title in ((1, 'Scenario 1: first launch after quit'), (2, 'Scenario 2: warm start (relaunch right after quitting)')):
    p(f'### {title}')
    p('')
    p(table('Event'))
    def su(b, key, sub='ms_from_exec'):
        r = ok.get((b, sc))
        if not r:
            return missing(b, sc)
        x = r['startup'].get(key)
        if isinstance(x, dict):
            x = x.get(sub)
        return f(x)
    p(row('window on screen', [su(b, 'window') for b in B]))
    p(row('`start` (first app code ran)', [su(b, 'start') for b in B]))
    p(row('`loaded` (fixture read + parsed)', [su(b, 'loaded') for b in B]))
    p(row('`mounted` (list rendered)', [su(b, 'mounted') for b in B]))
    p(row('first frame after `mounted`', [su(b, 'first_frame_after_mounted') for b in B]))
    p(row('first `j` posted by the harness', [first_press(b, sc) for b in B]))
    p(row('first input accepted (`highlight` mark)', [su(b, 'first_input') for b in B]))
    p(row('**first `j` → `highlight`** (the app\'s part)', [f"**{su(b, 'first_input', 'ms_from_first_press')}**" for b in B]))
    p(row('`j` presses posted before the mark / highlights in the log', [presses(b, sc) for b in B]))
    p('')
p('"First input accepted" from exec is mostly the harness: it posts the first `j` only once the window is settled '
  '(bounds unchanged over 5 polls, 100+ ms after it appears) and frontmost, then every 20 ms until a highlight mark arrives. '
  'The app\'s part is the first `j` → `highlight` row. In every build every press posted was accepted '
  '(as many highlight marks, one row further each, as presses): gpuix\'s "2-3 presses" are presses posted while its first was '
  'still being handled (it marks a key in ~35-50 ms, the harness re-presses every 20 ms). '
  'svelte-blitz V8\'s ~600 ms was the harness posting late (588 / 601 ms after exec; V8 marked the press 7 ms later), during the minutes '
  'when Stage Manager was moving windows (see "What went wrong"). Re-run on 23 Nov, scenarios 1-2 twice per build (`raw/firstinput/`): at every press the app was active and frontmost, and (asked through AX in one set) had a focused, main window; '
  'no press was dropped; first `j` → `highlight` was 18-23 ms (QuickJS), 17-23 (V8), 20-28 (Tauri), 38-49 (gpuix). The first key a fresh process gets costs more than later ones in every build (svelte-blitz handles the second in ~2 ms). '
  '`read` / `parsed` marks were added after the dry run.')
p('')

# ------------------------------------------------------------------ interactions
p('## 5-8, 10, 11. Interactions (ms, one run each)')
p('')
p('Each cell: **→ mark / → frame** (see "How it was measured" for what each means and its resolution). '
  'Clicks: ≈ from the release (estimate), in brackets as measured from the press. Keys: from the key press.')
p('')
labels = {5: 'Scenario 5: open an issue, then go back', 7: 'Scenario 7: change an issue\'s status', 8: 'Scenario 8: create an issue',
          10: 'Scenario 10: switch between the main views', 11: 'Scenario 11: resize the window'}
for sc in (5, 7, 8, 10, 11):
    p(f'### {labels[sc]}')
    p('')
    p(table('Step'))
    steps = None
    for b in B:
        r = ok.get((b, sc))
        if r:
            steps = [s['label'] for s in r['steps']]
            break
    for i, label in enumerate(steps or []):
        cells = []
        for b in B:
            r = ok.get((b, sc))
            if not r or i >= len(r['steps']):
                cells.append(missing(b, sc))
                continue
            s = r['steps'][i]
            if trigger(b, label) == 'up' and s.get('input_t') is not None:
                m_, fr = s.get('ms_to_mark'), s.get('ms_to_frame')
                cells.append(f"≈{f(m_ - HOLD)} / ≈{f(None if fr is None else fr - HOLD)} ({f(m_)} / {f(fr)})")
            else:
                cells.append(f"{f(s.get('ms_to_mark'))} / {f(s.get('ms_to_frame'))}")
        p(row(label.replace('|', '/'), cells))
    p('')
p('### Scenario 6: type `deploy cac` into search, one key every 100 ms')
p('')
p(table('Key'))
r0 = ok.get(('blitz', 6))
for i, s in enumerate(r0['steps']):
    cells = []
    for b in B:
        r = ok.get((b, 6))
        st_ = r['steps'][i] if r and i < len(r['steps']) else None
        cells.append(f"{f(st_.get('ms_to_mark'))} / {f(st_.get('ms_to_frame'))}" if st_ else missing(b, 6))
    p(row(s['label'].replace('key ', '').replace('"', '`'), cells))
med = []
for b in B:
    r = ok.get((b, 6))
    tm = [s['ms_to_mark'] for s in r['steps']]
    tf = [s['ms_to_frame'] for s in r['steps'] if s.get('ms_to_frame') is not None]
    med.append(f'**{f(st.median(tm))} / {f(st.median(tf))}**')
p(row('**median**', med))
p('')
p('With 1,000 issues each keystroke filters every issue; the list is windowed (only rows in view are mounted, ~28 rows), so this mostly measures filtering plus re-rendering one screen of rows.')
p('')

# ------------------------------------------------------------------ resource scenarios
p('## 3, 4, 9. Idle and scrolling (one run each)')
p('')
def res(b, sc):
    r = ok.get((b, sc))
    if not r:
        return None
    sec = r['measured'].get('seconds') or (r['measured']['to'] - r['measured']['from']) / 1e9
    u = r['usage']
    procs = r['footprint']['processes']
    app_only = sum(pp['footprint'] for pp in procs if pp['name'] not in ('SetStoreUpdateService', 'com.apple.SafariPlatformSupport.Helper'))
    return dict(sec=sec, cpu=u['cpu_ns'] / 1e9 / sec * 100, cpu_ms=u['cpu_ns'] / 1e6, wk=(u['pkg_idle_wkups'] + u['interrupt_wkups']) / sec,
                mem=r['footprint']['total'] / 2**20, mem_app=app_only / 2**20, n=len(procs),
                wheel=(r.get('extra') or {}).get('wheel'))
titles = {3: 'Scenario 3: idle 60 s, nothing animating', 4: 'Scenario 4: idle 60 s, a clock ticking once a second', 9: 'Scenario 9: Show all, then wheel-scroll the whole list (391 wheel events at 60 Hz, 100 px each)'}
for sc in (3, 4, 9):
    p(f'### {titles[sc]}')
    p('')
    p(table('Metric'))
    R = {b: res(b, sc) for b in B}
    def c(key, d=1, unit=''):
        return [(f(R[b][key], d) + unit) if R[b] else missing(b, sc) for b in B]
    p(row('measured for (s)', c('sec')))
    p(row('CPU, % of one core', c('cpu', 1, ' %')))
    p(row('CPU time (ms)', c('cpu_ms', 0)))
    p(row('wake-ups per s (idle + interrupt)', c('wk')))
    p(row('memory, all processes (MB)', c('mem', 0)))
    p(row('memory, app\'s own processes only (MB)', c('mem_app', 0)))
    p(row('processes counted', [str(R[b]['n']) if R[b] else missing(b, sc) for b in B]))
    if sc == 9:
        p(row('wheel interval median / p95 (ms)', [f"{f(R[b]['wheel']['interval_ms']['median'])} / {f(R[b]['wheel']['interval_ms']['p95'])}" if R[b] and R[b]['wheel'] else '—' for b in B]))
    p('')
p('Energy is not reported. The dry run recorded `ri_billed_energy` (rusage V4), which is **not** the process\'s energy: it is energy *other* processes '
  'spent on its behalf and billed to it through vouchers (XNU\'s bank ledger), so it barely moved with the app\'s own CPU. '
  'The harness now records `ri_energy_nj` (rusage V6, the process\'s own energy estimate in nJ; checked: populated, ~5.9 W for a busy P-core). '
  'It goes back in the report only once it agrees with powermetrics on at least one scenario (`compare/results/powermetrics_energy.py`).')
p('')
p('Scenario 9 latency and smoothness come only from the screen recording, which this dry run did not do; here it shows only the cost of scrolling.')
p('')

# ------------------------------------------------------------------ memory per process
p('## Memory by process (MB, at the end of scenario 5)')
p('')
names = []
for b in B:
    r = ok.get((b, 5))
    for pp in r['footprint']['processes']:
        if pp['name'] not in names:
            names.append(pp['name'])
p(table('Process'))
for n in names:
    cells = []
    for b in B:
        r = ok.get((b, 5))
        x = [pp['footprint'] for pp in r['footprint']['processes'] if pp['name'] == n]
        cells.append(f(x[0] / 2**20, 0) if x else '—')
    p(row(f'`{n}`', cells))
p(row('**total**', [f'**{f(ok[(b, 5)]["footprint"]["total"] / 2**20, 0)}**' for b in B]))
p('')
p('### Memory across scenarios (MB, all processes, end of each run)')
p('')
p(table('Scenario'))
for sc in range(1, 12):
    cells = []
    for b in B:
        r = ok.get((b, sc))
        cells.append(f(r['footprint']['total'] / 2**20, 0) if r and r.get('footprint') else missing(b, sc))
    p(row(f'{sc}. {next((x["scenario_name"] for x in records if int(x["scenario"]) == sc), "")}', cells))
p('')
p('**Why these swing (found after the dry run):** each build owns GPU memory that macOS reclaims about 1-2 s after the app last '
  'presented a frame, and that comes back with the next frame. svelte-blitz\'s "Owned physical footprint (unmapped) (graphics)" '
  'goes 312 → 92 MB with the app frontmost and doing nothing (no trace event, no CPU); Tauri\'s WebKit GPU process (~120 MB of graphics) '
  'shrinks or leaves; gpuix\'s graphics go 151 → 37 MB. Scenario 3 (no clock) ends a minute after the last frame, so it shows the '
  'low state; scenario 4\'s clock repaints every second, so it shows the high one; scenario 10 (2 s after the last click) and 9 '
  '(measured a few seconds after scrolling stopped) show the low one, the others the high one. Not a leak, not the app freeing memory: '
  'see "Memory over time" below. A memory figure means something only with "how long since the last frame" next to it.')
p('')

# ------------------------------------------------------------------ cpu per scenario
p('## CPU time per scenario (ms of CPU, all processes)')
p('')
p(table('Scenario'))
for sc in range(1, 12):
    cells = []
    for b in B:
        r = ok.get((b, sc))
        cells.append(f(r['usage']['cpu_ns'] / 1e6, 0) if r and r.get('usage') else missing(b, sc))
    p(row(f'{sc}. {next((x["scenario_name"] for x in records if int(x["scenario"]) == sc), "")}', cells))
p('')

# ------------------------------------------------------------------ added after the dry run
R = Path(__file__).parent / 'raw'
p('## Added after the dry run (23 Nov, separate launches, not part of dryrun-1)')
p('')
p('### Memory over time, left alone after `mounted` (MB, all processes; `run.mjs memwatch`)')
p('')
p('F = the app was frontmost at that sample. Focus moved to Zed, Finder and Chrome during these runs (logged), which is why some samples are not frontmost.')
p('')
p('| Build | clock | +0 s | +5 s | +10 s | +15 s | +30 s | +60 s |')
p('|---|---|---|---|---|---|---|---|')
for b in B:
    for clock in ('noclock', 'clock'):
        fp = R / 'memory' / f'memwatch-{b}-1000-{clock}.json'
        if not fp.exists():
            continue
        S_ = {round(x['s']): x for x in json.loads(fp.read_text())['samples']}
        cells = [f"{S_[k]['total'] / 2**20:.0f}{' F' if S_[k]['frontmost'] else ''}" if k in S_ else '—' for k in (0, 5, 10, 15, 30, 60)]
        p(f"| {NAMES[b]} | {'yes' if clock == 'clock' else 'no'} | " + ' | '.join(cells) + ' |')
fp = R / 'memory' / 'memwatch-blitzv8-1000-noclock-1s.json'
if fp.exists():
    d = json.loads(fp.read_text())
    g = lambda x: sum(c['dirty'] + c['swapped'] for c in x['processes'][0]['categories'] if c['name'] == 'Owned physical footprint (unmapped) (graphics)') / 2**20
    seq = ', '.join(f"+{x['s']:.0f} s {g(x):.0f}" for x in d['samples'][:4])
    p('')
    p(f'Sampled every second (svelte-blitz V8, frontmost throughout, last repaint at +0.05 s, no trace event after it): GPU memory {seq} MB, then flat.')
p('')
p('### What the memory is (MB; `run.mjs breakdown`: footprint categories and `vmmap --summary`, 5 s after `mounted` and after scenario 9)')
p('')
import importlib.util
spec = importlib.util.spec_from_file_location('mg', Path(__file__).parent / 'memory_groups.py')
mg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mg)
p('| Build | when | app\'s processes | ' + ' | '.join(mg.GROUPS) + ' |')
p('|---|---|---|' + '---|' * len(mg.GROUPS))
for b in B:
    for tag, label in (('mounted5', 'mounted + 5 s'), ('end', 'after scenario 9')):
        fp = R / 'memory' / f'breakdown-{b}-1000-s9-{tag}.footprint.json'
        if fp.exists():
            total, gr = mg.groups(fp)
            p(f'| {NAMES[b]} | {label} | {total:.0f} | ' + ' | '.join(f'{gr[k]:.0f}' for k in mg.GROUPS) + ' |')
p('')
p('Both svelte-blitz captures are in the low GPU state (a few seconds after the last frame); in the high state add ~220 MB of GPU memory (memwatch above). '
  'gpuix is in its low state at start-up (37 MB GPU) and its high one after scenario 9 (174 MB). Tauri\'s GPU process held ~120 MB of graphics at both points. '
  'svelte-blitz QuickJS at ~430 MB is therefore: ~310-350 MB GPU memory (Metal resources owned by the process, mostly "unmapped (graphics)", plus 15-30 MB of IOSurfaces, its window\'s drawables), '
  '~45-65 MB malloc (QuickJS\'s heap, which uses malloc, together with Blitz\'s DOM, Stylo and the Rust side: footprint cannot split them), 18 MB kernel-owned unmapped memory, ~10 MB binaries and stacks. '
  'V8 is the same plus ~10-15 MB of V8 heap regions (VM tag 16), with less malloc. Tauri: the WebContent process (JS and DOM: WebKit Malloc, JSC regions) grows from ~94 to ~280 MB over scenario 9; '
  'its GPU process holds ~120 MB graphics; the app process ~31 MB. gpuix: one process, Bun/JSC heap in tagged regions (20-40 MB), malloc ~30 MB, GPU 37-174 MB.')
p('')
p('### 10,000 issues (one launch each, bench mode, `run.mjs probe <build> 10000 5`; ms from exec)')
p('')
p(table('Event'))
K = {b: json.loads((R / '10k' / f'{b}.json').read_text()) for b in B if (R / '10k' / f'{b}.json').exists()}
for mk in ('start', 'read', 'parsed', 'loaded', 'mounted'):
    p(row(f'`{mk}`', [f((K[b]['marks'].get(mk) or {}).get('ms_from_exec')) if b in K else '—' for b in B]))
p(row('memory 5 s after `mounted` (MB, all processes)', [f(K[b]['footprint']['total'] / 2**20, 0) if b in K else '—' for b in B]))
p('')
p('No build takes seconds or gigabytes with 10,000 issues. Tauri\'s window was parked by Stage Manager (it launched behind Chrome) and was activated by the probe before its memory sample.')
p('')
p('### One clock tick in svelte-blitz (scenario 4 on the diagnostic builds; `diag_ticks.py`, ms per tick, median / max of ~60)')
p('')
p('| Phase | QuickJS | V8 |')
p('|---|---|---|')
TK = {}
for b in ('blitzdiag', 'blitzv8diag'):
    fp = R / 'diag' / f'{b}-ticks.json'
    if fp.exists():
        TK[b] = json.loads(fp.read_text())
def tk(b, k, d=2):
    if b not in TK:
        return '—'
    xs = [t[k] for t in TK[b]['ticks'] if t[k] is not None]
    return f'{st.median(xs):.{d}f} / {max(xs):.{d}f}'
for k, label in (('ops', 'ops applied'), ('forced_ms', 'layout the app forced (it measures): total'), ('style', '… style'), ('construct', '… box construction'),
                 ('layout', '… layout'), ('paint_tree', '… paint tree'), ('paint_present', 'repaint (`RedrawRequested`: Blitz paint + vello + present)')):
    p(f'| {label} | {tk("blitzdiag", k)} | {tk("blitzv8diag", k)} |')
p(f"| CPU per tick, all threads (rusage / ticks) | {TK['blitzdiag']['cpu_per_tick_ms']:.2f} | {TK['blitzv8diag']['cpu_per_tick_ms']:.2f} |" if len(TK) == 2 else '')
p(f"| wake-ups per second | {TK['blitzdiag']['wakeups_per_s']:.1f} | {TK['blitzv8diag']['wakeups_per_s']:.1f} |" if len(TK) == 2 else '')
p('')
p('A tick costs ~7.5 ms of CPU, almost all of it the repaint of the whole window for one changed text node; style + layout of the change is ~0.25 ms. '
  'The trace cannot split: the JavaScript of the turn (timer callback, Svelte\'s effects; with CPU ≈ the timed phases it is at most ~1 ms), '
  'the repaint into Blitz\'s scene building vs vello\'s rendering vs waiting for a drawable / presenting, the delay between the timer\'s deadline and the wake-up, '
  'and the time from present to the screen. The diagnostic hosts are never used for measured runs.')
p('')

# ------------------------------------------------------------------ failures
p('## What went wrong')
p('')
p('| Build | Scenario | Attempts | Outcome | Reason |')
p('|---|---|---|---|---|')
for (b, sc), a in sorted(attempts.items(), key=lambda kv: (B.index(kv[0][0]), kv[0][1])):
    bad = [x for x in a if x['status'] != 'ok']
    if not bad:
        continue
    outcome = 'ok on attempt ' + str(len(a)) if (b, sc) in ok else 'no valid run'
    p(f"| {NAMES[b]} | {sc} | {len(a)} | {outcome} | window moved/shrunk by Stage Manager after focus left the app ({', '.join(x['wall_end'][11:19] for x in bad)} UTC) |")
p('')
p(f'{sum(1 for k in attempts if k in ok)} of {len(attempts)} build×scenario pairs produced a valid run; {sum(len([x for x in a if x["status"] != "ok"]) for a in attempts.values())} attempts were aborted and none was measured. '
  'All aborts happened between 09:42 and 09:51 UTC while the person running it and the assistant were active in another app (Zed); '
  'no aborts in the 22 runs after that. No crashes, no app errors in any log, no missed input.')
p('')
p('## Not yet done (before the real passes)')
p('')
p('- Screen recording at 75 Hz (the independent frame/latency check, and the only smoothness measure for scrolling).')
p('- powermetrics energy impact (the user runs it; matched to runs by timestamp).')
p('- 10,000-issue fixture, warm-ups, and 3 (quick) / 10 (full) runs per scenario with median, p95 and worst.')
p('- Stage Manager off; nothing else touching the Mac during a pass.')
print('\n'.join(out))
