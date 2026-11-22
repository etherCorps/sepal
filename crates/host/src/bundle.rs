//! Wrap an app and the host binary into a macOS `.app` bundle.
//!
//! ```text
//! Counter.app/Contents/
//!   Info.plist              name, identifier, version, icon
//!   MacOS/Counter           the host binary
//!   Resources/              everything from dist/, plus AppIcon.icns
//! ```
//!
//! A bundle is what gives an app its name and icon in the Dock, the app
//! switcher and Activity Monitor. Started without arguments, the host looks
//! for `../Resources/app.js` next to itself (see `main.rs`).
//!
//! This only writes files, so it runs (and is tested) on every platform.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::{ImageFormat, RgbaImage};

use crate::app::{App, Icon};

/// Icon variants in an `.icns`, as (type, pixel size). These are the types
/// whose payload is a plain PNG; macOS scales them down for smaller uses.
const ICNS_VARIANTS: &[(&[u8; 4], u32)] = &[
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic13", 256),
    (b"ic14", 512),
];

/// Build `<out>/<name>.app` from the directory the Vite preset wrote and a
/// host binary. Returns the path of the bundle.
pub fn bundle_macos(dist: &Path, out: &Path, host: &Path) -> Result<PathBuf, String> {
    // Loading the app first means a broken manifest or icon fails here, not on launch.
    let manifest = App::from_dir(dist)?.manifest;

    let name = manifest
        .name
        .as_deref()
        .ok_or("app.json needs a `name` to build a bundle")?;
    if name.is_empty() || name.starts_with('.') || name.contains(['/', ':', '\0']) {
        return Err(format!("{name:?} cannot be used as a bundle name"));
    }
    let id = match &manifest.id {
        Some(id) => id.clone(),
        None => format!("app.svelte-blitz.{}", slug(name)),
    };
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return Err(format!(
            "app id {id:?} may only contain letters, digits, '-' and '.'"
        ));
    }
    let version = manifest.version.as_deref().unwrap_or("0.1.0");

    let bundle = out.join(format!("{name}.app"));
    let contents = bundle.join("Contents");
    let resources = contents.join("Resources");
    if bundle.exists() {
        // `name` comes from a config file and `out` from the command line. Together
        // they can name any application on the machine, so only replace our own.
        if !resources.join("app.js").is_file() {
            return Err(format!(
                "{} exists and was not built by svelte-blitz; not replacing it",
                bundle.display()
            ));
        }
        io(fs::remove_dir_all(&bundle), &bundle)?;
    }

    let macos = contents.join("MacOS");
    io(fs::create_dir_all(&macos), &macos)?;
    copy_dir(dist, &resources)?;
    // `fs::copy` keeps the executable bit and the linker's ad-hoc signature.
    io(fs::copy(host, macos.join(name)), host)?;

    let icon_file = match &manifest.icon {
        Some(icon) => {
            let path = resources.join("AppIcon.icns");
            io(fs::write(&path, icns(icon)?), &path)?;
            Some("AppIcon")
        }
        None => None,
    };

    let plist = contents.join("Info.plist");
    io(
        fs::write(&plist, info_plist(name, &id, version, icon_file)),
        &plist,
    )?;
    Ok(bundle)
}

/// Encode an `.icns` holding the icon at every size up to its own.
pub fn icns(icon: &Icon) -> Result<Vec<u8>, String> {
    if icon.width != icon.height {
        return Err(format!(
            "the icon must be square to build a bundle; it is {}×{}",
            icon.width, icon.height
        ));
    }
    let source = RgbaImage::from_raw(icon.width, icon.height, icon.rgba.clone())
        .ok_or("icon pixel data does not match its size")?;

    let mut entries = Vec::new();
    for (kind, size) in ICNS_VARIANTS {
        // Never scale up: a blurry large variant is worse than a missing one.
        if *size > icon.width {
            continue;
        }
        let scaled = imageops::resize(&source, *size, *size, FilterType::Lanczos3);
        let mut png = Vec::new();
        scaled
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .map_err(|e| format!("cannot encode the {size}px icon: {e}"))?;
        entries.extend_from_slice(*kind);
        entries.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        entries.extend_from_slice(&png);
    }
    if entries.is_empty() {
        return Err(format!(
            "the icon is {}px; a bundle needs at least 32px (1024px recommended)",
            icon.width
        ));
    }

    let mut file = b"icns".to_vec();
    file.extend_from_slice(&(entries.len() as u32 + 8).to_be_bytes());
    file.extend_from_slice(&entries);
    Ok(file)
}

fn info_plist(name: &str, id: &str, version: &str, icon_file: Option<&str>) -> String {
    let mut keys = vec![
        ("CFBundleName", name),
        ("CFBundleDisplayName", name),
        ("CFBundleExecutable", name),
        ("CFBundleIdentifier", id),
        ("CFBundleVersion", version),
        ("CFBundleShortVersionString", version),
        ("CFBundlePackageType", "APPL"),
        ("CFBundleInfoDictionaryVersion", "6.0"),
        ("NSPrincipalClass", "NSApplication"),
    ];
    if let Some(icon_file) = icon_file {
        keys.push(("CFBundleIconFile", icon_file));
    }

    let mut plist = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n",
    );
    for (key, value) in keys {
        plist.push_str(&format!(
            "\t<key>{key}</key>\n\t<string>{}</string>\n",
            xml_escape(value)
        ));
    }
    // Without this macOS renders the window at 1x and scales it up.
    plist.push_str("\t<key>NSHighResolutionCapable</key>\n\t<true/>\n</dict>\n</plist>\n");
    plist
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Lowercase letters and digits, everything else collapsed to single dashes.
fn slug(name: &str) -> String {
    let dashed: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = dashed
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() { "app".into() } else { slug }
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    io(fs::create_dir_all(to), to)?;
    for entry in io(fs::read_dir(from), from)? {
        let entry = io(entry, from)?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if io(entry.file_type(), &source)?.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            io(fs::copy(&source, &target), &source)?;
        }
    }
    Ok(())
}

fn io<T>(result: std::io::Result<T>, path: &Path) -> Result<T, String> {
    result.map_err(|e| format!("{}: {e}", path.display()))
}
