//! What an app ships: the bundle, its stylesheet and an optional manifest.
//!
//! The Vite preset writes `app.json` (and copies the icon) next to `app.js`.
//! Everything in the manifest is optional, and so is the manifest itself.

use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

/// A decoded app icon.
pub struct Icon {
    /// The file as shipped, for platforms that take an encoded image (macOS).
    pub png: Vec<u8>,
    /// Straight-alpha RGBA8, row by row, for winit.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct Manifest {
    pub name: Option<String>,
    /// Bundle identifier, e.g. `com.example.counter`. Used when bundling.
    pub id: Option<String>,
    pub version: Option<String>,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub icon: Option<Icon>,
}

pub struct App {
    pub js: String,
    pub css: String,
    pub manifest: Manifest,
    /// Where the app was loaded from. Relative URLs (`<img src="logo.png">`)
    /// resolve against it, and nothing outside it can be loaded.
    pub dir: Option<PathBuf>,
}

impl Manifest {
    /// Parse `app.json`. `dir` is the directory it came from; the icon path is
    /// relative to it.
    ///
    /// Fields of the wrong type are ignored. A missing or undecodable icon is an
    /// error: the app asked for one, and starting without it would hide that.
    pub fn parse(json: &str, dir: &Path) -> Result<Self, String> {
        let value: Value = serde_json::from_str(json).map_err(|e| format!("app.json: {e}"))?;
        let text = |v: &Value| v.as_str().map(str::to_owned);
        let size = |v: &Value| {
            v.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n > 0)
        };

        let window = &value["window"];
        Ok(Self {
            name: text(&value["name"]),
            id: text(&value["id"]),
            version: text(&value["version"]),
            title: text(&window["title"]),
            width: size(&window["width"]),
            height: size(&window["height"]),
            icon: value["icon"]
                .as_str()
                .map(|path| load_icon(dir, path))
                .transpose()?,
        })
    }

    /// Initial window size in logical pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width.unwrap_or(800), self.height.unwrap_or(600))
    }
}

impl App {
    /// Load a directory produced by the Vite preset: `app.js`, and optionally
    /// `app.css` and `app.json`.
    pub fn from_dir(dir: &Path) -> Result<Self, String> {
        let js =
            read(&dir.join("app.js"))?.ok_or_else(|| format!("{} has no app.js", dir.display()))?;
        let css = read(&dir.join("app.css"))?.unwrap_or_default();
        let manifest = match read(&dir.join("app.json"))? {
            Some(json) => Manifest::parse(&json, dir)?,
            None => Manifest::default(),
        };
        Ok(Self {
            js,
            css,
            manifest,
            dir: Some(dir.to_owned()),
        })
    }
}

/// Decode a PNG icon. Any colour type is accepted and converted to RGBA8.
pub fn decode_icon(png: Vec<u8>) -> Result<Icon, String> {
    let image = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|e| format!("not a valid PNG: {e}"))?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Ok(Icon {
        png,
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn load_icon(dir: &Path, path: &str) -> Result<Icon, String> {
    // The manifest travels with the app; it must not name files outside it.
    let inside = Path::new(path)
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    if !inside {
        return Err(format!(
            "app.json: icon {path:?} must be a path inside the app directory"
        ));
    }
    let file = dir.join(path);
    let png = fs::read(&file).map_err(|e| format!("cannot read icon {}: {e}", file.display()))?;
    decode_icon(png).map_err(|e| format!("icon {}: {e}", file.display()))
}

/// Read a text file, treating "does not exist" as `None`.
fn read(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}
