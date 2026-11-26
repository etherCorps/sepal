//! `examples/tracker`, the benchmark app (docs/tracker-plan.md): a sidebar
//! shell over the file router with `dom: true`. One test binary; the screens
//! have a module each, and this file holds the shared helpers, the shell and
//! the tests that cross screens.
//!
//! Each test runs under its own `manifest.id`, so edits.json and
//! settings.json go to a data directory of their own, deleted at the end.
//!
//! Build the bundle first: `pnpm run build` in the repository root.
//! `TRACKER_DIST=/some/dir` runs the tests against another build of the app.

mod bench;
mod board;
mod list;
mod overlays;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use keyboard_types::Modifiers;
use sepal::SvelteDocument;
use sepal::app::App;

/// The tracker with a data directory of its own, removed when dropped.
struct Tracker {
    doc: SvelteDocument,
    data_dir: PathBuf,
}

impl Drop for Tracker {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

impl std::ops::Deref for Tracker {
    type Target = SvelteDocument;
    fn deref(&self) -> &SvelteDocument {
        &self.doc
    }
}
impl std::ops::DerefMut for Tracker {
    fn deref_mut(&mut self) -> &mut SvelteDocument {
        &mut self.doc
    }
}

/// Where the host puts the `data` directory of an app id (crates/host/src/api/fs.rs).
fn data_dir(id: &str) -> PathBuf {
    let home = || {
        PathBuf::from(
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .unwrap(),
        )
    };
    let base = if cfg!(target_os = "macos") {
        home().join("Library/Application Support")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA").unwrap())
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local/share"))
    };
    base.join(id)
}

fn new_id() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!(
        "com.example.tracker-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn load() -> Tracker {
    load_as(new_id())
}

/// The built app: `TRACKER_DIST`, or examples/tracker/dist.
fn dist() -> PathBuf {
    std::env::var_os("TRACKER_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/tracker/dist"
            ))
        })
}

/// The app under a given id: a second load with the same id sees the first one's files.
fn load_as(id: String) -> Tracker {
    let mut app =
        App::from_dir(&dist()).expect("run `pnpm run build` in the repository root first");
    let data_dir = data_dir(&id);
    app.manifest.id = Some(id);
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    doc.layout(1280, 800);
    // The first navigation, and reading edits.json and settings.json, which
    // the host does on a thread.
    until(&mut doc, || false, "globalThis.__tracker.stats().loaded");
    doc.resolve();
    Tracker { doc, data_dir }
}

fn js(doc: &SvelteDocument, code: &str) -> String {
    doc.js().eval_string(code).unwrap()
}

