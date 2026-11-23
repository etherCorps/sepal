//! The overlays (command palette, New issue, shortcuts, toasts) and the
//! Settings screen. Nothing here presses Export CSV: it opens the system's
//! save dialog.

use super::*;

#[test]
fn palette_searches_and_navigates() {
    let mut doc = load();

    doc.press_with("k", Modifiers::META);
    doc.wait(0.3);
    assert!(doc.query("[data-testid=palette]").is_some());
    assert_eq!(focused(&doc).as_deref(), Some("palette-input"));
    // No query: actions and projects, no issues.
    assert!(text(&doc, "[data-testid=palette]").contains("Go to Board"));
    assert!(!text(&doc, "[data-testid=palette]").contains("ISSUES"));

    // The exact key comes first, out of all 5000.
    doc.type_text("eng-12");
    doc.settle();
    let items = doc.query_all("[data-testid=palette-item]");
    assert!(items.len() <= 8, "at most 8 issues: {}", items.len());
    assert!(text(&doc, "[data-active]").ends_with("ENG-12"));

    // Arrows move the highlight and wrap.
    doc.press("ArrowDown");
    doc.settle();
    assert!(!text(&doc, "[data-active]").ends_with("ENG-12"));
    doc.press("ArrowUp");
    doc.press("ArrowUp");
    doc.settle();
    assert_eq!(
        doc.query("[data-active]"),
        items.last().copied(),
        "up from the first is the last"
    );
    doc.press("ArrowDown");
    doc.settle();

    // Enter opens it and closes the palette.
    doc.press("Enter");
    doc.wait(0.4);
    assert!(doc.query("[data-testid=palette]").is_none());
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert!(
        doc.query("[data-testid=detail-title]").is_some(),
        "the pane is open"
    );

    // '>' limits the list to actions.
    doc.press_with("k", Modifiers::META);
    doc.wait(0.3);
    assert_eq!(
        doc.query_all("[data-testid=palette-item]").len(),
        13,
        "8 actions and 5 projects"
    );
    doc.type_text(">");
    doc.settle();
    assert_eq!(doc.query_all("[data-testid=palette-item]").len(), 8);
    doc.type_text("insights");
    doc.settle();
    assert_eq!(doc.query_all("[data-testid=palette-item]").len(), 1);
    doc.press("Enter");
    doc.wait(0.4);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Insights");

    // Escape closes it and gives the focus back.
    click(&mut doc, "[data-testid=nav-palette]");
    doc.wait(0.3);
    doc.type_text("web");
    doc.press("Escape");
    doc.wait(0.4);
    assert!(doc.query("[data-testid=palette]").is_none());
    assert_eq!(focused(&doc).as_deref(), Some("nav-palette"));

    // It opens empty again.
    click(&mut doc, "[data-testid=nav-palette]");
    doc.wait(0.3);
    assert_eq!(doc.query_all("[data-testid=palette-item]").len(), 13);
}

#[test]
fn palette_opens_new_issue_after_closing() {
    let mut doc = load();
    doc.press_with("k", Modifiers::META);
    doc.wait(0.3);
    doc.type_text("create");
    doc.press("Enter");
    doc.wait(0.5);
    assert!(doc.query("[data-testid=palette]").is_none());
    assert!(doc.query("[data-testid=new-issue]").is_some());
    assert_eq!(
        focused(&doc).as_deref(),
        Some("new-issue-title"),
        "the palette's focus return did not land after the new dialog took it"
    );
}

