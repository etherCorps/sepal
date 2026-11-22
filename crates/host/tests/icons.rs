//! Icons and images, checked on rendered pixels: the probe in `examples/icons`
//! is drawn on the CPU and each row is sampled.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::path::Path;
use std::sync::mpsc;

use blitz_dom::{NodeId, ns};
use blitz_traits::net::{Bytes, NetHandler, NetProvider, Request, Url};
use svelte_blitz_host::SvelteDocument;
use svelte_blitz_host::app::App;
use svelte_blitz_host::net::AppResources;
use svelte_blitz_host::screenshot::{render_png, render_rgba};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

type Rgb = (u8, u8, u8);
const HEX_INDIGO: Rgb = (79, 70, 229); // #4f46e5
const HEX_ROSE: Rgb = (225, 29, 72); // #e11d48
const TAILWIND_INDIGO: Rgb = (79, 57, 246); // indigo-600, oklch(51.1% 0.262 276.966)
const TAILWIND_ROSE: Rgb = (236, 0, 63); // rose-600, oklch(58.6% 0.253 17.585)
const AMBER: Rgb = (245, 158, 11); // #f59e0b
const EMERALD: Rgb = (16, 185, 129); // #10b981
const SKY: Rgb = (14, 165, 233); // #0ea5e9

fn dist() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/icons/dist"
    ))
}

fn load() -> SvelteDocument {
    let app = App::from_dir(dist()).expect("run `pnpm run build` in the repository root first");
    SvelteDocument::for_app(&app).expect("app failed to start")
}

/// One rendered frame, sampled in CSS pixels (the scale is 1).
struct Frame(Vec<u8>);

impl Frame {
    fn of(doc: &mut SvelteDocument) -> Self {
        Self(render_rgba(doc, WIDTH, HEIGHT, 1.0).0)
    }

    fn pixel(&self, x: u32, y: u32) -> Rgb {
        let at = ((y * WIDTH + x) * 4) as usize;
        (self.0[at], self.0[at + 1], self.0[at + 2])
    }

    /// The pixel in the middle of a node.
    fn centre(&self, doc: &SvelteDocument, node: NodeId) -> Rgb {
        let (x, y, width, height) = doc.rect(node).expect("node has no layout");
        self.pixel((x + width / 2.0) as u32, (y + height / 2.0) as u32)
    }

    /// How many pixels inside a node are (close to) a colour.
    fn count(&self, doc: &SvelteDocument, node: NodeId, colour: Rgb) -> usize {
        let (x, y, width, height) = doc.rect(node).expect("node has no layout");
        let mut count = 0;
        for py in y as u32..(y + height) as u32 {
            for px in x as u32..(x + width) as u32 {
                count += near(self.pixel(px, py), colour) as usize;
            }
        }
        count
    }
}

/// Close enough to be the same colour after colour-space rounding. Blended
/// edge pixels are much further away than this.
fn near(a: Rgb, b: Rgb) -> bool {
    let d = |a: u8, b: u8| a.abs_diff(b) <= 6;
    d(a.0, b.0) && d(a.1, b.1) && d(a.2, b.2)
}

#[track_caller]
fn assert_colour(what: &str, actual: Rgb, expected: Rgb) {
    assert!(
        near(actual, expected),
        "{what}: {actual:?}, expected {expected:?}"
    );
}

