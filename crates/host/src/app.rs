//! What an app ships: a web build, with `index.html` as its page, and an
//! optional manifest.
//!
//! The Vite preset writes `app.json` (and copies the icon) next to
//! `index.html`. Everything in the manifest is optional, and so is the
//! manifest itself.

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
    /// Where updates come from and the key they must be signed with.
    pub updater: Option<crate::updater::Config>,
    /// What the app may do outside its window. Nothing, unless listed.
    pub permissions: crate::api::Permissions,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// How small the window can be made, in logical pixels.
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub icon: Option<Icon>,
}

pub struct App {
    /// The page: `index.html`.
    pub html: String,
    pub manifest: Manifest,
    /// Where the app was loaded from. Relative URLs (`<img src="logo.png">`)
    /// resolve against it, and nothing outside it can be loaded.
    pub dir: Option<PathBuf>,
    /// The app bundle this process was started from, when it was. Only then
    /// can an update replace it.
    pub bundle: Option<PathBuf>,
    /// Started for development (`--watch`): what the host leaves out of the
    /// page is reported.
    pub dev: bool,
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
        let updater = match &value["updater"] {
            Value::Null => None,
            block => match (block["url"].as_str(), block["pubkey"].as_str()) {
                (Some(url), Some(pubkey)) => Some(crate::updater::Config {
                    url: url.to_owned(),
                    pubkey: pubkey.to_owned(),
                }),
                // A half-configured updater would silently never update.
                _ => return Err("app.json: `updater` needs both `url` and `pubkey`".into()),
            },
        };
        Ok(Self {
            name: text(&value["name"]),
            id: text(&value["id"]),
            version: text(&value["version"]),
            updater,
            permissions: crate::api::Permissions::parse(&value["permissions"])?,
            title: text(&window["title"]),
            width: size(&window["width"]),
            height: size(&window["height"]),
            min_width: size(&window["minWidth"]),
            min_height: size(&window["minHeight"]),
            icon: value["icon"]
                .as_str()
                .map(|path| load_icon(dir, path))
                .transpose()?,
        })
    }

    /// The identifier the app goes by on this machine: its `id`, or one made
    /// from its name. Names the bundle and the app's data directory.
    pub fn app_id(&self) -> Option<String> {
        self.id.clone().or_else(|| {
            self.name
                .as_deref()
                .map(|name| format!("app.sepal.{}", crate::bundle::slug(name)))
        })
    }

    /// Initial window size in logical pixels, never below the minimum.
    pub fn size(&self) -> (u32, u32) {
        let (min_width, min_height) = self.min_size().unwrap_or((0, 0));
        (
            self.width.unwrap_or(800).max(min_width),
            self.height.unwrap_or(600).max(min_height),
        )
    }

    /// How small the window can be made, in logical pixels, if the app says:
    /// a layout has a width below which it cannot work. 0 for the side it
    /// leaves out.
    pub fn min_size(&self) -> Option<(u32, u32)> {
        (self.min_width.is_some() || self.min_height.is_some())
            .then(|| (self.min_width.unwrap_or(0), self.min_height.unwrap_or(0)))
    }
}

impl App {
    /// Load an app folder produced by the Vite preset: `index.html`, and
    /// optionally `app.json`.
    pub fn from_dir(dir: &Path) -> Result<Self, String> {
        let html = read(&dir.join("index.html"))?
            .ok_or_else(|| format!("{} has no index.html", dir.display()))?;
        let manifest = match read(&dir.join("app.json"))? {
            Some(json) => Manifest::parse(&json, dir)?,
            None => Manifest::default(),
        };
        Ok(Self {
            html,
            manifest,
            dir: Some(dir.to_owned()),
            bundle: None,
            dev: false,
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
