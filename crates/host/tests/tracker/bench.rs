//! The benchmark of docs/tracker-plan.md ("Perf scenarios"). Not a test: it
//! prints a table. Run it on a release build, once per engine:
//!
//! ```sh
//! cargo test --release --features screenshot --test tracker -- --ignored --nocapture bench
//! cargo test --release --features "screenshot engine-v8" --test tracker -- --ignored --nocapture bench
//! ```
//!
//! Each figure is the time from the input to the page being ready to paint
//! (the app's work, applying its ops, and the layout after), as README
//! "Performance" measures it. Headless: no painting, no GPU.
//! `TRACKER_BENCH_SIZES=1000,5000` picks the sizes (default 1000, 5000, 20000).

use super::*;

struct Sample {
    ms: f64,
    ops: usize,
    created: usize,
}

/// Run `action`, then the turn and layout that make the page ready to paint.
fn measure(doc: &mut SvelteDocument, action: impl FnOnce(&mut SvelteDocument)) -> Sample {
    let before = doc.totals();
    let started = Instant::now();
    action(doc);
    doc.settle();
    doc.resolve();
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    let after = doc.totals();
    Sample {
        ms,
        ops: after.ops - before.ops,
        created: after.created - before.created,
    }
}

/// Activity Monitor's "Memory": the process's physical footprint, in MB.
#[cfg(target_os = "macos")]
fn footprint_mb() -> f64 {
    #[repr(C)]
    #[derive(Default)]
    struct RusageV0 {
        uuid: [u8; 16],
        user_time: u64,
        system_time: u64,
        pkg_idle_wkups: u64,
        interrupt_wkups: u64,
        pageins: u64,
        wired_size: u64,
        resident_size: u64,
        phys_footprint: u64,
        proc_start_abstime: u64,
        proc_exit_abstime: u64,
    }
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut RusageV0) -> i32;
    }
    let mut usage = RusageV0::default();
    unsafe { proc_pid_rusage(std::process::id() as i32, 0, &mut usage) };
    usage.phys_footprint as f64 / 1048576.0
}
#[cfg(not(target_os = "macos"))]
fn footprint_mb() -> f64 {
    f64::NAN
}

struct Table {
    engine: &'static str,
    rows: Vec<String>,
}

impl Table {
    fn add(&mut self, doc: &SvelteDocument, scenario: &str, n: usize, samples: &[Sample]) {
        let mut sorted: Vec<f64> = samples.iter().map(|s| s.ms).collect();
        sorted.sort_by(f64::total_cmp);
        let first = &samples[0];
        let median = sorted[sorted.len() / 2];
        let worst = sorted[sorted.len() - 1];
        let row = format!(
            "| {scenario} | {n} | {} | {:.1} | {} | {} | {} | {} | {} | {} | {:.0} |",
            self.engine,
            first.ms,
            if samples.len() > 1 {
                format!("{median:.1}")
            } else {
                "".into()
            },
            if samples.len() > 1 {
                format!("{worst:.1}")
            } else {
                "".into()
            },
            samples.len(),
            first.ops,
            first.created,
            doc.live_nodes(),
            footprint_mb(),
        );
        println!("{row}");
        self.rows.push(row);
    }
}

/// Open a Select (not timed), then time picking an item.
fn choose(doc: &mut SvelteDocument, trigger: &str, value: &str) -> Sample {
    click(doc, trigger);
    doc.wait(0.2);
    let (x, y) = floating_offset(doc);
    let item = doc
        .rect(
            doc.query(&format!("[data-select-item][data-value='{value}']"))
                .unwrap_or_else(|| panic!("no item {value} in {trigger}")),
        )
        .unwrap();
    let sample = measure(doc, |doc| {
        doc.click_at(
            (x + item.0 + item.2 / 2.0) as f32,
            (y + item.1 + item.3 / 2.0) as f32,
        )
    });
    doc.wait(0.4);
    sample
}

/// Time opening a Select, then close it (not timed). Three times.
fn open_select(doc: &mut SvelteDocument, trigger: &str) -> Vec<Sample> {
    (0..3)
        .map(|_| {
            let sample = measure(doc, |doc| click(doc, trigger));
            assert!(
                doc.query("[data-select-content]").is_some(),
                "{trigger} opened"
            );
            doc.press("Escape");
            doc.wait(0.4);
            sample
        })
        .collect()
}

fn nav(doc: &mut SvelteDocument, to: &str) -> Sample {
    let sample = measure(doc, |doc| click(doc, &format!("[data-testid=nav-{to}]")));
    doc.wait(0.1);
    sample
}

