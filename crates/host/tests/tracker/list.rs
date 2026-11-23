//! The issues list and the detail pane (docs/tracker-design.md, Issues list and Detail pane).

use super::*;

#[test]
fn search_filters_on_every_key_and_shows_the_empty_state() {
    let mut doc = load();
    assert_eq!(paged(&doc), 200, "one page of rows");

    click(&mut doc, "[data-testid=search]");
    doc.type_text("deploy");
    assert_eq!(text(&doc, "[data-testid=page-count]"), "652 of 5,000");
    assert!(
        doc.query_all("[data-testid=row] a")
            .iter()
            .all(|&a| doc.text(a).to_lowercase().contains("deploy")),
        "every row matches"
    );

    doc.type_text("zzq");
    assert!(doc.query("[data-testid=row]").is_none());
    assert_eq!(
        text(&doc, "[data-testid=empty] p"),
        "No issues match ‘deployzzq’."
    );
    click(&mut doc, "[data-testid=clear-filters]");
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,000");
    assert_eq!(paged(&doc), 200);
    assert!(doc.query("[data-testid=row]").is_some());
}

#[test]
fn filters_group_and_sort() {
    let mut doc = load();

    pick(&mut doc, "[data-testid=filter-status]", "done");
    let shown = text(&doc, "[data-testid=page-count]");
    assert!(shown.ends_with(" of 5,000"), "{shown}");
    assert_eq!(count(&doc, "[data-testid=group-header]"), 1);
    assert_eq!(
        doc.attr(
            doc.query("[data-testid=group-header]").unwrap(),
            "data-group"
        )
        .as_deref(),
        Some("done")
    );
    assert!(text(&doc, "[data-testid=filter-status]").contains("Done"));

    // "All statuses" clears it again.
    pick(&mut doc, "[data-testid=filter-status]", "all");
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,000");
    assert_eq!(text(&doc, "[data-testid=filter-status]"), "Status");

    pick(&mut doc, "[data-testid=group-by]", "priority");
    assert_eq!(
        doc.attr(
            doc.query("[data-testid=group-header]").unwrap(),
            "data-group"
        )
        .as_deref(),
        Some("urgent"),
        "urgent first"
    );
    pick(&mut doc, "[data-testid=group-by]", "none");
    assert_eq!(count(&doc, "[data-testid=group-header]"), 0);

    pick(&mut doc, "[data-testid=sort-by]", "title");
    let titles: Vec<String> = doc
        .query_all("[data-testid=row] a span.truncate")
        .into_iter()
        .map(|node| doc.text(node))
        .collect();
    let mut sorted = titles.clone();
    sorted.sort();
    assert_eq!(titles, sorted, "sorted by title");
}

#[test]
fn paging_and_collapsing_groups() {
    let mut doc = load();
    js(&doc, "globalThis.__tracker.reseed(1000), ''");
    doc.settle();
    assert_eq!(paged(&doc), 200);
    click(&mut doc, "[data-testid=show-more]");
    assert_eq!(paged(&doc), 400);
    click(&mut doc, "[data-testid=show-all]");
    assert_eq!(paged(&doc), 1000);
    assert!(doc.query("[data-testid=show-more]").is_none());

    // Collapsing the first group pages out its rows and keeps its header;
    // the next group's header comes up under it.
    let first = doc.query("[data-testid=group-header]").unwrap();
    let group = doc.attr(first, "data-group").unwrap();
    let in_group: usize = text(&doc, "[data-testid=group-header] span.font-mono")
        .replace(',', "")
        .parse()
        .unwrap();
    click(&mut doc, "[data-testid=group-header]");
    assert_eq!(paged(&doc), 1000 - in_group);
    assert_eq!(
        doc.attr(
            doc.query("[data-testid=group-header]").unwrap(),
            "aria-expanded"
        )
        .as_deref(),
        Some("false"),
        "{group} collapsed"
    );
    let headers = doc.query_all("[data-testid=group-header]");
    assert!(headers.len() >= 2);
    assert_eq!(
        doc.rect(headers[1]).unwrap().1,
        doc.rect(headers[0]).unwrap().1 + 32.0
    );
    click(&mut doc, "[data-testid=group-header]");
    assert_eq!(paged(&doc), 1000);
}