/// Turn until `done()` or the JavaScript `condition` holds. File access runs
/// on the host's threads, in real time, so this sleeps between turns.
#[track_caller]
fn until(doc: &mut SvelteDocument, done: impl Fn() -> bool, condition: &str) {
    let started = Instant::now();
    loop {
        doc.settle();
        doc.resolve();
        if done() || js(doc, &format!("String(!!({condition}))")) == "true" {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "{condition} never held"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Turn until the app is at `path` and `selector` is on the page. An error
/// page waits for SvelteKit's version check (`updated.check()`), a `fetch`
/// the host answers on a thread, in real time: `doc.wait` alone races it.
#[track_caller]
fn until_shown(doc: &mut SvelteDocument, path: &str, selector: &str) {
    until(
        doc,
        || false,
        &format!("location.pathname === '{path}' && document.querySelector('{selector}') !== null"),
    );
}

/// Turn until the file at `path` contains `contains`, and return it.
#[track_caller]
fn until_file(doc: &mut SvelteDocument, path: PathBuf, contains: &str) -> String {
    let check = || {
        std::fs::read_to_string(&path)
            .map(|text| text.contains(contains))
            .unwrap_or(false)
    };
    until(doc, check, "false");
    std::fs::read_to_string(&path).unwrap()
}

#[track_caller]
fn text(doc: &SvelteDocument, selector: &str) -> String {
    let node = doc
        .query(selector)
        .unwrap_or_else(|| panic!("nothing matches {selector}"));
    doc.text(node)
}

#[track_caller]
fn click(doc: &mut SvelteDocument, selector: &str) {
    let node = doc
        .query(selector)
        .unwrap_or_else(|| panic!("nothing matches {selector}"));
    click_node(doc, node);
}

fn click_node(doc: &mut SvelteDocument, node: blitz_dom::NodeId) {
    // As a user would: what is not under the pointer at its middle (scrolled
    // out of its box, which clips clicks, or under a toast) is scrolled to
    // the middle of its box first.
    let (x, y, width, height) = doc.rect(node).expect("node has no layout");
    let hit = doc
        .base()
        .hit((x + width / 2.0) as f32, (y + height / 2.0) as f32)
        .map(|hit| hit.node_id);
    let reaches = |mut at: Option<blitz_dom::NodeId>| {
        while let Some(id) = at {
            if id == node {
                return true;
            }
            at = doc.base().get_node(id).and_then(|n| n.parent);
        }
        false
    };
    if !reaches(hit) {
        doc.scroll_into_view(node);
    }
    doc.click(node);
    doc.settle();
    doc.resolve();
    // SvelteKit lets the page repaint before it follows a link: a
    // `requestAnimationFrame`, then a `setTimeout(0)`, which runs a turn later.
    doc.wait(0.04);
}

/// Scroll the list back to its top: "Show more" and "Show all" are at its
/// end, and clicking one scrolls down to it first (`click_node`).
fn list_to_top(doc: &mut SvelteDocument) {
    js(
        doc,
        "document.querySelector('[data-testid=list]').scrollTop = 0, ''",
    );
    doc.settle();
    doc.wait(0.04);
}

fn count(doc: &SvelteDocument, selector: &str) -> usize {
    doc.query_all(selector).len()
}

fn number(text: &str) -> usize {
    text.replace(',', "").trim().parse().unwrap()
}

fn focused(doc: &SvelteDocument) -> Option<String> {
    doc.query(":focus")
        .and_then(|node| doc.attr(node, "data-testid"))
}

/// The keys of the mounted rows, in screen order.
fn keys(doc: &SvelteDocument) -> Vec<String> {
    doc.query_all("[data-testid=row]")
        .into_iter()
        .map(|node| doc.attr(node, "data-key").unwrap())
        .collect()
}

/// The keys of every row the list has paged in (`Show more`, `Show all`), in
/// screen order. The list mounts only those in view: [`keys`].
fn order(doc: &SvelteDocument) -> Vec<String> {
    js(doc, "globalThis.__tracker.order().join()")
        .split(',')
        .filter(|key| !key.is_empty())
        .map(String::from)
        .collect()
}

/// How many rows the list has paged in.
fn paged(doc: &SvelteDocument) -> usize {
    number(&js(doc, "String(globalThis.__tracker.stats().paged)"))
}

/// Where the list is scrolled to.
fn scroll_top(doc: &SvelteDocument) -> f64 {
    js(
        doc,
        "String(document.querySelector('[data-testid=list]').scrollTop)",
    )
    .parse()
    .unwrap()
}

fn row(key: &str) -> String {
    format!("[data-testid=row][data-key={key}]")
}

fn issue_field(doc: &SvelteDocument, key: &str, field: &str) -> String {
    js(
        doc,
        &format!("String(globalThis.__tracker.issue('{key}')?.{field})"),
    )
}

/// The `translate(x, y)` Bits UI puts on an open list's wrapper; `doc.rect` leaves it out.
#[track_caller]
fn floating_offset(doc: &SvelteDocument) -> (f64, f64) {
    let style = doc
        .attr(
            doc.query("[data-bits-floating-content-wrapper]")
                .expect("a Select is open"),
            "style",
        )
        .unwrap();
    let inner = style.split("translate(").nth(1).unwrap();
    let inner = &inner[..inner.find(')').unwrap()];
    let mut parts = inner
        .split(',')
        .map(|part| part.trim().trim_end_matches("px").parse::<f64>().unwrap());
    (parts.next().unwrap(), parts.next().unwrap())
}

/// Open a Select by its trigger and click one of its items, through hit testing.
#[track_caller]
fn pick(doc: &mut SvelteDocument, trigger: &str, value: &str) {
    click(doc, trigger);
    doc.wait(0.2);
    let (x, y) = floating_offset(doc);
    let item = doc
        .rect(
            doc.query(&format!("[data-select-item][data-value='{value}']"))
                .unwrap_or_else(|| panic!("no item {value} in {trigger}")),
        )
        .unwrap();
    doc.click_at(
        (x + item.0 + item.2 / 2.0) as f32,
        (y + item.1 + item.3 / 2.0) as f32,
    );
    doc.wait(0.4);
    assert!(
        doc.query("[data-select-content]").is_none(),
        "the Select closed"
    );
}

/// Click inside a centred Dialog: its `translate(-50%, -50%)` is lost on
/// `doc.rect` and kept by hit testing.
#[track_caller]
fn click_in_dialog(doc: &mut SvelteDocument, selector: &str) {
    let (_, _, dialog_width, dialog_height) = doc
        .rect(doc.query("[data-slot=dialog-content]").unwrap())
        .unwrap();
    let (x, y, width, height) = doc.rect(doc.query(selector).unwrap()).unwrap();
    doc.click_at(
        (x - dialog_width / 2.0 + width / 2.0) as f32,
        (y - dialog_height / 2.0 + height / 2.0) as f32,
    );
    doc.settle();
    doc.resolve();
}

/// The newest toast's action: the stack enters with a 160 ms slide, so let it land first.
fn undo_from_toast(doc: &mut SvelteDocument) {
    doc.wait(0.3);
    let newest = *doc
        .query_all("[data-testid=toast-action]")
        .last()
        .expect("a toast with an action");
    doc.click(newest);
    doc.settle();
    doc.resolve();
}

#[test]
fn shell_mounts_and_navigates() {
    let mut doc = load();

    // `/` redirects to the list.
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,000");
    assert!(doc.query("[data-testid=row]").is_some(), "rows are mounted");
    assert!(
        doc.query("[data-testid=nav-issues]").is_some()
            && doc.query("[data-testid=nav-board]").is_some()
            && doc.query("[data-testid=nav-insights]").is_some()
            && doc.query("[data-testid=nav-settings]").is_some()
    );
    assert_eq!(
        doc.query_all("[data-testid^=nav-project-]").len(),
        5,
        "five projects in the sidebar"
    );

    // The harness hook outlives the components (QuickJS frees what nothing references).
    assert_eq!(js(&doc, "String(globalThis.__tracker.stats().n)"), "5000");
    assert_eq!(
        js(&doc, "String(globalThis.__tracker.stats().loaded)"),
        "true"
    );

    click(&mut doc, "[data-testid=nav-board]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Board");
    assert!(doc.query("[data-testid=row]").is_none(), "the list went");

    click(&mut doc, "[data-testid=nav-insights]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Insights");

    click(&mut doc, "[data-testid=nav-issues]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert!(doc.query("[data-testid=row]").is_some());

    // A project link is the list with a fixed filter.
    click(&mut doc, "[data-testid=nav-project-ENG]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Engine");
    let open: usize = text(&doc, "[data-testid=nav-project-ENG] span:last-child")
        .replace(',', "")
        .parse()
        .unwrap();
    assert!(open > 0 && open < 5000);
}

#[test]
fn reseed_changes_the_count() {
    let mut doc = load();
    js(&doc, "globalThis.__tracker.reseed(1000), ''");
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=page-count]"), "1,000");
}

#[test]
fn shell_keys() {
    let mut doc = load();

    // A g-chord navigates.
    doc.press("g");
    doc.press("b");
    doc.settle();
    doc.resolve();
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Board");

    // Cmd+K opens the palette; Escape closes it (Bits UI's Dialog handles it).
    doc.press_with("k", Modifiers::META);
    doc.wait(0.3);
    assert!(doc.query("[data-testid=palette]").is_some(), "palette open");
    doc.press("Escape");
    doc.wait(0.4);
    assert!(
        doc.query("[data-testid=palette]").is_none(),
        "palette closed"
    );

    // c opens New issue; typing in its input is text, not shortcuts.
    doc.press("c");
    doc.wait(0.3);
    assert!(doc.query("[data-testid=new-issue]").is_some());
    assert_eq!(
        doc.attr(doc.query(":focus").unwrap(), "data-testid")
            .as_deref(),
        Some("new-issue-title"),
        "the dialog focuses its input"
    );
    doc.type_text("gi c");
    assert_eq!(
        text(&doc, "[data-testid=page-title]"),
        "Board",
        "g i typed into the input did not navigate"
    );
    doc.press("Escape");
    doc.wait(0.4);
    assert!(doc.query("[data-testid=new-issue]").is_none());
}

#[test]
fn edits_survive_a_restart() {
    let id = new_id();
    let mut doc = load_as(id.clone());
    js(
        &doc,
        "globalThis.__tracker.update('ENG-1', { title: 'Renamed in a test' }), ''",
    );
    doc.settle();
    // One 500 ms timer, then the write.
    doc.wait(0.6);
    let file = doc.data_dir.join("edits.json");
    until(&mut doc, || file.exists(), "false");
    let saved =
        std::fs::read_to_string(doc.data_dir.join("edits.json")).expect("edits.json written");
    assert!(saved.contains("Renamed in a test"), "{saved}");
    assert_eq!(js(&doc, "String(globalThis.__tracker.stats().edits)"), "1");

    // A second start, while the first still holds the directory, applies the overlay over the seed.
    let again = load_as(id);
    assert_eq!(
        js(&again, "String(globalThis.__tracker.stats().edits)"),
        "1"
    );
    let data_dir = again.data_dir.clone();
    drop(again);
    assert!(!data_dir.exists(), "the test's data directory is removed");
}

#[test]
fn shift_click_checks_a_range_and_the_bulk_bar_saves_it() {
    let mut doc = load();
    let rows = keys(&doc);

    // A click checks one row and sets the anchor; Shift+click checks up to it.
    click(&mut doc, &format!("{} input", row(&rows[2])));
    let to = doc.query(&format!("{} input", row(&rows[6]))).unwrap();
    doc.click_with(to, Modifiers::SHIFT);
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "5 selected");
    // Shift+click on a box the range already checked keeps it ticked.
    let inside = doc.query(&format!("{} input", row(&rows[4]))).unwrap();
    doc.click_with(inside, Modifiers::SHIFT);
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "5 selected");
    for key in &rows[2..=6] {
        assert!(
            doc.query(&format!("{} input:checked", row(key))).is_some(),
            "{key} ticked"
        );
    }
    // Shift+click on a row's link extends the selection instead of opening it.
    let link = doc.query(&format!("{} a", row(&rows[8]))).unwrap();
    doc.click_with(link, Modifiers::SHIFT);
    doc.wait(0.1);
    assert!(doc.query("[data-testid=pane]").is_none(), "no navigation");
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "7 selected");
    assert!(
        doc.query(&format!("{} input:checked", row(&rows[1])))
            .is_none()
    );

    pick(&mut doc, "[data-testid=bulk-status]", "done");
    for key in &rows[2..=8] {
        assert_eq!(issue_field(&doc, key, "status"), "done", "{key}");
    }
    assert_eq!(
        text(&doc, "[data-testid=toast-text]"),
        "Moved 7 issues to Done"
    );
    // The rows moved to the Done group, which comes after the first page.
    assert!(doc.query(&row(&rows[2])).is_none());

    // One 500 ms timer later the overlay is on disk.
    doc.wait(0.6);
    let file = doc.data_dir.join("edits.json");
    let saved = until_file(&mut doc, file, &rows[8]);
    for key in &rows[2..=8] {
        assert!(saved.contains(&format!("\"{key}\":{{")), "{key} in {saved}");
    }
    assert!(saved.contains("\"status\":\"done\""));
}

