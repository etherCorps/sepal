Host calls that need no window started before it exists, against main, at 1,000 issues. 

Session `early-1`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app.

### Start-up: first launch after quit (scenario 1)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 63.1<br><sub>62.9–64.2</sub> | 63.0<br><sub>62.9–63.2</sub> | -0.1 |
| first JavaScript (`start` mark), ms | 58.1<br><sub>58.0–58.5</sub> | 58.2<br><sub>58.1–58.7</sub> | +0.1 |
| window on screen, ms | 232.0<br><sub>221.2–241.2</sub> | 227.0<br><sub>218.5–232.4</sub> | -5.0 |
| first frame painted, ms | 223.4<br><sub>215.7–234.9</sub> | 221.1<br><sub>211.4–234.7</sub> | -2.2 |
| fixture read (`read` mark), ms | 234.3<br><sub>218.6–246.7</sub> | 222.6<br><sub>212.8–236.1</sub> | -11.7 |
| fixture read, after the first frame painted, ms | 10.9<br><sub>2.9–11.8</sub> | 1.4<br><sub>1.3–1.4</sub> | -9.5 |
| issues on the page (`mounted` mark), ms | 243.0<br><sub>227.8–255.8</sub> | 231.9<br><sub>221.9–245.4</sub> | -11.1 |
| first frame after `mounted`, ms | 253.7<br><sub>244.0–266.8</sub> | 252.0<br><sub>238.5–263.3</sub> | -1.7 |
| first key press answered (`j`), ms | 348.8<br><sub>346.2–354.7</sub> | 342.7<br><sub>341.6–360.2</sub> | -6.1 |
| RAM 5 s after load, MB | 135.1<br><sub>123.9–135.4</sub> | 134.5<br><sub>123.8–136.0</sub> | -0.6 |
| CPU time, launch to settled, ms | 381<br><sub>343–385</sub> | 384<br><sub>341–388</sub> | +4 |

<sub>Runs: main: 3, branch: 3.</sub>

The first frame painted had the issues in it: main 0 of 3, branch 0 of 3.

### Start-up: warm start (scenario 2)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 57.1<br><sub>57.1–57.6</sub> | 57.7<br><sub>56.1–57.8</sub> | +0.6 |
| first JavaScript (`start` mark), ms | 49.8<br><sub>49.4–50.2</sub> | 49.8<br><sub>48.5–50.0</sub> | +0.1 |
| window on screen, ms | 195.4<br><sub>194.3–220.1</sub> | 196.5<br><sub>194.8–203.8</sub> | +1.1 |
| first frame painted, ms | 196.4<br><sub>183.1–197.9</sub> | 202.9<br><sub>194.2–204.8</sub> | +6.5 |
| fixture read (`read` mark), ms | 200.8<br><sub>185.8–207.5</sub> | 212.5<br><sub>195.6–214.1</sub> | +11.7 |
| fixture read, after the first frame painted, ms | 2.9<br><sub>2.7–11.1</sub> | 9.3<br><sub>1.3–9.7</sub> | +6.4 |
| issues on the page (`mounted` mark), ms | 209.5<br><sub>194.8–216.6</sub> | 221.7<br><sub>204.8–223.1</sub> | +12.2 |
| first frame after `mounted`, ms | 225.1<br><sub>201.6–227.7</sub> | 234.2<br><sub>221.6–235.7</sub> | +9.1 |
| first key press answered (`j`), ms | 317.0<br><sub>310.7–349.8</sub> | 320.0<br><sub>310.0–323.9</sub> | +3.0 |
| RAM 5 s after load, MB | 134.3<br><sub>120.8–135.7</sub> | 134.0<br><sub>125.3–135.2</sub> | -0.3 |
| CPU time, launch to settled, ms | 372<br><sub>333–376</sub> | 372<br><sub>335–377</sub> | -0 |

<sub>Runs: main: 3, branch: 3.</sub>

The first frame painted had the issues in it: main 0 of 3, branch 0 of 3.

### Idle, 20 s (scenario 3)

| | main | branch | branch − main |
|---|---:|---:|---:|
| CPU time, ms | 9.7<br><sub>9.6–10.7</sub> | 10.7<br><sub>9.9–79.7</sub> | +1.0 |
| GPU time, ms | 0.00 | 0.00<br><sub>0.00–6.39</sub> | +0.00 |
| frames presented | 0 | 0<br><sub>0–9</sub> | +0 |
| interrupt wake-ups | 34 | 37<br><sub>34–74</sub> | +3 |
| RAM 5 s after load, MB | 130.6<br><sub>105.4–130.9</sub> | 124.6<br><sub>114.3–130.8</sub> | -5.9 |
| RAM at the end, MB | 124.4<br><sub>97.2–124.8</sub> | 114.2<br><sub>106.1–124.6</sub> | -10.3 |

<sub>Runs: main: 3, branch: 3.</sub>

### Notes

- **No change on the Tracker, as measured.** Over all six start-ups the
  issues are on the page at a median of 222.5 ms on both builds; scenario
  1 is 11 ms sooner on the branch and scenario 2 12 ms later, each within
  the other's ranges. In none of the twelve runs, on either build, did the
  first frame painted have the issues: the fixture is read 1-12 ms after it.
- **Why:** in bench mode the Tracker reads `bench.json`, and only once that
  is answered `fixture.json` (`bench.ts`). On the branch `bench.json` is
  read while the page starts, but its answer waits in the channel for the
  next turn, and before the window the next turn is the window's first
  poll: the fixture is read from there on both builds. The branch saves the
  round trip for one small file, too little for these runs to show.
- **What would put the issues in the first frame:** the fixture read
  started before the first poll. Either the app asks for both files at
  once, or the host runs a turn at some point before the window, so that a
  call made in answer to another starts too. Neither is in this change.
- **Idle** is the same: no frames, 34 wake-ups in 20 s, in five of the six
  runs. In the sixth, a branch run, the pointer crossed the window: 34
  repaints of rows and nav items lighting up under it (CSS `:hover`), no
  change by the app, and 79.7 ms of CPU. It is in the ranges above.
- RAM and CPU to settled are higher on both builds than in `alloc-1`
  (124 against 135 MB, 343 against 381 ms): this session's machine state.

