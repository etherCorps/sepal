QuickJS-ng's Map and Set hash patched, against main, at 1,000 issues. 

Session `alloc-1`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app (start-up) or from the input (navigation).

### Start-up: first launch after quit (scenario 1)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 63.2<br><sub>63.0–64.1</sub> | 63.4<br><sub>63.3–64.9</sub> | +0.2 |
| first JavaScript (`start` mark), ms | 62.7<br><sub>62.4–65.1</sub> | 63.5<br><sub>63.0–69.0</sub> | +0.8 |
| window on screen, ms | 219.6<br><sub>219.1–225.3</sub> | 230.5<br><sub>226.1–240.0</sub> | +10.8 |
| issues on the page (`mounted` mark), ms | 231.0<br><sub>227.0–256.8</sub> | 251.5<br><sub>224.0–252.6</sub> | +20.5 |
| fixture loaded → issues on the page, ms | 8.1<br><sub>7.7–8.4</sub> | 7.4<br><sub>7.3–7.5</sub> | -0.7 |
| first frame after `mounted`, ms | 242.6<br><sub>237.8–268.0</sub> | 262.2<br><sub>239.4–263.9</sub> | +19.6 |
| first key press answered (`j`), ms | 335.8<br><sub>332.9–340.1</sub> | 349.6<br><sub>342.9–357.0</sub> | +13.9 |
| RAM 5 s after load, MB | 123.8<br><sub>120.3–124.2</sub> | 124.7<br><sub>120.5–125.0</sub> | +0.9 |
| CPU time, launch to settled, ms | 343<br><sub>343–346</sub> | 347<br><sub>343–348</sub> | +4 |

<sub>Runs: main: 3, branch: 3.</sub>

### Start-up: warm start (scenario 2)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 56.7<br><sub>55.2–56.7</sub> | 57.4<br><sub>57.0–58.4</sub> | +0.7 |
| first JavaScript (`start` mark), ms | 49.5<br><sub>48.7–50.1</sub> | 50.7<br><sub>50.7–51.3</sub> | +1.2 |
| window on screen, ms | 199.4<br><sub>196.5–199.4</sub> | 196.5<br><sub>195.9–196.5</sub> | -2.9 |
| issues on the page (`mounted` mark), ms | 215.8<br><sub>205.9–221.9</sub> | 208.6<br><sub>207.1–212.5</sub> | -7.2 |
| fixture loaded → issues on the page, ms | 8.0<br><sub>7.9–8.3</sub> | 7.2<br><sub>7.2–7.5</sub> | -0.8 |
| first frame after `mounted`, ms | 231.2<br><sub>221.3–233.0</sub> | 223.9<br><sub>222.6–227.8</sub> | -7.3 |
| first key press answered (`j`), ms | 320.1<br><sub>312.0–320.7</sub> | 311.7<br><sub>310.4–313.2</sub> | -8.4 |
| RAM 5 s after load, MB | 124.1<br><sub>123.8–125.2</sub> | 124.1<br><sub>124.0–124.5</sub> | +0.0 |
| CPU time, launch to settled, ms | 334<br><sub>333–339</sub> | 332<br><sub>332–338</sub> | -2 |

<sub>Runs: main: 3, branch: 3.</sub>

### Idle, 20 s (scenario 3)

| | main | branch | branch − main |
|---|---:|---:|---:|
| CPU time, ms | 10.4<br><sub>10.3–10.5</sub> | 9.5<br><sub>7.7–10.6</sub> | -1.0 |
| GPU time, ms | 0.00 | 0.00 | +0.00 |
| frames presented | 0 | 0 | +0 |
| interrupt wake-ups | 33<br><sub>32–34</sub> | 34<br><sub>33–34</sub> | +1 |
| RAM 5 s after load, MB | 106.5<br><sub>103.0–121.7</sub> | 107.4<br><sub>105.5–122.1</sub> | +0.8 |
| RAM at the end, MB | 98.3<br><sub>96.8–115.6</sub> | 99.2<br><sub>97.4–115.9</sub> | +0.9 |

<sub>Runs: main: 3, branch: 3.</sub>

### Open an issue (scenario 5)

| | main | branch | branch − main |
|---|---:|---:|---:|
| click row → mark, ms | 25.0<br><sub>23.2–25.6</sub> | 22.5<br><sub>22.5–23.3</sub> | -2.5 |
| click row → frame, ms | 37.1<br><sub>35.0–37.9</sub> | 34.5<br><sub>34.5–34.5</sub> | -2.5 |
| Escape → mark, ms | 38.9<br><sub>35.0–40.9</sub> | 35.5<br><sub>34.4–36.9</sub> | -3.4 |
| Escape → frame, ms | 53.5<br><sub>49.5–55.6</sub> | 51.5<br><sub>49.9–51.7</sub> | -2.0 |
| click row: the turn that renders it, ms | 24.7<br><sub>24.7–25.1</sub> | 23.7<br><sub>23.4–23.9</sub> | -1.0 |
| Escape: the key press that renders it, ms | 23.2<br><sub>22.4–23.2</sub> | 24.4<br><sub>22.0–25.1</sub> | +1.2 |

<sub>Runs: main: 3, branch: 3.</sub>

### Switch screens (scenario 10)