#[test]
fn a_status_change_reaches_the_row_and_the_board() {
    let mut doc = load();
    // Ungrouped, newest update first: the edited row comes to the top.
    pick(&mut doc, "[data-testid=group-by]", "none");
    // A row further down, still on screen.
    let key = keys(&doc)[10].clone();
    let was = issue_field(&doc, &key, "status");
    let (to, name) = if was == "progress" {
        ("review", "In Review")
    } else {
        ("progress", "In Progress")
    };
    click(&mut doc, &format!("{} a", row(&key)));
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), key);
    assert!(
        doc.query(&format!("{} svg.text-status-{was}", row(&key)))
            .is_some()
    );

    pick(&mut doc, "[data-testid=status-select]", to);
    assert_eq!(
        text(&doc, "[data-testid=toast-text]"),
        format!("Moved {key} to {name}")
    );
    // The row draws the new glyph and moves to the top; the pane stays on it.
    assert!(
        doc.query(&format!("{} svg.text-status-{to}", row(&key)))
            .is_some(),
        "the row follows"
    );
    assert_eq!(keys(&doc)[0], key);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), key);

    click(&mut doc, "[data-testid=nav-board]");
    let column =
        |status: &str| format!("[data-testid=board-column][data-status={status}] [data-key={key}]");
    assert!(
        doc.query(&column(to)).is_some(),
        "the board has it in {name}"
    );

    // Undo from the toast, on another screen than the edit.
    undo_from_toast(&mut doc);
    assert_eq!(issue_field(&doc, &key, "status"), was);
    assert!(doc.query(&column(to)).is_none());
}

