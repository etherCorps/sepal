# Tracker: design

Hallmark, 23 Nov 2026. Companion to `docs/tracker-plan.md` (scope, data, tests).
This file is the visual system and the layout of every screen. Build to it.

```
Hallmark · genre: modern-minimal · scope: app (multi-screen tool, not a landing page)
macrostructure: Index-First (adapted: the list IS the app) · theme: Cobalt (light only)
nav: N3 side-rail (as an app sidebar) + N4 ⌘K palette · footer: none · enrichment: none
inferred: audience = an engineering team triaging all day; use = get through the queue
          (find, open, change, next) with the keyboard; tone = technical, instrument panel
```

## Rules the engine imposes (do not break)

- No `position: sticky`. Toolbars sit outside the scroller: a flex column with
  `min-h-0` and the list in `overflow-y-scroll` (never `overflow-auto`).
- No container queries, 3D transforms, `backdrop-filter`, `:focus-within`.
- No infinite animations (no Skeleton, no `animate-spin`, no pulsing dots). The app
  must go idle. The only keyframes: shadcn's Select/Dialog/Tooltip enter and exit
  (finite) and the toast enter.
- **No `transition` on list rows, board cards, or anything repeated.** Hovering
  200 rows must not start 200 animations. Hover is an instant background change.
  `transition-colors` is fine on single controls (buttons, nav items).
- No Select, Tooltip or Checkbox component *inside* rows or cards. Rows use a
  plain `<input type="checkbox">`. Selects live in the toolbar, the bulk bar,
  the detail pane and the New issue dialog only.
- No `<textarea>` (untested); comments use `<Input>`.
- SVG colours via `currentColor` only (Tailwind colours reach Blitz as `lab()`).
- No `-webkit-line-clamp`: clamp with `max-height` + `overflow-hidden`, and
  `truncate` (`text-overflow: ellipsis; white-space: nowrap`) for one line.
- No images. Avatars are initials.
- Keep every `data-testid` from the plan: `search, row, row-active, group-header,
  show-more, show-all, bulk-bar, detail-title, status-select, board-column,
  card, palette, palette-item, new-issue, toast, nav-<name>`.

## Tokens

Shadcn variable names are kept, so the copied components restyle without edits.
`src/app.css` of the example, in this order:

```css
/* Hallmark · genre: modern-minimal · macrostructure: Index-First (app) · theme: Cobalt · enrichment: none · nav: N3+N4 · footer: none */
@import 'tailwindcss';
@import 'tw-animate-css';
@import 'shadcn-svelte/tailwind.css';
@import '@fontsource-variable/inter/wght.css';      /* use the latin-only file if the package has one */
@import '@fontsource/space-grotesk/latin-500.css';
@import '@fontsource/space-grotesk/latin-600.css';
@import '@fontsource/jetbrains-mono/latin-400.css';
@import '@fontsource/jetbrains-mono/latin-500.css';

:root {
	/* paper and ink: cool, never #fff / #000 */
	--background: oklch(0.985 0.004 250);
	--foreground: oklch(0.24 0.02 258);
	--ink-2: oklch(0.34 0.018 257);          /* body text a notch lighter */
	--card: oklch(0.995 0.003 250);
	--card-foreground: var(--foreground);
	--popover: oklch(0.995 0.003 250);
	--popover-foreground: var(--foreground);

	/* the one signal */
	--primary: oklch(0.58 0.2 256);           /* cobalt */
	--primary-foreground: oklch(0.985 0.004 250);
	--ring: oklch(0.58 0.2 256);

	--secondary: oklch(0.955 0.006 252);
	--secondary-foreground: var(--foreground);
	--muted: oklch(0.965 0.005 252);
	--muted-foreground: oklch(0.48 0.018 257);
	--accent: oklch(0.945 0.01 255);          /* hover surface */
	--accent-foreground: var(--foreground);
	--destructive: oklch(0.56 0.2 25);
	--border: oklch(0.9 0.008 255);           /* hairlines do the work */
	--input: oklch(0.87 0.01 255);
	--radius: 0.375rem;                       /* 6px: drawn with a ruler */

	--sidebar: oklch(0.97 0.005 252);
	--sidebar-foreground: var(--ink-2);
	--sidebar-primary: oklch(0.58 0.2 256);
	--sidebar-primary-foreground: oklch(0.985 0.004 250);
	--sidebar-accent: oklch(0.93 0.01 255);
	--sidebar-accent-foreground: var(--foreground);
	--sidebar-border: var(--border);
	--sidebar-ring: var(--ring);

	/* the dark beat: command palette and bulk bar only */
	--graphite: oklch(0.22 0.016 260);
	--graphite-2: oklch(0.29 0.018 260);      /* active item, inner hairline */
	--graphite-foreground: oklch(0.94 0.006 255);
	--graphite-muted: oklch(0.68 0.015 257);

	/* list states */
	--row-hover: oklch(0.955 0.008 255);
	--row-active: oklch(0.94 0.03 256);       /* j/k highlight */
	--row-checked: oklch(0.96 0.02 256);

	/* data encodings: small glyphs only, never fills of large areas */
	--status-backlog: var(--muted-foreground);
	--status-todo: var(--ink-2);
	--status-progress: oklch(0.58 0.2 256);
	--status-review: oklch(0.58 0.15 300);
	--status-done: oklch(0.58 0.13 155);
	--status-canceled: var(--muted-foreground);
	--priority-urgent: var(--destructive);
	--chart-1: oklch(0.58 0.2 256);           /* created */
	--chart-2: oklch(0.8 0.07 256);           /* closed */
	--chart-3: oklch(0.6 0.015 257);
	--chart-4: oklch(0.45 0.018 257);
	--chart-5: oklch(0.32 0.018 257);
	/* projects and labels: one lightness and chroma, hue varies */
	--hue-1: oklch(0.62 0.13 256);
	--hue-2: oklch(0.62 0.13 155);
	--hue-3: oklch(0.62 0.13 60);
	--hue-4: oklch(0.62 0.13 300);
	--hue-5: oklch(0.62 0.13 20);
	--hue-6: oklch(0.62 0.13 200);
	--hue-7: oklch(0.62 0.13 100);
	--hue-8: oklch(0.62 0.13 340);
	--hue-9: oklch(0.62 0.13 230);
	--hue-10: oklch(0.62 0.13 130);
	--hue-11: oklch(0.62 0.13 30);
	--hue-12: oklch(0.62 0.13 280);
}

@theme inline {
	--font-sans: 'Inter Variable', ui-sans-serif, system-ui, sans-serif;
	--font-display: 'Space Grotesk', var(--font-sans);
	--font-mono: 'JetBrains Mono', ui-monospace, monospace;
	--color-ink-2: var(--ink-2);
	--color-graphite: var(--graphite);
	--color-graphite-2: var(--graphite-2);
	--color-graphite-foreground: var(--graphite-foreground);
	--color-graphite-muted: var(--graphite-muted);
	--color-row-hover: var(--row-hover);
	--color-row-active: var(--row-active);
	--color-row-checked: var(--row-checked);
	--color-status-backlog: var(--status-backlog);
	--color-status-todo: var(--status-todo);
	--color-status-progress: var(--status-progress);
	--color-status-review: var(--status-review);
	--color-status-done: var(--status-done);
	--color-status-canceled: var(--status-canceled);
	--color-priority-urgent: var(--priority-urgent);
	/* ...plus every shadcn mapping from examples/shadcn/src/app.css (@theme inline block), and --color-hue-1..12 */
}

@layer base {
	* { @apply border-border outline-ring/50; }
	html { @apply font-sans; font-size: 16px; }
	body { @apply bg-background text-ink-2; font-size: 13px; line-height: 1.45; }
}
```

The `/* ... */` line in `@theme inline` is a placeholder: copy the full shadcn mapping
from `examples/shadcn/src/app.css` and add `--color-hue-N: var(--hue-N)` for 1 to 12.
Add the three font packages with `pnpm --filter tracker add -D` (latin subsets only,
since every subset is inlined into `app.css` as a data URL and parsed at start).

**Locked tokens.** Components use these by name (`bg-row-active`, `text-status-done`,
`font-display`, `font-mono`, `text-[var(--hue-3)]`). No raw `oklch()`/hex in components.

## Type

| Role | Face | Size / weight | Use |
|---|---|---|---|
| Page title | Space Grotesk | 18px / 500, tracking -0.02em | header of each screen |
| Detail title | Space Grotesk | 22px / 500, tracking -0.02em | issue title in the pane |
| Stat | Space Grotesk | 32px / 500 (hero tile 48px) | Insights numbers, `tabular-nums` |
| Body, rows | Inter | 13px / 400, titles in rows 500 | everything else |
| Input text | Inter | 14px / 400 | inputs, palette input 15px |
| Meta | Inter | 12px / 400, `text-muted-foreground` | dates in sentences, helper text |
| Machine | JetBrains Mono | 11–12px / 400–500 | issue keys `ENG-123`, counts, dates in rows, kbd hints |
| Label | JetBrains Mono | 11px / 500 UPPERCASE, tracking 0.06em | sidebar group titles, palette group titles, chart axes |

