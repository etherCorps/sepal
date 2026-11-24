//! The board and Insights screens (docs/tracker-design.md).

use super::*;
use sepal::screenshot::render_rgba;

/// The key of the card with the focus ring.
fn focused_card(doc: &SvelteDocument) -> Option<String> {
    let cards = doc.query_all("[data-testid=card].ring-2");
    assert!(cards.len() <= 1, "one focused card at most");
    cards.first().and_then(|&card| doc.attr(card, "data-key"))
}

/// Which column a card is in.
fn column_of(doc: &SvelteDocument, key: &str) -> Option<&'static str> {
    ["backlog", "todo", "progress", "review", "done", "canceled"]
        .into_iter()
        .find(|status| {
            doc.query(&format!(
                "[data-testid=board-column][data-status={status}] [data-key={key}]"
            ))
            .is_some()
        })
}

fn column_count(doc: &SvelteDocument, status: &str) -> usize {
    number(&text(
        doc,
        &format!("[data-testid=board-column][data-status={status}] [data-testid=column-count]"),
    ))
}

fn board() -> Tracker {
    let mut doc = load();
    click(&mut doc, "[data-testid=nav-board]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Board");
    doc
}

#[test]
fn board_mounts_six_capped_columns() {
    let doc = board();
    let columns = doc.query_all("[data-testid=board-column]");
    assert_eq!(columns.len(), 6);
    let mut total = 0;
    for status in ["backlog", "todo", "progress", "review", "done", "canceled"] {
        let count = column_count(&doc, status);
        let cards = doc
            .query_all(&format!(
                "[data-testid=board-column][data-status={status}] [data-testid=card]"
            ))
            .len();
        assert_eq!(cards, count.min(50), "{status}: at most 50 cards");
        if count > 50 {
            assert_eq!(
                text(
                    &doc,
                    &format!(
                        "[data-testid=board-column][data-status={status}] [data-testid=column-more]"
                    )
                ),
                format!("+ {} more in list", fmt(count - 50))
            );
        }
        total += count;
    }
    assert_eq!(total, 5000);
}

#[test]
fn board_keys_focus_and_move_cards() {
    let mut doc = board();
    assert_eq!(focused_card(&doc), None);

    // The first key focuses the first card; j/k and h/l walk.
    doc.press("j");
    let first = focused_card(&doc).expect("a card has the focus");
    assert_eq!(column_of(&doc, &first), Some("backlog"));
    doc.press("j");
    let second = focused_card(&doc).unwrap();
    assert_ne!(second, first);
    doc.press("k");
    assert_eq!(focused_card(&doc).as_deref(), Some(first.as_str()));
    doc.press("l");
    let right = focused_card(&doc).unwrap();
    assert_eq!(column_of(&doc, &right), Some("todo"));
    doc.press("h");
    assert_eq!(focused_card(&doc).as_deref(), Some(first.as_str()));

    // Shift+l moves the card one status right, and the focus goes with it.
    let backlog = column_count(&doc, "backlog");
    let todo = column_count(&doc, "todo");
    doc.press_with("L", Modifiers::SHIFT);
    doc.wait(0.3);
    assert_eq!(column_of(&doc, &first), Some("todo"));
    assert_eq!(focused_card(&doc).as_deref(), Some(first.as_str()));
    assert_eq!(column_count(&doc, "backlog"), backlog - 1);
    assert_eq!(column_count(&doc, "todo"), todo + 1);
    assert!(
        text(&doc, "[data-testid=toast]").contains(&format!("Moved {first} to Todo")),
        "{}",
        text(&doc, "[data-testid=toast]")
    );

    // Undo puts it back.
    click(&mut doc, "[data-testid=toast-action]");
    assert_eq!(column_of(&doc, &first), Some("backlog"));
    assert_eq!(column_count(&doc, "backlog"), backlog);

    // Shift+h at the first column does nothing.
    doc.press_with("H", Modifiers::SHIFT);
    assert_eq!(column_of(&doc, &first), Some("backlog"));

    // Escape lets go of the card.
    doc.press("Escape");
    assert_eq!(focused_card(&doc), None);
}

