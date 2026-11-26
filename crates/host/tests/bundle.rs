//! The macOS `.app` bundle, checked as files on disk. Nothing here needs macOS.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::fs;
use std::path::{Path, PathBuf};

use sepal::app::decode_icon;
use sepal::bundle::bundle_macos;

fn counter_dist() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/counter/dist"
    ))
}

/// An empty scratch directory, unique to one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sepal-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A stand-in for the host binary.
fn fake_host(dir: &Path) -> PathBuf {
    let path = dir.join("host");
    fs::write(&path, b"#!/bin/sh\n").unwrap();
    path
}

/// A minimal app directory with the given manifest.
fn dist_with(dir: &Path, manifest: &str) -> PathBuf {
    let dist = dir.join("dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(dist.join("index.html"), "").unwrap();
    fs::write(dist.join("app.json"), manifest).unwrap();
    dist
}

/// Every file under a directory, relative to it.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(
                files(&path)
                    .into_iter()
                    .map(|file| Path::new(path.file_name().unwrap()).join(file)),
            );
        } else {
            found.push(PathBuf::from(path.file_name().unwrap()));
        }
    }
    found
}

/// The entries of an `.icns` file as (type, payload).
fn icns_entries(file: &[u8]) -> Vec<(String, Vec<u8>)> {
    assert_eq!(&file[..4], b"icns");
    let total = u32::from_be_bytes(file[4..8].try_into().unwrap()) as usize;
    assert_eq!(total, file.len(), "header length covers the whole file");

    let mut entries = Vec::new();
    let mut at = 8;
    while at < file.len() {
        let kind = String::from_utf8(file[at..at + 4].to_vec()).unwrap();
        let length = u32::from_be_bytes(file[at + 4..at + 8].try_into().unwrap()) as usize;
        entries.push((kind, file[at + 8..at + length].to_vec()));
        at += length;
    }
    entries
}

#[test]
fn bundles_the_counter_example() {
    let dir = scratch("bundle");
    let host = fake_host(&dir);
    let bundle = bundle_macos(counter_dist(), &dir, &host).expect("run `pnpm run build` first");
    assert_eq!(bundle, dir.join("Counter.app"));

    let contents = bundle.join("Contents");
    assert_eq!(
        fs::read(contents.join("MacOS/Counter")).unwrap(),
        fs::read(&host).unwrap()
    );
    // The whole app folder: the page, its chunks and stylesheet, app.json, the icon.
    let copied = files(counter_dist());
    assert!(copied.contains(&PathBuf::from("index.html")) && copied.len() > 4);
    for file in copied {
        assert_eq!(
            fs::read(contents.join("Resources").join(&file)).unwrap(),
            fs::read(counter_dist().join(&file)).unwrap(),
            "{} copied unchanged",
            file.display()
        );
    }

    let plist = fs::read_to_string(contents.join("Info.plist")).unwrap();
    for (key, value) in [
        ("CFBundleName", "Counter"),
        ("CFBundleExecutable", "Counter"),
        ("CFBundleIdentifier", "com.example.counter"),
        ("CFBundleShortVersionString", "0.1.0"),
        ("CFBundleIconFile", "AppIcon"),
    ] {
        let entry = format!("<key>{key}</key>\n\t<string>{value}</string>");
        assert!(plist.contains(&entry), "{key} missing from:\n{plist}");
    }

    // Every icon variant is a PNG of the size its type promises.
    let entries = icns_entries(&fs::read(contents.join("Resources/AppIcon.icns")).unwrap());
    let sizes: Vec<(String, u32)> = entries
        .into_iter()
        .map(|(kind, png)| {
            let icon = decode_icon(png).unwrap();
            assert_eq!(icon.width, icon.height);
            (kind, icon.width)
        })
        .collect();
    let expected = [
        ("ic07", 128),
        ("ic08", 256),
        ("ic09", 512),
        ("ic10", 1024),
        ("ic11", 32),
        ("ic12", 64),
        ("ic13", 256),
        ("ic14", 512),
    ];
    assert_eq!(
        sizes,
        expected
            .map(|(kind, size)| (kind.to_owned(), size))
            .to_vec()
    );

    // Building again replaces our own bundle.
    fs::write(bundle.join("stale.txt"), "").unwrap();
    bundle_macos(counter_dist(), &dir, &host).unwrap();
    assert!(!bundle.join("stale.txt").exists());
}