Headings are roman, never italic. No eyebrow tags above page titles.

## Space and shape

- 4px scale (Tailwind default). Row height 36px (compact 28px). Group header 32px.
  Header bar 48px. Toolbar 44px. Controls 28px (`size="sm"`) in toolbars, 32px elsewhere.
- Radius 6px on controls, 8px on cards and dialogs. No pills.
- Depth from hairlines. Shadows: none, except `shadow-sm` on popovers, toasts and
  dialogs (shadcn default) and a 2px cobalt `ring` on the focused board card.

## Glyphs (src/lib/glyphs/, 14–16px inline SVG, `currentColor`)

- **Status:** Backlog = dashed circle (`text-status-backlog`); Todo = empty circle;
  In Progress = circle half filled (cobalt); In Review = three quarters filled (violet);
  Done = filled circle with a check (green); Canceled = circle with an x (muted).
- **Priority:** No priority = three short dashes (muted); Low/Medium/High = signal bars,
  1/2/3 of 3 filled (`text-ink-2`, unfilled bars at 30% opacity); Urgent = filled
  rounded square with `!` (`text-priority-urgent`).
- **Avatar:** 20px circle (24px in the detail pane), initials 10px/600, background
  `color-mix(in oklch, var(--hue-N) 18%, var(--background))`, text `var(--hue-N)`
  darkened (`color-mix(in oklch, var(--hue-N) 70%, black)`), N from a hash of the person.
- **Project:** 10px rounded square in `var(--hue-N)`. **Label:** 6px dot in `var(--hue-N)`.
- UI icons: `@lucide/svelte`, 16px, stroke 1.75, `text-muted-foreground`.

## Screens

### Shell (`+layout.svelte`)

```
┌───────────────┬──────────────────────────────────────────────────────────┐
│ Tracker    ⌘K │ Issues  5,000                       [List|Board]  ⋯       │ 48px header
│ [+ New issue C]├──────────────────────────────────────────────────────────┤
│               │ [⌕ Search issues… /] [Status▾][Priority▾][Assignee▾][Project▾] │ 44px toolbar
│ ▣ Issues      │                         Group: Status ▾  Sort: Updated ▾  │
│ ◎ My issues   ├──────────────────────────────────────────────────────────┤
│ ▦ Board       │  (screen content, the only scroller)                      │
│ ◔ Insights    │                                                          │
│ PROJECTS      │                                                          │
│ ■ Engine   412│                                                          │
│ ■ Web      380│                                                          │
│ …             │                                                          │
│               │                                                          │
│ ⚙ Settings    │                                                          │
│ (SM) Shivam   │                                                          │
└───────────────┴──────────────────────────────────────────────────────────┘
```

- Grid `240px 1fr`, full window height, `overflow-hidden` on the root.
- Sidebar `bg-sidebar border-r`, padding 12px. Wordmark "Tracker" Space Grotesk 600
  15px. Beside it a bordered 28px search button showing `⌘K` in mono; opens the palette.
- "New issue" `Button` full width, 32px, primary (the one cobalt button on screen),
  with a `C` kbd hint at the right in `text-primary-foreground/70`. `data-testid="new-issue"`
  goes on the dialog, the button is `nav-new`.
- Nav items 30px, radius 6, icon + label, `text-sidebar-foreground`;
  hover `bg-sidebar-accent`; active `bg-sidebar-accent text-foreground font-medium`
  and the icon `text-primary`. No left stripe. `data-testid="nav-issues"` etc.
- "PROJECTS" mono label, then 5 rows: project square, name, open count right-aligned
  in mono `text-muted-foreground`.
- Bottom: Settings item, then a row with the avatar and the display name.
- Toasts: bottom-right, 16px from the edges, 360px wide, stacked with 8px gap, at most 3.

### Issues list (`/issues`, `/my`, `/projects/[key]`)

- Header: page title + count in mono (`5,000`, or `214 of 5,000` when filtered).
- Toolbar: search `Input` 260px with a lucide Search icon inside left and a `/` kbd
  hint right (`data-testid="search"` on the input). Four filter `Select`s, outline,
  `size="sm"`; a set filter shows its value and a cobalt dot. Right-aligned: "Group"
  and "Sort" Selects in ghost style with mono labels.
- Group header (`group-header`, 32px, `bg-muted border-y`): chevron (collapses),
  status glyph, name 12px/500, count mono muted. Not sticky.