#[test]
#[ignore]
fn bench() {
    let engine = if cfg!(feature = "engine-v8") {
        "V8"
    } else {
        "QuickJS"
    };
    let sizes: Vec<usize> = std::env::var("TRACKER_BENCH_SIZES")
        .unwrap_or_else(|_| "1000,5000,20000".into())
        .split(',')
        .map(|n| n.trim().parse().unwrap())
        .collect();
    let mut table = Table {
        engine,
        rows: Vec::new(),
    };
    println!(
        "| scenario | N | engine | first ms | median ms | worst ms | samples | ops | created | live nodes | footprint MB |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|");

    // P1: start to first layout, at the default size (5000; the app seeds before it mounts).
    {
        let dist = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/tracker/dist"
        ));
        let id = new_id();
        let started = Instant::now();
        let mut app = App::from_dir(&dist).unwrap();
        app.manifest.id = Some(id.clone());
        let mut doc = SvelteDocument::for_app(&app).unwrap();
        let mounted = started.elapsed().as_secs_f64() * 1000.0;
        doc.layout(1280, 800);
        let laid_out = started.elapsed().as_secs_f64() * 1000.0;
        let row = format!(
            "| P1 start: load, seed and mount (then first layout at {laid_out:.0} ms) | 5000 | {engine} | {mounted:.1} | | | 1 | | | {} | {:.0} |",
            doc.live_nodes(),
            footprint_mb()
        );
        println!("{row}");
        table.rows.push(row);
        drop(doc);
        let _ = std::fs::remove_dir_all(data_dir(&id));
    }

    for &n in &sizes {
        let mut doc = load();
        let reseed = measure(&mut doc, |doc| {
            js(doc, &format!("globalThis.__tracker.reseed({n}); ''"));
        });
        table.add(&doc, "P1 reseed: generate and render", n, &[reseed]);

        // P2: one key at a time into the search, then back out.
        click(&mut doc, "[data-testid=search]");
        let typed: Vec<_> = "deploy"
            .chars()
            .map(|c| measure(&mut doc, |doc| doc.press(&c.to_string())))
            .collect();
        table.add(&doc, "P2 search: a keystroke (typing 'deploy')", n, &typed);
        let erased: Vec<_> = (0..6)
            .map(|_| measure(&mut doc, |doc| doc.press("Backspace")))
            .collect();
        table.add(&doc, "P2 search: Backspace", n, &erased);
        doc.press("Escape");
        doc.wait(0.1);

        // P3: j/k with the pane closed, then open.
        let moves: Vec<_> = (0..20)
            .map(|_| measure(&mut doc, |doc| doc.press("j")))
            .collect();
        table.add(&doc, "P3 j: move the highlight", n, &moves);
        let open = measure(&mut doc, |doc| doc.press("Enter"));
        table.add(&doc, "P3 Enter: open the issue pane", n, &[open]);
        let moves: Vec<_> = (0..20)
            .map(|_| measure(&mut doc, |doc| doc.press("j")))
            .collect();
        table.add(&doc, "P3 j with the pane open (route + load)", n, &moves);
        let status = open_select(&mut doc, "[data-testid=status-select]");
        table.add(&doc, "P4 open the pane's status Select", n, &status);
        let close = measure(&mut doc, |doc| doc.press("Escape"));
        table.add(&doc, "P3 Escape: close the pane", n, &[close]);
        doc.wait(0.1);

        // P4 on the first page, P5 regroup and resort over all N.
        let filter = open_select(&mut doc, "[data-testid=filter-status]");
        table.add(&doc, "P4 open the Status filter (200 rows)", n, &filter);
        let regroup: Vec<_> = ["assignee", "priority", "none", "status"]
            .iter()
            .map(|value| choose(&mut doc, "[data-testid=group-by]", value))
            .collect();
        table.add(
            &doc,
            "P5 group by (assignee, priority, none, status)",
            n,
            &regroup,
        );
        let resort: Vec<_> = ["priority", "title", "updated"]
            .iter()
            .map(|value| choose(&mut doc, "[data-testid=sort-by]", value))
            .collect();
        table.add(&doc, "P5 sort by (priority, title, updated)", n, &resort);
        let filtered = choose(&mut doc, "[data-testid=filter-status]", "done");
        table.add(&doc, "P5 filter status = Done", n, &[filtered]);
        choose(&mut doc, "[data-testid=filter-status]", "all");

        // P8: the palette, while the list is a page long.
        let palette = measure(&mut doc, |doc| doc.press_with("k", Modifiers::META));
        table.add(&doc, "P8 Cmd+K: open the palette", n, &[palette]);
        doc.wait(0.2);
        let typed: Vec<_> = "eng-12"
            .chars()
            .map(|c| measure(&mut doc, |doc| doc.press(&c.to_string())))
            .collect();
        table.add(&doc, "P8 palette: a keystroke ('eng-12')", n, &typed);
        let go = measure(&mut doc, |doc| doc.press("Enter"));
        table.add(&doc, "P8 palette: Enter (open ENG-12)", n, &[go]);
        doc.wait(0.3);
        doc.press("Escape");
        doc.wait(0.1);

        // P9: create, then a 50-row bulk edit.
        let open = measure(&mut doc, |doc| doc.press("c"));
        table.add(&doc, "P9 c: open New issue", n, &[open]);
        doc.wait(0.3);
        doc.type_text("Bench issue");
        let create = measure(&mut doc, |doc| doc.press_with("Enter", Modifiers::META));
        table.add(&doc, "P9 Cmd+Enter: create the issue", n, &[create]);
        doc.wait(0.6);
        let rows = keys(&doc);
        click(&mut doc, &format!("{} input", row(&rows[2])));
        let to = doc.query(&format!("{} input", row(&rows[51]))).unwrap();
        let range = measure(&mut doc, |doc| doc.click_with(to, Modifiers::SHIFT));
        table.add(&doc, "P9 Shift+click: check 50 rows", n, &[range]);
        let bulk = choose(&mut doc, "[data-testid=bulk-status]", "done");
        table.add(&doc, "P9 bulk: set status of 50", n, &[bulk]);

        // P10: the save that follows, 500 ms later.
        doc.wait(0.45);
        let save = measure(&mut doc, |doc| doc.wait(0.1));
        let stats = js(&doc, "JSON.stringify(globalThis.__tracker.stats())");
        table.add(
            &doc,
            "P10 save edits.json (the turn with the write)",
            n,
            &[save],
        );
        println!("    stats: {stats}");

        // P14: teammates' edits arriving on the list.
        let teammates: Vec<_> = (0..30)
            .map(|_| {
                measure(&mut doc, |doc| {
                    js(doc, "globalThis.__tracker.simulate_once(); ''");
                })
            })
            .collect();
        table.add(&doc, "P14 a teammate's edit, on /issues", n, &teammates);
        doc.wait(6.0);

        // P7, P11 and P13: the other screens, and coming back.
        let board = nav(&mut doc, "board");
        table.add(&doc, "P7 open Board", n, &[board]);
        doc.press("j");
        let moves: Vec<_> = (0..3)
            .map(|_| measure(&mut doc, |doc| doc.press_with("L", Modifiers::SHIFT)))
            .collect();
        table.add(&doc, "P7 Shift+L: move a card", n, &moves);
        doc.press("Escape");
        let teammates: Vec<_> = (0..30)
            .map(|_| {
                measure(&mut doc, |doc| {
                    js(doc, "globalThis.__tracker.simulate_once(); ''");
                })
            })
            .collect();
        table.add(&doc, "P14 a teammate's edit, on /board", n, &teammates);
        doc.wait(6.0);
        let insights = nav(&mut doc, "insights");
        table.add(&doc, "P11 open Insights", n, &[insights]);
        let range: Vec<_> = ["26", "4", "12"]
            .iter()
            .map(|weeks| {
                measure(&mut doc, |doc| {
                    click(doc, &format!("[data-testid=range-{weeks}]"))
                })
            })
            .collect();
        table.add(&doc, "P11 change the range (26w, 4w, 12w)", n, &range);
        let settings = nav(&mut doc, "settings");
        table.add(&doc, "open Settings", n, &[settings]);
        let back = nav(&mut doc, "issues");
        table.add(&doc, "P13 back to Issues", n, &[back]);

        let mut churn = Vec::new();
        for _ in 0..10 {
            for to in ["board", "insights", "settings", "issues"] {
                churn.push(nav(&mut doc, to));
            }
        }
        table.add(&doc, "P13 cycle 4 screens x10 (each nav)", n, &churn);
        // The nav link keeps the focus after a click, and Enter would follow it.
        js(&doc, "document.activeElement?.blur(); ''");
        doc.press("j");
        let mut churn = Vec::new();
        for _ in 0..25 {
            churn.push(measure(&mut doc, |doc| doc.press("Enter")));
            churn.push(measure(&mut doc, |doc| doc.press("Escape")));
        }
        table.add(&doc, "P13 open and close an issue x25", n, &churn);
        doc.wait(1.1);
        doc.collect_garbage();
        doc.resolve();
        table.add(
            &doc,
            "P13 after the churn and a GC (live nodes)",
            n,
            &[Sample {
                ms: 0.0,
                ops: 0,
                created: 0,
            }],
        );

        // P6: the rows cliff, last because it makes every later step slower.
        let more = measure(&mut doc, |doc| click(doc, "[data-testid=show-more]"));
        table.add(&doc, "P6 Show 200 more", n, &[more]);
        let all = measure(&mut doc, |doc| click(doc, "[data-testid=show-all]"));
        let shown = count(&doc, "[data-testid=row]");
        table.add(&doc, &format!("P6 Show all ({shown} rows)"), n, &[all]);
        if shown < n {
            // The 10 s watchdog stopped the render; the app is not usable now.
            println!("    Show all stopped by the watchdog at {shown} of {n} rows");
            continue;
        }
        let moves: Vec<_> = (0..10)
            .map(|_| measure(&mut doc, |doc| doc.press("j")))
            .collect();
        table.add(&doc, "P6 j with every row shown", n, &moves);
        let filter = open_select(&mut doc, "[data-testid=filter-status]");
        table.add(
            &doc,
            "P4 open the Status filter (every row shown)",
            n,
            &filter,
        );
        let leave = nav(&mut doc, "board");
        table.add(&doc, "P6 leave Show all (to Board)", n, &[leave]);
        doc.wait(0.85);
        let gc = measure(&mut doc, |doc| doc.wait(0.2));
        table.add(&doc, "P6 the deferred GC, 1 s later", n, &[gc]);

        // Idle: nothing left to do.
        doc.wait(6.0);
        let idle = measure(&mut doc, |doc| {
            doc.settle();
        });
        table.add(&doc, "idle turn", n, &[idle]);
    }

    let out = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target"))
        .join(format!("tracker-bench-{}.md", engine.to_lowercase()));
    std::fs::write(&out, table.rows.join("\n") + "\n").unwrap();
    println!("written to {}", out.display());
}

