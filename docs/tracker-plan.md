# Tracker: plan

Written 23 Nov 2026. The benchmark app. Design: /hallmark, using the shadcn components in the verified set.

## Tracker: an offline, keyboard-first issue tracker in the style of Linear (examples/tracker)

Tracker is a local-first issue tracker like Linear or GitHub Issues: a sidebar shell, a list and detail split over 5000 seeded issues with live search, filters, grouping and sorting, j/k navigation, bulk triage, a Kanban board, an Insights dashboard with SVG charts, a Cmd+K palette, a New issue form, Undo toasts, settings, and edits saved to disk. It is the shape most people mean by a real-world app, and normal use of it reaches every expensive path in svelte-blitz. Keystroke filtering tests QuickJS. The board tests Blitz layout. Selects and Dialogs force layouts over a big page (invariant 2). Navigating routes mounts and unmounts Bits UI under the router. Persistence and simulated teammate edits test timers, host calls and idle. All data comes from a fixed seed with no network, so runs are deterministic headless or in a window, on QuickJS or V8, at 1k, 5k and 20k issues.

## Screens

### src/routes/+layout.svelte (shell, always mounted)

A sidebar with Issues, My issues, Board, Insights, 5 projects with open counts (/projects/[key], which is the list with a fixed filter) and Settings. The shell holds the only document keydown listener, the palette, New issue and the toast stack. setTitle follows the route.

- <a href> navigation through the built-in router
- One document.addEventListener('keydown') in an $effect (dom:true, no <svelte:window>): Cmd/Ctrl+K, c, /, g-chords with a 1 s timeout, ? for the shortcuts Dialog
- Single-key shortcuts are skipped while document.activeElement is an input
- Escape has a fixed order: Select, then Dialog, then palette, then pane, then toast

### /issues, /my, /projects/[key] (issues/+layout.svelte + +page.svelte; one shared list component)

The main list. A toolbar outside the scroller (no sticky) with search, Select filters (status, priority, assignee, project), Group by and Sort. Rows are flex or grid under group headers with counts, 200 per page, with 'Show more' and 'Show all' for stress. Each row has a plain checkbox, and a bulk bar appears when rows are checked. Rows hold no Bits UI component.

- Search filters on every keystroke, with no debounce
- Filter, Group by and Sort Selects regroup all N issues
- j/k move the highlight, Enter or o opens, x checks the row
- Shift+click selects a range; the bulk bar offers Set status, Assign, Delete (confirm Dialog)
- Collapse a group; Show more and Show all

### /issues/[key] (inside the issues layout; +page.ts returns params.key, error(404) for an unknown key; +error.svelte)

The detail pane: a title edited in place, a description, property Selects (status, priority, assignee, project), label Checkboxes, Tabs for Comments and Activity (generated lazily from hash(key)), a comment Input, and Copy link (by hand only).

- Escape goes back to /issues and keeps the highlight
- j/k with the pane open: a route change plus load() per press
- A Select change saves an overlay edit, adds activity, updates the list and board, shows an Undo toast
- Title: Enter saves, Escape cancels, empty shows an inline error
- Enter posts the comment

### /board

Kanban with 6 status columns of at most 50 cards each (about 300), sharing the filter store. No drag and drop.

- Board mount, the largest DOM build
- h/l and j/k move the focus; Shift+h and Shift+l move the card
- s opens the status Select on the focused card
- A filter change re-renders every column

### /insights

Aggregates over all issues: 4 stat tiles, a created-against-closed SVG bar chart over 12 weeks, open issues by assignee and priority, the 10 oldest open issues. All $derived: the pure-JS benchmark. Values are inline labels, not <svg><title>.

- The range (4, 12 or 26 weeks) and the project Select recompute everything
- Clicking a bar goes to /issues?week=N (page.url.searchParams.get)
- A Tooltip on the tiles

### /settings (Tabs: General, Data)

Display name (validated), compact rows and show labels Switches, default Group by. Data tab: issue count, Reset to sample (a confirm Dialog that clears edits.json), Export CSV (by hand only), and a Simulate teammates Switch, off by default.