#[test]
fn unknown_keys_and_routes_are_404s() {
    let mut doc = load();

    // Inside the issues layout: the list stays, the pane says what is missing.
    js(&doc, "globalThis.__tracker.goto('/issues/ENG-99999'), ''");
    until_shown(&mut doc, "/issues/ENG-99999", "[data-testid=error-message]");
    assert_eq!(
        text(&doc, "[data-testid=error-message]"),
        "No issue ENG-99999."
    );
    assert!(
        doc.query("[data-testid=pane] [data-testid=error-message]")
            .is_some()
    );
    assert_eq!(paged(&doc), 200, "the list stays");
    assert!(doc.query("[data-testid=row]").is_some());
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    click(&mut doc, "[data-testid=pane] a[href='/issues']");
    doc.wait(0.1);
    assert!(doc.query("[data-testid=pane]").is_none());

    // An unknown project and an unknown route: the root error page, in the shell.
    for href in ["/projects/NOPE", "/nowhere"] {
        js(&doc, &format!("globalThis.__tracker.goto('{href}'), ''"));
        until_shown(&mut doc, href, "[data-testid=error-message]");
        assert_eq!(
            text(&doc, "[data-testid=page-title]"),
            "Not found",
            "{href}"
        );
        assert!(doc.query("[data-testid=row]").is_none());
        assert!(
            doc.query("[data-testid=nav-issues]").is_some(),
            "the sidebar stays"
        );
    }
    click(&mut doc, "[data-testid=nav-issues]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
}

/// docs/tracker-plan.md, Step 1: the probe navigated only from clicks.
#[test]
fn a_navigation_from_a_timer_while_a_select_is_open() {
    let mut doc = load();
    click(&mut doc, "[data-testid=filter-status]");
    doc.wait(0.2);
    assert!(doc.query("[data-select-content]").is_some());

    js(
        &doc,
        "setTimeout(() => globalThis.__tracker.goto('/board'), 50), ''",
    );
    doc.wait(0.5);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Board");
    assert!(
        doc.query("[data-select-content]").is_none()
            && doc.query("[data-bits-floating-content-wrapper]").is_none(),
        "the list went with its page"
    );
    // The page takes clicks again: the first one navigates.
    click(&mut doc, "[data-testid=nav-insights]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Insights");
    // And Selects still open and pick.
    pick(&mut doc, "[data-testid=insights-project]", "WEB");
    assert_eq!(text(&doc, "[data-testid=insights-project]"), "Web");

    // In the pane: the page stays mounted from one issue to the next, so the
    // open Select has to follow the issue, not keep the old one's value.
    click(&mut doc, "[data-testid=nav-issues]");
    let rows = keys(&doc);
    click(&mut doc, &format!("{} a", row(&rows[0])));
    doc.wait(0.1);
    click(&mut doc, "[data-testid=status-select]");
    doc.wait(0.2);
    js(
        &doc,
        &format!(
            "setTimeout(() => globalThis.__tracker.goto('/issues/{}'), 50), ''",
            rows[1]
        ),
    );
    doc.wait(0.5);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), rows[1]);
    if doc.query("[data-select-content]").is_some() {
        doc.press("Escape");
        doc.wait(0.4);
    }
    pick(&mut doc, "[data-testid=status-select]", "review");
    assert_eq!(issue_field(&doc, &rows[1], "status"), "review");
    assert_eq!(
        issue_field(&doc, &rows[0], "status"),
        "backlog",
        "the previous issue is untouched"
    );
}