#[test]
fn new_issue_validates_then_creates() {
    let mut doc = load();
    doc.press("c");
    doc.wait(0.3);
    assert_eq!(focused(&doc).as_deref(), Some("new-issue-title"));
    assert_eq!(
        text(&doc, "[data-testid=new-issue-error]"),
        "3 to 200 characters."
    );

    // Tab stays inside the dialog.
    for _ in 0..25 {
        doc.press("Tab");
        doc.settle();
        assert!(
            doc.query("[data-testid=new-issue]:focus, [data-testid=new-issue] :focus")
                .is_some(),
            "focus left the dialog"
        );
    }
    click(&mut doc, "[data-testid=new-issue-title]");

    // An empty submit shows the error and the dialog stays.
    doc.press("Enter");
    doc.settle();
    assert!(doc.query("[data-testid=new-issue]").is_some());
    assert_eq!(
        text(&doc, "[data-testid=new-issue-error]"),
        "Title can't be empty. Type at least 3 characters."
    );
    assert!(
        doc.attr(
            doc.query("[data-testid=new-issue-title]").unwrap(),
            "aria-invalid"
        )
        .is_some()
    );

    // Then it is checked as you type.
    doc.type_text("ab");
    doc.settle();
    assert_eq!(
        text(&doc, "[data-testid=new-issue-error]"),
        "Type at least 3 characters."
    );
    doc.type_text("c deploy");
    doc.settle();
    assert_eq!(
        text(&doc, "[data-testid=new-issue-error]"),
        "3 to 200 characters."
    );

    // A label, then Cmd+Enter.
    // New issue has no translate (Selects are placed from measured boxes), so a plain click lands.
    click(&mut doc, "#new-issue-label-l1");
    assert_eq!(
        doc.attr(doc.query("#new-issue-label-l1").unwrap(), "data-state")
            .as_deref(),
        Some("checked")
    );
    // A Select inside the dialog: its list sits under the trigger and above the
    // dialog, so a click on an item that overlaps the dialog picks it.
    click(&mut doc, "[data-testid=new-issue-assignee]");
    doc.wait(0.3);
    let (dx, dy) = floating_offset(&doc);
    let (x, y, w, h) = doc
        .rect(doc.query("[data-select-content] [data-value=p2]").unwrap())
        .unwrap();
    let (tx, _, _, _) = doc
        .rect(doc.query("[data-testid=new-issue-assignee]").unwrap())
        .unwrap();
    // Item-aligned: the list moves left by its own padding to line its text up with the trigger's.
    assert!(
        (dx - tx).abs() < 40.0,
        "the list is under its trigger, not where the untransformed dialog would put it: {dx} vs {tx}"
    );
    doc.click_at((x + dx + w / 2.0) as f32, (y + dy + h / 2.0) as f32);
    doc.wait(0.3);
    assert!(doc.query("[data-select-content]").is_none());
    assert!(text(&doc, "[data-testid=new-issue-assignee]").ends_with("Ada Lovelace"));

    doc.press_with("Enter", Modifiers::META);
    doc.wait(0.4);
    assert!(doc.query("[data-testid=new-issue]").is_none());
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,001");
    assert!(text(&doc, "[data-testid=toast]").starts_with("Created ENG-1501"));
    // The first row (Backlog, the default status, is the first group), highlighted.
    assert_eq!(keys(&doc)[0], "ENG-1501");
    assert!(text(&doc, "[data-testid=row-active]").starts_with("ENG-1501"));

    // The toast's Open goes to it.
    click(&mut doc, "[data-testid=toast-action]");
    doc.wait(0.3);
    assert_eq!(text(&doc, "[data-testid=detail-title]"), "abc deploy");
    assert!(doc.query("[data-testid=toast]").is_none());

    // Opening again starts from a blank form.
    doc.press("Escape");
    doc.settle();
    doc.press("c");
    doc.wait(0.3);
    assert_eq!(
        text(&doc, "[data-testid=new-issue-error]"),
        "3 to 200 characters."
    );
    assert_eq!(
        doc.attr(doc.query("#new-issue-label-l1").unwrap(), "data-state")
            .as_deref(),
        Some("unchecked")
    );
}

#[test]
fn shortcuts_dialog() {
    let mut doc = load();
    doc.press("?");
    doc.wait(0.3);
    let list = text(&doc, "[data-testid=shortcuts]");
    assert!(
        list.contains("Command palette") && list.contains("G then I"),
        "{list}"
    );
    doc.press("Escape");
    doc.wait(0.4);
    assert!(doc.query("[data-testid=shortcuts]").is_none());
}