- Row (`row`; the highlighted row also `row-active`), 36px, a CSS grid:
  `16px 16px 76px 16px minmax(0,1fr) auto 20px 56px`, gap 10px, padding-x 16px,
  `border-b` hairline:
  checkbox · priority glyph · key (mono 12 muted) · status glyph · title (13/500 `text-foreground`,
  `truncate`) · up to 2 labels (6px dot + name, 11px muted; hidden in compact) ·
  avatar · updated (mono 11 muted, `Nov 21`).
  Hover `bg-row-hover`; highlighted `bg-row-active`; checked `bg-row-checked`
  (the checkbox is plain, `accent-color: var(--primary)`, 14px). Instant, no transition.
- After every 200 rows: a 36px row with "Show 200 more" (`show-more`) and
  "Show all 4,612" (`show-all`) as ghost buttons, plus the hidden count in mono.
- Empty: centred block 120px from the top: "No issues match ‘deploy’." 14/500 and
  a ghost "Clear filters" button. Not an illustration.
- Bulk bar (`bulk-bar`), when any row is checked: absolute, bottom 16px, centred
  (`left-1/2` with a negative margin of half its fixed 560px width, not a translate,
  so tests can click it), 44px, `bg-graphite text-graphite-foreground rounded-lg
  shadow-sm`, padding 6px 6px 6px 14px: "12 selected" mono 12 · Set status (Select,
  ghost on graphite) · Assign (Select) · Delete (ghost, destructive text) · ✕ clear.

### Detail pane (`/issues/[key]`, inside `issues/+layout.svelte`)

- The issues layout becomes `minmax(0,1fr) 480px`; the pane has `border-l bg-card`
  and its own `overflow-y-scroll`. The list keeps its highlight.
- Pane header 48px: key in mono 12, a "Copy link" ghost icon button, prev/next
  (↑ ↓ icon buttons, `j`/`k`), close ✕ (`Esc`).
- Title (`detail-title`): Space Grotesk 22/500, padding 20px. Click turns it into an
  `Input` of the same size; Enter saves, Escape cancels; empty shows the error
  "Title can't be empty. Type at least 3 characters." under it in `text-destructive` 12px.
- Description: 14/1.6 `text-ink-2`, max 62ch.
- Properties: a two-column grid (`96px 1fr`, rows 32px): label 12 muted · ghost
  `Select size="sm"` showing glyph + value. Status Select has `data-testid="status-select"`.
  Rows: Status, Priority, Assignee, Project, Estimate. Then Labels: the set labels as
  outline `Badge`s with their dot, and an "Edit" ghost button that shows a 3-column
  grid of `Checkbox` + label.
- `Tabs` (line style, bottom hairline): Comments (n) · Activity (n).
  Comment: avatar, name 13/500, date mono 11 muted, body 13 on the next line, 16px apart.
  Activity: one line each, 12px muted, glyph + "Shivam changed status to Done · Nov 21".
- Comment box at the end: `Input` + "Comment" button (disabled while empty), Enter posts.
- 404 (`+error.svelte` in this layout): "No issue ENG-9999." 16/500, then
  "It may have been deleted. Back to issues" with the link.

### Board (`/board`)

- Same header and toolbar (search + filters only; no Group/Sort).
- Columns (`board-column`): 280px, gap 12px, padding 16px, the row of columns in
  `overflow-x-scroll`; each column body in `overflow-y-scroll`. Column header 32px:
  status glyph, name 12/500, count mono muted. Column background `bg-muted` radius 8.
- Card (`card`): `bg-card border rounded-md p-2.5`, gap 6px:
  top line key (mono 11 muted) and avatar at the right; title 13/500 two lines max
  (`max-h-[2.9em] overflow-hidden`); bottom line priority glyph + up to 2 label dots
  + estimate in mono. Focused card: `ring-2 ring-ring` (instant). Hover: `border-input`.
  50 per column, then "+ 112 more in list" as a link to `/issues` with that status.

### Insights (`/insights`)

- Header: title, then at the right range `Tabs` (`4w · 12w · 26w`, segmented) and a
  project `Select`. Content padding 24px, max-w 1120px, gap 24px.
- Tiles, one row, asymmetric `2fr 1fr 1fr 1fr`: the wide one "Open" with a 48px
  number; then "Created", "Closed", "Median age" at 32px; label above in 12 muted,
  a one-line delta under in mono 11. `Card` with border, no shadow. Tooltip on each
  tile explains the measure (delay 800ms, as Bits UI does).
- "Created against closed": SVG bar chart, width of the content, 220px high,
  pairs of bars per week (`text-chart-1` created, `text-chart-2` closed, via
  `fill="currentColor"`), hairline gridlines (`text-border`), axis labels mono 11.
  Values as text labels above the tallest bars only. Clicking a week goes to
  `/issues?week=N`. Legend: two 8px squares + labels, top right.
