//! The macOS `.app` bundle, checked as files on disk. Nothing here needs macOS.
//!
//! Build the bundle first: `pnpm run build` in the repository root.

use std::fs;
use std::path::{Path, PathBuf};

use svelte_blitz_host::app::decode_icon;
use svelte_blitz_host::bundle::bundle_macos;

fn counter_dist() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/counter/dist"
    ))
}

/// An empty scratch directory, unique to one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("svelte-blitz-{}-{name}", std::process::id()));
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
    fs::write(dist.join("app.js"), "").unwrap();
    fs::write(dist.join("app.json"), manifest).unwrap();
    dist
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
    for file in ["app.js", "app.css", "app.json", "icon.png"] {
        assert_eq!(
            fs::read(contents.join("Resources").join(file)).unwrap(),
            fs::read(counter_dist().join(file)).unwrap(),
            "{file} copied unchanged"
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
        plist.contains("<string>app.svelte-blitz.tom-jerry-3</string>"),
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
