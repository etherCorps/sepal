The Tracker's modules loaded from QuickJS bytecode, against main, at 1,000 issues. 

Session `bc-1`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app (start-up) or from the input (navigation).

### Start-up: first launch after quit (scenario 1)

| | main | bytecode | bytecode − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 82.2<br><sub>79.6–82.8</sub> | 61.9<br><sub>61.7–67.5</sub> | -20.3 |
| first JavaScript (`start` mark), ms | 77.4<br><sub>72.8–77.9</sub> | 54.3<br><sub>51.6–63.2</sub> | -23.0 |
| window on screen, ms | 237.5<br><sub>224.0–261.5</sub> | 229.0<br><sub>219.4–247.5</sub> | -8.6 |
| issues on the page (`mounted` mark), ms | 249.8<br><sub>229.5–253.7</sub> | 243.3<br><sub>239.3–245.7</sub> | -6.6 |
| first frame after `mounted`, ms | 269.3<br><sub>236.6–269.9</sub> | 259.9<br><sub>257.0–266.4</sub> | -9.4 |
| first key press answered (`j`), ms | 353.8<br><sub>343.6–386.3</sub> | 349.3<br><sub>345.5–364.6</sub> | -4.5 |
| RAM 5 s after load, MB | 125.8<br><sub>123.0–126.5</sub> | 120.4<br><sub>120.0–125.7</sub> | -5.4 |
| CPU time, launch to settled, ms | 368<br><sub>361–370</sub> | 345<br><sub>341–359</sub> | -23 |

<sub>Runs: main: 3, bytecode: 3.</sub>

### Start-up: warm start (scenario 2)

| | main | bytecode | bytecode − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 77.5<br><sub>76.4–79.0</sub> | 58.7<br><sub>58.5–59.4</sub> | -18.8 |
| first JavaScript (`start` mark), ms | 66.6<br><sub>66.6–69.7</sub> | 50.1<br><sub>49.7–51.3</sub> | -16.5 |
| window on screen, ms | 244.3<br><sub>240.0–246.1</sub> | 199.1<br><sub>198.9–200.1</sub> | -45.2 |
| issues on the page (`mounted` mark), ms | 253.3<br><sub>218.6–259.6</sub> | 218.8<br><sub>217.9–220.9</sub> | -34.5 |
| first frame after `mounted`, ms | 271.1<br><sub>225.5–274.9</sub> | 239.7<br><sub>238.5–241.0</sub> | -31.4 |
| first key press answered (`j`), ms | 361.1<br><sub>356.0–370.6</sub> | 317.9<br><sub>315.1–320.0</sub> | -43.3 |
| RAM 5 s after load, MB | 126.4<br><sub>125.9–127.3</sub> | 120.3<br><sub>119.6–120.4</sub> | -6.1 |
| CPU time, launch to settled, ms | 367<br><sub>364–367</sub> | 345<br><sub>340–348</sub> | -22 |

<sub>Runs: main: 3, bytecode: 3.</sub>

### Idle, 20 s (scenario 3)

| | main | bytecode | bytecode − main |
|---|---:|---:|---:|
| CPU time, ms | 11.4<br><sub>10.2–11.9</sub> | 7.0<br><sub>6.9–8.1</sub> | -4.4 |
| GPU time, ms | 0.00 | 0.00 | +0.00 |
| frames presented | 0 | 0 | +0 |
| interrupt wake-ups | 34<br><sub>33–34</sub> | 35<br><sub>33–35</sub> | +1 |
| RAM 5 s after load, MB | 123.5<br><sub>108.3–123.6</sub> | 106.2<br><sub>105.4–122.3</sub> | -17.4 |
| RAM at the end, MB | 117.4<br><sub>100.1–117.6</sub> | 98.0<br><sub>97.3–116.3</sub> | -19.5 |

<sub>Runs: main: 3, bytecode: 3.</sub>

### Open an issue (scenario 5)

| | main | bytecode | bytecode − main |
|---|---:|---:|---:|
| click row → mark, ms | 25.7<br><sub>25.4–26.4</sub> | 22.7<br><sub>22.2–22.9</sub> | -3.0 |
| click row → frame, ms | 38.1<br><sub>38.0–38.9</sub> | 39.2<br><sub>38.9–39.4</sub> | +1.1 |
| Escape → mark, ms | 35.1<br><sub>17.7–37.5</sub> | 36.0<br><sub>35.1–43.3</sub> | +0.9 |
| Escape → frame, ms | 52.4<br><sub>29.0–52.8</sub> | 50.4<br><sub>50.2–62.8</sub> | -2.1 |

<sub>Runs: main: 3, bytecode: 3.</sub>

### Switch screens (scenario 10)

