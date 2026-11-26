SvelteKit's split build against one bundle, both loaded from index.html, and main, at 1,000 issues. 

Session `html-2`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app. Frames are those presented (damage tracking drew all or part of the window). All three run the same app; split and single run the same host.

### Start-up: first launch after quit (scenario 1)

| | main | split | single |
|---|---:|---:|---:|
| first JavaScript (`start` mark), ms | 86.2<br><sub>85.6–89.7</sub> | 76.7<br><sub>76.2–77.0</sub> | 91.4<br><sub>90.1–91.7</sub> |
| window on screen, ms | 268.9<br><sub>255.1–271.1</sub> | 248.8<br><sub>246.6–264.2</sub> | 259.7<br><sub>259.7–271.2</sub> |
| issues on the page (`mounted` mark), ms | 268.1<br><sub>225.9–270.7</sub> | 268.2<br><sub>239.3–268.8</sub> | 286.5<br><sub>273.8–286.7</sub> |
| first frame after `mounted`, ms | 283.5<br><sub>233.6–286.2</sub> | 280.5<br><sub>246.6–290.3</sub> | 297.1<br><sub>289.8–297.6</sub> |
| first key press answered (`j`), ms | 391.5<br><sub>378.0–392.1</sub> | 375.3<br><sub>369.1–391.0</sub> | 383.4<br><sub>382.8–392.6</sub> |
| RAM 5 s after load, MB | 116.7<br><sub>116.0–117.1</sub> | 111.2<br><sub>108.7–111.3</sub> | 116.5<br><sub>116.5–116.8</sub> |
| CPU time, launch to settled, ms | 366<br><sub>361–366</sub> | 371<br><sub>370–414</sub> | 374<br><sub>374–377</sub> |

<sub>Runs: main: 3, split: 3, single: 3.</sub>

### Start-up: warm start (scenario 2)

| | main | split | single |
|---|---:|---:|---:|
| first JavaScript (`start` mark), ms | 70.8<br><sub>70.3–71.0</sub> | 62.1<br><sub>62.0–63.2</sub> | 76.6<br><sub>73.2–76.7</sub> |
| window on screen, ms | 216.1<br><sub>214.3–220.9</sub> | 218.0<br><sub>214.4–236.6</sub> | 219.8<br><sub>216.3–242.0</sub> |
| issues on the page (`mounted` mark), ms | 235.9<br><sub>225.7–238.4</sub> | 229.0<br><sub>211.6–236.1</sub> | 235.0<br><sub>215.9–241.4</sub> |
| first frame after `mounted`, ms | 247.5<br><sub>241.4–249.5</sub> | 244.9<br><sub>219.1–247.2</sub> | 251.0<br><sub>223.4–252.8</sub> |
| first key press answered (`j`), ms | 333.4<br><sub>333.4–336.2</sub> | 341.7<br><sub>334.6–365.3</sub> | 340.9<br><sub>334.5–371.0</sub> |
| RAM 5 s after load, MB | 116.0<br><sub>115.5–116.3</sub> | 111.0<br><sub>107.6–112.6</sub> | 116.6<br><sub>116.1–117.3</sub> |
| CPU time, launch to settled, ms | 354<br><sub>353–359</sub> | 356<br><sub>353–360</sub> | 361<br><sub>359–366</sub> |

<sub>Runs: main: 3, split: 3, single: 3.</sub>

### Open an issue (scenario 5)

| | main | split | single |
|---|---:|---:|---:|
| click row → mark, ms | 19.0<br><sub>18.9–20.2</sub> | 28.7<br><sub>27.8–28.8</sub> | 23.3<br><sub>23.0–24.6</sub> |
| click row → frame, ms | 31.8<br><sub>31.5–32.6</sub> | 41.4<br><sub>40.4–41.5</sub> | 36.1<br><sub>35.2–37.2</sub> |
| Escape → mark, ms | 33.3<br><sub>22.2–39.3</sub> | 50.9<br><sub>49.7–51.7</sub> | 38.8<br><sub>37.7–39.1</sub> |
| Escape → frame, ms | 49.1<br><sub>34.2–54.0</sub> | 66.5<br><sub>65.9–70.8</sub> | 54.0<br><sub>53.5–54.8</sub> |

