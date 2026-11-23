"""Energy impact per run, from a powermetrics log and a session's runs.jsonl.

    python3 compare/results/powermetrics_energy.py compare/results/raw/powermetrics.log \
        compare/results/raw/<session>/runs.jsonl [--json out.json]
    python3 compare/results/powermetrics_energy.py --selftest

Record the log with (needs sudo; plist is the format this reads best):

    sudo powermetrics --samplers tasks --show-process-energy -i 1000 -f plist \
        -o compare/results/raw/powermetrics.log

Text output (`-f text`, the default) is read too, assuming the tasks table's last column
is "Energy Impact" when the header names it. Neither format has been seen on this Mac yet
(powermetrics needs root): check the first numbers by hand against Activity Monitor.

For each run: every sample whose window overlaps the run's measured interval (mach `measured`
turned into wall time through `t_exec`/`wall_exec`; runs without those use wall_start..wall_end)
contributes energy_impact_per_s x overlap seconds, summed over the run's pids (`procs` and
`procs_after`). The sample window is taken to end at the sample's timestamp. Output per run:
`ei_s` (energy impact x seconds, comparable with Activity Monitor's "12 hr Power" only in
kind), `ei_per_s` (its mean rate), and how much of the interval samples covered.
"""
import json
import plistlib
import re
import sys
from datetime import datetime, timedelta, timezone


def samples(path):
    """[(end: datetime, seconds, {pid: energy_impact_per_s})]"""
    raw = open(path, 'rb').read()
    if raw.lstrip()[:5] in (b'<?xml', b'<!DOC', b'bplis'):
        return list(plist_samples(raw))
    return list(text_samples(raw.decode('utf-8', 'replace')))


def plist_samples(raw):
    for chunk in raw.split(b'\0'):
        if not chunk.strip():
            continue
        d = plistlib.loads(chunk.strip())
        secs = d.get('elapsed_ns', 0) / 1e9
        end = d['timestamp']
        if end.tzinfo is None:
            end = end.replace(tzinfo=timezone.utc)  # plistlib dates are UTC
        rates = {}
        for t in d.get('tasks', []):
            if 'energy_impact_per_s' in t:
                rate = t['energy_impact_per_s']
            elif 'energy_impact' in t and secs:
                rate = t['energy_impact'] / secs
            else:
                continue
            rates[int(t['pid'])] = rates.get(int(t['pid']), 0) + rate
        yield end, secs, rates


HEAD = re.compile(r'^\*\*\* Sampled system activity \((.+?)\) \(([\d.]+)ms elapsed\) \*\*\*')


def text_samples(text):
    cur = None
    in_tasks = has_energy = False
    for line in text.splitlines():
        m = HEAD.match(line)
        if m:
            if cur:
                yield cur
            end = datetime.strptime(m.group(1), '%a %b %d %H:%M:%S %Y %z')
            cur = (end, float(m.group(2)) / 1000, {})
            in_tasks = False
            continue
        if cur is None:
            continue
        if line.startswith('*** Running tasks'):
            in_tasks = True
            continue
        if in_tasks and line.startswith('Name'):
            has_energy = 'Energy Impact' in line
            continue
        if in_tasks and (not line.strip() or line.startswith('***')):
            in_tasks = line.strip() == '' and not cur[2]  # a blank line before the rows is fine
            continue
        if in_tasks and has_energy:
            tok = line.split()
            # Name may hold spaces; the pid is the first integer followed only by numbers.
            for i, x in enumerate(tok):
                if x.isdigit() and all(re.fullmatch(r'-?[\d.]+', y) for y in tok[i + 1:]) and len(tok) > i + 1:
                    cur[2][int(x)] = cur[2].get(int(x), 0) + float(tok[-1])
                    break
    if cur:
        yield cur


def iso(s):
    return datetime.fromisoformat(s.replace('Z', '+00:00'))


def interval(r):
    m = r.get('measured') or {}
    if r.get('t_exec') is not None and r.get('wall_exec') is not None and m.get('from') is not None:
        base = datetime.fromtimestamp(r['wall_exec'] / 1000, timezone.utc)
        at = lambda t: base + timedelta(microseconds=(t - r['t_exec']) / 1000)
        return at(m['from']), at(m['to']), 'measured'
    return iso(r['wall_start']), iso(r['wall_end']), 'wall_start..wall_end'


