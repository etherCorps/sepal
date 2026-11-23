"""Memory by kind, per build, from the `breakdown` runs (footprint categories, MB).

    python3 compare/results/memory_groups.py
"""
import json
from pathlib import Path

RAW = Path(__file__).parent / 'raw' / 'memory'
HELPERS = ('SetStoreUpdateService', 'com.apple.SafariPlatformSupport.Helper', 'com.apple.audio.SandboxHelper')


def group(name):
    if 'graphics' in name or name.startswith(('IOSurface', 'IOAccelerator', 'CoreAnimation')):
        return 'GPU (Metal/IOSurface)'
    if name.startswith(('Malloc', 'WebKit Malloc')):
        return 'malloc heaps'
    if name.startswith(('JS ', 'WebAssembly', 'App-Specific Tag')):
        return 'JS engine regions'
    if name == 'Owned physical footprint (unmapped)':
        return 'kernel-owned, unmapped'
    return 'other (binaries, stacks, page tables...)'


GROUPS = ['GPU (Metal/IOSurface)', 'malloc heaps', 'JS engine regions', 'kernel-owned, unmapped', 'other (binaries, stacks, page tables...)']


def groups(file):
    j = json.loads(file.read_text())
    out = {g: 0.0 for g in GROUPS}
    total = 0.0
    for p in j['processes']:
        if p['name'] in HELPERS:
            continue
        total += p['footprint'] / 2**20
        cats = {k: (c['dirty'] + c['swapped']) / 2**20 for k, c in p['categories'].items()}
        for k, v in cats.items():
            out[group(k)] += v
        # footprint also counts what no category's dirty+swapped does (WebKit's reclaimable pages)
        out['other (binaries, stacks, page tables...)'] += p['footprint'] / 2**20 - sum(cats.values())
    return total, out


if __name__ == '__main__':
    print(f"{'build':8} {'when':9} {'app procs':>9} " + ' '.join(f'{g[:22]:>22}' for g in GROUPS))
    for b in ['blitz', 'blitzv8', 'tauri', 'gpuix']:
        for tag in ('mounted5', 'end'):
            f = RAW / f'breakdown-{b}-1000-s9-{tag}.footprint.json'
            if not f.exists():
                continue
            total, g = groups(f)
            print(f"{b:8} {tag:9} {total:9.0f} " + ' '.join(f'{g[k]:22.0f}' for k in GROUPS))