/// Show all pages in every row, but the list mounts only those in view (and a
/// few screens' worth around them): the cliff of docs/tracker-perf.md is gone.
#[test]
fn show_all_mounts_only_the_rows_in_view() {
    let mut doc = load();
    let mounted = count(&doc, "[data-testid=row]");
    assert!(mounted > 15 && mounted < 40, "{mounted} rows mounted");

    click(&mut doc, "[data-testid=show-all]");
    assert_eq!(paged(&doc), 5000);
    assert!(doc.query("[data-testid=show-all]").is_none());
    assert_eq!(count(&doc, "[data-testid=row]"), mounted, "the same window");
    assert!(doc.live_nodes() < 20_000, "{} live nodes", doc.live_nodes());
    // The list is as tall as every row would make it: 36px a row, 32 a group header.
    let groups = number(&js(
        &doc,
        "String(new Set(globalThis.__tracker.order().map((key) => globalThis.__tracker.issue(key).status)).size)",
    ));
    assert_eq!(
        js(
            &doc,
            "String(document.querySelector('[data-testid=list]').scrollHeight)"
        ),
        (groups * 32 + 5000 * 36).to_string()
    );
}

#[test]
fn the_wheel_scrolls_the_list_and_mounts_the_rows_in_view() {
    let mut doc = load();
    pick(&mut doc, "[data-testid=group-by]", "none");
    click(&mut doc, "[data-testid=show-all]");
    let order = order(&doc);
    assert_eq!(order.len(), 5000);
    let list = doc.query("[data-testid=list]").unwrap();
    let (x, top, width, height) = doc.rect(list).unwrap();
    let (cx, cy) = ((x + width / 2.0) as f32, (top + height / 2.0) as f32);

    // 100 rows down.
    doc.wheel(cx, cy, 0.0, 3600.0);
    assert_eq!(scroll_top(&doc), 3600.0);
    let mounted = keys(&doc);
    assert!(mounted.len() < 60, "{} rows mounted", mounted.len());
    assert!(mounted.contains(&order[100]), "row 100 is mounted");
    assert!(!mounted.contains(&order[0]), "row 0 is not");
    // What is drawn at the top of the list is row 100: a click there ticks it.
    let checkbox = doc
        .rect(doc.query(&format!("{} input", row(&order[100]))).unwrap())
        .unwrap();
    doc.click_at((checkbox.0 + checkbox.2 / 2.0) as f32, (top + 18.0) as f32);
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "1 selected");
    assert!(
        doc.query(&format!("{} input:checked", row(&order[100])))
            .is_some()
    );

    // A little further: the rows still in view are the same nodes, moved.
    let kept = doc.query(&row(&order[110])).unwrap();
    doc.wheel(cx, cy, 0.0, 360.0);
    assert_eq!(scroll_top(&doc), 3960.0);
    assert_eq!(
        doc.query(&row(&order[110])),
        Some(kept),
        "reused, not re-created"
    );
    assert!(
        doc.query(&row(&order[105])).is_some(),
        "row 105 is in the overscan above"
    );
    assert!(doc.query(&row(&order[150])).is_none());

    // To the end: the host stops at the last row.
    doc.wheel(cx, cy, 0.0, 1_000_000.0);
    assert_eq!(scroll_top(&doc), 5000.0 * 36.0 - height);
    let last = doc
        .rect(
            doc.query(&row(&order[4999]))
                .expect("the last row is mounted"),
        )
        .unwrap();
    assert_eq!(last.1, top + height - 36.0, "at the bottom of the list");
    assert!(count(&doc, "[data-testid=row]") < 60);

    // A search shrinks the list under it: it ends up at its new end, not past
    // it. (The click reaches the search, not a row scrolled up under it.)
    click(&mut doc, "[data-testid=search]");
    doc.type_text("deploy");
    assert_eq!(text(&doc, "[data-testid=page-count]"), "652 of 5,000");
    let shrunk = super::order(&doc);
    assert_eq!(shrunk.len(), 652, "still showing all");
    assert_eq!(scroll_top(&doc), 652.0 * 36.0 - height);
    let last = doc
        .rect(
            doc.query(&row(&shrunk[651]))
                .expect("the last row is mounted"),
        )
        .unwrap();
    assert_eq!(last.1, top + height - 36.0, "at the bottom of the list");
}

