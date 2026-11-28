Start-up in parallel (startup-parallel), against main, at 1,000 issues. 

Session `startup-1`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app.

### Start-up: first launch after quit (scenario 1)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 64.0<br><sub>63.4–65.3</sub> | 28.2<br><sub>28.2–28.9</sub> | -35.8 |
| first JavaScript (`start` mark), ms | 64.4<br><sub>61.1–67.4</sub> | 71.7<br><sub>71.3–71.8</sub> | +7.3 |
| window on screen, ms | 221.4<br><sub>217.3–245.2</sub> | 205.0<br><sub>199.1–208.5</sub> | -16.3 |
| first frame presented, ms | 223.1<br><sub>217.9–238.9</sub> | 202.0<br><sub>202.0–210.5</sub> | -21.1 |
| fixture read (`read` mark), ms | 232.8<br><sub>219.3–248.3</sub> | 163.9<br><sub>162.6–164.4</sub> | -68.9 |
| fixture read, after the first frame presented, ms | 9.5<br><sub>1.4–9.7</sub> | -38.1<br><sub>-47.9–-37.5</sub> | -47.6 |
| issues on the page (`mounted` mark), ms | 242.4<br><sub>229.2–258.3</sub> | 172.6<br><sub>171.5–173.5</sub> | -69.8 |
| first frame after `mounted`, ms | 259.0<br><sub>245.8–270.7</sub> | 204.6<br><sub>204.4–221.2</sub> | -54.4 |
| first key press answered (`j`), ms | 351.0<br><sub>345.6–371.0</sub> | 331.9<br><sub>326.0–344.7</sub> | -19.1 |
| RAM 5 s after load, MB | 125.3<br><sub>124.2–125.4</sub> | 124.1<br><sub>120.6–124.4</sub> | -1.1 |
| CPU time, launch to settled, ms | 348<br><sub>347–351</sub> | 363<br><sub>356–365</sub> | +16 |

<sub>Runs: main: 3, branch: 3.</sub>

The first frame presented had the issues in it: main 0 of 3, branch 3 of 3.

### Start-up: warm start (scenario 2)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 59.4<br><sub>57.7–60.5</sub> | 28.1<br><sub>28.0–29.0</sub> | -31.3 |
| first JavaScript (`start` mark), ms | 51.3<br><sub>50.7–51.6</sub> | 53.2<br><sub>51.2–54.7</sub> | +1.8 |
| window on screen, ms | 219.9<br><sub>215.4–227.9</sub> | 202.5<br><sub>179.4–205.4</sub> | -17.4 |
| first frame presented, ms | 217.3<br><sub>211.8–220.6</sub> | 193.9<br><sub>187.4–205.8</sub> | -23.4 |
| fixture read (`read` mark), ms | 222.0<br><sub>218.6–223.2</sub> | 150.0<br><sub>148.4–151.9</sub> | -71.9 |
| fixture read, after the first frame presented, ms | 1.4<br><sub>1.3–11.3</sub> | -43.9<br><sub>-53.9–-39.0</sub> | -45.2 |
| issues on the page (`mounted` mark), ms | 231.5<br><sub>228.1–232.8</sub> | 159.0<br><sub>157.2–160.8</sub> | -72.5 |
| first frame after `mounted`, ms | 247.5<br><sub>245.2–248.7</sub> | 189.5<br><sub>174.0–196.4</sub> | -58.0 |
| first key press answered (`j`), ms | 348.5<br><sub>343.4–355.0</sub> | 323.5<br><sub>315.2–325.4</sub> | -25.1 |
| RAM 5 s after load, MB | 124.3<br><sub>124.1–124.9</sub> | 124.6<br><sub>120.6–125.3</sub> | +0.3 |
| CPU time, launch to settled, ms | 342<br><sub>338–344</sub> | 345<br><sub>344–348</sub> | +2 |

<sub>Runs: main: 3, branch: 3.</sub>

The first frame presented had the issues in it: main 0 of 3, branch 3 of 3.

### Idle, 20 s (scenario 3)