#[test]
fn board_status_select_belongs_to_the_focused_card() {
    let mut doc = board();
    assert!(
        doc.query("[data-testid=card-status-trigger]").is_none(),
        "no Select before a card has the focus"
    );
    doc.press("j");
    let key = focused_card(&doc).unwrap();
    assert_eq!(
        doc.query_all("[data-testid=card-status-trigger]").len(),
        1,
        "one Select for the board"
    );

    // s opens it next to the card; Escape closes it and gives the keys back.
    doc.press("s");
    doc.wait(0.3);
    assert!(doc.query("[data-testid=card-status]").is_some(), "open");
    let card = doc
        .rect(doc.query(&format!("[data-key={key}]")).unwrap())
        .unwrap();
    let style = doc
        .attr(
            doc.query("[data-bits-floating-content-wrapper]").unwrap(),
            "style",
        )
        .unwrap();
    let inner = style.split_once("translate(").unwrap().1;
    let (x, y) = inner.split_once(')').unwrap().0.split_once(',').unwrap();
    let px = |v: &str| v.trim().trim_end_matches("px").parse::<f64>().unwrap();
    assert!(
        (px(x) - card.0).abs() < 2.0 && (px(y) - (card.1 + card.3)).abs() < 8.0,
        "anchored under the card {card:?}: {style}"
    );
    doc.press("Escape");
    doc.wait(0.3);
    assert!(doc.query("[data-testid=card-status]").is_none(), "closed");
    assert_ne!(
        doc.query(":focus")
            .and_then(|node| doc.attr(node, "data-testid"))
            .as_deref(),
        Some("card-status-trigger"),
        "the hidden trigger let go of the focus"
    );
    doc.press("j");
    assert_ne!(
        focused_card(&doc).as_deref(),
        Some(key.as_str()),
        "j moves again"
    );
    doc.press("k");

    // Arrows and Enter pick a status: Backlog -> Done is four down.
    doc.press("s");
    doc.wait(0.3);
    for _ in 0..4 {
        doc.press("ArrowDown");
    }
    doc.press("Enter");
    doc.wait(0.3);
    assert!(doc.query("[data-testid=card-status]").is_none());
    assert_eq!(column_of(&doc, &key), Some("done"));
    assert!(text(&doc, "[data-testid=toast]").contains(&format!("Moved {key} to Done")));
}

#[test]
fn board_follows_the_filters() {
    let mut doc = board();
    click(&mut doc, "[data-testid=search]");
    doc.type_text("deploy");
    doc.settle();
    let filtered = number(js(&doc, "String(globalThis.__tracker.stats().filtered)").as_str());
    assert!(filtered > 0 && filtered < 5000);
    assert_eq!(
        text(&doc, "[data-testid=page-count]"),
        format!("{} of 5,000", fmt(filtered))
    );
    let sum: usize = ["backlog", "todo", "progress", "review", "done", "canceled"]
        .iter()
        .map(|status| column_count(&doc, status))
        .sum();
    assert_eq!(sum, filtered, "every column re-counted");
    for card in doc.query_all("[data-testid=card]") {
        let title = doc.text(card).to_lowercase();
        assert!(title.contains("deploy"), "{title}");
    }
}

#[test]
fn board_links_open_the_issue_and_the_list() {
    let mut doc = board();
    let card = doc.query("[data-testid=card]").unwrap();
    let key = doc.attr(card, "data-key").unwrap();
    doc.click(card);
    doc.settle();
    doc.resolve();
    assert!(
        doc.query("[data-testid=pane]").is_some(),
        "a card opens the issue"
    );
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert!(
        text(&doc, "[data-testid=pane]").contains(&key),
        "{}",
        text(&doc, "[data-testid=pane]")
    );

    // "+ N more in list" opens the list filtered to the column's status.
    click(&mut doc, "[data-testid=nav-board]");
    let backlog = column_count(&doc, "backlog");
    click(
        &mut doc,
        "[data-testid=board-column][data-status=backlog] [data-testid=column-more]",
    );
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
    assert_eq!(
        text(&doc, "[data-testid=page-count]"),
        format!("{} of 5,000", fmt(backlog))
    );
}