#[test]
fn nested_layouts_keep_what_they_share() {
    let mut doc = load();
    let sidebar = doc.query("[data-testid=nav-issues]").unwrap();
    let list = doc.query("[data-testid=list]").unwrap();
    click(&mut doc, "[data-testid=search]");
    doc.type_text("deploy");
    let rows = keys(&doc);

    // /issues -> /issues/A -> /issues/B -> /issues: the issues layout, and the
    // list in it, stay mounted; only the pane comes and goes.
    for key in [&rows[0], &rows[1]] {
        js(
            &doc,
            &format!("globalThis.__tracker.goto('/issues/{key}'), ''"),
        );
        doc.wait(0.1);
        assert_eq!(text(&doc, "[data-testid=detail-key]"), *key);
        assert_eq!(doc.query("[data-testid=list]"), Some(list), "the same list");
    }
    // The search still has the focus: the first Escape leaves it, the second closes the pane.
    doc.press("Escape");
    doc.press("Escape");
    doc.wait(0.1);
    assert!(doc.query("[data-testid=pane]").is_none());
    assert_eq!(doc.query("[data-testid=list]"), Some(list));
    assert_eq!(text(&doc, "[data-testid=page-count]"), "652 of 5,000");

    // /board is outside the issues layout: the list goes, the shell stays.
    click(&mut doc, "[data-testid=nav-board]");
    assert!(doc.query("[data-testid=list]").is_none());
    assert_eq!(doc.query("[data-testid=nav-issues]"), Some(sidebar));

    // /my renders the same list from its own page, not the issues layout: a new one.
    click(&mut doc, "[data-testid=nav-my]");
    let mine = doc.query("[data-testid=list]").unwrap();
    assert_ne!(mine, list);
    assert_eq!(doc.query("[data-testid=nav-issues]"), Some(sidebar));
    // Opening an issue from it moves into the issues layout.
    let key = keys(&doc)[0].clone();
    click(&mut doc, &format!("{} a", row(&key)));
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert_eq!(text(&doc, "[data-testid=detail-key]"), key);
    assert_eq!(doc.query("[data-testid=nav-issues]"), Some(sidebar));
}

