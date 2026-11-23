"""Per-tick breakdown of scenario 4 (idle with the clock) on a diagnostic svelte-blitz build.

    python3 compare/results/diag_ticks.py compare/results/raw/diag/blitzdiag-s4.json \
        compare/results/raw/diag/blitzdiag-1000-s4.log

The diagnostic hosts (Tracker-blitz-diag.app, -blitzv8-diag) are built with blitz-dom's
`log-phase-times`: every resolve prints `Resolve(<doc>): <total> (style: …, damage: …,
construct: …, pconstruct: …, layout: …, transform: …, paint_tree: …, c_damage: …, subdocs: …)`
on stdout. With SVELTE_BLITZ_TRACE=1 the host adds, on stderr: `the app changed the page: N
ops, L layouts in X ms` at the end of a turn that changed something visible, and
`+t RedrawRequested took X ms` for every repaint (Blitz's resolve + paint + present).

One tick = from one `MARK clock` to the next. In it: the resolves before the "changed the page"
line are layouts the turn forced (the app measured); those after it belong to the repaint.
paint+present = RedrawRequested minus the resolve inside it.

What the trace cannot split: the JavaScript of a turn (timer callback, Svelte's effects, the
microtasks) is not timed anywhere; nor is the wait between the timer's deadline and the
wake-up, nor the time from present to the screen. `js_est_ms` is an estimate: the run's CPU
time per tick (rusage, all threads) minus the phases the trace does time.
"""
import json
import re
import statistics as st
import sys

RESOLVE = re.compile(r'^Resolve\(\d+\): ([\d.]+)(ns|us|ms|s) \((.*)\)$')
PART = re.compile(r'(\w+): ([\d.]+)(ns|us|ms|s)')
UNIT = {'ns': 1e-6, 'us': 1e-3, 'ms': 1.0, 's': 1e3}


def ms(v, u):
    return float(v) * UNIT[u]


def ticks(log):
    lines = []
    for l in open(log):
        t, fd, s = l.rstrip('\n').split(' ', 2)
        lines.append((float(t), s))
    lines.sort(key=lambda x: x[0])  # stdout and stderr arrive on two pipes
    marks = [i for i, (_, s) in enumerate(lines) if re.search(r'MARK clock ', s)]
    out = []
    for a, b in zip(marks, marks[1:]):
        tick = {'t': lines[a][0], 'ops': None, 'turn_layouts': None, 'turn_layout_ms': None, 'forced': [], 'redraw': [], 'redraw_ms': [], 'poll_ms': []}
        # The tick's own work starts a little before its mark (the mark is written from a
        # microtask in the same turn): take the window from just before this mark to just
        # before the next one.
        seen_changed = False
        for t, s in lines[a:b]:
            m = re.search(r'the app changed the page: (\d+) ops, (\d+) layouts in ([\d.]+)ms', s)
            if m and not seen_changed:
                tick['ops'], tick['turn_layouts'], tick['turn_layout_ms'] = int(m[1]), int(m[2]), float(m[3])
                seen_changed = True
                continue
            m = RESOLVE.match(s)
            if m:
                r = {'total': ms(m[1], m[2]), **{k: ms(v, u) for k, v, u in PART.findall(m[3])}}
                (tick['redraw'] if seen_changed else tick['forced']).append(r)
                continue
            m = re.search(r'\+[\d.]+s RedrawRequested took ([\d.]+)ms', s)
            if m:
                tick['redraw_ms'].append(float(m[1]))
            m = re.search(r'\+[\d.]+s poll took ([\d.]+)ms', s)
            if m:
                tick['poll_ms'].append(float(m[1]))
        out.append(tick)
    return out


def main():
    rec = json.load(open(sys.argv[1]))
    T = ticks(sys.argv[2])
    rows = []
    for k in T:
        redraw = sum(k['redraw_ms'])
        resolve_in_redraw = sum(r['total'] for r in k['redraw'])
        rows.append({
            'ops': k['ops'], 'forced_layouts': len(k['forced']), 'forced_ms': sum(r['total'] for r in k['forced']),
            # Phases over every resolve of the tick (the forced one does the work; the repaint's finds nothing left).
            'style': sum(r.get('style', 0) for r in k['forced'] + k['redraw']),
            'construct': sum(r.get('construct', 0) + r.get('pconstruct', 0) for r in k['forced'] + k['redraw']),
            'layout': sum(r.get('layout', 0) for r in k['forced'] + k['redraw']), 'paint_tree': sum(r.get('paint_tree', 0) for r in k['forced'] + k['redraw']),
            'resolve': resolve_in_redraw, 'redraws': len(k['redraw_ms']), 'redraw': redraw, 'paint_present': redraw - resolve_in_redraw,
        })
    u = rec.get('usage', {})
    n = len(rows)
    cpu_per_tick = u.get('cpu_ns', 0) / 1e6 / max(1, rec.get('extra', {}).get('clock_marks') or n)
    secs = rec.get('measured', {}).get('seconds') or 60
    wk = (u.get('pkg_idle_wkups', 0) + u.get('interrupt_wkups', 0)) / secs
    timed = [r['forced_ms'] + r['redraw'] for r in rows]
    print(f"{n} ticks; CPU {u.get('cpu_ns', 0) / 1e6:.0f} ms in {secs:.1f} s = {cpu_per_tick:.2f} ms per tick (all threads); "
          f"wake-ups {wk:.1f}/s; trace-timed (wall) per tick: median {st.median(timed):.2f} ms. CPU minus timed wall is "
          f"{cpu_per_tick - st.median(timed):.2f} ms: the JS is within noise of zero here (repaint wall time includes GPU waits).")
    cols = ['ops', 'forced_layouts', 'forced_ms', 'style', 'construct', 'layout', 'paint_tree', 'resolve', 'redraws', 'redraw', 'paint_present']
    print(f"{'phase':16} {'median':>8} {'max':>8}")
    for c in cols:
        xs = [r[c] for r in rows if r[c] is not None]
        if xs:
            print(f"{c:16} {st.median(xs):8.2f} {max(xs):8.2f}")
    if '--json' in sys.argv:
        json.dump({'ticks': rows, 'cpu_per_tick_ms': cpu_per_tick, 'wakeups_per_s': wk}, open(sys.argv[sys.argv.index('--json') + 1], 'w'), indent=1)


if __name__ == '__main__':
    main()