#[test]
fn j_past_the_bottom_of_the_list_scrolls_it() {
    let mut doc = load();
    let order = order(&doc);
    let (_, top, _, height) = doc.rect(doc.query("[data-testid=list]").unwrap()).unwrap();
    for _ in 0..30 {
        doc.press("j");
    }
    doc.settle();
    let active = doc
        .query("[data-testid=row-active]")
        .expect("the highlighted row is mounted");
    assert!(text(&doc, "[data-testid=row-active]").starts_with(&order[29]));
    // Scrolled just enough to bring it in at the bottom, as `block: 'nearest'`.
    let y = doc.rect(active).unwrap().1;
    assert_eq!(
        y,
        top + height - 36.0,
        "the highlighted row is at the bottom"
    );
    assert_eq!(scroll_top(&doc), 32.0 + 30.0 * 36.0 - height);

    // k back to the first row: the list goes back to the top, header and all.
    for _ in 0..29 {
        doc.press("k");
    }
    doc.settle();
    assert!(text(&doc, "[data-testid=row-active]").starts_with(&order[0]));
    assert_eq!(scroll_top(&doc), 0.0);
}

#[test]
fn keys_move_open_and_close() {
    let mut doc = load();
    let rows = keys(&doc);

    doc.press("j");
    doc.press("j");
    doc.settle();
    assert!(
        text(&doc, "[data-testid=row-active]").starts_with(&rows[1]),
        "j moved twice"
    );
    doc.press("k");
    doc.settle();
    assert!(text(&doc, "[data-testid=row-active]").starts_with(&rows[0]));

    // Enter opens the highlighted issue in the pane.
    doc.press("Enter");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), rows[0]);

    // j/k with the pane open go to the next issue: a route change each.
    doc.press("j");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), rows[1]);
    assert!(text(&doc, "[data-testid=row-active]").starts_with(&rows[1]));
    click(&mut doc, "[data-testid=detail-next]");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), rows[2]);
    click(&mut doc, "[data-testid=detail-prev]");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), rows[1]);

    // Escape closes the pane and keeps the highlight.
    doc.press("Escape");
    doc.wait(0.1);
    assert!(doc.query("[data-testid=pane]").is_none());
    assert!(text(&doc, "[data-testid=row-active]").starts_with(&rows[1]));

    // x checks the highlighted row; Escape clears the selection.
    doc.press("x");
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "1 selected");
    doc.press("Escape");
    doc.settle();
    assert!(doc.query("[data-testid=bulk-bar]").is_none());

    // Escape in the search leaves it, so j/k work on the results.
    click(&mut doc, "[data-testid=search]");
    doc.type_text("deploy");
    doc.press("Escape");
    doc.press("j");
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=page-count]"), "652 of 5,000");
    let first = keys(&doc)[0].clone();
    assert!(text(&doc, "[data-testid=row-active]").starts_with(&first));

    // A click on a row opens it, and the click on its checkbox does not.
    let second = keys(&doc)[1].clone();
    click(&mut doc, &format!("{} input", row(&second)));
    doc.wait(0.1);
    assert!(doc.query("[data-testid=pane]").is_none(), "no navigation");
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "1 selected");
    click(&mut doc, &format!("{} a", row(&second)));
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=detail-key]"), second);
}