/// A bundle from before apps were web builds (0.2.0-alpha.0 and earlier:
/// `app.js` and `app.css` where `index.html` is now) is Sepal's too. Dev
/// keeps one in `node_modules/.sepal`; refused, dev ran the bare host, and
/// the Dock said "calyx".
#[test]
fn replaces_a_bundle_from_before_index_html() {
    let dir = scratch("old-layout");
    let host = fake_host(&dir);
    let old = dir.join("Counter.app/Contents");
    fs::create_dir_all(old.join("MacOS")).unwrap();
    fs::create_dir_all(old.join("Resources")).unwrap();
    fs::write(old.join("MacOS/Counter"), "an older calyx").unwrap();
    for file in ["app.js", "app.css", "app.json"] {
        fs::write(old.join("Resources").join(file), "").unwrap();
    }

    bundle_macos(counter_dist(), &dir, &host).unwrap();
    assert!(old.join("Resources/index.html").is_file());
    assert!(
        !old.join("Resources/app.js").exists(),
        "the old app went with its bundle"
    );
}

#[test]
fn never_replaces_an_app_it_did_not_build() {
    let dir = scratch("foreign");
    let host = fake_host(&dir);
    let foreign = dir.join("Counter.app/Contents/MacOS");
    fs::create_dir_all(&foreign).unwrap();
    fs::write(foreign.join("Counter"), "someone else's app").unwrap();

    let error = bundle_macos(counter_dist(), &dir, &host).err().unwrap();
    assert!(error.contains("not replacing it"), "{error}");
    assert_eq!(
        fs::read_to_string(foreign.join("Counter")).unwrap(),
        "someone else's app"
    );
}

// The name has a `<` in it, which a file on Windows cannot.
#[cfg(not(windows))]
#[test]
fn defaults_and_escaping() {
    let dir = scratch("defaults");
    let host = fake_host(&dir);

    // No id, version or icon: derived id, default version, no icon key.
    let dist = dist_with(&dir, r#"{ "name": "Tom & Jerry <3" }"#);
    let bundle = bundle_macos(&dist, &dir, &host).unwrap();
    let plist = fs::read_to_string(bundle.join("Contents/Info.plist")).unwrap();
    assert!(
        plist.contains("<string>Tom &amp; Jerry &lt;3</string>"),
        "{plist}"
    );
    assert!(
        plist.contains("<string>app.sepal.tom-jerry-3</string>"),
        "{plist}"
    );
    assert!(plist.contains("<string>0.1.0</string>"));
    assert!(!plist.contains("CFBundleIconFile"));
    assert!(!bundle.join("Contents/Resources/AppIcon.icns").exists());
}

#[test]
fn rejects_what_cannot_become_a_bundle() {
    let dir = scratch("rejects");
    let host = fake_host(&dir);
    let cases = [
        (r#"{}"#, "needs a `name`"),
        (r#"{ "name": "../Evil" }"#, "bundle name"),
        (r#"{ "name": ".hidden" }"#, "bundle name"),
        (r#"{ "name": "Ok", "id": "not valid!" }"#, "app id"),
    ];
    for (manifest, expected) in cases {
        let dist = dist_with(&dir, manifest);
        let error = bundle_macos(&dist, &dir, &host).err().unwrap();
        assert!(error.contains(expected), "{manifest}: {error}");
    }

    // A non-square icon would be distorted, a tiny one has nothing to offer.
    for (width, height, expected) in [(64, 32, "square"), (16, 16, "at least 32px")] {
        let dist = dist_with(&dir, r#"{ "name": "Ok", "icon": "icon.png" }"#);
        image::RgbaImage::new(width, height)
            .save(dist.join("icon.png"))
            .unwrap();
        let error = bundle_macos(&dist, &dir, &host).err().unwrap();
        assert!(error.contains(expected), "{width}x{height}: {error}");
    }
}