#[test]
fn every_row_renders_in_its_colour() {
    let mut doc = load();
    let frame = Frame::of(&mut doc);

    // Inline <svg>: the stroke icon has a bar through its centre, the fill icon is a disc.
    for (row, colour) in [
        ("#inline-hex", HEX_INDIGO),
        ("#inline-tailwind", TAILWIND_INDIGO),
    ] {
        let stroke = doc.query(&format!("{row} svg.stroke")).unwrap();
        let fill = doc.query(&format!("{row} svg.fill")).unwrap();
        assert_colour(&format!("{row} stroke"), frame.centre(&doc, stroke), colour);
        assert_colour(&format!("{row} fill"), frame.centre(&doc, fill), colour);
    }

    // Lucide components are outlines: no pixel is guaranteed, a share of them is.
    for (row, colour) in [
        ("#lucide-hex", HEX_ROSE),
        ("#lucide-tailwind", TAILWIND_ROSE),
    ] {
        let icons = doc.query_all(&format!("{row} svg"));
        assert_eq!(icons.len(), 3, "{row}");
        for (index, icon) in icons.into_iter().enumerate() {
            let drawn = frame.count(&doc, icon, colour);
            assert!(
                drawn > 30,
                "{row} icon {index}: only {drawn} pixels in {colour:?}"
            );
        }
    }
    // `size` and Tailwind's `size-8` both reach the element.
    let sizes: Vec<f64> = doc
        .query_all("#lucide-tailwind svg")
        .into_iter()
        .map(|icon| doc.rect(icon).unwrap().2)
        .collect();
    assert_eq!(sizes, [24.0, 40.0, 32.0]);

    // <img>: a data: URL in the markup, a file next to app.js, an asset Vite inlined.
    for (row, colour) in [
        ("#image", AMBER),
        ("#image-file", EMERALD),
        ("#image-import", SKY),
    ] {
        let image = doc.query(&format!("{row} img")).unwrap();
        assert_colour(row, frame.centre(&doc, image), colour);
    }

    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/icons-probe.png");
    std::fs::write(out, render_png(&mut doc, WIDTH, HEIGHT, 1.0)).unwrap();
}

#[test]
fn icons_follow_a_colour_change() {
    let mut doc = load();
    doc.layout(WIDTH, HEIGHT);
    let button = doc.query("#recolour").unwrap();
    let disc = doc.query("#recolour svg.fill").unwrap();
    let heart = doc.query_all("#recolour svg")[1];

    let before = Frame::of(&mut doc);
    assert_colour("disc before", before.centre(&doc, disc), TAILWIND_INDIGO);
    assert!(before.count(&doc, heart, TAILWIND_INDIGO) > 30);

    // Only the class on an ancestor changes; the <svg> nodes are untouched.
    doc.click(button);
    assert!(doc.attr(button, "class").unwrap().contains("text-rose-600"));

    let after = Frame::of(&mut doc);
    assert_colour("disc after", after.centre(&doc, disc), TAILWIND_ROSE);
    assert!(after.count(&doc, heart, TAILWIND_ROSE) > 30);
    assert_eq!(
        after.count(&doc, heart, TAILWIND_INDIGO),
        0,
        "nothing left in the old colour"
    );
}

/// The namespace of an element is decided from its name (`SVG_ELEMENTS` in
/// `ops.rs`). An SVG element missing from that list becomes an unknown HTML
/// element and silently does not draw, so catch it here.
#[test]
fn everything_inside_an_svg_is_an_svg_element() {
    let mut doc = load();
    doc.layout(WIDTH, HEIGHT);
    let base = doc.base();

    let mut checked = 0;
    for svg in doc.query_all("svg") {
        let mut stack: Vec<NodeId> = base.get_node(svg).unwrap().children.to_vec();
        while let Some(id) = stack.pop() {
            let node = base.get_node(id).unwrap();
            stack.extend(node.children.iter().copied());
            let Some(element) = node.element_data() else {
                continue;
            };
            let name = &*element.name.local;
            let html_too = ["a", "title", "style", "script"].contains(&name);
            assert!(
                element.name.ns == ns!(svg) || html_too,
                "<{name}> inside an <svg> is not in the SVG namespace: add it to SVG_ELEMENTS in ops.rs"
            );
            checked += 1;
        }
    }
    assert!(checked > 10, "the probe has SVG content to check");
}

struct Capture(mpsc::Sender<Vec<u8>>);

impl NetHandler for Capture {
    fn bytes(self: Box<Self>, _resolved_url: String, bytes: Bytes) {
        let _ = self.0.send(bytes.to_vec());
    }
}

/// What the loader answers for a URL. Empty means "refused or not found".
fn fetch(resources: &AppResources, url: &str) -> Vec<u8> {
    let (sender, received) = mpsc::channel();
    let request = Request::get(Url::parse(url).unwrap());
    resources.fetch(0, request, Box::new(Capture(sender)));
    received
        .try_recv()
        .expect("the loader always answers, and synchronously")
}

