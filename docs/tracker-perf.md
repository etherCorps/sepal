# Tracker: performance

Measured 23 Nov 2026 on an M4 Pro Mac mini (macOS 27), release builds, headless.
`examples/tracker` (docs/tracker-plan.md) with 1,000, 5,000 and 20,000 seeded issues,
on QuickJS (the default) and V8 (`--features engine-v8`).

Each figure is the time from the input to the page being ready to paint: the
app's JavaScript, applying its ops, and the layout after. No painting (no GPU).
"first" is the first time, "median" is over the samples (j/k 20 presses,
keystrokes 6, Selects 3, navigations 40, and so on). Reproduce:

```sh
pnpm --filter tracker build
cd crates/host
cargo test --release --features screenshot --test tracker -- --ignored --nocapture bench
cargo test --release --features "screenshot engine-v8" --test tracker -- --ignored --nocapture bench
# TRACKER_BENCH_SIZES=5000 for one size
```

## At a glance (5,000 issues, median ms)

| What the user does | QuickJS | V8 | Notes |
|---|---|---|---|
| j/k through the list | 1.5 | 0.6 | |
| A search keystroke | 37 | 11 | filtering is < 1 ms; the rest is re-rendering the 200 rows |
| Open an issue (Enter) / close it | 33 / 32 | 21 / 17 | |
| j/k with the issue pane open | 5.5 | 1.6 | a route change and a load() each |
| Open a Select (filter, status) | 28 | 23 | mostly forced layouts: barely moves with the engine |
| Cmd+K / a palette keystroke | 18 / 4 | 14 / 1.7 | |
| Navigate to Issues (mounts 200 rows) | 107 | 28–39 | ~0.5 ms per row in QuickJS |
| Open Board (~300 cards) | 118 | 33 | |
| Regroup / resort / filter | 90–110 | 30–40 | each re-creates the visible rows |
| Bulk-set status of 50 | 40 | 17 | |
| A teammate's edit arriving | 6.8 | 1.7 | 31 / 7 at 20,000: a full re-sort per edit |
| Start: load, seed 5,000, mount | 240 | 118 | first layout at 278 / 157 ms |
| Idle turn | 0.1 | 0.1 | no timers left; no wake-ups |

## The cliffs

| | QuickJS | V8 |
|---|---|---|
| Show all 1,000 rows | 0.39 s, 40k nodes | 0.10 s |
| Show all 5,000 rows | **2.9 s**, 195k nodes, 767 MB | 0.72 s |
| Show all 20,000 rows | **stopped by the 10 s watchdog** at 17,314 rows, 2.7 GB | 3.2 s, 775k nodes, 2.5 GB |
| j with 5,000 rows shown (4 ops) | 58 ms | 34 ms |
| j with 20,000 rows shown (4 ops) | — | 141 ms |
| Open a Select with 5,000 rows shown | 494 ms | 494 ms |
| Open a Select with 20,000 rows shown | — | 1,965 ms |
| Leave 5,000 rows, then the deferred GC 1 s later | 325 ms, then 390 ms | 73 ms, then 124 ms |

## What the numbers say

1. **Rendering rows is the JavaScript cost.** A row is ~40 host nodes and ~140
   ops; mounting one costs ~0.5 ms in QuickJS and ~0.15 ms in V8. Everything
   that re-creates the visible 200 rows (navigate, regroup, resort, filter,
   search) lands at ~100 ms in QuickJS and ~35 ms in V8. V8 is 2.5–4x faster
   on these turns.
2. **Some turns are bound by Blitz, not JavaScript, and no engine helps.**
   Opening a Select costs ~25 ms on a 200-row page on both engines (forced
   layouts: Floating UI measures, writes, measures). With every row shown it is
   494 ms at 5,000 and ~2 s at 20,000 on both: each forced layout lays out the
   whole page. Likewise j on a 195k-node page changes 2 class names (4 ops) and
   takes 34–58 ms; at 775k nodes, 141 ms. A small change to a big page
   restyles and lays out far more than it changed. This is the case CLAUDE.md
   asks for a trace before optimising Rust; the trace says layout here.
