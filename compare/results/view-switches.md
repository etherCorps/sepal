# Frames per view switch (scenario 10, one recorded run per app, 1,000 issues)

Session `j4-views` (raw data: `compare/results/raw/j4-views` on branch tracker-compare): Sepal HEAD (4a2ae4b) on a debug host that also prints
each frame's changed commands (`SB_DEBUG_CHANGES`, local only, not committed),
gpuix-svelte and Tauri 2, each with a screen recording (`frames/`).

## Changed frames per switch, from the screen recordings (display-rate frames that differ)

| | Board | Insights | Settings | Issues |
|---|---|---|---|---|
| Sepal | 10 | 14 | 16 | 12 |
| gpuix-svelte | 3 | 3 | 5 | 4 |
| Tauri 2 | 14 | 13 | 15 | 14 |

## Sepal: every frame presented, and what caused it (from the host's trace)

Board, 17 frames:
- 3 ms: 1 full. First frame after the idle release (4a2ae4b): the window had been idle 5 s,
  so nothing was kept.
- 13-30 ms: 4 partial. The pointer reached Board: its hover background fades in (CSS
  `transition-colors`, 150 ms, on every sidebar item: `+layout.svelte`, `item`).
- 142 ms: 1 full. The JS turn after the click renders the Board view: 19,242 ops.
- 149-269 ms: 11 partial. Issues, the item that was active, fades its background out (the
  same transition); Board's hover fade ends in the first of them.

Insights, 16 frames:
- 3-29 ms: 4 partial, Insights hover fade-in.
- 81 ms: 1 full, the JS turn rendering Insights: 3,090 ops.
- 87-229 ms: 11 partial, Board fading out (with Insights' fade-in ending).

Settings, 20 frames:
- 1-39 ms: 5 partial, Settings hover fade-in.
- 79 ms: 1 full, the JS turn rendering Settings: 1,041 ops.
- 85-237 ms: 13 partial, Insights fading out.
- 2,063 ms: 1 partial, the pointer reaching Issues for the next click.

Issues, 17 frames:
- 0-29 ms: 4 partial, Issues hover fade-in.
- 78 ms: 1 full, the JS turn rendering the list: 4,529 ops.
- 93-230 ms: 12 partial, Settings fading out.

So of ~17 frames per switch, one is the view (a JS turn); the rest are two 150 ms colour
transitions on the sidebar (hover in on the new item, active out on the old). Tauri runs the
same CSS and shows the same count; gpuix's port has no such transitions.