#[test]
fn created_keys_are_never_reused() {
    let mut doc = load();
    let make = |doc: &mut Tracker| {
        let key = js(
            doc,
            "globalThis.__tracker.create({ title: 'New one', project: 'ENG' }).key",
        );
        doc.settle();
        key
    };
    let first = make(&mut doc);
    // Moved to another project, it keeps its key; the next ENG issue gets another.
    js(
        &doc,
        &format!("globalThis.__tracker.update('{first}', {{ project: 'WEB' }}), ''"),
    );
    let second = make(&mut doc);
    assert_ne!(first, second);
    // Edited, then deleted: its activity outlives it, so its key is not handed out again.
    js(
        &doc,
        &format!("globalThis.__tracker.update('{second}', {{ priority: 'urgent' }}), ''"),
    );
    js(
        &doc,
        &format!("globalThis.__tracker.delete(['{second}']), ''"),
    );
    doc.settle();
    assert_eq!(issue_field(&doc, &second, "key"), "undefined");
    let third = make(&mut doc);
    assert_ne!(third, second);
    assert_ne!(third, first);
}

#[test]
fn undo_keeps_what_a_teammate_changed_since() {
    let mut doc = load();
    let key = keys(&doc)[0].clone();
    let priority = issue_field(&doc, &key, "priority");
    let other = if priority == "urgent" {
        "low"
    } else {
        "urgent"
    };
    js(
        &doc,
        &format!("globalThis.__tracker.update('{key}', {{ priority: '{other}' }}), ''"),
    );
    js(
        &doc,
        &format!("globalThis.__tracker.bulk_update(['{key}'], {{ status: 'done' }}, 'p2'), ''"),
    );
    js(&doc, "globalThis.__tracker.undo(), ''");
    doc.settle();
    assert_eq!(issue_field(&doc, &key, "priority"), priority);
    assert_eq!(
        issue_field(&doc, &key, "status"),
        "done",
        "the teammate's edit stays"
    );
    let last = js(
        &doc,
        &format!(
            "(a => a[a.length - 1].author + ' ' + a[a.length - 1].text)(globalThis.__tracker.thread('{key}').activity)"
        ),
    );
    assert_eq!(last, "p2 changed status to Done");
}