fn fmt(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn insights() -> Tracker {
    let mut doc = load();
    click(&mut doc, "[data-testid=nav-insights]");
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Insights");
    doc
}

#[test]
fn insights_recompute_on_range_and_project() {
    let mut doc = insights();
    assert_eq!(doc.query_all("[data-testid=chart-week]").len(), 12);
    let open = number(&text(&doc, "[data-testid=stat-open]"));
    let created_12 = number(&text(&doc, "[data-testid=stat-created]"));
    assert!(open > 0 && open < 5000);

    click(&mut doc, "[data-testid=range-4]");
    assert_eq!(doc.query_all("[data-testid=chart-week]").len(), 4);
    let created_4 = number(&text(&doc, "[data-testid=stat-created]"));
    assert!(created_4 < created_12, "{created_4} < {created_12}");
    assert_eq!(
        number(&text(&doc, "[data-testid=stat-open]")),
        open,
        "open is today's"
    );

    click(&mut doc, "[data-testid=range-26]");
    assert_eq!(doc.query_all("[data-testid=chart-week]").len(), 26);

    // Project: open the Select, one down from "All projects" is Engine.
    click(&mut doc, "[data-testid=insights-project]");
    doc.wait(0.3);
    doc.press("ArrowDown");
    doc.press("Enter");
    doc.wait(0.3);
    assert_eq!(text(&doc, "[data-testid=insights-project]"), "Engine");
    let engine_open = number(&text(&doc, "[data-testid=nav-project-ENG] span:last-child"));
    assert_eq!(number(&text(&doc, "[data-testid=stat-open]")), engine_open);
    assert_eq!(doc.query_all("[data-testid=oldest-row]").len(), 10);
    for row in doc.query_all("[data-testid=oldest-row]") {
        assert!(doc.text(row).contains("ENG-"), "{}", doc.text(row));
    }
}

#[test]
fn insights_tiles_explain_themselves() {
    let mut doc = insights();
    let (x, y, w, h) = doc
        .rect(doc.query("[data-testid=tile-closed]").unwrap())
        .unwrap();
    doc.move_to(5.0, 790.0);
    doc.move_to((x + w / 2.0) as f32, (y + h / 2.0) as f32);
    doc.wait(1.0);
    assert_eq!(
        text(&doc, "[data-tooltip-content]"),
        "Issues done or canceled in the last 12 weeks."
    );
    doc.move_to(5.0, 790.0);
    doc.wait(0.5);
    assert!(doc.query("[data-tooltip-content]").is_none());
}

#[test]
fn insights_chart_draws_bars_and_links_weeks() {
    let mut doc = insights();
    let weeks = doc.query_all("[data-testid=chart-week]");
    assert_eq!(weeks.len(), 12);
    let (pixels, width, _) = render_rgba(&mut doc, 1280, 800, 1.0);
    let at = |x: f64, y: f64| {
        let i = (y as usize * width as usize + x as usize) * 4;
        (pixels[i], pixels[i + 1], pixels[i + 2])
    };
    for &week in &weeks {
        // Each link covers its week of the plot: the bars stand on its bottom edge.
        let (left, top, w, h) = doc.rect(week).unwrap();
        let bottom = top + h;
        let created = at(left + 0.36 * w, bottom - 3.0);
        let closed = at(left + 0.64 * w, bottom - 3.0);
        let gap = at(left + 0.05 * w, bottom - 3.0);
        // chart-1 is cobalt, chart-2 a pale cobalt, the card nearly white.
        assert!(
            created.2 > 180 && created.0 < 60 && created.1 < 150,
            "created bar {created:?}"
        );
        assert!(
            closed.2 > 200 && closed.0 > 120 && closed.0 < 200,
            "closed bar {closed:?}"
        );
        assert!(
            gap.0 > 240 && gap.1 > 240 && gap.2 > 240,
            "background {gap:?}"
        );
        // The zero line is a hairline, darker than the card.
        let line = at(left + 0.05 * w, bottom - 0.5);
        assert!(line.0 < 235 && line.0 > 200, "zero line {line:?}");
    }

    // A week is a link to the list of issues created in it.
    let last = *weeks.last().unwrap();
    assert_eq!(doc.attr(last, "href").as_deref(), Some("/issues?week=0"));
    doc.click(last);
    doc.settle();
    doc.resolve();
    assert_eq!(text(&doc, "[data-testid=page-title]"), "Issues");
}
