SvelteKit's split build after the DOM layer stopped walking the page for selectors that end in a tag, against one bundle and main, at 1,000 issues. 

Session `html-3`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from the input. Single and split run the same host; `split − single` is the difference of the medians.

### Open an issue (scenario 5)

| | main | single | split, fixed | split − single |
|---|---:|---:|---:|---:|
| click row → mark, ms | 20.6<br><sub>19.1–21.1</sub> | 23.4<br><sub>23.3–23.8</sub> | 26.3<br><sub>25.7–27.2</sub> | +2.9 |
| click row → frame, ms | 33.5<br><sub>31.5–33.8</sub> | 36.2<br><sub>35.8–36.3</sub> | 38.9<br><sub>38.9–40.4</sub> | +2.8 |
| Escape → mark, ms | 36.6<br><sub>35.8–37.5</sub> | 26.4<br><sub>24.4–39.7</sub> | 37.2<br><sub>31.9–38.4</sub> | +10.8 |
| Escape → frame, ms | 52.0<br><sub>51.0–52.9</sub> | 40.8<br><sub>38.4–54.7</sub> | 53.1<br><sub>45.4–53.7</sub> | +12.3 |

<sub>Runs: main: 3, single: 3, split, fixed: 3.</sub>

### Switch screens (scenario 10)

| | main | single | split, fixed | split − single |
|---|---:|---:|---:|---:|
| board → mark, ms | 78.3<br><sub>76.9–78.5</sub> | 94.0<br><sub>93.9–94.0</sub> | 105.1<br><sub>103.7–106.8</sub> | +11.1 |
| insights → mark, ms | 62.7<br><sub>60.6–64.2</sub> | 76.8<br><sub>62.6–79.0</sub> | 68.5<br><sub>65.3–71.8</sub> | -8.3 |
| settings → mark, ms | 49.9<br><sub>48.4–60.2</sub> | 53.1<br><sub>45.5–58.7</sub> | 52.6<br><sub>49.9–64.5</sub> | -0.5 |
| issues → mark, ms | 57.9<br><sub>53.9–63.0</sub> | 63.2<br><sub>58.0–66.3</sub> | 69.2<br><sub>69.0–72.9</sub> | +6.0 |
| board → frame, ms | 115.2<br><sub>113.3–115.5</sub> | 124.8<br><sub>124.5–134.0</sub> | 135.7<br><sub>134.4–137.6</sub> | +11.0 |
| insights → frame, ms | 77.7<br><sub>74.1–79.3</sub> | 91.9<br><sub>84.0–94.2</sub> | 83.0<br><sub>79.2–85.1</sub> | -8.9 |
| settings → frame, ms | 59.4<br><sub>57.7–70.0</sub> | 62.1<br><sub>54.6–66.5</sub> | 61.5<br><sub>58.8–73.5</sub> | -0.5 |
| issues → frame, ms | 75.2<br><sub>74.3–79.6</sub> | 78.8<br><sub>74.2–82.1</sub> | 84.9<br><sub>84.7–88.6</sub> | +6.1 |
| board: the turn that renders it, ms | 89.6<br><sub>89.0–90.7</sub> | 98.1<br><sub>97.4–98.5</sub> | 107.5<br><sub>106.7–108.3</sub> | +9.4 |
| insights: the turn that renders it, ms | 39.1<br><sub>38.0–43.9</sub> | 43.1<br><sub>40.0–43.7</sub> | 47.6<br><sub>40.5–50.4</sub> | +4.5 |
| settings: the turn that renders it, ms | 29.1<br><sub>26.8–30.7</sub> | 27.0<br><sub>24.4–28.9</sub> | 31.0<br><sub>30.5–31.5</sub> | +4.0 |
| issues: the turn that renders it, ms | 44.4<br><sub>43.6–45.7</sub> | 47.4<br><sub>47.1–49.4</sub> | 48.3<br><sub>47.9–52.0</sub> | +0.9 |

<sub>Runs: main: 3, single: 3, split, fixed: 3.</sub>

### Against the 2 ms mark