<sub>Runs: main: 3, split: 3, single: 3.</sub>

### Switch screens (scenario 10)

| | main | split | single |
|---|---:|---:|---:|
| board → mark, ms | 75.6<br><sub>74.9–84.4</sub> | 99.5<br><sub>96.4–105.0</sub> | 99.7<br><sub>97.5–101.8</sub> |
| insights → mark, ms | 59.6<br><sub>56.8–61.6</sub> | 77.7<br><sub>75.0–84.1</sub> | 56.5<br><sub>50.1–57.0</sub> |
| settings → mark, ms | 47.0<br><sub>43.8–48.3</sub> | 55.9<br><sub>52.7–60.6</sub> | 50.9<br><sub>42.1–89.6</sub> |
| issues → mark, ms | 63.8<br><sub>63.3–64.0</sub> | 73.3<br><sub>71.8–74.3</sub> | 64.8<br><sub>44.9–83.9</sub> |
| board → frame, ms | 111.3<br><sub>111.3–120.6</sub> | 129.2<br><sub>126.8–136.1</sub> | 130.4<br><sub>127.1–139.5</sub> |
| insights → frame, ms | 74.5<br><sub>70.5–76.9</sub> | 91.2<br><sub>88.6–96.4</sub> | 72.0<br><sub>66.9–72.0</sub> |
| settings → frame, ms | 58.3<br><sub>53.1–61.7</sub> | 63.9<br><sub>60.3–68.6</sub> | 59.8<br><sub>52.1–99.2</sub> |
| issues → frame, ms | 80.2<br><sub>80.0–80.2</sub> | 88.7<br><sub>87.4–88.8</sub> | 81.0<br><sub>60.9–99.5</sub> |

<sub>Runs: main: 3, split: 3, single: 3.</sub>

### RAM 5 s after load, every run (scenarios 1, 2, 5 and 10)

| | main | split | single |
|---|---:|---:|---:|
| RAM, MB | 114.3<br><sub>97.3–117.1</sub> | 108.6<br><sub>93.4–112.6</sub> | 115.1<br><sub>95.2–117.3</sub> |

<sub>Runs: main: 12, split: 12, single: 12.</sub>

### Notes

- **The start-up rows are valid.** With Stage Manager off, every window
  was on screen at 214-271 ms, and none waited for the harness to
  activate it.
- **Start-up.** Single starts its JavaScript 15 ms after split (one
  570 KB module is parsed before its first line runs; split parses the
  entry and its imports module by module) and 5 ms after main. On a
  first launch the issues are on the page 18 ms after split and main; on
  a warm start the three are within each other's ranges. CPU time from
  launch to settled is the same within 10 ms.
- **Memory.** Split holds 5 MB less 5 s after load (111 against 116.5);
  single holds what main holds. Most likely that is the code of the
  routes not opened yet, which one bundle loads and compiles at start-up.
- **Navigation.** Single takes back most of what split loses: opening an
  issue is 23.3 ms to its mark (split 28.7, main 19.0), Escape 38.8 (50.9,
  33.3), and Insights, Settings and the way back to Issues come out as
  main's, give or take 4 ms.
- **The Board's mark row is misleading.** Single's Board → mark (99.7)
  equals split's, but the turn that renders the Board, read from the
  window trace, is 96.4 ms for single, 103.9 for split and 87.1 for main
  (the same order headless: 112, 119, 104). Single sets its mark later
  inside that turn (73 ms in, against 58 on main), and the mark row
  counts that as more time. What is left between single and main is
  roughly in proportion to the work: 1.5-2.5 ms on a small navigation,
  8-9 ms on the Board, whose turn is 20,000 ops. Not looked into.

### Escape back to the list: the 325 ops