#[test]
fn each_toast_undoes_its_own_edit() {
    let mut doc = load();
    let rows = keys(&doc);
    click(&mut doc, &format!("{} input", row(&rows[0])));
    let assignee = issue_field(&doc, &rows[0], "assignee");
    let status = issue_field(&doc, &rows[0], "status");
    pick(&mut doc, "[data-testid=bulk-status]", "done");
    let other = if assignee == "p1" { "p2" } else { "p1" };
    pick(&mut doc, "[data-testid=bulk-assign]", other);
    assert_eq!(count(&doc, "[data-testid=toast]"), 2);
    // The older toast's Undo reverts the status, not the newer assignment.
    doc.wait(0.3);
    let first = doc.query_all("[data-testid=toast-action]")[0];
    doc.click(first);
    doc.settle();
    assert_eq!(issue_field(&doc, &rows[0], "status"), status);
    assert_eq!(issue_field(&doc, &rows[0], "assignee"), other);
}

/// Leaving a page frees what it showed without waiting for a GC that an idle
/// app never triggers. The rows of "Show all" detach as one large tree; the
/// host collects garbage once, about a second later, and the nodes go. The
/// list mounts only the rows in view, so the window is made 40,000px tall for
/// it to mount a thousand.
#[test]
fn a_long_list_is_freed_after_leaving_it() {
    let mut doc = load();
    doc.layout(1280, 40_000);
    click(&mut doc, "[data-testid=show-all]");
    doc.resolve();
    assert!(count(&doc, "[data-testid=row]") > 1000);
    click(&mut doc, "[data-testid=nav-board]");
    doc.resolve();
    assert!(
        doc.live_nodes() > 20_000,
        "the rows are detached but not yet collected"
    );

    doc.wait(0.5);
    doc.resolve();
    assert!(
        doc.live_nodes() > 20_000,
        "no collection inside the navigation's second"
    );

    doc.wait(0.6);
    doc.resolve();
    assert!(
        doc.live_nodes() < 10_000,
        "still holding {} nodes after leaving the list",
        doc.live_nodes()
    );
    assert!(doc.base().tree().len() < 12_000);
}