Not there. The fixed split build is within 2 ms of single on Settings and
Insights, and on Escape, where single's median sits on a fast run (its
range is 24-40 ms), but opening an issue is +2.9 ms to its mark and the
Board's turn +9.4 ms. Both are first visits: every navigation in these
scenarios opens a route for the first time since launch.

### Headless: first and second visits

`drive` on the release host, 1200x800, 1,000 issues, a cycle collection
forced before each step, each step timed whole. Median of 7 rounds
(issues) and 5 (screens), ms, on a quiet machine after the session.
"Split, before" is 1d4c8d0's build, "after" f225c91's.

| | open 1 | Escape 1 | open 2 | Escape 2 | open 3 | Escape 3 |
|---|---:|---:|---:|---:|---:|---:|
| main | 28.3 | 13.3 | 18.6 | 11.5 | 16.9 | 11.4 |
| split, before | 39.6 | 19.4 | 24.7 | 18.3 | 24.1 | 18.4 |
| split, after | 36.3 | 14.6 | 19.5 | 13.1 | 18.7 | 12.8 |
| single | 32.6 | 14.7 | 19.9 | 13.2 | 18.7 | 13.2 |

| | Board | Insights | Settings | Issues | Board again | Insights again | Settings again | Issues again |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| main | 103.5 | 24.1 | 13.1 | 23.9 | 74.9 | 21.5 | 11.6 | 23.8 |
| split, before | 120.1 | 41.1 | 19.1 | 29.5 | 90.2 | 34.0 | 15.2 | 29.2 |
| split, after | 122.1 | 30.0 | 15.3 | 25.8 | 89.4 | 23.8 | 12.3 | 25.7 |
| single | 112.0 | 25.6 | 13.6 | 25.9 | 86.6 | 22.9 | 12.2 | 25.8 |
| split, after, without SvelteKit's link scan | 111.1 | 28.8 | 15.0 | 24.6 | 79.5 | 22.6 | 12.1 | 24.4 |

The last row is a copy of the fixed build with one loop taken out (below),
from a second round of 5; the other rows of that round were within 1 ms
of these.

### Walks of the whole page per navigation

Document-wide `querySelector`, `querySelectorAll`, `getElementsByTagName`
and `getElementById` calls that walked the page, counted in instrumented
copies of each build (headless, the same steps):

| | open issue | Escape | Board | Insights | Settings | Issues |
|---|---:|---:|---:|---:|---:|---:|
| main | 2 | 2 | 2 | 2 | 2 | 2 |
| split, before | 17 | 15 | 13 | 14 | 11 | 18 |
| split, after | 1 | 1 | 2 | 2 | 2 | 2 |

Before: `meta[property=csp-nonce]` 5-9 times (Vite's preload helper; no
page has one, so each walked everything), `title`, `link` on first visits,
`body`, and SvelteKit's own two. After: SvelteKit's own two, `[autofocus]`
and `querySelectorAll('a')`; the second is answered from the index on the
issue pages (64 links or fewer) and walks on the others.

### Where the rest goes

- **Against single: loading a route on its first visit.** The second
  visits are level (within 1 ms; the Board within 3). The first visit
  reads the route's chunks (Board: one file, `nodes/7`, 36 KB; an issue:
  two) and QuickJS compiles and runs them: Board +10 ms, Insights +4,
  an issue +3.7, Settings +2. Single pays for all of it at start-up (its
  `start` mark is 15 ms later in `html-2`). Bytecode is what would make
  that cheap for both.
- **Against main, also: SvelteKit's link scan.** After every navigation
  SvelteKit looks at each `<a>` in `document.documentElement` (its URL,
  its `data-sveltekit-*` options up the ancestors) to set up preloading.
  On main the DOM layer's `<html>` was a stand-in with no children, so the
  scan found nothing; now `<html>` is the page, and the Board's 316 links
  cost 10 ms (32 µs a link), Issues and Insights 1.5 ms. It is what a
  browser does; the cost per link is the DOM layer's (`href`, attributes,
  `closest`) and can come down without touching the build.
- **Left over:** without the scan, second visits are within about 1 ms of main,
  and the Board 4.7 ms behind. Not looked into.

