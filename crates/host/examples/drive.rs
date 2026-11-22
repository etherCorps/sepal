//! Drive an app without a window and print what happens. For finding out why
//! something does not work; tests belong in `tests/`.
//!
//! ```sh
//! cargo run --features screenshot --example drive -- <app dir> [WIDTHxHEIGHT] <step>...
//! ```
//!
//! Steps, in order: `click:SELECTOR`, `clickat:X,Y`, `move:X,Y`, `time:SECONDS` (timers
//! and animations move on), `text:SELECTOR`, `count:SELECTOR`,
//! `rect:SELECTOR`, `attr:SELECTOR@NAME`, `eval:JAVASCRIPT`, `shot:FILE.png`.
//!
//! `click:` aims at the middle of the element's box before CSS transforms; for
//! something moved by `translate`, use `clickat:`.

use svelte_blitz_host::{SvelteDocument, app::App};
fn main() {
    let mut args = std::env::args().skip(1);
    let app = App::from_dir(std::path::Path::new(&args.next().unwrap())).unwrap();
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    let mut args = args.peekable();
    let size = args
        .peek()
        .and_then(|arg| arg.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
    let (width, height) = match size {
        Some(size) => {
            args.next();
            size
        }
        None => (640, 360),
    };
    doc.layout(width, height);
    for step in args {
        let (verb, rest) = step.split_once(':').unwrap();
        match verb {
            "click" => match doc.query(rest) {
                Some(node) => {
                    doc.click(node);
                    doc.wait(0.12);
                    println!("  clicked {rest}")
                }
                None => println!("  click {rest}: NOT FOUND"),
            },
            "clickat" => {
                let (x, y) = rest.split_once(',').unwrap();
                doc.click_at(x.parse().unwrap(), y.parse().unwrap());
                doc.wait(0.12);
                println!("  clickat {rest}")
            }
            "move" => {
                let (x, y) = rest.split_once(',').unwrap();
                doc.move_to(x.parse().unwrap(), y.parse().unwrap());
                doc.wait(0.12);
                println!("  move {rest}")
            }
            "eval" => println!("  eval: {:?}", doc.js().eval_string(rest)),
            "shot" => {
                let png = svelte_blitz_host::screenshot::render_png(&mut doc, width, height, 1.0);
                std::fs::write(rest, png).unwrap();
                println!("  shot {rest}")
            }
            "time" => {
                doc.wait(rest.parse().unwrap());
                println!("  time {rest}")
            }
            "rect" => println!(
                "  rect {rest}: {:?}",
                doc.query(rest).and_then(|n| doc.rect(n))
            ),
            "text" => println!("  text {rest}: {:?}", doc.query(rest).map(|n| doc.text(n))),
            "count" => println!("  count {rest}: {}", doc.query_all(rest).len()),
            "attr" => {
                let (sel, name) = rest.split_once('@').unwrap();
                println!(
                    "  attr {rest}: {:?}",
                    doc.query(sel).and_then(|n| doc.attr(n, name))
                )
            }
            _ => println!("  unknown step {step}"),
        }
    }
}
