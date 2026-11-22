//! Files, within the directories the manifest allows.
//!
//! A path is either absolute, or relative to a named directory (`data`,
//! `documents`, `downloads`, `desktop`, `home`). Before anything is read or
//! written the path is made canonical and has to lie inside an allowed
//! directory, or be something the user picked in a dialog.

use std::fs;
use std::path::{Component, Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};

use super::{Api, Scope, denied};

fn home() -> Result<PathBuf, String> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{variable} is not set, so there is no home directory"))
}

/// Where a named directory is on this machine. `data` is the app's own and
/// needs the app's id.
pub fn named_dir(name: &str, app_id: Option<&str>) -> Result<PathBuf, String> {
    match name {
        "home" => home(),
        "documents" => Ok(home()?.join("Documents")),
        "downloads" => Ok(home()?.join("Downloads")),
        "desktop" => Ok(home()?.join("Desktop")),
        "data" => {
            let id = app_id.ok_or("the app needs an `id` or a `name` to have a data directory")?;
            let base = if cfg!(target_os = "macos") {
                home()?.join("Library/Application Support")
            } else if cfg!(windows) {
                std::env::var_os("APPDATA")
                    .map(PathBuf::from)
                    .ok_or("APPDATA is not set")?
            } else {
                match std::env::var_os("XDG_DATA_HOME") {
                    Some(dir) => PathBuf::from(dir),
                    None => home()?.join(".local/share"),
                }
            };
            Ok(base.join(id))
        }
        _ => Err(format!(
            "{name:?} is not a directory name; use data, documents, downloads, desktop or home"
        )),
    }
}

/// The canonical form of a path that may not exist yet: the deepest existing
/// ancestor is resolved (symlinks and all), and the rest is appended as
/// written. The rest may not contain `..`.
fn canonical(path: &Path) -> Result<PathBuf, String> {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(resolved) = existing.canonicalize() {
            return Ok(rest
                .iter()
                .rev()
                .fold(resolved, |path, part| path.join(part)));
        }
        match (existing.parent(), existing.components().next_back()) {
            (Some(parent), Some(Component::Normal(name))) => {
                rest.push(name);
                existing = parent;
            }
            _ => return Err(format!("{} is not a usable path", path.display())),
        }
    }
}

impl Api {
    /// The path a call means: absolute, or relative to a named directory.
    fn path_argument(&self, args: &Value) -> Result<PathBuf, String> {
        let path = args["path"].as_str().ok_or("a path is needed")?;
        match args["dir"].as_str() {
            Some(dir) => {
                let inside = Path::new(path)
                    .components()
                    .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
                if !inside {
                    return Err(format!(
                        "{path:?} must be a path inside the {dir} directory"
                    ));
                }
                Ok(named_dir(dir, self.app_id.as_deref())?.join(path))
            }
            None if Path::new(path).is_absolute() => Ok(PathBuf::from(path)),
            None => Err(format!(
                "{path:?} is relative: give an absolute path, or name a directory with `dir`"
            )),
        }
    }

    /// Canonicalise a path and check that the app may use it.
    pub(super) fn allowed_path(&self, path: &Path) -> Result<PathBuf, String> {
        let path = canonical(path)?;
        let within = |root: &Path| canonical(root).is_ok_and(|root| path.starts_with(root));

        let by_manifest = match &self.permissions.fs {
            Scope::None => false,
            Scope::All => true,
            Scope::Some(entries) => entries.iter().any(|entry| {
                if Path::new(entry).is_absolute() {
                    within(Path::new(entry))
                } else {
                    named_dir(entry, self.app_id.as_deref()).is_ok_and(|dir| within(&dir))
                }
            }),
        };
        let by_user = || {
            self.picked
                .lock()
                .unwrap()
                .iter()
                .any(|picked| within(picked))
        };
        if by_manifest || by_user() {
            Ok(path)
        } else {
            Err(denied(&format!("using {}", path.display()), "fs"))
        }
    }
}

pub(super) fn call(api: &Api, action: &str, args: &Value) -> Result<String, String> {
    // The one call that touches nothing: where is this directory?
    if action == "dir" {
        let name = args["name"].as_str().ok_or("a directory name is needed")?;
        let dir = named_dir(name, api.app_id.as_deref())?;
        return Ok(json!(dir.to_string_lossy()).to_string());
    }

    let path = api.allowed_path(&api.path_argument(args)?)?;
    let io = |e: std::io::Error| format!("{}: {e}", path.display());
    let make_parent = || match path.parent() {
        Some(parent) => fs::create_dir_all(parent).map_err(io),
        None => Ok(()),
    };

    let result = match action {
        "read_text" => json!(fs::read_to_string(&path).map_err(io)?),
        "read" => json!(BASE64.encode(fs::read(&path).map_err(io)?)),
        "write_text" => {
            let text = args["text"].as_str().ok_or("writeTextFile needs text")?;
            make_parent()?;
            fs::write(&path, text).map_err(io)?;
            Value::Null
        }
        "write" => {
            let encoded = args["base64"].as_str().ok_or("writeFile needs bytes")?;
            let bytes = BASE64
                .decode(encoded)
                .map_err(|_| "the bytes are not valid base64")?;
            make_parent()?;
            fs::write(&path, bytes).map_err(io)?;
            Value::Null
        }
        "exists" => json!(path.exists()),
        "mkdir" => {
            fs::create_dir_all(&path).map_err(io)?;
            Value::Null
        }
        "read_dir" => {
            let mut entries = Vec::new();
            for entry in fs::read_dir(&path).map_err(io)? {
                let entry = entry.map_err(io)?;
                entries.push(json!({
                    "name": entry.file_name().to_string_lossy(),
                    "isDirectory": entry.file_type().map_err(io)?.is_dir(),
                }));
            }
            entries.sort_by_key(|entry| entry["name"].as_str().map(str::to_owned));
            Value::Array(entries)
        }
        "remove" => {
            if !path.is_dir() {
                fs::remove_file(&path).map_err(io)?;
            } else if args["recursive"].as_bool() == Some(true) {
                fs::remove_dir_all(&path).map_err(io)?;
            } else {
                // Refuses a directory with anything in it, on purpose.
                fs::remove_dir(&path).map_err(io)?;
            }
            Value::Null
        }
        _ => return Err(format!("this host has no call named fs.{action}")),
    };
    Ok(result.to_string())
}
