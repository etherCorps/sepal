# Bench protocol (shared by the three apps and the harness)

Every app (Sepal QuickJS, Sepal V8, Tauri 2, gpuix-svelte) follows this
exactly. Anything an app cannot follow goes in `compare/PORTING.md`.

## Identity and data directory

| Column | App id (data dir under `~/Library/Application Support/`) |
|---|---|
| Sepal QuickJS | `com.compare.tracker.blitz` |
| Sepal V8 | `com.compare.tracker.blitzv8` |
| Tauri 2 | `com.compare.tracker.tauri` |
| gpuix-svelte | `com.compare.tracker.gpuix` |

Product name `Tracker` everywhere. Window 1200x800 logical, not maximised.

## Files the harness puts in the data directory before each launch

- `fixture.json`: copied from `compare/shared/fixtures/issues-<N>.json`.
  Shape: `{ "n": N, "issues": Issue[] }`, `Issue` exactly as in
  `examples/tracker/src/lib/seed.ts`. People, projects, labels, statuses and
  priorities stay the constants of `seed.ts`; details (description, comments,
  activity) stay generated lazily from `hash(key)` as today.
- `bench.json`: `{ "clock": boolean, "rects": boolean }`.
  - When `bench.json` exists the app is in **bench mode**: it loads issues from
    `fixture.json` (not the seed), ignores any `edits.json`/`settings.json`
    (the harness deletes them anyway), and emits marks.
  - `clock: true` shows the clock (scenario 4). `rects: true` is only for the
    harness's calibration launch: it emits RECT lines (below). Measured runs
    use `rects: false`, so measuring never adds layout work.
- With no `bench.json`, the app behaves exactly as before (tests rely on this).

## Output: lines on stderr

The app writes single lines to **stderr**. The harness timestamps each line on
arrival with its own monotonic clock (`mach_absolute_time`, the same clock it
stamps the input events with), so input and output share one clock. The app
also includes its own time for reference.

```
MARK <name> <app_ms>
RECT <testid> <x> <y> <w> <h>        (calibration launch only; logical px,
                                      relative to the window's content area)
```

`app_ms` = `performance.timeOrigin + performance.now()` (wall clock, ms with
whatever resolution the runtime gives: 1 ms in WKWebView).

How each runtime writes a line:
- Sepal: `console.log(line)` (the host prints it to stderr).
- Tauri: `invoke('mark', { line })` to a Rust command that `eprintln!`s it
  (async IPC: the line arrives later than JS ran; the report says so).
- gpuix: `process.stderr.write(line + '\n')`.

### Marks

| name | when |
|---|---|
| `start` | first statement of the app's entry module |
| `read` | `fixture.json`'s text returned by the runtime's fs API, before `JSON.parse` |
| `parsed` | `JSON.parse` of it returned |
| `loaded` | `fixture.json` parsed and in the store |
| `mounted` | root layout mounted and the Issues list rendered |
| `nav:<path>` | after a navigation, once the new page's DOM is updated (Svelte `tick()` after the route change) |
| `search:<query>` | after the list for a search query is rendered |
| `status:<key>:<status>` | after a status change is applied and rendered |
| `created:<key>` | after the New issue dialog closed and the new row is rendered |
| `resize:<w>x<h>` | when the app sees a window size change, after re-render |
| `highlight:<key>` | after j/k (or anything) moves the list highlight, after re-render (scenarios 1-2: first accepted input) |
| `shown:all` | after "Show all" pages in every issue, after re-render (scenario 9) |
| `clock` | each clock tick (scenario 4 only) |
| `frame:<name>` | the runtime's "frame presented" signal after mark `<name>` |

Frame signals (three different clocks; the report labels them):
- Sepal: the host's `SEPAL_TRACE=1` repaint lines on stderr; the
  harness takes the first repaint after a mark. The app does not emit `frame:`.
- Tauri: `requestAnimationFrame(() => requestAnimationFrame(() => mark('frame:<name>')))`.
- gpuix: `next_frame().then(() => mark('frame:<name>'))`.

## Ids the scenarios use

All are `data-testid` values (gpuix: the same strings as the element's id/test
attribute it supports, reported through RECT lines).

`list`, `search`, `row` (with `data-key`), `pane`, `detail-close`, `status-select`,
`new-issue`, `nav-new`, `new-issue-title`, `new-issue-submit`, `nav-issues`,
`nav-board`, `nav-insights`, `nav-settings`, `nav-my`, `bench-clock`, `toast`, `show-all`, `show-more`,
plus Select items as `[data-select-item][data-value=<status>]` (gpuix: `status-option-<status>`).

## How latency is measured (harness)

- Every press is stamped event by event: a click is `move`, 16 ms, `down`,
  hold (30 ms), `up`; a key is `down`, hold (8 ms), `up` (modifier keys around
  it). All stamps are kept in the run record (`inputs`, and per step
  `move_t`/`down_t`/`up_t`/`hold_ms`).
- **Latency is measured from the event that makes the control act**, found
  per build and per control by the calibration launch: the control is pressed
  once with a 300 ms hold; a mark before the release means it acts on the
  press (`down`), else on the release (`up`). Stored as `triggers` in
  `calibration-<build>-<size>.json`; each step records `trigger`,
  `ms_to_mark` (from that event) and also `ms_from_down` / `ms_from_up`.
  Measured 23 Nov: in all four builds every clicked control (row, Select
  item, nav links, Show all) acts on the release and every key (j, Escape,
  typing, Cmd+Enter) on the press.
- Memory is sampled at the same two points in every scenario: **5 s after
  `mounted`** (non-start-up scenarios start then; `footprint_mounted5`) and
  at the scenario's end (`footprint`).

## Scenario steps (what the harness does; the apps only need the ids and marks)

Every scenario but 1-2 starts 5 s after `mounted`.

1. first launch after quit (cold only after a cache purge): exec → `start`/`read`/`parsed`/`loaded`/`mounted`/first frame; then first accepted input = first `j` press that produces `nav`/highlight change (posted every 20 ms until one lands; each press's stamps and whether the app was active and frontmost when it was posted are kept, AX key/main state with `--focus-ax`).
2. warm start: same, immediately after a previous launch of the same build quit.
3. idle 60 s, `clock: false`. 4. idle 60 s, `clock: true`.
5. click a row → `nav:/issues/<key>`; Escape → `nav:/issues`.
6. click `search`, type `deploy cac` (10 keys, 100 ms apart) → `search:<prefix>` each.
7. open an issue, click `status-select`, click the `done` item → `status:<key>:done`.
8. click `nav-new`, type a 12-char title, Cmd+Enter → `created:<key>`.
9. Show all, then wheel from top to bottom at a fixed rate (pixel deltas, 60 events/s).
10. `nav-board` → `nav-insights` → `nav-settings` → `nav-issues`, 2 s apart.
11. resize 1200x800 → 800x600 → 1200x800 via the Accessibility API.
