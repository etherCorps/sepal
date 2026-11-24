"""Live GPU resources from a SVELTE_BLITZ_GPU_LOG log (wgpu-hal change 2), at the first line matching
a marker (default: the end). Usage: gpu_live.py log [marker-regex]"""
import re, sys
log = open(sys.argv[1]).read().splitlines()
marker = re.compile(sys.argv[2]) if len(sys.argv) > 2 else None
live = {}
for line in log:
    if marker and marker.search(line):
        break
    m = re.match(r'\[gpu\] \+ (texture|buffer) #(\w+) "(.*?)": (.*?), (\d+) B allocated', line)
    if m:
        kind, ident, label, desc, size = m.groups()
        live[ident] = (int(size), kind, label, desc)
        continue
    m = re.match(r'\[gpu\] \+ buffer #(\w+) "(.*?)": (\d+) B, (\d+) B allocated', line)
    if m:
        ident, label, size, alloc = m.groups()
        live[ident] = (int(alloc), 'buffer', label, f'{size} B')
        continue
    m = re.match(r'\[gpu\] - (texture|buffer) #(\w+)', line)
    if m:
        live.pop(m.group(2), None)
items = sorted(live.values(), reverse=True)
total = sum(i[0] for i in items)
print(f'{len(items)} live, {total/2**20:.1f} MB allocated')
for size, kind, label, desc in items:
    if size >= 64 * 1024:
        print(f'{size/2**20:8.2f} MB  {kind:7} {label!r:42} {desc}')
small = [i for i in items if i[0] < 64 * 1024]
print(f'{sum(i[0] for i in small)/2**20:8.2f} MB  {len(small)} more under 64 KB each')