/// For a profiler (`sample <pid>`, `SVELTE_BLITZ_TRACE=1`, or
/// `--features blitz-dom/log-phase-times` for Blitz's own phases): every row
/// shown, then `j` and the Status filter in a loop, then the styles Bits UI
/// puts on `<body>` while a Select is open, one at a time.
/// `TRACKER_PROFILE=5000,40,3` sets the issues, the presses and the Selects.
///
/// ```sh
/// cargo test --release --features screenshot --test tracker -- --ignored --nocapture profile_every_row_shown
/// ```
#[test]
#[ignore]
fn profile_every_row_shown() {
    let spec = std::env::var("TRACKER_PROFILE").unwrap_or_else(|_| "5000,40,3".into());
    let spec: Vec<usize> = spec.split(',').map(|n| n.trim().parse().unwrap()).collect();
    let (n, presses, selects) = (spec[0], spec[1], spec[2]);
    let mut doc = load();
    js(&doc, &format!("globalThis.__tracker.reseed({n}); ''"));
    doc.settle();
    doc.resolve();
    click(&mut doc, "[data-testid=show-all]");
    println!(
        "{} rows, {} nodes",
        count(&doc, "[data-testid=row]"),
        doc.live_nodes()
    );
    let mut j: Vec<f64> = (0..presses)
        .map(|_| measure(&mut doc, |doc| doc.press("j")).ms)
        .collect();
    j.sort_by(f64::total_cmp);
    if let Some(median) = j.get(j.len() / 2) {
        println!("j: median {median:.1} ms");
    }
    for _ in 0..selects {
        let s = measure(&mut doc, |doc| click(doc, "[data-testid=filter-status]"));
        println!("open the Status filter: {:.1} ms", s.ms);
        doc.press("Escape");
        doc.wait(0.4);
    }
    for code in [
        "document.body.style.setProperty('overflow', 'hidden')",
        "document.body.style.setProperty('pointer-events', 'none')",
        "document.body.style.removeProperty('pointer-events')",
        "document.body.style.removeProperty('overflow')",
    ] {
        let s = measure(&mut doc, |doc| {
            js(doc, &format!("{code}; ''"));
        });
        println!("{code}: {:.1} ms", s.ms);
    }
}