#[test]
fn bulk_set_status_delete_and_undo() {
    let mut doc = load();
    let rows = keys(&doc);
    for key in &rows[..3] {
        click(&mut doc, &format!("{} input", row(key)));
    }
    assert_eq!(text(&doc, "[data-testid=bulk-count]"), "3 selected");

    pick(&mut doc, "[data-testid=bulk-status]", "done");
    for key in &rows[..3] {
        assert_eq!(issue_field(&doc, key, "status"), "done");
    }
    assert!(text(&doc, "[data-testid=toast]").contains("Moved 3 issues to Done"));
    undo_from_toast(&mut doc);
    for key in &rows[..3] {
        assert_eq!(issue_field(&doc, key, "status"), "backlog", "undone");
    }

    // The bar stays clear of the toast that every action leaves.
    pick(&mut doc, "[data-testid=bulk-assign]", "p1");
    let bar = doc
        .rect(doc.query("[data-testid=bulk-bar]").unwrap())
        .unwrap();
    let toast = doc.rect(doc.query("[data-testid=toast]").unwrap()).unwrap();
    assert!(
        toast.1 + toast.3 <= bar.1,
        "bulk bar {bar:?} under toast {toast:?}"
    );
    let list = doc.rect(doc.query("[data-testid=list]").unwrap()).unwrap();
    let off_centre = (bar.0 + bar.2 / 2.0) - (list.0 + list.2 / 2.0);
    assert!(
        off_centre.abs() < 1.0,
        "bulk bar {bar:?} centred in {list:?}"
    );
    assert_eq!(issue_field(&doc, &rows[0], "assignee"), "p1");

    // Delete asks first.
    click(&mut doc, "[data-testid=bulk-delete]");
    doc.wait(0.3);
    assert_eq!(
        text(
            &doc,
            "[data-testid=confirm-delete] [data-slot=dialog-title]"
        ),
        "Delete 3 issues?"
    );
    // Single-key shortcuts stay out while the Dialog is open: x would check another row.
    doc.press("j");
    doc.press("x");
    doc.settle();
    assert_eq!(
        text(
            &doc,
            "[data-testid=confirm-delete] [data-slot=dialog-title]"
        ),
        "Delete 3 issues?"
    );
    click_in_dialog(&mut doc, "[data-testid=confirm-delete-submit]");
    doc.wait(0.4);
    assert!(doc.query("[role=dialog]").is_none());
    assert_eq!(text(&doc, "[data-testid=page-count]"), "4,997");
    assert!(
        doc.query("[data-testid=bulk-bar]").is_none(),
        "selection cleared"
    );
    assert!(doc.query(&row(&rows[0])).is_none());
    undo_from_toast(&mut doc);
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,000");
    assert!(doc.query(&row(&rows[0])).is_some(), "back after Undo");
}