- A Switch saves settings.json and shows a toast
- Compact rows restyles every mounted row
- Simulate teammates starts and stops one setInterval (2 s) that edits a random issue

### overlay: Command palette (src/lib/CommandPalette.svelte; Dialog plus Input plus the app's own list, not Bits UI Command)

Cmd+K over Actions, Projects and Issues (top 8 across N).

- Typing searches N issues per key
- Arrow keys and Enter run the item (goto plus close); Escape closes and gives the focus back
- A '>' prefix limits the list to actions

### overlay: New issue (Dialog) and Toasts

Form: title (3 to 200 characters), Selects for project, status, priority and assignee, label Checkboxes. Validation on submit, then live; Cmd+Enter submits; the new issue goes to the top, highlighted, with an Open toast. Toasts: at most 3, a finite CSS keyframe, a one-shot 5 s setTimeout.

- An empty submit shows errors and the dialog stays open
- Tab stays inside the Dialog
- Undo reverts the last overlay edit
- All timers end, so the app goes idle

## Data

src/lib/seed.ts uses mulberry32 with seed 42 and word lists, no dependencies. It generates 25 people (initials, no images), 5 projects, 12 labels, 6 statuses, 5 priorities and N issues (default 5000; the harness sets N with globalThis.__tracker.reseed(n), so no rebuild). An issue has key, project, number, title, status, priority, assignee, labelIds, estimate, createdAt and updatedAt. Dates fall in the 26 weeks before a fixed epoch and are formatted by hand (no Intl). Descriptions, comments and activity (5 to 40) are generated lazily from hash(key) and cached. store.svelte.ts holds an $state issues array, a Map by key, a shared filter and view object, $derived filtered, grouped, sorted and paged lists, and the Insights aggregates. ui.svelte.ts holds palette, dialog, toast and highlight state. Persistence uses @ethercorps/svelte-blitz/fs with fs ['data']. Only user edits are saved, as an overlay in data/edits.json (created, changed fields by key, deleted, comments), written 500 ms after the last edit by one setTimeout and applied over the seed at start. data/settings.json is written on change. __tracker = { reseed, stats } exists for the harness: N, filtered count, and the last filter and stringify ms from performance.now().

## Stack

examples/tracker uses svelteBlitz() with the built-in file router (+layout.svelte, issues/+layout.svelte, issues/[key]/+page.ts|svelte, my, projects/[key], board, insights, settings, +error.svelte; goto and back, page). app.config.ts: dom: true, id com.example.tracker, a 1280x800 window, permissions { fs: ['data'], window: true, clipboard: true, dialog: true }, no http. shadcn components copied from examples/shadcn, verified set only: Button, Badge, Card, Input, Label, Separator, Switch, Checkbox, Tabs, Dialog, Select, Tooltip. Not used: Skeleton (never idles), and DropdownMenu, Popover, Command and Sonner (untried). Tailwind v4, @lucide/svelte, fontsource Inter. Only structure ships before the design, with a data-testid contract the design must keep: search, row, row-active, group-header, show-more, show-all, bulk-bar, detail-title, status-select, board-column, card, palette, palette-item, new-issue, toast, nav-*. Runes-only .svelte.ts stores. Measurement: drive.rs (extended), SVELTE_BLITZ_TRACE=1, crates/host/tests/tracker.rs, --features engine-v8.

## Perf scenarios

