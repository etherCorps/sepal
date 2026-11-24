# svelte-blitz: fewer command buffers, and scrolling by moving the kept frame

The Tracker, bench build, 1,000 issues, window 1200x800 at 2x, Apple M4 Pro.
Compare harness, scenarios 4 (clock, 20 s), 8 (New issue dialog), 9 (Show all
and wheel-scroll the list), 10 (switch views); 3 runs per build, builds
interleaved run by run (`--interleave`), so both see the same machine. Cells:
median (min-max). Baseline for everything before: `damage-baseline.md`.

## Command buffers (committed)

A clock tick committed 10 Metal command buffers in 3 submits; now 5 (one main
submit with the uploads, the scene and the copy to the surface, plus the
fence's signal and the present), and 5 render passes instead of 6.
Session `p3-abc`:

| | HEAD before | + empty command buffers not committed | + kept frame copied in the frame's submit |
|---|---|---|---|
| clock: GPU ms | 14.1 (9.0-20.5) | 13.4 (13.2-14.1) | 15.1 (15.1-15.6) |
| clock: CPU ms | 97 | 95 | 94 |
| dialog: GPU ms | 67.1 (65.9-83.1) | 65.8 (62.6-70.8) | 65.0 (57.9-65.5) |
| dialog: CPU ms | 268 | 250 | 249 |
| scroll: GPU ms | 518 (429-542) | 546 (364-555) | 525 (510-589) |
| scroll: CPU s | 3.42 | 3.42 | 3.43 |
| views: GPU ms | 62.6 (60.9-63.2) | 61.9 (61.5-62.0) | 39.4 (37.4-42.7) |
| views: CPU ms | 383 | 382 | 340 |

Empty command buffers are cheap: not committing them changed GPU time by
nothing these runs can resolve. Copying the kept frame in the frame's own
submit, in a pass that clears instead of loading (so wgpu no longer clears
every new surface texture in a pass of its own), cut view switches by 37%.
Copying only the damaged rectangles to the surface is not sound on Metal: a
drawable's contents are undefined when it is acquired (CAMetalLayer rotates a
small pool, with no buffer age), so the whole kept frame is copied every frame.

## Scrolling by moving the kept frame (not committed)

When only a scrolled box's offset changed, move its pixels in the kept frame
(two render passes through a scratch texture; a texture-to-texture copy would
be a blit) and draw only the exposed strip, what changed inside, what is drawn
over the box (the overlay scrollbar) and changes elsewhere. Verify mode
(`SVELTE_BLITZ_DAMAGE=verify`) found no frame differing from a full repaint at
1,000 and 10,000 issues (`raw/p4-verify`).

First version, separate renders for up to 4 regions, detection on every frame
(`raw/p4-shift`, against the separate session `depth-on`): scroll GPU 533 ->
788 ms, CPU 3.46 -> 3.99 s; dialog CPU +49%, clock CPU +29%.

Second version, on top of the committed changes above: one render clipped to
the regions, detection only when the plain diff repaints over a quarter of the
window. Session `p4-shift2`, interleaved:

| | HEAD | + moving the kept frame |
|---|---|---|
| scroll: GPU ms | 519 (485-532) | 534 (514-551) |
| scroll: CPU s | 3.41 | 3.33 |
| scroll: frames repainted in full | 384 | 31 (351 moved instead) |
| dialog: GPU ms | 68.8 (62.9-76.4) | 55.9 (47.2-57.1) |
| dialog: CPU ms | 206 (204-240) | 306 (284-309) |
| clock: GPU ms | 18.2 (17.5-19.4) | 13.8 (10.3-22.5) |
| views: GPU ms | 50.3 (49.4-50.5) | 45.5 (42.5-51.2) |

Scrolling: 351 of 384 full repaints became moves, for 2.5% less CPU and no
change in GPU time. Drawing the list was not what a scroll frame costs. The
dialog's large frames now pay for looking for a scrolled box (+49% CPU). Not
worth it; the code is kept in a git stash.