- Two columns below (`3fr 2fr`): "Open by assignee" (top 10, name · bar in
  `bg-chart-1` on a `bg-muted` track · count mono) and "By priority" (one stacked
  bar, segments `chart-1`…`chart-5`, then the legend list with counts).
- "Oldest open": the 10 oldest, rendered with the list row (no checkbox column).

### Settings (`/settings`)

- Max-w 640, padding 32px. `Tabs`: General · Data.
- General: "Display name" `Label` above `Input` (32px), helper "Shown on comments
  and activity." below in 12 muted; error replaces it ("Name can't be empty."),
  helper keeps a reserved one-line height. Then switch rows, separated by hairlines:
  label 13/500 + one-line description 12 muted on the left, `Switch` on the right:
  "Compact rows", "Show labels in the list". Then "Default grouping" with a Select.
- Data: a definition line "5,000 issues · 37 local edits · saved to Application Support"
  in mono 12. "Simulate teammates" switch row ("Edits a random issue every 2 seconds").
  Buttons: "Export CSV" outline; "Reset to sample data" outline with destructive text,
  which opens a confirm `Dialog`: "Reset to sample data?" / "This deletes your 37
  local edits. The 5,000 sample issues stay." / Cancel · "Delete 37 edits" (destructive).

### Command palette (overlay; `palette`)

- shadcn `Dialog`, content 560px, top at 18vh (override the centring with `top-[18vh]
  translate-y-0`), `bg-graphite text-graphite-foreground border-graphite-2`, no close button.
- Input 48px, no border, 15px, placeholder "Type a command or search issues…",
  bottom hairline `border-graphite-2`.
- Results, max 360px, `overflow-y-scroll`: group titles in the mono label style,
  `text-graphite-muted` ("ACTIONS", "PROJECTS", "ISSUES"); items (`palette-item`) 36px,
  icon + text + right side (key in mono, or a kbd hint). Active item `bg-graphite-2`
  with the text `text-graphite-foreground` and icon `text-primary`.
- Footer 32px, mono 11 `text-graphite-muted`: `↑↓ move · ↵ open · esc close · > actions`.

### New issue (overlay; `new-issue`)

- `Dialog` 560px. Title "New issue" Space Grotesk 16/500.
- "Title" Label + Input (16px text) with helper "3 to 200 characters."; error replaces it.
- A row of four `Select size="sm"` (outline): Project · Status · Priority · Assignee.
- "Labels": 3-column grid of `Checkbox` + dot + name.
- Footer: ghost "Cancel", primary "Create issue" with a `⌘↵` hint. Validation on
  submit, then live once touched.

### Shortcuts (`?`)

- `Dialog` 480px, two columns of rows: kbd (mono 11, `border rounded px-1.5 bg-muted`)
  and description 13. Groups: Navigation, List, Issue, Global.

### Toast (`toast`)

- 44px min, `bg-popover border rounded-lg shadow-sm`, padding 10px 12px, 13px text,
  a 16px glyph left (status glyph when relevant), action "Undo" or "Open" as a
  `text-primary` text button, ✕ at the right. Enter: 160ms `opacity` + `translateY(8px)`
  ease-out, once. Gone after 5s. No exit animation.

## States (every interactive element)

The shadcn components carry hover, focus-visible ring, active, disabled and
`aria-invalid`. For the custom ones:

| Element | Hover | Focus-visible | Active / selected | Disabled | Error |
|---|---|---|---|---|---|
| Row | `bg-row-hover` | `ring-2 ring-ring ring-inset` | `bg-row-active` (highlight), `bg-row-checked` | — | — |
| Card | `border-input` | `ring-2 ring-ring` | same as focus | — | — |
| Nav item | `bg-sidebar-accent` | ring | `bg-sidebar-accent font-medium` + cobalt icon | — | — |
| Palette item | `bg-graphite-2` | — (focus stays in input) | `bg-graphite-2` | — | — |
| Group header | `bg-accent` | ring | collapsed: chevron rotated (instant) | — | — |
| Comment button | shadcn | shadcn | shadcn | while empty | — |
| Title edit | — | ring | — | — | message + `aria-invalid` |

Loading: there is none worth showing: everything is local. Saving is silent.
Success: silent, except the Undo toast after an edit.

## Copy

Verbs name the result: "Create issue", "Set status", "Assign", "Delete 12 issues",
"Reset to sample data", "Show 200 more". Errors say what broke and what to do.
No exclamation marks, no "Oops". Counts are exact. Dates are `Nov 21` in rows and
`21 Nov 2026, 14:05` in the detail pane.
