The app as a web build, loaded from index.html: Sepal before and after, and Tauri 2, at 1,000 issues. 

Session `html-1`, interleaved, 1 warm-up and 3 runs per build and scenario. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app. Frames are those presented (damage tracking drew all or part of the window); Tauri has no frame trace.

### Start-up: first launch after quit (scenario 1)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| first JavaScript (`start` mark), ms | 93.5<br><sub>84.9–94.0</sub> | 81.2<br><sub>74.1–83.3</sub> | 264.0<br><sub>256.7–290.4</sub> |
| window on screen, ms | 261.5<br><sub>249.2–286.5</sub> | 277.8<br><sub>255.4–296.8</sub> | 5,248.9<br><sub>200.3–5,262.9</sub> |
| issues on the page (`mounted` mark), ms | 248.5<br><sub>240.0–286.1</sub> | 269.2<br><sub>248.7–281.3</sub> | 274.5<br><sub>267.0–294.5</sub> |
| first frame after `mounted`, ms | 266.1<br><sub>247.1–296.6</sub> | 295.6<br><sub>255.7–318.4</sub> | 5,121.1<br><sub>367.3–5,139.5</sub> |
| first key press answered (`j`), ms | 422.8<br><sub>404.6–467.9</sub> | 446.8<br><sub>387.9–478.3</sub> | 5,911.4<br><sub>386.6–5,932.8</sub> |
| RAM 5 s after load, MB | 120.7<br><sub>115.5–120.8</sub> | 112.9<br><sub>111.5–116.2</sub> | 138.8<br><sub>138.1–152.1</sub> |
| GPU memory 5 s after load, MB | 44.3<br><sub>43.4–46.0</sub> | 44.4<br><sub>44.4–46.0</sub> | 7.1<br><sub>6.0–7.1</sub> |
| CPU time, launch to settled, ms | 438<br><sub>428–446</sub> | 431<br><sub>427–438</sub> | 737<br><sub>562–770</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Start-up: warm start (scenario 2)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| first JavaScript (`start` mark), ms | 77.8<br><sub>74.6–78.8</sub> | 69.2<br><sub>67.3–70.1</sub> | 283.8<br><sub>254.8–319.9</sub> |
| window on screen, ms | 263.2<br><sub>253.9–281.7</sub> | 249.1<br><sub>244.4–309.0</sub> | 5,233.4<br><sub>231.9–5,235.5</sub> |
| issues on the page (`mounted` mark), ms | 246.7<br><sub>233.6–287.4</sub> | 266.1<br><sub>254.9–307.3</sub> | 295.9<br><sub>265.5–355.9</sub> |
| first frame after `mounted`, ms | 267.7<br><sub>241.1–304.0</sub> | 290.4<br><sub>286.1–329.1</sub> | 5,130.0<br><sub>384.6–5,130.0</sub> |
| first key press answered (`j`), ms | 428.7<br><sub>400.3–435.5</sub> | 420.8<br><sub>382.9–469.5</sub> | 5,889.0<br><sub>416.3–5,937.4</sub> |
| RAM 5 s after load, MB | 116.8<br><sub>116.2–116.9</sub> | 112.0<br><sub>111.9–115.8</sub> | 151.6<br><sub>138.9–186.9</sub> |
| GPU memory 5 s after load, MB | 44.2<br><sub>44.2–44.4</sub> | 44.2<br><sub>44.2–44.5</sub> | 7.3<br><sub>6.5–55.3</sub> |
| CPU time, launch to settled, ms | 431<br><sub>423–433</sub> | 434<br><sub>433–445</sub> | 747<br><sub>553–757</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Idle, 20 s (scenario 3)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| CPU time, ms | 11.6<br><sub>9.3–13.9</sub> | 12.2<br><sub>10.0–51.4</sub> | 22.4<br><sub>20.4–32.0</sub> |
| GPU time, ms | 0.00 | 0.00 | 0.00 |
| frames presented | 0 | 0 | n/a |
| interrupt wake-ups | 34 | 35<br><sub>34–40</sub> | 176<br><sub>174–228</sub> |
| RAM 5 s after load, MB | 96.7<br><sub>95.9–102.1</sub> | 111.2<br><sub>96.9–114.5</sub> | 149.3<br><sub>149.1–266.8</sub> |
| RAM at the end, MB | 90.8<br><sub>90.0–94.1</sub> | 105.2<br><sub>88.8–106.5</sub> | 137.2<br><sub>125.3–137.2</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Open an issue (scenario 5)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| click row → mark, ms | 19.3<br><sub>19.3–20.1</sub> | 28.4<br><sub>28.1–29.2</sub> | 13.9<br><sub>13.5–24.3</sub> |
| click row → frame, ms | 31.9<br><sub>31.3–32.0</sub> | 40.2<br><sub>40.1–41.8</sub> | 23.9<br><sub>23.4–36.6</sub> |
| Escape → mark, ms | 32.5<br><sub>29.4–37.6</sub> | 50.4<br><sub>33.9–51.1</sub> | 30.5<br><sub>27.1–34.0</sub> |
| Escape → frame, ms | 48.0<br><sub>44.5–52.5</sub> | 65.1<br><sub>46.5–70.2</sub> | 53.3<br><sub>44.0–56.7</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Switch screens (scenario 10)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| board → mark, ms | 82.4<br><sub>81.2–85.7</sub> | 107.1<br><sub>97.7–108.1</sub> | 50.1<br><sub>42.4–54.3</sub> |
| insights → mark, ms | 59.5<br><sub>58.7–65.1</sub> | 71.1<br><sub>70.8–77.1</sub> | 38.4<br><sub>35.5–44.9</sub> |
| settings → mark, ms | 52.6<br><sub>43.6–53.1</sub> | 54.6<br><sub>40.3–61.0</sub> | 30.0<br><sub>24.7–35.8</sub> |
| issues → mark, ms | 60.7<br><sub>56.0–61.6</sub> | 65.4<br><sub>64.5–69.8</sub> | 31.5<br><sub>28.5–37.5</sub> |
| board → frame, ms | 119.5<br><sub>118.2–122.1</sub> | 137.5<br><sub>136.7–139.1</sub> | 70.2<br><sub>64.4–74.4</sub> |
| insights → frame, ms | 74.1<br><sub>72.4–80.2</sub> | 89.9<br><sub>84.2–91.1</sub> | 52.1<br><sub>48.7–57.9</sub> |
| settings → frame, ms | 61.8<br><sub>52.7–62.5</sub> | 63.4<br><sub>48.0–69.4</sub> | 41.3<br><sub>36.3–47.0</sub> |
| issues → frame, ms | 77.0<br><sub>72.5–78.0</sub> | 81.0<br><sub>80.1–85.9</sub> | 45.4<br><sub>41.6–51.6</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### RAM 5 s after load, all start-up and idle runs (scenarios 1, 2 and 3)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| RAM, MB | 116.2<br><sub>95.9–120.8</sub> | 112.0<br><sub>96.9–116.2</sub> | 149.3<br><sub>138.1–266.8</sub> |