#[test]
fn only_data_urls_and_the_apps_own_files_load() {
    let resources = AppResources::new(Some(dist()));
    let base = Url::parse(&resources.base_url().unwrap()).unwrap();
    let relative = |path: &str| base.join(path).unwrap().to_string();

    assert_eq!(
        fetch(&resources, "data:text/plain,hello%20there"),
        b"hello there"
    );
    assert_eq!(fetch(&resources, "data:text/plain;base64,aGk="), b"hi");
    assert_eq!(
        fetch(&resources, &relative("dot.svg")),
        std::fs::read(dist().join("dot.svg")).unwrap()
    );

    // A path from the root is from the app's root, as on a web server.
    assert_eq!(
        fetch(&resources, "file:///dot.svg"),
        std::fs::read(dist().join("dot.svg")).unwrap()
    );

    // Out of the app directory, by `..` or by absolute path: refused.
    assert!(fetch(&resources, &relative("../package.json")).is_empty());
    assert!(fetch(&resources, &relative("../../../crates/host/Cargo.toml")).is_empty());
    assert!(fetch(&resources, "file:///etc/hosts").is_empty());
    // No network, and nothing for a file that is not there.
    assert!(fetch(&resources, "https://example.com/logo.png").is_empty());
    assert!(fetch(&resources, &relative("missing.png")).is_empty());

    // An app that was not loaded from a directory can still use data: URLs.
    let no_dir = AppResources::new(None);
    assert!(no_dir.base_url().is_none());
    assert_eq!(fetch(&no_dir, "data:text/plain,x"), b"x");
    assert!(fetch(&no_dir, "file:///etc/hosts").is_empty());
}

#[cfg(unix)]
#[test]
fn a_symlink_cannot_lead_out_of_the_app_directory() {
    let dir = std::env::temp_dir().join(format!("svelte-blitz-{}-symlink", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(dir.join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(dir.join("secret.txt"), dir.join("app/link.txt")).unwrap();
    std::fs::write(dir.join("app/ok.txt"), "ok").unwrap();

    let resources = AppResources::new(Some(&dir.join("app")));
    let base = Url::parse(&resources.base_url().unwrap()).unwrap();
    assert_eq!(
        fetch(&resources, base.join("ok.txt").unwrap().as_str()),
        b"ok"
    );
    assert!(fetch(&resources, base.join("link.txt").unwrap().as_str()).is_empty());
}

/// URLs come from app content, so a bad one must never take the host down.
/// Upstream blitz-dom panics on both cases below; see crates/blitz-dom/PATCH.md.
#[test]
fn bad_image_urls_do_not_panic() {
    use blitz_dom::{DocumentConfig, LocalName, QualName};

    // No directory: the base URL is Blitz's default, which cannot resolve relative paths.
    let mut doc = SvelteDocument::new("", "", DocumentConfig::default()).unwrap();
    for src in ["logo.png", "http://[", "//", ""] {
        let mut base = doc.base_mut();
        let mut mutator = base.mutate();
        let body = mutator.doc.query_selector("body").unwrap().unwrap();
        let img = mutator.create_element(
            QualName::new(None, ns!(html), LocalName::from("img")),
            Vec::new(),
        );
        mutator.set_attribute(img, QualName::new(None, ns!(), LocalName::from("src")), src);
        mutator.append_children(body, &[img]);
    }
    doc.layout(200, 200);

    // With a directory: a missing file and a malformed URL are reported, not fatal.
    let app = App::from_dir(dist()).unwrap();
    let mut doc = SvelteDocument::for_app(&app).unwrap();
    for src in ["missing.png", "http://[", "../../package.json"] {
        let mut base = doc.base_mut();
        let mut mutator = base.mutate();
        let body = mutator.doc.query_selector("body").unwrap().unwrap();
        let img = mutator.create_element(
            QualName::new(None, ns!(html), LocalName::from("img")),
            Vec::new(),
        );
        mutator.set_attribute(img, QualName::new(None, ns!(), LocalName::from("src")), src);
        mutator.append_children(body, &[img]);
    }
    render_rgba(&mut doc, 200, 200, 1.0);
}
