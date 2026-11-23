//! Drive an app without a window and print what happens. For finding out why
//! something does not work; tests belong in `tests/`.
//!
//! ```sh
//! cargo run --features screenshot --example drive -- <app dir> [WIDTHxHEIGHT] <step>...
//! ```
//!
//! Steps, in order: `click:SELECTOR`, `clickat:X,Y`, `move:X,Y`, `key:NAME`
//! (as `KeyboardEvent.key`), `type:TEXT`, `time:SECONDS` (timers
//! and animations move on), `text:SELECTOR`, `count:SELECTOR`,
//! `rect:SELECTOR`, `attr:SELECTOR@NAME`, `eval:JAVASCRIPT`, `hud:on`/`hud:off`
//! (the performance overlay), `shot:FILE.png`
//! (CPU renderer), `gpushot:FILE.png` (the window's renderer; needs a GPU).
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
                    let began = std::time::Instant::now();
                    doc.click(node);
                    println!("  ({:.1}ms)", began.elapsed().as_secs_f64() * 1000.0);
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
            "key" => {
                doc.press(rest);
                doc.wait(0.12);
                println!("  key {rest}")
            }
            "type" => {
                doc.type_text(rest);
                doc.wait(0.12);
                println!("  type {rest}")
            }
            "eval" => println!("  eval: {:?}", doc.js().eval_string(rest)),
            "hud" => {
                doc.set_hud(rest == "on");
                println!("  hud {rest}")
            }
            "shot" => {
                let png = svelte_blitz_host::screenshot::render_png(&mut doc, width, height, 1.0);
                std::fs::write(rest, png).unwrap();
                println!("  shot {rest}")
            }
            // Like `shot:`, with the renderer the window uses instead of the
            // CPU one. Needs a GPU. `gpushot-nocache:` turns the glyph atlas off.
            #[cfg(feature = "window")]
            "gpushot" | "gpushot-nocache" => {
                anyrender_vello_hybrid::set_glyph_cache(verb == "gpushot");
                doc.resolve();
                let mut base = doc.base_mut();
                let pixels = anyrender_vello_hybrid::render_to_rgba(width, height, |scene| {
                    blitz_paint::paint_scene(scene, &mut base, 1.0, width, height, 0, 0)
                })
                .unwrap();
                drop(base);
                let mut png = Vec::new();
                let mut encoder = png::Encoder::new(&mut png, width, height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&pixels)
                    .unwrap();
                std::fs::write(rest, png).unwrap();
                println!("  {verb} {rest}")
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