In session `html-1` the Escape turn had 1,417 ops on the branch against
1,092 on main. Logged headless (`drive`, the same steps, every batch the
host applied), the two turns differ in one op kind only: 350 `DROP`
against 25. Every other kind, `CREATE_*`, `SET_*`, `INSERT`, `REMOVE`,
`LISTEN`, comes to the same count; the only difference in targets is that
the list goes back into the parsed mount node (`page#61`) instead of
main's root (`#1`). The 325 extra `DROP`s are the first list's nodes
(141 text nodes, 87 comments, 76 spans, the icons) freed by a QuickJS
cycle collection that the branch's allocations bring forward into that
turn; on main it falls in a later one (headless, in the second open). It
costs that turn about 5 ms: with a collection forced before each step,
Escape is 19.6 ms instead of 24.5. A shift in where the collection lands,
not more work.

### Opening the same issue twice, headless

`drive` on the release host, 1200x800, 1,000 issues: open the first row,
Escape, three times; each step timed whole (the click, the frame and the
timer SvelteKit waits for, settled). Median of 7 rounds, with a cycle
collection forced before each step so that it does not land in one of
them. ms:

| | open 1 | Escape 1 | open 2 | Escape 2 | open 3 | Escape 3 |
|---|---:|---:|---:|---:|---:|---:|
| main | 29.4 | 13.4 | 18.3 | 12.1 | 17.6 | 11.9 |
| split | 41.6 | 19.6 | 25.9 | 19.0 | 24.9 | 18.7 |
| single | 33.9 | 15.1 | 20.6 | 13.7 | 19.6 | 13.6 |
| split, nonce lookup taken out | 35.7 | 16.2 | 22.9 | 15.6 | 22.1 | 15.4 |
| split, and the URL parses cached | 37.4 | 14.7 | 20.6 | 14.0 | 19.8 | 13.8 |

- **Compiling the route is 2-4 ms of the first visit.** The first open
  costs split 15.7 ms more than the second, and single 13.3 ms more
  (main 11.1: the first render of the issue page, whatever the build).
  The difference, 2.4 ms, is reading and compiling the route's chunks;
  against the last row (split, helper patched) it is 3.5 ms.
- **The rest of the gap is paid on every visit, the second too:** split
  is about 5 ms slower than single on each navigation. It is Vite's module
  preload helper (`__vitePreload`), which SvelteKit's split build calls
  for every route node on every navigation, loaded or not: it looks up
  `meta[property=csp-nonce]` with `document.querySelector` (a walk of the
  whole tree in the DOM layer: 0.39 ms a call on the issues page), and parses two `URL`s per dependency (8 to 25 per node)
  before its cache of what it has seen. In a copy of the split build with
  the nonce lookup taken out, a navigation is about 3 ms faster; with the
  URL parses cached as well, it is as fast as single. Bytecode would not
  remove this; a faster `querySelector` for that selector in the DOM
  layer, or `sepal()` replacing the helper, would.
- **Single is still 1.5-2.5 ms slower than main per navigation.** Not
  looked into.

### The app and the host

| | main | split | single |
|---|---:|---:|---:|
| app folder (the bench Tracker): files | 13 | 60 | 15 |
| app folder: bytes in the files | 871,912 | 845,442 | 873,261 |
| app folder: on disk (`du -sk`) | 880 KB | 972 KB | 892 KB |
| files the app reads to show its first screen | 9 | 46 | 10 |

Single: `index.html`, `app.json`, the icon, then from the page one
JavaScript module (`bundle.*.js`, 570 KB), one stylesheet and the same 5
fonts. Counted with `drive … files:` after the first layout.

Release host (`calyx`, `--features screenshot`, thin LTO, stripped),
bytes, from the release workflow: the published 0.2.0-alpha.0, and the
dry run on html-entry. Split and single use the same host.

| Platform | 0.2.0-alpha.0 (main) | html-entry | |
|---|---:|---:|---|
| darwin-arm64 | 25,493,936 | 26,007,952 | +514,016 (+2.0 %) |
| darwin-x64 | 26,499,360 | 27,303,208 | +803,848 (+3.0 %) |
| linux-arm64-gnu | 29,766,944 | 30,496,032 | +729,088 (+2.4 %), now over 30,000,000 (29.1 MiB) |
| linux-x64-gnu | 32,608,272 | 33,530,736 | +922,464 (+2.8 %), over before this branch |