3. **Per-row memory is high.** ~4.4 KB per Blitz node, so 5,000 rows is
   ~800 MB and 20,000 is ~2.5 GB. Leaving such a page frees the nodes after
   the deferred GC (QuickJS: 390 ms pause), and Activity Monitor drops to
   ~535 MB, not back to the start.
4. **Everyday interactions are fine on both engines** at 200 rows a page:
   j/k, palette, pane, Selects, edits all stay under ~40 ms, most well
   under 16 ms on V8. QuickJS misses a 16 ms frame on every re-render of the
   list.
5. **Data size hardly matters until it is rendered.** 20,000 issues cost the
   same as 5,000 for anything that shows 200 rows, except work over all
   issues per edit (a teammate's edit, moving a card: ~30 ms in QuickJS at
   20,000, from re-sorting everything).

## What would move them, in order of payoff

1. **Scroll events and `scrollTop` in the DOM layer** (done since: see
   "Scrolling" in the README), so an app can render only the rows in view. That removes the cliffs
   outright: "Show all" would mount ~25 rows, not 5,000. Done: see "The
   cliffs, with the list virtualised" below.
2. **Blitz: restyle and relayout only what a change touches.** Profile `j` on
   the 195k-node page and a Select's forced layouts (`SVELTE_BLITZ_TRACE=1`,
   then a sampler) before changing anything; candidates are style invalidation
   of the class change and Taffy re-laying out unchanged siblings.
3. **Fewer nodes per row in the app.** The two glyphs are SVGs of several
   shapes each, and Svelte adds a comment anchor per `{#if}`/`{#each}`/component.
   Halving the nodes halves render time, memory and the GC pause.
4. **App: incremental updates for single edits** (teammate edits, card moves)
   instead of re-sorting every issue; and per-row highlight state (the review
   finding left open), which makes j on a long list O(1) in JavaScript.
5. **V8 as an option for heavy apps:** 2.5–4x on render-bound turns, no
   help on layout-bound ones, 62 MB binary instead of 24 (HANDOVER "V8 spike").

## The cliffs, with the list virtualised

The list now mounts only the rows in view plus 300px either side (28 rows in
an 800px window), whatever is paged in. QuickJS, release, same machine, before
and after in one sitting (absolute numbers were noisier than the tables above):

| | 5,000 before | 5,000 after | 20,000 before | 20,000 after |
|---|---|---|---|---|
| Show all | 1,625 ms | 0.6 ms | 6,950 ms | 1.1 ms |
| Scroll the Show-all list one screen (wheel, median) | — | 6.2 ms | — | 6.1 ms |
| j with every row shown (median) | 25 ms | 0.4 ms | 165 ms | 0.4 ms |
| Open the Status filter, every row shown (median) | 493 ms | 14 ms | 1,910 ms | 14 ms |
| Leave Show all (to Board) | 182 ms | 85 ms | 1,215 ms | 83 ms |
| The deferred GC, 1 s later | 224 ms | 1.1 ms | 905 ms | 1.1 ms |
| Live nodes with every row shown | 108,932 | ~7,000 | 430,724 | ~7,000 |
| Footprint after Show all | 599 MB | 73 MB | 2,015 MB | 82 MB |

The first j after scrolling away jumps the list back to the highlight and
mounts a screen of rows (~9 ms). Blitz issues found on the way, worked around
in the app: hit testing does not clip to a scrolled box (a click on the
toolbar landed on a row scrolled up under it; the header and toolbar are
`relative z-10`), and a box keeps its scroll offset when its content shrinks
(the list writes `scrollTop` back to its end).

## Not measured here

- Painting and the GPU: headless has none. Scrolling the 5,000-row list with a
  wheel, and idle CPU in a real window, need a window pass (`SVELTE_BLITZ_TRACE=1`
  and `top -stats cpu,idlew`).