def per_run(log, runs):
    S = samples(log)
    out = []
    for r in runs:
        if r.get('status') != 'ok':
            continue
        a, b, basis = interval(r)
        pids = {p['pid'] for p in (r.get('procs') or []) + (r.get('procs_after') or [])}
        ei = covered = 0.0
        for end, secs, rates in S:
            start = end - timedelta(seconds=secs)
            ov = (min(b, end) - max(a, start)).total_seconds()
            if ov <= 0:
                continue
            covered += ov
            ei += sum(rates.get(p, 0) for p in pids) * ov
        dur = (b - a).total_seconds()
        out.append({'key': r['key'], 'build': r['build'], 'scenario': r['scenario'], 'basis': basis, 'seconds': dur,
                    'covered_s': covered, 'ei_s': ei if covered else None, 'ei_per_s': ei / covered if covered else None,
                    'pids': sorted(pids)})
    return out


def selftest():
    import tempfile, os
    t0 = datetime(2026, 11, 23, 10, 0, 0, tzinfo=timezone.utc)
    chunks = []
    for i in range(1, 4):
        chunks.append(plistlib.dumps({'timestamp': (t0 + timedelta(seconds=i)).replace(tzinfo=None), 'elapsed_ns': 1_000_000_000,
                                      'tasks': [{'pid': 10, 'name': 'Tracker', 'energy_impact_per_s': 2.0},
                                                {'pid': 11, 'name': 'other', 'energy_impact_per_s': 50.0}]}))
    text = '\n'.join([
        '*** Sampled system activity (Mon Nov 23 10:00:01 2026 +0000) (1000.00ms elapsed) ***', '',
        '*** Running tasks ***', '',
        'Name                               ID     CPU ms/s  User%  Deadlines (<2 ms, 2-5 ms)  Wakeups (Intr, Pkg idle)  Energy Impact',
        'Tracker Web Content                10     1.00      50.0   0.00   0.00              1.00   0.00              2.00',
        'other                              11     9.00      50.0   0.00   0.00              1.00   0.00              50.00', '',
        '*** Sampled system activity (Mon Nov 23 10:00:02 2026 +0000) (1000.00ms elapsed) ***', '',
        '*** Running tasks ***', '',
        'Name                               ID     CPU ms/s  User%  Deadlines (<2 ms, 2-5 ms)  Wakeups (Intr, Pkg idle)  Energy Impact',
        'Tracker Web Content                10     1.00      50.0   0.00   0.00              1.00   0.00              2.00', ''])
    run = {'key': 'k', 'build': 'b', 'scenario': 3, 'status': 'ok', 'procs': [{'pid': 10}],
           'wall_start': '2026-11-23T10:00:00.500Z', 'wall_end': '2026-11-23T10:00:02.000Z'}
    d = tempfile.mkdtemp()
    for name, data in (('p.log', b'\0'.join(chunks)), ('t.log', text.encode())):
        f = os.path.join(d, name)
        open(f, 'wb').write(data)
        [r] = per_run(f, [run])
        assert abs(r['ei_s'] - 3.0) < 1e-9 and abs(r['covered_s'] - 1.5) < 1e-9, (name, r)  # 2/s over 1.5 s, pid 11 ignored
    print('selftest ok')


if __name__ == '__main__':
    if sys.argv[1:] == ['--selftest']:
        selftest()
        sys.exit()
    log, runs_file = sys.argv[1], sys.argv[2]
    runs = [json.loads(l) for l in open(runs_file) if l.strip()]
    res = per_run(log, runs)
    if '--json' in sys.argv:
        json.dump(res, open(sys.argv[sys.argv.index('--json') + 1], 'w'), indent=1)
    print(f"{'run':34} {'basis':9} {'s':>6} {'covered':>7} {'EI·s':>9} {'EI/s':>7}")
    for r in res:
        f = lambda x, d: '—' if x is None else f'{x:.{d}f}'
        print(f"{r['key']:34} {r['basis'][:9]:9} {r['seconds']:6.1f} {r['covered_s']:7.1f} {f(r['ei_s'], 1):>9} {f(r['ei_per_s'], 2):>7}")