<sub>Runs: Sepal, before: 9, Sepal, after: 9, Tauri 2: 9.</sub>

### Notes

- **Tauri's window rows are not valid in this session.** Stage Manager kept
  the rebuilt Tauri app's window off screen until the harness activated it
  after 5 s (12 of its 20 runs; 1 of Sepal's, outside the start-up rows).
  That is the window, first frame and first key rows; its `mounted` mark,
  memory and CPU are unaffected. In session `sk-1` its window was on
  screen at about 200 ms.
- **Start-up.** JavaScript starts 9-12 ms sooner (no 1 MB script to parse
  before the first line runs), and the issues are on the page 20 ms later:
  36 modules are read and compiled instead of one. Without bytecode
  (the next job), each is parsed by QuickJS from source.
- **Navigation pays for what is loaded late.** Opening an issue is 9 ms
  slower to its mark: its route's chunk is read and compiled on the first
  open (2 more files), where the single bundle paid for it at start-up. In
  the window trace the navigation turn is 30.5 ms against 20.4, with the same
  number of ops. The Board and Insights, opened for the first time in
  scenario 10, are 25 and 12 ms slower; Settings and the way back to
  Issues, whose code is mostly loaded already, are 2 and 5 ms slower. Escape back to the list is 18 ms slower
  at the median, with 1,417 ops against 1,092 in its turn; that difference in
  ops is not explained yet.
- **Idle** is the same: no frames, the same wake-ups.
- **Memory.** RAM 5 s after load is about 4 MB lower over all start-up and
  idle runs (one script's source and its compiled code no longer held
  whole); single scenarios scatter by 15-25 MB, depending on whether the
  kept frame and depth buffer had been let go yet (CLAUDE.md, "Where memory
  goes in a window").

### The app and the host

Measured on 25 Nov 2026, the builds above.

| | Sepal, before | Sepal, after |
|---|---:|---:|
| app folder (the bench Tracker): files | 13 | 60 |
| app folder: bytes in the files | 871,907 | 845,436 |
| app folder: on disk (`du -sk`) | 880 KB | 972 KB |
| files the app reads to show its first screen | 9 | 46 |

Before: `app.js`, `app.css`, `app.json`, the icon and 5 fonts. After:
`index.html`, `app.json`, the icon, then 43 from the page: 36 JavaScript
modules (of 45 chunks), 2 stylesheets and the same 5 fonts. Counted with
`drive … files:` after the first layout, and on main with a log line in a
throwaway build. More bytes on disk for fewer bytes of files: 60 files of
4 KB blocks.

Release host (`calyx`, `--features screenshot`, thin LTO, stripped), bytes:

| Platform | 0.2.0-alpha.0 (main) | html-entry | |
|---|---:|---:|---|
| darwin-arm64, built here | 25,437,648 | 26,102,336 | +664,688 (+2.6 %) |
| darwin-arm64, release CI | 25,493,936 | | |
| darwin-x64, release CI | 26,499,360 | | |
| linux-arm64-gnu, release CI | 29,766,944 | | |
| linux-x64-gnu, release CI | 32,608,272 | | over the 30 MB budget before this branch |

The release CI column is the published 0.2.0-alpha.0. The branch's other
platforms come from the release workflow's dry run once the branch is
pushed; the difference should be about the same 0.66 MB (html5ever and the
QuickJS module loader), which keeps darwin and linux-arm64 under 30 MB.