| | main | bytecode | bytecode − main |
|---|---:|---:|---:|
| board → mark, ms | 111.3<br><sub>101.9–112.7</sub> | 107.6<br><sub>104.0–112.4</sub> | -3.8 |
| insights → mark, ms | 64.8<br><sub>62.0–66.9</sub> | 61.2<br><sub>60.3–64.8</sub> | -3.6 |
| settings → mark, ms | 46.1<br><sub>45.2–56.6</sub> | 42.5<br><sub>39.8–49.9</sub> | -3.6 |
| issues → mark, ms | 67.1<br><sub>65.0–68.3</sub> | 67.8<br><sub>63.3–72.6</sub> | +0.8 |
| board → frame, ms | 141.9<br><sub>131.8–142.6</sub> | 144.3<br><sub>134.9–146.2</sub> | +2.4 |
| insights → frame, ms | 79.0<br><sub>75.8–81.8</sub> | 76.1<br><sub>73.9–80.5</sub> | -2.9 |
| settings → frame, ms | 55.6<br><sub>53.7–64.5</sub> | 51.2<br><sub>47.3–58.7</sub> | -4.4 |
| issues → frame, ms | 84.4<br><sub>79.6–87.1</sub> | 83.8<br><sub>79.0–88.1</sub> | -0.5 |
| board: the turn that renders it, ms | 106.7<br><sub>106.1–108.9</sub> | 110.0<br><sub>108.7–111.2</sub> | +3.3 |
| insights: the turn that renders it, ms | 49.7<br><sub>46.8–52.1</sub> | 45.5<br><sub>40.6–46.9</sub> | -4.2 |
| settings: the turn that renders it, ms | 31.7<br><sub>29.4–34.8</sub> | 28.0<br><sub>20.7–31.3</sub> | -3.7 |
| issues: the turn that renders it, ms | 47.4<br><sub>46.5–50.1</sub> | 49.6<br><sub>47.4–51.0</sub> | +2.2 |

<sub>Runs: main: 3, bytecode: 3.</sub>

### RAM 5 s after load, all start-up and idle runs (scenarios 1, 2 and 3)

| | main | bytecode | bytecode − main |
|---|---:|---:|---:|
| RAM, MB | 125.8<br><sub>108.3–127.3</sub> | 120.3<br><sub>105.4–125.7</sub> | -5.5 |

<sub>Runs: main: 9, bytecode: 9.</sub>

### Notes

- **Start-up is where it shows.** The host's own count from parsing
  `index.html` to the page settled is 19-20 ms shorter, as is the first
  line of JavaScript (`start`): 36 modules are loaded from bytecode instead
  of parsed and compiled. CPU from launch to settled is 23 ms lower. Later
  marks (`mounted`, the first frame, the first key press) are 5-10 ms
  sooner on a first launch; they wait for the fixture, read on a thread.
  On a warm start the window was on screen 45 ms sooner, with tight
  ranges: more than the JavaScript saved, and not explained. Take it as
  this session's until it is seen again.
- **First visits barely move**, as expected: a route's own chunk is small
  (the Board's, `nodes/6`, is 6 KB and compiles in 0.38 ms), so its first
  visit costs rendering, not compiling. Opening an issue is 3 ms sooner to
  its mark; the screens move by 1-4 ms either way, within the ranges.
- **Idle** is the same: no frames, 34 and 35 wake-ups in 20 s. CPU over
  the 20 s is 11.4 against 7.0 ms.
- **Memory.** RAM 5 s after load is 5.5 MB lower over all start-up and
  idle runs: QuickJS keeps no source text for functions loaded without it.
- No module fell back to its JavaScript in any run (`[host] … loading the
  JavaScript instead` appears in no log).

### Where start-up and first visits spent their time on main

Headless (`drive` on the release host, the bench Tracker, 1,000 issues,
1200x800), timed in the QuickJS loader of a throwaway build; medians of 5.
Parse and compile are one pass in QuickJS; a graph is linked in one pass,
so linking is per navigation, not per module.

| | modules | JavaScript | read | parse and compile | link | evaluate |
|---|---:|---:|---:|---:|---:|---:|
| start-up | 36 | 442 KB | 1.08 ms | 22.4 ms | 0.33 ms | 3.10 ms |
| open an issue | 2 | 28 KB | 0.19 | 2.05 | 0.04 | 0.12 |
| Board | 1 | 6 KB | 0.10 | 0.38 | 0.03 | 0.02 |
| Insights | 1 | 35 KB | 0.11 | 1.81 | 0.04 | 0.07 |
| Settings | 1 | 15 KB | 0.09 | 0.79 | 0.03 | 0.04 |

The page's own script (`page.js` and the page's nodes, 6.9 KB) costs
0.30 ms to compile and run at every launch, and stays JavaScript.

### Bytecode sizes, the Tracker's 45 modules

| | bytes | of the JavaScript |
|---|---:|---:|
| the minified JavaScript | 541,963 | 1.00 |
| bytecode with source text and debug info | 1,685,410 | 3.11 |
| **bytecode without source text, with debug info** | **896,939** | **1.65** |
| the same with calyx's headers: what `--compile` writes | 903,000 | 1.67 |
| bytecode without debug info (no file or line in an error's stack) | 761,655 | 1.41 |

Loading all 45 from bytecode takes 4.4 ms where compiling them takes 24.7
(a throwaway program on rquickjs 0.14, QuickJS-ng 0.16.2).

### The app folder, the build and the host

| | main | bytecode |
|---|---:|---:|
| app folder (the bench Tracker): files | 60 | 105 |
| app folder: bytes in the files | 847,618 | 1,750,620 |
| app folder: on disk (`du -sk`) | 972 KB | 1,964 KB |
| files the app reads to show its first screen | 38 | 38 |
| `pnpm --filter tracker build`, from nothing | 1.91 s | 1.97 s |
| the same after an edit to one component (`Row.svelte`) | 2.03 s | 2.09 s |
| of which `calyx --compile` (45 modules) | | 38-41 ms |
| dist host, darwin-arm64, bytes | 18,320,896 | 18,403,696 (+82,800) |

Build times are medians of 3, interleaved. The edit was timed with two
builds in a row: `vite build --watch` saw no file events from the shell
these ran in, on main as on the branch; a watch rebuild runs the same
adapter and the same compile. The `.js` files ship as well: they are what
loads when the bytecode does not fit the host, and what V8 loads.