#[test]
fn settings_save_and_survive_a_restart() {
    let id = new_id();
    let mut doc = load_as(id.clone());
    click(&mut doc, "[data-testid=nav-settings]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Settings");

    // A switch saves settings.json and says so.
    click(&mut doc, "[data-testid=settings-compact]");
    let file = doc.data_dir.join("settings.json");
    until_file(&mut doc, file.clone(), "\"compact\":true");
    // The toast follows the write's answer, a turn or more after the file is on disk.
    until(
        &mut doc,
        || false,
        "document.querySelector('[data-testid=toast-text]')?.textContent === 'Compact rows on'",
    );

    // Three more toasts: the oldest goes, three stay. Each toast waits for its
    // write, which the host finishes on a thread, so wait for all three.
    for _ in 0..3 {
        click(&mut doc, "[data-testid=settings-labels]");
    }
    until(
        &mut doc,
        || false,
        "[...document.querySelectorAll('[data-testid=toast-text]')]
            .filter((toast) => toast.textContent.startsWith('Labels in the list')).length === 3",
    );
    assert_eq!(doc.query_all("[data-testid=toast]").len(), 3);

    // The name: empty is refused, a valid one shows at once and is saved on Enter.
    click(&mut doc, "[data-testid=settings-name]");
    doc.press("End");
    for _ in 0.."Shivam".len() {
        doc.press("Backspace");
    }
    doc.settle();
    assert_eq!(
        text(&doc, "[data-testid=settings-name-help]"),
        "Name can't be empty."
    );
    doc.type_text("Ada");
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=user-name]"), "Ada");
    assert_eq!(
        text(&doc, "[data-testid=settings-name-help]"),
        "Shown on comments and activity."
    );
    doc.press("Enter");
    until_file(&mut doc, file.clone(), "\"displayName\":\"Ada\"");

    let mut again = load_as(id);
    // The shell mounts after the first navigation, which may finish after `loaded`.
    until(
        &mut again,
        || false,
        "document.querySelector('[data-testid=user-name]')?.textContent === 'Ada'",
    );
}

#[test]
fn teammates_reset_and_idle() {
    let mut doc = load();
    click(&mut doc, "[data-testid=nav-settings]");
    click(&mut doc, "[data-testid=settings-tab-data]");
    doc.wait(0.1);
    assert!(
        text(&doc, "[data-testid=settings-counts]").starts_with("5,000 issues · 0 local edits")
    );
    assert!(
        doc.attr(
            doc.query("[data-testid=settings-reset]").unwrap(),
            "disabled"
        )
        .is_some(),
        "nothing to reset"
    );

    // One edit every 2 s while on, none after.
    click(&mut doc, "[data-testid=settings-simulate]");
    doc.wait(2.05);
    doc.wait(2.05);
    assert_eq!(js(&doc, "String(globalThis.__tracker.stats().edits)"), "2");
    click(&mut doc, "[data-testid=settings-simulate]");
    doc.wait(4.1);
    assert_eq!(js(&doc, "String(globalThis.__tracker.stats().edits)"), "2");
    assert!(text(&doc, "[data-testid=settings-counts]").contains("2 local edits"));

    // Reset asks first, then drops them.
    click(&mut doc, "[data-testid=settings-reset]");
    doc.wait(0.3);
    assert!(text(&doc, "[data-testid=reset-dialog]").contains("This deletes your 2 local edits."));
    click_in_dialog(&mut doc, "[data-testid=reset-confirm]");
    until(
        &mut doc,
        || false,
        "globalThis.__tracker.stats().edits === 0",
    );
    doc.wait(0.4);
    assert!(doc.query("[data-testid=reset-dialog]").is_none());
    assert!(text(&doc, "[data-testid=settings-counts]").contains("0 local edits"));

    // Every timer ends: toasts go after 5 s, the teammates stopped.
    doc.wait(5.1);
    assert!(doc.query("[data-testid=toast]").is_none());
    assert_eq!(
        doc.js().next_timer(),
        None,
        "no timer left: the app is idle"
    );
    // And an idle turn does nothing.
    {
        use blitz_dom::Document;
        for _ in 0..3 {
            assert!(!doc.poll(None), "poll reports no change while idle");
            assert_eq!(doc.last_turn().ops, 0);
        }
        assert!(!doc.base().has_changes());
    }
}