#[test]
fn detail_pane_edits() {
    let mut doc = load();
    let key = keys(&doc)[0].clone();
    click(&mut doc, &format!("{} a", row(&key)));
    doc.wait(0.1);
    let title = issue_field(&doc, &key, "title");
    assert_eq!(text(&doc, "[data-testid=detail-title]"), title);

    // A property Select edits the issue, the row follows, Undo reverts.
    let activity: usize = text(&doc, "[data-testid=tab-activity] span")
        .parse()
        .unwrap();
    pick(&mut doc, "[data-testid=status-select]", "progress");
    assert_eq!(issue_field(&doc, &key, "status"), "progress");
    assert!(text(&doc, "[data-testid=status-select]").contains("In Progress"));
    assert_eq!(
        text(&doc, "[data-testid=tab-activity] span"),
        (activity + 1).to_string(),
        "the change is in the activity"
    );
    assert!(text(&doc, "[data-testid=toast]").contains(&format!("Moved {key} to In Progress")));
    undo_from_toast(&mut doc);
    assert_eq!(issue_field(&doc, &key, "status"), "backlog");

    pick(&mut doc, "[data-testid=priority-select]", "urgent");
    assert_eq!(issue_field(&doc, &key, "priority"), "urgent");
    pick(&mut doc, "[data-testid=assignee-select]", "none");
    assert_eq!(issue_field(&doc, &key, "assignee"), "null");

    // Labels: Edit shows a Checkbox per label.
    let labels = issue_field(&doc, &key, "labelIds.length");
    click(&mut doc, "[data-testid=labels-edit]");
    let has_l1 = issue_field(&doc, &key, "labelIds.includes('l1')") == "true";
    click(&mut doc, "[data-testid=label-l1]");
    assert_eq!(
        issue_field(&doc, &key, "labelIds.includes('l1')"),
        (!has_l1).to_string()
    );
    assert_ne!(issue_field(&doc, &key, "labelIds.length"), labels);

    // The title: empty shows the error and keeps the editor; Enter saves; Escape cancels.
    click(&mut doc, "[data-testid=detail-title] button");
    let input = doc.query("[data-testid=title-input]").expect("editing");
    // Put the caret at the end (focus() leaves it at the start).
    let (x, y, width, height) = doc.rect(input).unwrap();
    doc.click_at((x + width - 4.0) as f32, (y + height / 2.0) as f32);
    for _ in 0..title.chars().count() {
        doc.press("Backspace");
    }
    doc.press("Enter");
    doc.settle();
    assert_eq!(
        text(&doc, "[data-testid=title-error]"),
        "Title can't be empty. Type at least 3 characters."
    );
    doc.type_text("Renamed here");
    assert!(
        doc.query("[data-testid=title-error]").is_none(),
        "valid again"
    );
    doc.press("Enter");
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=detail-title]"), "Renamed here");
    assert!(
        text(&doc, &row(&key)).contains("Renamed here"),
        "the row follows"
    );
    click(&mut doc, "[data-testid=detail-title] button");
    doc.type_text("x");
    doc.press("Escape");
    doc.settle();
    assert_eq!(text(&doc, "[data-testid=detail-title]"), "Renamed here");
    assert!(
        doc.query("[data-testid=pane]").is_some(),
        "Escape in the editor did not close the pane"
    );

    // A comment: Enter posts it.
    let comments: usize = text(&doc, "[data-testid=tab-comments] span")
        .parse()
        .unwrap();
    click(&mut doc, "[data-testid=comment-input]");
    doc.type_text("Looks good");
    doc.press("Enter");
    doc.settle();
    assert_eq!(
        text(&doc, "[data-testid=tab-comments] span"),
        (comments + 1).to_string()
    );
    let all = doc.query_all("[data-testid=comment]");
    assert!(doc.text(*all.last().unwrap()).contains("Looks good"));

    // Escape leaves the comment field: the pane and the draft stay.
    click(&mut doc, "[data-testid=comment-input]");
    doc.type_text("Draft");
    doc.press("Escape");
    doc.settle();
    assert!(doc.query("[data-testid=pane]").is_some());
    assert_ne!(focused(&doc).as_deref(), Some("comment-input"));
    assert!(
        doc.attr(
            doc.query("[data-testid=comment-submit]").unwrap(),
            "disabled"
        )
        .is_none(),
        "the draft is kept"
    );

    // Deleted while open: the pane says so.
    js(&doc, &format!("globalThis.__tracker.delete(['{key}']), ''"));
    doc.settle();
    assert_eq!(
        text(&doc, "[data-testid=error-message]"),
        format!("No issue {key}.")
    );
}

#[test]
fn my_issues_and_projects_share_the_list() {
    let mut doc = load();
    click(&mut doc, "[data-testid=nav-my]");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "My issues");
    assert!(
        doc.query("[data-testid=filter-assignee]").is_none(),
        "the route fixes the assignee"
    );
    assert!(
        keys(&doc)
            .iter()
            .all(|key| issue_field(&doc, key, "assignee") == "p1")
    );

    click(&mut doc, "[data-testid=nav-project-WEB]");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Web");
    assert!(keys(&doc).iter().all(|key| key.starts_with("WEB-")));

    // Opening an issue from a project goes to the issues list with the pane.
    let key = keys(&doc)[0].clone();
    click(&mut doc, &format!("{} a", row(&key)));
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert_eq!(text(&doc, "[data-testid=detail-key]"), key);
}

