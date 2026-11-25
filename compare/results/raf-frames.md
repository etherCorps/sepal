requestAnimationFrame on real frames: Sepal before and after, and Tauri 2, at 1,000 issues. 

Session `raf-1`, interleaved, 1 warm-up and 3 runs per build and scenario. M4 Pro, 5K display at 75 Hz: 120 Hz is untested. Frames are those presented (damage tracking drew all or part of the window); Tauri has no frame trace.

### Open an issue (scenario 5)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| click row → mark, ms | 35.4<br><sub>33.4–36.9</sub> | 18.9<br><sub>18.8–19.1</sub> | 13.7<br><sub>13.3–14.3</sub> |
| click row → frame, ms | 48.4<br><sub>47.5–53.6</sub> | 31.5<br><sub>31.4–31.5</sub> | 27.2<br><sub>24.2–28.2</sub> |
| Escape → mark, ms | 35.3<br><sub>35.0–36.9</sub> | 38.4<br><sub>35.3–38.8</sub> | 27.9<br><sub>27.8–30.2</sub> |
| Escape → frame, ms | 50.0<br><sub>49.6–53.8</sub> | 53.6<br><sub>51.4–53.9</sub> | 51.6<br><sub>46.1–57.9</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Switch screens (scenario 10)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| board → mark, ms | 98.0<br><sub>97.5–99.7</sub> | 78.5<br><sub>78.1–81.8</sub> | 42.9<br><sub>40.0–46.5</sub> |
| insights → mark, ms | 68.8<br><sub>65.8–73.7</sub> | 63.2<br><sub>62.1–63.5</sub> | 41.3<br><sub>36.7–42.4</sub> |
| settings → mark, ms | 62.2<br><sub>55.0–78.5</sub> | 44.1<br><sub>42.2–45.4</sub> | 26.5<br><sub>22.5–29.6</sub> |
| issues → mark, ms | 75.1<br><sub>71.7–78.7</sub> | 64.3<br><sub>54.8–69.2</sub> | 32.8<br><sub>24.4–33.0</sub> |
| board → frame, ms | 135.9<br><sub>135.6–138.6</sub> | 114.8<br><sub>114.1–125.2</sub> | 63.5<br><sub>60.6–66.7</sub> |
| insights → frame, ms | 83.8<br><sub>80.8–95.1</sub> | 78.8<br><sub>78.4–84.6</sub> | 54.8<br><sub>50.4–56.3</sub> |
| settings → frame, ms | 70.2<br><sub>64.7–87.8</sub> | 53.1<br><sub>51.4–54.7</sub> | 40.2<br><sub>38.1–43.6</sub> |
| issues → frame, ms | 91.7<br><sub>88.1–95.2</sub> | 81.2<br><sub>68.5–84.7</sub> | 46.6<br><sub>38.0–47.1</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### New issue (scenario 8)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| Cmd+Enter → mark, ms | 31.1<br><sub>21.0–35.7</sub> | 37.0<br><sub>35.4–38.9</sub> | 22.7<br><sub>20.9–25.1</sub> |
| Cmd+Enter → frame, ms | 51.4<br><sub>37.8–55.7</sub> | 57.3<br><sub>54.5–58.9</sub> | 41.5<br><sub>38.1–44.1</sub> |
| CPU time, ms | 318<br><sub>307–326</sub> | 316<br><sub>305–318</sub> | 245<br><sub>244–255</sub> |
| GPU time, ms | 109.4<br><sub>108.5–110.8</sub> | 109.6<br><sub>106.0–110.4</sub> | 27.7<br><sub>26.5–31.0</sub> |
| frames presented | 37<br><sub>36–38</sub> | 37<br><sub>35–39</sub> | n/a |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### The New issue dialog's animation: open and close it ten times (scenario 12)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| frames per second, 200 ms after opening | 61<br><sub>59–62</sub> | 61<br><sub>60–61</sub> | n/a |
| frames per second, 200 ms after Escape | 74<br><sub>74–75</sub> | 74<br><sub>74–76</sub> | n/a |
| frames per second, whole span | 47<br><sub>47–48</sub> | 46<br><sub>46–46</sub> | n/a |
| frames presented | 414<br><sub>414–415</sub> | 400<br><sub>399–405</sub> | n/a |
| redraws asked for | 426<br><sub>425–428</sub> | 414<br><sub>413–417</sub> | n/a |
| CPU time, ms | 2,742<br><sub>2,736–2,769</sub> | 2,652<br><sub>2,648–2,708</sub> | 2,088<br><sub>2,079–2,104</sub> |
| GPU time, ms | 1,349.5<br><sub>1,343.7–1,363.8</sub> | 1,329.0<br><sub>1,315.8–1,330.6</sub> | 301.5<br><sub>300.1–301.6</sub> |
| span, s | 8.74<br><sub>8.73–8.76</sub> | 8.74<br><sub>8.74–8.75</sub> | 8.76<br><sub>8.75–8.76</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Idle, 20 s (scenario 3)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| CPU time, ms | 10.7<br><sub>9.2–11.0</sub> | 9.8<br><sub>9.1–11.6</sub> | 23.7<br><sub>23.2–24.4</sub> |
| GPU time, ms | 0.00 | 0.00 | 0.00 |
| frames presented | 0 | 0 | n/a |
| interrupt wake-ups | 35<br><sub>34–35</sub> | 34<br><sub>33–35</sub> | 172<br><sub>167–173</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Clock, 20 s (scenario 4)

| | Sepal, before | Sepal, after | Tauri 2 |
|---|---:|---:|---:|
| CPU time, ms | 525.3<br><sub>520.8–531.0</sub> | 514.4<br><sub>497.6–527.4</sub> | 1,208.7<br><sub>1,196.8–1,219.3</sub> |
| GPU time, ms | 65.53<br><sub>64.20–67.21</sub> | 69.00<br><sub>67.14–69.22</sub> | 58.54<br><sub>57.97–59.78</sub> |
| frames presented | 59 | 59 | n/a |
| interrupt wake-ups | 192<br><sub>182–193</sub> | 201<br><sub>177–202</sub> | 3,719<br><sub>3,714–3,738</sub> |

<sub>Runs: Sepal, before: 3, Sepal, after: 3, Tauri 2: 3.</sub>

### Notes

- **Links.** SvelteKit follows a link after a `requestAnimationFrame` and a
  `setTimeout(0)`. Before, that was a 16 ms timer and one more turn; now it is
  the next frame the host draws and the turn right after it. Opening an issue
  is 16 ms sooner; the screen switches gain 5-20 ms.
- **Escape and Cmd+Enter** go through `goto` and a key handler, which wait for
  no frame. In the traces the app's own turn is the same on both builds (the
  Cmd+Enter turn 6-7 ms); what varies is when the key reaches the app, 21-35 ms
  after it was posted, with no frame drawn in between. The medians differ by
  that, not by this change.
- **The dialog's animation** is 100 ms in and 100 ms out, but on both builds
  frames are drawn at the display's rate for about 280 ms after each open and
  each Escape. That was so before this change too; what it is (another
  animation or transition running on) is a job of its own. Frames, CPU and
  GPU time during it are the same before and after: requestAnimationFrame
  callbacks ride on frames the animation draws anyway.
- **Idle and the clock** draw the same frames and wake as often: with no
  callback queued, no frame is asked for.
- **120 Hz is untested.** On a ProMotion display, callbacks would run up to
  120 times a second while something waits for frames, where the 16 ms timer
  ran them at most 60 times. CSS animations already drew at the display's rate
  before this change.