| | main | branch | branch − main |
|---|---:|---:|---:|
| CPU time, ms | 10.8<br><sub>9.9–11.5</sub> | 9.8<br><sub>9.7–11.5</sub> | -0.9 |
| GPU time, ms | 0.00 | 0.00 | +0.00 |
| frames presented | 0 | 0 | +0 |
| interrupt wake-ups | 34<br><sub>32–35</sub> | 31<br><sub>31–33</sub> | -3 |
| RAM 5 s after load, MB | 122.5<br><sub>121.7–122.6</sub> | 106.4<br><sub>105.6–107.1</sub> | -16.1 |
| RAM at the end, MB | 116.4<br><sub>115.7–116.4</sub> | 98.1<br><sub>97.3–98.8</sub> | -18.3 |

<sub>Runs: main: 3, branch: 3.</sub>

### Notes

- **The window is on screen 16-17 ms sooner, its first frame 21-23 ms
  sooner, and that frame has the issues**: 3 of 3 runs in each scenario
  against 0 of 3 on main. The issues are on the page 70-72 ms sooner,
  before the window is shown; the first key press is answered 19-25 ms
  sooner.
- **"The page started" no longer compares like for like.** It is timed
  from the document's creation to the page settled, and on the branch the
  system's fonts are listed before that, on a thread, while the event loop
  is made: 28 against 64 ms says where the fonts went, not that the page
  starts 36 ms sooner. The first JavaScript (`start`) runs 2-7 ms later,
  as the event loop is now made before the page.
- **First frame presented** is the first full repaint from 10 ms before
  the window is on screen. Repaints before that are drawn while macOS has
  not shown the window and are dropped (the surface is occluded). The
  branch measured here (the first form of its last change) painted two of
  those, ~150 and ~163 ms from exec; the final form paints no more of them
  than main (below).
- **CPU from launch to settled** is 16 ms higher in scenario 1 and 2 ms
  in scenario 2. With the probe launcher, the process's CPU over its first
  3 s was 225.7 ms on main and 229.9 on the final branch (10 launches
  each), and the same for both forms of change 4. Not broken down.
- **Idle** is the same: no frames, 31-34 wake-ups in 20 s.
- **RAM** 5 s after load is the same in scenarios 1 and 2. The idle runs
  show 16 MB less on the branch, but main's idle RAM was 105-107 MB in
  `nav-1` and `alloc-1`: this session's 122 MB on main is the odd one. 15-16
  MB is about one drawable (2400 x 1600 x 4 bytes), which the surface keeps
  or does not from launch to launch. The branch does not lower RAM.
- **Host size:** 18,784,576 bytes packaged (dist, `screenshot`), 33,200
  more than main.

### Each change alone

Headless timings cannot see a window, so each change was measured with
start-up probes in a dist host (`[probe]` lines: wall time and the UI
thread's CPU time) and the bench Tracker bundled, 5 or 10 launches each,
alternating with the change before it. ms from exec, medians:

| change | window visible | first frame | issues on the page | also |
|---|---:|---:|---:|---|
| 1. fonts listed on a thread while the event loop is made | 253 -> 228 | 260 -> 236 | | |
| 2. the Han fallback list asked for on that thread | 225 -> 222 | | | window and first layout 56.8 -> 52.7 |
| 3. GPU device and pipelines on a thread | 217 -> 197 | 226 -> 204 | 211 -> 192 | window and first layout 51.4 -> 35.1 |
| 4. the first poll without ResumeReady | 200 -> 204 | 209 -> 212 | 194 -> 165 | issues 37 ms before the window, not 5 |

Change 4's two forms, alternating, 10 launches each: the first, which
called blitz-shell's `complete_resume` early, rendered 3 (2-3) frames
before the window was visible, all dropped; the final one, which polls the
document with a waker of its own and leaves the first paint to
blitz-shell, renders 2 (1-2), and in a batch against main 1 (0-2), as main
does (1, 0-2). Window, first frame, issues and CPU were the same for both
forms. The final branch against main in that batch: window visible 242.6
-> 205.5, first frame 253.6 -> 214.0, issues 233.1 -> 166.1, the first
frame with the issues 7 of 10 -> 10 of 10.

### The first layout

The first style and layout, made when the window is, took 20-24 ms:
style 1.1-1.6, layout 0.2-0.3, and building the layout's boxes and text
(`construct`) 18-22. Of that, font work was ~5 ms: the Han fallback list
(4.3 ms, now asked for on the font thread), 390 family lookups and 391 font
files (~0.1 ms), 2,261 font matches (~0.9 ms). The 192 inline layouts took
under 0.12 ms each. The other ~14 ms of `construct` is not font work and
was not traced.