- **P1 Cold start to first layout**: Bench: Instant around for_app plus layout(1280,800) plus resolve(), without an edits.json and with 100 edits. Record live_nodes(), bundle bytes and the JS seed ms, at N 1k, 5k and 20k, on both engines. Peak RSS from /usr/bin/time -l. _Launch time: bundle parse plus seed plus first mount. Baselines: counter 0.17 s, shadcn 0.10 s._
- **P2 Search keystroke over N**: Click [data-testid=search], then type 'deploy' one character at a time with each timed (typeeach:), with ops, layouts and layout ms. Median and worst at 1k, 5k and 20k. _The most frequent input and the clearest JS cost: a filter plus a keyed {#each} diff. Target under 16 ms at 5k._
- **P3 j/k with the pane closed and open**: key:j 20 times, timed: highlight only, then goto plus load plus detail plus lazy activity. First press, median, and a second pass over opened issues. _The path keyboard users repeat all day. It separates router cost from rendering, and V8's first run from its warm speed._
- **P4 Select forced layouts on a big page**: Open the detail status Select and the assignee filter with 200 rows mounted, then with Show all. Click ms plus trace layouts and layout ms. Baseline: shadcn Select, 7 layouts, 9 of 22 ms. _Invariant 2 lays out the whole document, so this shows how the forced layout scales with page size._
- **P5 Regroup and resort**: Timed clicks through Group by status, assignee, priority and none, and Sort by priority and title; ms and ops for each. _The real-app version of shuffling 1000 rows (36 ms QuickJS, 10 ms V8)._
- **P6 Show all: the rows cliff**: Show more, then Show all at 1k, 5k and 20k: ms, ops, layout ms, count:[data-testid=row], live_nodes(), RSS. Wheel-scroll it in a window with the trace on, by hand. _Finds where the lack of a virtual list hurts, for README Next steps 5._
- **P7 Board mount and card move**: Click nav-board and time it (about 300 cards), then Shift+l on a card; ms, ops, layout ms. _The layout stress case: is the turn bound by JS or by Blitz?_
- **P8 Command palette**: key:Meta+k, then typeeach:eng-12, then key:Enter, each step timed. _A Dialog open, a per-key search inside a modal, and a navigation from it._
- **P9 Create and bulk edit**: c, an empty submit (errors), fix, two Selects, Cmd+Enter, then assert the first row. Shift+click a 50-row range (new click_with), then Set status Done. Each step timed with its ops. _The form path and a mass change that moves rows between groups._
- **P10 Persist an edit**: Edit, wait(0.6), resolve, assert edits.json in the per-test data dir. Time the click turn and the save turn apart, and read the stringify ms, at 1, 100 and 1000 overlay edits. _Shows that saving never blocks input (invariant 0) and what the JSON costs._
- **P11 Insights recompute**: Edit, then go to /insights and time the mount; change the range from 4 to 26 weeks; shot: and check the chart's pixels. _JS aggregation plus SVG, apart from list layout._
- **P12 Restyle: compact rows**: With Show all active, toggle compact; the trace splits layout from JS. _The first scenario where Rust (restyle) might be the cost, the CLAUDE.md test for whether to optimise Rust at all._
- **P13 Route churn and memory**: Open a row and back() 50 times, then cycle issues, board, insights, settings 10 times; live_nodes() after GC and RSS at start and end. _Leaks from Bits UI unmounting under the router._
- **P14 Simulated teammates, then idle**: Simulate on, then wait(2.0) 30 times, timing each turn and its ops on /issues and /board. Simulate off and toasts expired, then assert no next timer and that an idle turn does nothing. In a window, top -stats cpu,idlew for 10 s, on and off. _Steady reactive updates, and invariant 7: every timer ends at 0% CPU._
- **P15 QuickJS against V8**: The whole bench, release build, default features and then engine-v8: first run and median of 5 for each scenario, plus RSS. _A real workload for the open V8 decision (is it worth 38 MB)._

## Risks and constraints

- The router plus dom:true is unverified: shadcn is the only app with dom:true and has one page. Router root click listener at packages/vite/src/routing.svelte.js:376. Bits UI unmounting across routes is untested. Probe first.
- Scroll positions never reach JS: scrollTop and scrollLeft are fixed and scrollIntoView and scrollTo do nothing (packages/core/src/dom.js:762-763, :846-847, :1214). So: paging, no virtual list, and the j/k highlight can go off screen.
- CSS constraints for /hallmark: no sticky, no overflow: auto (use overflow-y-scroll), no :focus-within, no container queries, no 3D transforms, no backdrop-filter, no infinite animations (Skeleton, animate-spin), which would spoil P14.
- Forced layouts grow with the page (crates/host/src/document.rs, Page::laid_out_at), so Selects go in toolbars, the detail pane and the focused card only, never in rows.
- A measurement inside an event handler uses the last reported boxes. Open a Select from a keydown handler in a microtask after changing the page.
- drive.rs only times click: (crates/host/examples/drive.rs:37-46), and key: has no modifiers. testing.rs has no click with modifiers and no wheel (crates/host/src/testing.rs:86-140).
- The data dir is the real ~/Library/Application Support/<id> (crates/host/src/api/fs.rs:26-37). Tests use a per-pid manifest.id and delete it. No dialogs or clipboard in tests.
- QuickJS speed and the 10 s watchdog (js.rs) at 20k: keep generation lazy, and treat 20k as the stress size only.
- Every load() reruns on every navigation, so loads stay trivial.
- QuickJS frees module values that no live function holds, so the store and __tracker need host tests, not only Node tests.
- Textarea typing is untested in Blitz (bindings.js:18 rewrites it), so comments use an <input>.
- Boxes and doc.click ignore transforms, so the centred Dialog needs click_at or clickat:.
- Tailwind colours reach Blitz as lab(), so charts use currentColor.
- Numbers only count from --release builds.

## Build steps

- 0. Hand this plan to /hallmark (screens, data-testid contract, CSS constraints). Step 1 runs in parallel.
- 1. Probe the router with dom:true: two routes, each with a Select, a Dialog and a Tooltip; navigate 20 times headless; check focus restore, no leftover portals, live_nodes() flat after GC. Fix in dom.js or the router before going on.
- 2. Scaffold examples/tracker from examples/shadcn (workspace:* dependencies, verified ui folders, app.config.ts as set out in stack); pnpm --filter tracker build.
- 3. seed.ts, format.ts, store.svelte.ts, persist.ts (the overlay), the __tracker hook, and one Node check that the seed is deterministic and the counts are right.
- 4. Shell, then issues list and detail, board, insights, settings, palette, New issue, toasts, applying /hallmark's design.
- 5. drive.rs: ms and ops for every step, Meta+/Shift+ on key:, typeeach:. testing.rs: click_with(Modifiers). Optional: layouts and layout_ms in TurnStats. cargo fmt; leave crates/blitz-dom alone.
- 6. crates/host/tests/tracker.rs: functional tests (search, filter, detail and back, j/k, Select plus toast, create, bulk, board, 404, overlay write, Insights pixels, idle) plus one #[ignore] bench printing P1-P15. Make sure test:host builds tracker.
- 7. Measure with release builds at 1k, 5k and 20k, on QuickJS then V8, then the window pass for P6 and P14.
- 8. Write a Tracker block in README Performance, the V8 table in HANDOVER, the gaps under Limitations, and a Layout line in CLAUDE.md. Touch Rust only if the trace says layout is the cost. pnpm test; commit only when Shivam asks.

## Harness

(A) Headless bench: an #[ignore] test in crates/host/tests/tracker.rs, run with 'SVELTE_BLITZ_TRACE=1 cargo test --release --manifest-path crates/host/Cargo.toml --features screenshot --test tracker -- --ignored --nocapture bench', then with --features "screenshot engine-v8". It loads App::from_dir(examples/tracker/dist) with manifest.id set to com.example.tracker-test-<pid> (data dir deleted at the end), then layout(1280,800). For N in [1000, 5000, 20000] it evals __tracker.reseed(N) and runs P1 to P14 through the testing.rs helpers (click, click_at, press, press_with(META|SHIFT), a new click_with, type_text one character at a time, wait). Each step is wrapped in Instant and collects last_turn() ops, visible and created, live_nodes(), and layouts and layout ms (new TurnStats fields, or the '[trace] N ops, M layouts in X ms' line that settle() prints headless, document.rs:456). Interactive steps run 5 times, reporting the first and the median. JS-side ms come from __tracker.stats(). RSS from /usr/bin/time -l per engine and size. (B) drive.rs, extended: every step prints ms and ops; key:Meta+k and key:Shift+j; typeeach:TEXT. Run with --release, e.g. 'cargo run --release --features screenshot --example drive -- ../../examples/tracker/dist 1280x800 click:[data-testid=search] typeeach:deploy key:Escape key:j key:Enter click:[data-testid=nav-board] count:[data-testid=card] shot:board.png'. (C) Window: a release host with --watch and SVELTE_BLITZ_TRACE=1, stdin click, key and type at positions from drive rect:, wheel scrolling by hand for P6, and top -stats cpu,idlew,mem for P14. Table columns: scenario | N | engine | first ms | median ms | worst ms | ops | layouts | layout ms | live nodes | RSS MB | notes. Reference rows from README and HANDOVER: counter click 1 ms, select in 1000 5/1.8 ms, shuffle 36/10 ms, Dialog 8 ms, Select 22 ms with 7 layouts, start 0.17/0.10 s. Targets are reported, not asserted: a keystroke under 16 ms at 5k, a navigation under 50 ms, start under 1 s, 0% CPU idle.

## Corrections the judge made to the planners

- Plans 1 and 2 say there is no URL, URLSearchParams or history in QuickJS. With dom:true, dom.js provides URL and URLSearchParams (packages/core/src/dom.js:18, :1189) and a history object with pushState and replaceState (dom.js:986-989). Their advice to keep filter state in the store or router still holds.
- Plan 3 says the test points the data directory at a temp dir the way tests/desktop.rs does. It does not. named_dir('data') always resolves to ~/Library/Application Support/<id> (crates/host/src/api/fs.rs:26-37), and desktop.rs swaps app.manifest.permissions for scratch absolute paths (app_with, tests/desktop.rs:40-50). What works: set app.manifest.id to a per-test id and delete that directory afterwards, or deny fs so the app falls back to the in-memory seed.
- Plan 2 says the preset rewrites bind:value only for inputs and that textarea is unverified. bindings.js rewrites input, textarea and select (packages/vite/src/bindings.js:18). What is untested is typing into a textarea in Blitz: no host test uses one.
- Plans 1 and 2 assume the router and dom:true work together. No example combines them: examples/shadcn, the only app with dom:true, has one page, and examples/routing has no dom. The router's root click listener (packages/vite/src/routing.svelte.js:376) and Bits UI unmounting across navigations are untested together. Plan 3 is right to probe first.
- Plan 1 implies SVELTE_BLITZ_TRACE layout numbers need a window. settle() prints '[trace] the app changed the page: N ops, M layouts in X ms' headless too (crates/host/src/document.rs:456).
- No plan says drive and cargo test default to the dev profile. Perf numbers need --release to compare with README 'Performance'.
- Confirmed, not wrong: drive.rs times only click: (drive.rs:37-46), and key: has no modifiers. testing.rs has press_with(Modifiers) but no click with modifiers and no wheel. scrollTop, scrollIntoView and scrollTo do nothing (dom.js:762, :846, :847). Keys with nothing focused reach the app only through host_hooks.outside (dom.js:1134).

## Step 1 result: router + dom:true probe (23 Nov 2026)

Passes with no code changes (`examples/router-dom-probe`, `crates/host/tests/router_dom.rs`, QuickJS only):
20 link navigations with a Select, Dialog and Tooltip on each route, no leftover portals, focus restored,
`live_nodes()` flat at 289 after GC, a link inside an open Dialog navigates (mouse and keyboard).

For the tracker build and its tests:
- The Dialog is centred with a translate: click inside it with `click_at` (see `click_in_dialog()` in router_dom.rs).
- Select items sit under a `translate(x, y)` on `[data-bits-floating-content-wrapper]`: add that offset.
- While a Select or Dialog is open the page has `pointer-events: none`: the first outside click only closes it.
- Components mounted together need distinct ids / `data-testid`s.
- Not covered yet: `goto()` from a timer or effect while a Select is open, and nested layouts. Test both in tracker.rs.
