Async context and the web module (backend PR A), against main, at 1,000 issues. 

Session `be-1`, interleaved, 1 warm-up and 3 runs per build and scenario, Stage Manager off, no screen recording. M4 Pro, macOS 27.0.1, 5K display at 75 Hz. Times are from `exec` of the app.

### Start-up: first launch after quit (scenario 1)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 28.2<br><sub>27.5–28.8</sub> | 27.3<br><sub>27.3–27.7</sub> | -0.9 |
| first JavaScript (`start` mark), ms | 70.0<br><sub>68.3–72.0</sub> | 69.0<br><sub>68.9–72.9</sub> | -1.0 |
| window on screen, ms | 204.5<br><sub>197.6–205.9</sub> | 219.9<br><sub>203.1–224.6</sub> | +15.4 |
| first frame presented, ms | 204.8<br><sub>202.3–206.9</sub> | 217.3<br><sub>215.0–219.6</sub> | +12.5 |
| fixture read (`read` mark), ms | 163.0<br><sub>160.7–165.5</sub> | 167.3<br><sub>161.3–171.2</sub> | +4.3 |
| fixture read, after the first frame presented, ms | -41.7<br><sub>-41.9–-41.4</sub> | -48.1<br><sub>-48.5–-47.7</sub> | -6.4 |
| issues on the page (`mounted` mark), ms | 171.8<br><sub>169.3–173.8</sub> | 177.2<br><sub>171.5–181.6</sub> | +5.4 |
| first frame after `mounted`, ms | 209.7<br><sub>207.9–212.4</sub> | 225.1<br><sub>221.8–229.5</sub> | +15.4 |
| first key press answered (`j`), ms | 322.2<br><sub>320.2–335.5</sub> | 344.2<br><sub>324.2–352.3</sub> | +22.0 |
| RAM 5 s after load, MB | 123.9<br><sub>120.5–124.4</sub> | 120.9<br><sub>120.8–121.1</sub> | -3.0 |
| CPU time, launch to settled, ms | 365<br><sub>362–367</sub> | 362<br><sub>357–371</sub> | -4 |

<sub>Runs: main: 3, branch: 3.</sub>

The first frame presented had the issues in it: main 3 of 3, branch 2 of 3.

### Start-up: warm start (scenario 2)

| | main | branch | branch − main |
|---|---:|---:|---:|
| the page started (host trace), ms | 27.5<br><sub>27.3–28.7</sub> | 27.6<br><sub>27.0–28.0</sub> | +0.1 |
| first JavaScript (`start` mark), ms | 55.2<br><sub>50.6–55.9</sub> | 53.5<br><sub>52.7–55.2</sub> | -1.6 |
| window on screen, ms | 203.3<br><sub>199.5–205.1</sub> | 207.2<br><sub>198.7–207.3</sub> | +3.9 |
| first frame presented, ms | 196.5<br><sub>193.5–204.3</sub> | 207.9<br><sub>200.8–208.4</sub> | +11.4 |
| fixture read (`read` mark), ms | 151.8<br><sub>149.3–154.5</sub> | 156.0<br><sub>147.7–163.2</sub> | +4.2 |
| fixture read, after the first frame presented, ms | -47.3<br><sub>-52.5–-39.0</sub> | -52.4<br><sub>-53.1–-44.7</sub> | -5.2 |
| issues on the page (`mounted` mark), ms | 160.4<br><sub>157.7–163.5</sub> | 165.9<br><sub>157.5–173.4</sub> | +5.5 |
| first frame after `mounted`, ms | 199.0<br><sub>177.5–209.0</sub> | 215.4<br><sub>213.1–222.9</sub> | +16.5 |
| first key press answered (`j`), ms | 329.7<br><sub>314.6–332.5</sub> | 329.3<br><sub>322.8–331.1</sub> | -0.4 |
| RAM 5 s after load, MB | 124.2<br><sub>124.2–124.3</sub> | 121.0<br><sub>120.3–124.6</sub> | -3.3 |
| CPU time, launch to settled, ms | 347<br><sub>345–360</sub> | 347<br><sub>345–351</sub> | -0 |

<sub>Runs: main: 3, branch: 3.</sub>

The first frame presented had the issues in it: main 3 of 3, branch 3 of 3.

### Idle, 20 s (scenario 3)

| | main | branch | branch − main |
|---|---:|---:|---:|
| CPU time, ms | 11.7<br><sub>11.1–12.4</sub> | 8.8<br><sub>7.7–9.9</sub> | -2.9 |
| GPU time, ms | 0.00 | 0.00 | +0.00 |
| frames presented | 0 | 0 | +0 |
| interrupt wake-ups | 34<br><sub>34–36</sub> | 33<br><sub>31–33</sub> | -1 |
| RAM 5 s after load, MB | 106.2<br><sub>102.5–106.3</sub> | 106.2<br><sub>104.3–106.3</sub> | -0.1 |
| RAM at the end, MB | 97.9<br><sub>96.2–98.2</sub> | 97.9<br><sub>97.8–98.2</sub> | +0.0 |

<sub>Runs: main: 3, branch: 3.</sub>

### Notes

- **Idle is the same**: no frames, 33 against 34 wake-ups in 20 s, CPU 8.8
  against 11.7 ms, RAM the same.
- **Start-up is 5 ms later to the issues and 11-12 ms later to the first
  frame.** The page starts as fast (27-28 ms) and the first JavaScript runs
  at the same time, so loading the web module's bytecode is not it (~0.4 ms
  measured on its own). The first difference is the shell's first redraw,
  after the window is made: 162 against 157 ms (medians over both
  scenarios), everything after it moving with it. Both builds have the same
  one slow turn after the first paint (12-14 ms, the rows drawn). Not traced
  further; the bigger heap the web module brings (one more automatic
  collection while the app starts) is the likeliest cause, and making the
  streams and FormData load on first use would be the fix to try.
- **RAM** 5 s after load is 3 MB lower in scenarios 1 and 2 and the same
  idle: within what a launch varies (one drawable is 15 MB).
- **The first frame had the issues** in 2 of 3 first launches on the branch
  against 3 of 3, and 3 of 3 on both warm.

### Sizes

| | main | branch |
|---|---:|---:|
| host, packaged (dist, `screenshot`), bytes | 18,784,576 | 18,801,168 (+16,592) |
| counter's JavaScript, bytes | 66,190 | 156,605 (+90,415) |
| Tracker's JavaScript, bytes | 543,081 | 620,063 (+76,982) |
| Tracker's bytecode, bytes | 905,060 | 1,000,807 (+95,747) |

The app's growth is the web module: web-streams-polyfill (68 KB of
JavaScript, 82 KB of bytecode), formdata-node (6 KB), and the Fetch API
grown to its full Body. The counter grows by all of it; the Tracker
already had URL, TextEncoder and AbortController from the DOM layer.