- Compact rows (P12): the switch is on Settings, where no rows are mounted.

## Full results

| scenario | N | QuickJS first | QuickJS median | V8 first | V8 median | ops (QuickJS) | live nodes after | footprint MB (QuickJS) |
|---|---|---|---|---|---|---|---|---|
| P1 start: load, seed and mount (then first layout at 278 ms) | 5000 | 240.5 |  | — | — |  | 8475 | 72 |
| P1 reseed: generate and render | 1000 | 114.9 |  | 33.6 |  | 21253 | 14477 | 81 |
| P2 search: a keystroke (typing 'deploy') | 1000 | 37.7 | 37.7 | 10.6 | 10.6 | 8574 | 23735 | 101 |
| P2 search: Backspace | 1000 | 1.4 | 41.7 | 0.5 | 10.7 | 0 | 35824 | 137 |
| P3 j: move the highlight | 1000 | 2.3 | 1.6 | 1.4 | 0.7 | 2 | 35824 | 138 |
| P3 Enter: open the issue pane | 1000 | 36.9 |  | 36.1 |  | 2995 | 36676 | 139 |
| P3 j with the pane open (route + load) | 1000 | 6.8 | 6.4 | 3.2 | 1.6 | 162 | 8799 | 145 |
| P4 open the pane's status Select | 1000 | 30.7 | 29.8 | 29.3 | 23.4 | 604 | 8403 | 145 |
| P3 Escape: close the pane | 1000 | 32.2 |  | 16.8 |  | 3153 | 9428 | 145 |
| P4 open the Status filter (200 rows) | 1000 | 29.4 | 29.4 | 23.9 | 22.7 | 632 | 10049 | 146 |
| P5 group by (assignee, priority, none, status) | 1000 | 148.4 | 115.1 | 35.0 | 39.1 | 29214 | 16548 | 113 |
| P5 sort by (priority, title, updated) | 1000 | 32.8 | 32.8 | 13.5 | 13.5 | 4269 | 20940 | 113 |
| P5 filter status = Done | 1000 | 107.4 |  | 38.6 |  | 27137 | 16601 | 113 |
| P8 Cmd+K: open the palette | 1000 | 18.5 |  | 16.1 |  | 1115 | 24951 | 113 |
| P8 palette: a keystroke ('eng-12') | 1000 | 3.6 | 2.6 | 1.7 | 1.2 | 452 | 25262 | 113 |
| P8 palette: Enter (open ENG-12) | 1000 | 33.2 |  | 15.8 |  | 2756 | 26036 | 113 |
| P9 c: open New issue | 1000 | 30.1 |  | 17.6 |  | 1814 | 27715 | 113 |
| P9 Cmd+Enter: create the issue | 1000 | 7.8 |  | 4.7 |  | 220 | 9602 | 113 |
| P9 Shift+click: check 50 rows | 1000 | 11.8 |  | 4.1 |  | 148 | 9753 | 113 |
| P9 bulk: set status of 50 | 1000 | 33.0 |  | 12.0 |  | 6765 | 11859 | 113 |
| P10 save edits.json (the turn with the write) | 1000 | 0.6 |  | 0.6 |  | 0 | 11859 | 113 |
| P14 a teammate's edit, on /issues | 1000 | 2.9 | 1.8 | 1.4 | 0.5 | 120 | 12193 | 113 |
| P7 open Board | 1000 | 119.1 |  | 32.8 |  | 30134 | 20877 | 115 |
| P7 Shift+L: move a card | 1000 | 5.0 | 5.0 | 2.0 | 1.9 | 288 | 21179 | 115 |
| P14 a teammate's edit, on /board | 1000 | 3.1 | 3.1 | 1.1 | 1.1 | 100 | 22137 | 117 |
| P11 open Insights | 1000 | 29.6 |  | 12.1 |  | 3648 | 10991 | 117 |
| P11 change the range (26w, 4w, 12w) | 1000 | 7.8 | 4.9 | 3.2 | 2.3 | 627 | 11095 | 117 |
| open Settings | 1000 | 12.2 |  | 5.0 |  | 1040 | 11413 | 117 |
| P13 back to Issues | 1000 | 107.8 |  | 39.4 |  | 28175 | 19547 | 117 |
| P13 cycle 4 screens x10 (each nav) | 1000 | 123.7 | 111.6 | 33.6 | 28.4 | 30145 | 27488 | 142 |
| P13 after the churn and a GC (live nodes) | 1000 | 0.0 |  | 0.0 |  | 0 | 9228 | 142 |
| P6 Show 200 more | 1000 | 100.6 |  | 36.2 |  | 27424 | 17062 | 142 |
| P6 Show all (1001 rows) | 1000 | 388.0 |  | 95.4 |  | 82262 | 40494 | 176 |
| P6 j with every row shown | 1000 | 10.9 | 10.1 | 5.2 | 3.3 | 4 | 40494 | 176 |
| P4 open the Status filter (every row shown) | 1000 | 114.9 | 114.9 | 98.1 | 98.1 | 632 | 41115 | 180 |
| P6 leave Show all (to Board) | 1000 | 146.1 |  | 38.7 |  | 30145 | 49829 | 215 |
| P6 the deferred GC, 1 s later | 1000 | 91.1 |  | 33.5 |  | 40012 | 9817 | 146 |
| idle turn | 1000 | 0.1 |  | 0.1 |  | 0 | 9817 | 146 |
| P1 reseed: generate and render | 5000 | 44.2 |  | 15.1 |  | 0 | 8475 | 36 |
| P2 search: a keystroke (typing 'deploy') | 5000 | 33.2 | 37.0 | 10.8 | 10.8 | 6274 | 19016 | 38 |
| P2 search: Backspace | 5000 | 5.4 | 41.0 | 1.0 | 9.8 | 0 | 14263 | 41 |
| P3 j: move the highlight | 5000 | 2.2 | 1.5 | 1.2 | 0.6 | 2 | 14263 | 41 |
| P3 Enter: open the issue pane | 5000 | 33.5 |  | 21.2 |  | 2646 | 15009 | 41 |
| P3 j with the pane open (route + load) | 5000 | 7.0 | 5.5 | 3.0 | 1.6 | 763 | 17387 | 41 |
| P4 open the pane's status Select | 5000 | 29.7 | 28.5 | 29.2 | 29.2 | 604 | 8301 | 41 |
| P3 Escape: close the pane | 5000 | 32.1 |  | 17.2 |  | 3165 | 9330 | 41 |
| P4 open the Status filter (200 rows) | 5000 | 28.3 | 28.3 | 24.6 | 22.7 | 632 | 9951 | 41 |
| P5 group by (assignee, priority, none, status) | 5000 | 107.1 | 107.1 | 40.4 | 40.4 | 27375 | 16507 | 42 |
| P5 sort by (priority, title, updated) | 5000 | 90.1 | 90.1 | 29.0 | 36.5 | 20086 | 14809 | 42 |
| P5 filter status = Done | 5000 | 108.2 |  | 32.0 |  | 27088 | 22642 | 42 |
| P8 Cmd+K: open the palette | 5000 | 18.0 |  | 14.2 |  | 1115 | 30961 | 43 |
| P8 palette: a keystroke ('eng-12') | 5000 | 7.6 | 4.2 | 3.7 | 1.7 | 455 | 9594 | 43 |
| P8 palette: Enter (open ENG-12) | 5000 | 32.8 |  | 17.4 |  | 2759 | 10368 | 43 |
| P9 c: open New issue | 5000 | 30.9 |  | 23.7 |  | 1814 | 12051 | 43 |
| P9 Cmd+Enter: create the issue | 5000 | 13.0 |  | 6.3 |  | 220 | 12110 | 43 |
| P9 Shift+click: check 50 rows | 5000 | 12.2 |  | 4.0 |  | 148 | 12261 | 43 |
| P9 bulk: set status of 50 | 5000 | 40.3 |  | 16.8 |  | 6977 | 14417 | 43 |
| P10 save edits.json (the turn with the write) | 5000 | 0.6 |  | 0.6 |  | 0 | 14417 | 43 |
| P14 a teammate's edit, on /issues | 5000 | 8.9 | 6.8 | 3.0 | 1.7 | 125 | 14641 | 43 |
| P7 open Board | 5000 | 117.9 |  | 32.8 |  | 29990 | 23294 | 43 |
| P7 Shift+L: move a card | 5000 | 10.8 | 9.9 | 3.6 | 4.3 | 274 | 23584 | 43 |
| P14 a teammate's edit, on /board | 5000 | 8.4 | 7.9 | 2.8 | 2.5 | 98 | 24390 | 43 |
| P11 open Insights | 5000 | 30.3 |  | 13.2 |  | 3586 | 10935 | 43 |
| P11 change the range (26w, 4w, 12w) | 5000 | 9.2 | 6.9 | 4.3 | 3.5 | 627 | 11041 | 43 |
| open Settings | 5000 | 11.8 |  | 4.9 |  | 1040 | 11359 | 43 |
| P13 back to Issues | 5000 | 107.6 |  | 39.0 |  | 28287 | 19515 | 43 |
| P13 cycle 4 screens x10 (each nav) | 5000 | 115.4 | 106.6 | 33.2 | 28.2 | 29976 | 27476 | 55 |
| P13 open and close an issue x25 | 5000 | 33.8 | 32.4 | 14.8 | 14.2 | 3646 | 14415 | 151 |
| P13 after the churn and a GC (live nodes) | 5000 | 0.0 |  | 0.0 |  | 0 | 9249 | 55 |
| P6 Show 200 more | 5000 | 94.7 |  | 34.4 |  | 26804 | 16873 | 55 |
| P6 Show all (5001 rows) | 5000 | 2891.9 |  | 715.9 |  | 625326 | 195096 | 767 |
| P6 j with every row shown | 5000 | 58.0 | 57.6 | 33.6 | 30.6 | 4 | 195096 | 767 |
| P4 open the Status filter (every row shown) | 5000 | 494.4 | 504.1 | 494.5 | 494.5 | 632 | 195717 | 781 |
| P6 leave Show all (to Board) | 5000 | 325.1 |  | 72.9 |  | 29976 | 204390 | 815 |
| P6 the deferred GC, 1 s later | 5000 | 389.7 |  | 123.7 |  | 194615 | 9775 | 535 |
| idle turn | 5000 | 0.1 |  | 0.1 |  | 0 | 9775 | 535 |
| P1 reseed: generate and render | 20000 | 227.1 |  | 58.1 |  | 19896 | 14092 | 58 |
| P2 search: a keystroke (typing 'deploy') | 20000 | 72.2 | 72.2 | 15.8 | 15.8 | 6224 | 18117 | 59 |
| P2 search: Backspace | 20000 | 23.4 | 51.6 | 3.0 | 10.7 | 0 | 29588 | 61 |
| P3 j: move the highlight | 20000 | 2.3 | 1.5 | 1.0 | 0.6 | 2 | 29588 | 61 |
| P3 Enter: open the issue pane | 20000 | 40.2 |  | 19.8 |  | 2905 | 30408 | 61 |
| P3 j with the pane open (route + load) | 20000 | 6.8 | 5.7 | 2.3 | 1.6 | 544 | 32468 | 62 |
| P4 open the pane's status Select | 20000 | 32.5 | 31.0 | 29.7 | 23.9 | 604 | 8545 | 62 |
| P3 Escape: close the pane | 20000 | 33.7 |  | 18.5 |  | 3249 | 9602 | 62 |
| P4 open the Status filter (200 rows) | 20000 | 30.1 | 28.7 | 28.3 | 23.6 | 632 | 10223 | 62 |
| P5 group by (assignee, priority, none, status) | 20000 | 112.3 | 119.0 | 36.4 | 36.4 | 27643 | 16560 | 65 |
| P5 sort by (priority, title, updated) | 20000 | 120.1 | 122.9 | 35.2 | 43.9 | 19048 | 16148 | 66 |
| P5 filter status = Done | 20000 | 119.7 |  | 34.1 |  | 27370 | 24083 | 66 |
| P8 Cmd+K: open the palette | 20000 | 18.5 |  | 14.7 |  | 1115 | 32429 | 66 |
| P8 palette: a keystroke ('eng-12') | 20000 | 35.3 | 14.9 | 8.9 | 3.7 | 454 | 9663 | 66 |
| P8 palette: Enter (open ENG-12) | 20000 | 33.3 |  | 17.6 |  | 2750 | 10437 | 66 |
| P9 c: open New issue | 20000 | 29.3 |  | 18.0 |  | 1814 | 12148 | 66 |
| P9 Cmd+Enter: create the issue | 20000 | 35.7 |  | 14.9 |  | 220 | 12207 | 66 |
| P9 Shift+click: check 50 rows | 20000 | 11.9 |  | 3.9 |  | 148 | 12358 | 66 |
| P9 bulk: set status of 50 | 20000 | 63.4 |  | 22.4 |  | 6912 | 14485 | 66 |
| P10 save edits.json (the turn with the write) | 20000 | 0.6 |  | 0.6 |  | 0 | 14485 | 66 |
| P14 a teammate's edit, on /issues | 20000 | 33.1 | 31.1 | 9.5 | 7.4 | 119 | 14556 | 66 |
| P7 open Board | 20000 | 125.9 |  | 36.1 |  | 29940 | 23211 | 66 |
| P7 Shift+L: move a card | 20000 | 34.4 | 34.4 | 9.7 | 8.9 | 283 | 23509 | 66 |
| P14 a teammate's edit, on /board | 20000 | 30.4 | 30.1 | 19.5 | 7.8 | 105 | 24364 | 66 |
| P11 open Insights | 20000 | 43.7 |  | 14.7 |  | 3536 | 10918 | 66 |
| P11 change the range (26w, 4w, 12w) | 20000 | 21.6 | 20.9 | 5.9 | 5.4 | 621 | 11022 | 66 |
| open Settings | 20000 | 13.2 |  | 4.9 |  | 1040 | 11340 | 66 |
| P13 back to Issues | 20000 | 127.7 |  | 40.8 |  | 28328 | 19504 | 66 |
| P13 cycle 4 screens x10 (each nav) | 20000 | 129.0 | 118.1 | 34.5 | 29.2 | 29927 | 27475 | 79 |
| P13 after the churn and a GC (live nodes) | 20000 | 0.0 |  | 0.0 |  | 0 | 9257 | 79 |
| P6 Show 200 more | 20000 | 99.3 |  | 36.1 |  | 26985 | 16953 | 79 |
| P6 Show all (QuickJS: 17314 rows; V8: 20001 rows) | 20000 | 13102.4 |  | 3209.3 |  | 2504741 | 731527 | 2683 |
| P6 j with every row shown | 20000 | 63.2 | 63.1 | 141.5 | 136.8 | 0 | 731527 | 2686 |
| P1 start: load, seed and mount (then first layout at 157 ms) | 5000 | — | — | 118.4 |  | — | 8475 | — |
| P4 open the Status filter (every row shown) | 20000 | — | — | 1965.2 | 1969.1 | — | 775448 | — |
| P6 leave Show all (to Board) | 20000 | — | — | 740.7 |  | — | 9777 | — |
| P6 the deferred GC, 1 s later | 20000 | — | — | 37.8 |  | — | 9777 | — |
| idle turn | 20000 | — | — | 0.1 |  | — | 9777 | — |