| | main | branch | branch − main |
|---|---:|---:|---:|
| board → mark, ms | 101.2<br><sub>95.3–102.8</sub> | 78.8<br><sub>76.5–79.2</sub> | -22.3 |
| insights → mark, ms | 64.7<br><sub>55.2–81.5</sub> | 51.0<br><sub>49.1–64.1</sub> | -13.8 |
| settings → mark, ms | 47.8<br><sub>46.2–55.4</sub> | 47.3<br><sub>42.9–52.9</sub> | -0.6 |
| issues → mark, ms | 57.8<br><sub>57.5–59.8</sub> | 61.7<br><sub>54.4–63.6</sub> | +3.8 |
| board → frame, ms | 130.5<br><sub>124.0–131.6</sub> | 108.1<br><sub>104.1–108.4</sub> | -22.4 |
| insights → frame, ms | 79.5<br><sub>69.6–96.2</sub> | 65.2<br><sub>61.8–79.2</sub> | -14.3 |
| settings → frame, ms | 56.6<br><sub>54.8–64.1</sub> | 56.1<br><sub>51.6–61.6</sub> | -0.5 |
| issues → frame, ms | 74.4<br><sub>74.0–78.2</sub> | 77.5<br><sub>70.1–79.3</sub> | +3.1 |
| board: the turn that renders it, ms | 100.1<br><sub>99.0–100.9</sub> | 78.9<br><sub>77.6–79.0</sub> | -21.2 |
| insights: the turn that renders it, ms | 44.6<br><sub>44.1–46.8</sub> | 38.7<br><sub>35.9–42.6</sub> | -5.9 |
| settings: the turn that renders it, ms | 29.0<br><sub>27.9–31.5</sub> | 28.1<br><sub>26.6–28.2</sub> | -0.9 |
| issues: the turn that renders it, ms | 47.9<br><sub>47.2–49.3</sub> | 43.0<br><sub>40.3–47.4</sub> | -4.9 |

<sub>Runs: main: 3, branch: 3.</sub>

### RAM 5 s after load, all start-up and idle runs (scenarios 1, 2 and 3)

| | main | branch | branch − main |
|---|---:|---:|---:|
| RAM, MB | 123.8<br><sub>103.0–125.2</sub> | 124.0<br><sub>105.5–125.0</sub> | +0.2 |

<sub>Runs: main: 9, branch: 9.</sub>

### Notes

- **Screens drawn for the first time are where it shows.** The turn that
  renders the Board is 21 ms shorter (100 to 79 ms), Insights' 6 ms and
  Issues' 5 ms; to the mark and to the frame the Board is 22 ms sooner and
  Insights 14. Settings has few nodes and moves by less than 1 ms. Those
  turns make thousands of nodes, and the renderer keys its node maps
  (`elements`, `by_tag`) by node id: ids that only go up, which QuickJS-ng
  0.16.2's hash sent to a few buckets (fixed upstream in #1931, not yet
  released).
- **Opening an issue** is 2.5 ms sooner to its mark and its frame, and its
  render turn 1 ms shorter. Escape's key press is within its range.
- **Start-up is the same.** The JavaScript that renders the list, from the
  fixture's `loaded` mark to `mounted`, is 7.3-7.5 ms on the branch against
  7.7-8.4 on main. On a first launch `loaded` comes at either ~220 or ~245
  ms, on both builds (the fixture is read on a thread): two of the
  branch's three runs and one of main's came late, which is all the
  +20 ms on `mounted` is. Warm starts are 7 ms sooner, within the ranges.
- **Idle** is the same: no frames, 33 and 34 wake-ups in 20 s.
- **Memory and size.** RAM 5 s after load is within 1 MB over the nine
  start-up and idle runs. The host is the same size: 18,751,376 bytes
  packaged (dist, `screenshot`), 18,387,184 as measured here (dist), on
  both.

### Headless, the same change

`drive`'s `eval:` on the release host, median of three launches. One
round: a new Map that sets, gets, tests and deletes 10,000 keys, then a
new Set that adds and tests them. ms a round:

| keys | before | after |
|---|---:|---:|
| ints 1 to 10,000 | 91.5 | 3.35 |
| small BigInts | 94.2 | 3.30 |
| floats (i + 0.5) | 19.9 | 3.45 |
| objects | 21.8 | 3.25 |
| symbols | 14.1 | 3.25 |
| ints from 1,000,000 | 2.55 | 3.40 |
| strings (unchanged) | 3.10 | 3.20 |

Ints from a million were spread well by luck (their doubles' high words
change with every key) and now cost what strings do.

The bench Tracker's Board at 1,000 issues, 1200x800, five launches each,
a cycle collection before each timed step:

| ms | first visit | repeat |
|---|---:|---:|
| before | 137-147 | 79 |
| after | 113-123 | 73-76 |

### What remains of a first visit

The Board's first visit still costs ~40 ms over a repeat. With counters
added to QuickJS for the purpose (a throwaway build), the first visit and
the repeat do the same work within 9%: 85k against 78k objects, 401k
against 369k mallocs, 17.8k against 16.7k shapes. QuickJS's other hash
tables are not the cause: shapes and property tables take their bucket
from the top bits of a multiplied hash, atoms hash every character into
the low bits, and each averages about one probe a lookup (Maps, patched,
1.3). The first visit runs two automatic collections (8-11 ms inside
them) where the repeat runs none, but with automatic collection off it is
only 3-6 ms quicker. The rest is the same operations costing more the
first time, which these counters do not explain; it is left for its own
job.