#[test]
fn compact_rows_restyle_the_list() {
    let mut doc = load();
    let key = keys(&doc)[0].clone();
    assert_eq!(doc.rect(doc.query(&row(&key)).unwrap()).unwrap().3, 36.0);

    click(&mut doc, "[data-testid=nav-settings]");
    doc.wait(0.1);
    click(&mut doc, "[data-testid=settings-compact]");
    doc.wait(0.1);
    click(&mut doc, "[data-testid=nav-issues]");
    doc.wait(0.1);
    assert_eq!(doc.rect(doc.query(&row(&key)).unwrap()).unwrap().3, 28.0);
    assert!(
        doc.query("[data-testid=row] a span.whitespace-nowrap")
            .is_none(),
        "no labels in compact rows"
    );
}

#[test]
fn a_week_from_insights_filters_the_list() {
    let mut doc = load();
    click(&mut doc, "[data-testid=nav-insights]");
    doc.wait(0.1);
    click(&mut doc, "[data-testid=chart-week][data-week='1']");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    let shown = text(&doc, "[data-testid=page-count]");
    assert!(shown.ends_with(" of 5,000"), "{shown}");
    assert!(doc.query("[data-testid=filter-week]").is_some());

    // Opening an issue and closing it again keeps the week.
    let key = keys(&doc)[0].clone();
    click(&mut doc, &format!("{} a", row(&key)));
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-count]"), shown);
    doc.press("Escape");
    doc.wait(0.1);
    assert!(doc.query("[data-testid=pane]").is_none());
    assert_eq!(text(&doc, "[data-testid=page-count]"), shown);

    // The chip clears it.
    click(&mut doc, "[data-testid=filter-week]");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,000");
    assert!(doc.query("[data-testid=filter-week]").is_none());

    // The board has no week chip, so the week does not follow the user there.
    click(&mut doc, "[data-testid=nav-insights]");
    doc.wait(0.1);
    click(&mut doc, "[data-testid=chart-week][data-week='1']");
    doc.wait(0.1);
    assert!(doc.query("[data-testid=filter-week]").is_some());
    click(&mut doc, "[data-testid=nav-board]");
    doc.wait(0.1);
    assert_eq!(text(&doc, "[data-testid=page-count]"), "5,000");
}

/// The wheel reaches "Show 200 more" under the first page, and pressing it
/// lets the list go on into the next group. The list once stopped one row
/// short: its end left that row out, so the first page (all Backlog) was all
/// there was.
#[test]
fn the_wheel_reaches_show_more_and_the_next_page() {
    let mut doc = load();
    let list = doc.query("[data-testid=list]").unwrap();
    let (x, top, width, height) = doc.rect(list).unwrap();
    let (cx, cy) = ((x + width / 2.0) as f32, (top + height / 2.0) as f32);
    for _ in 0..50 {
        doc.wheel(cx, cy, 0.0, 400.0);
    }
    doc.resolve();
    let more = doc
        .query("[data-testid=show-more]")
        .expect("Show more is mounted");
    let (_, y, _, h) = doc.rect(more).unwrap();
    assert!(
        y + h <= top + height,
        "Show more is inside the list: {} > {}",
        y + h,
        top + height
    );
    let before = paged(&doc);
    doc.click(more);
    doc.settle();
    doc.resolve();
    assert_eq!(paged(&doc), before + 200);
    for _ in 0..50 {
        doc.wheel(cx, cy, 0.0, 400.0);
    }
    doc.resolve();
    assert!(
        scroll_top(&doc) > 7000.0,
        "the list scrolls past the first page"
    );
}
