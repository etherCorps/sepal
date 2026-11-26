Navigation after the DOM layer's three fixes, against main, at 1,000 issues. 

Session `nav-1`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from the input.

### Idle, 20 s (scenario 3)

| | main | branch | branch − main |
|---|---:|---:|---:|
| CPU time, ms | 6.5<br><sub>6.3–7.0</sub> | 6.8<br><sub>6.3–8.3</sub> | +0.3 |
| GPU time, ms | 0.00 | 0.00 | +0.00 |
| frames presented | 0 | 0 | +0 |
| interrupt wake-ups | 34<br><sub>33–35</sub> | 32<br><sub>31–35</sub> | -2 |
| RAM 5 s after load, MB | 105.1<br><sub>102.3–105.2</sub> | 105.4<br><sub>102.8–105.8</sub> | +0.3 |
| RAM at the end, MB | 96.9<br><sub>96.1–97.1</sub> | 97.2<br><sub>96.6–97.6</sub> | +0.3 |

<sub>Runs: main: 3, branch: 3.</sub>

### Open an issue (scenario 5)

| | main | branch | branch − main |
|---|---:|---:|---:|
| click row → mark, ms | 22.5<br><sub>22.4–22.9</sub> | 24.6<br><sub>24.1–28.3</sub> | +2.1 |
| click row → frame, ms | 44.0<br><sub>38.1–45.1</sub> | 36.9<br><sub>36.1–40.6</sub> | -7.1 |
| Escape → mark, ms | 30.3<br><sub>27.4–30.4</sub> | 24.7<br><sub>18.0–29.9</sub> | -5.6 |
| Escape → frame, ms | 44.5<br><sub>43.1–44.5</sub> | 43.4<br><sub>36.6–44.6</sub> | -1.1 |

<sub>Runs: main: 3, branch: 3.</sub>

### Switch screens (scenario 10)

| | main | branch | branch − main |
|---|---:|---:|---:|
| board → mark, ms | 109.3<br><sub>108.0–112.4</sub> | 98.5<br><sub>97.0–112.1</sub> | -10.8 |
| insights → mark, ms | 51.3<br><sub>51.0–54.9</sub> | 58.8<br><sub>58.3–61.9</sub> | +7.6 |
| settings → mark, ms | 40.2<br><sub>39.5–47.3</sub> | 41.2<br><sub>38.2–44.3</sub> | +1.1 |
| issues → mark, ms | 61.5<br><sub>53.5–62.6</sub> | 51.0<br><sub>50.5–59.8</sub> | -10.4 |
| board → frame, ms | 143.0<br><sub>140.1–147.0</sub> | 133.7<br><sub>128.1–141.7</sub> | -9.3 |
| insights → frame, ms | 65.5<br><sub>64.8–69.0</sub> | 76.2<br><sub>72.2–77.3</sub> | +10.7 |
| settings → frame, ms | 48.8<br><sub>48.7–54.7</sub> | 49.7<br><sub>46.8–51.7</sub> | +0.9 |
| issues → frame, ms | 77.8<br><sub>68.9–86.7</sub> | 66.2<br><sub>65.7–75.2</sub> | -11.6 |
| board: the turn that renders it, ms | 109.2<br><sub>107.8–109.2</sub> | 103.7<br><sub>102.7–105.7</sub> | -5.5 |
| insights: the turn that renders it, ms | 39.6<br><sub>39.0–42.0</sub> | 40.5<br><sub>38.2–40.9</sub> | +0.9 |
| settings: the turn that renders it, ms | 23.8<br><sub>23.5–25.9</sub> | 23.3<br><sub>22.7–25.1</sub> | -0.5 |
| issues: the turn that renders it, ms | 44.4<br><sub>44.2–46.1</sub> | 41.2<br><sub>40.7–42.1</sub> | -3.2 |

<sub>Runs: main: 3, branch: 3.</sub>

### Notes

- **The render turns are the cleaner number.** The Board's is 5.5 ms
  shorter and Issues' 3.2; Settings and Insights are within 1 ms. The
  marks and frames add the wait before that turn: SvelteKit follows a
  link from a `requestAnimationFrame` callback, which waits for the next
  vsync, and while the nav item's hover transition draws, frames are
  coming every 13 ms. Where the click lands in that cycle moves a mark by
  up to a frame. Insights shows it: the same render turn on both builds,
  and its mark 7.6 ms later on the branch. Headless, where there are no
  frames, its first visit is 1 ms quicker.
- **Idle** is the same: no frames, 34 and 32 wake-ups in 20 s.

### Where SvelteKit's link scan went

After every navigation SvelteKit takes `querySelectorAll('a')` and, for
each link, its `href`, a `new URL` of it, and five `getAttribute`s on every
ancestor up to `<html>`, to set up preloading. Timed piece by piece on the
Board (316 links, 8.9 ancestors each, 3,529 elements), headless, µs a link:

| | before | after |
|---|---:|---:|
| `a.href` | 4.9 | 0.47 |
| `new URL(a.href, document.baseURI)` | 4.27 | 0.87 |
| the ancestor walk (`getAttribute` and `parentNode`) | 4.35 | 4.11 |
| the loop, whole | 16.3 | 7.28 |
| `querySelectorAll('a')`, per call | 1,775 | 800 |
| `querySelector('[autofocus]')`, per call | 2,475 | 750 |

One commit for each, with a cycle collection before each timed step
(`drive`'s `gc:`): the Board's repeat visit went from 76 to 73 ms with the
`href` cache, to 70 with the selector tests, and to 69 with url.js. The
ancestor walk is SvelteKit's own five calls a level, which no change on
this side makes fewer.

The 4.7 ms the Board trailed the old main by (`preload-fix.md`, without the
scan) was `[autofocus]`, the other walk of the whole page: with both
walks taken out, the Board's repeat visit was 72 ms, the old main's
(`bae68bb`) 72 ms.

### First visits

A screen's first visit is not slower in the split build for any reason of
its own: headless, the split build trails a single bundle by 1-1.5 ms on
Insights, Settings and opening an issue (the route's chunk read and Vite's
preload helper adding a `modulepreload` link), and not at all on the
Board. The route's chunk is read in the turn that renders it, no turn waits
for a stylesheet, and the turns are the same: the click's frame, then the
timer that navigates and renders.

The Board's first visit costs ~50 ms over a repeat (136 against 76 ms
here), on the old main too. It is spread evenly over the 300 cards it
draws (210-320 µs a card, against 130-145), and it is neither `cn()` (0.55
ms), nor Svelte's templates (a custom renderer gets new nodes every time),
nor Bits UI, the app's data, QuickJS's collector or its warm-up. Making and
dropping 20,000 of the renderer's nodes before the first visit takes all of
it away: the process makes nodes in numbers it has not before. Which part
of that growth costs, JavaScript's or the allocator's, is the next job.

| headless, ms | first visit | repeat | first, 20,000 nodes made first | repeat, same |
|---|---:|---:|---:|---:|
| before | 134-136 | 76-77 | 76-78 | 70-72 |
| after | 132-136 | 72-73 | 73 | 67-68 |

