//! What an app can ask the host to do outside its window: HTTP requests,
//! files, dialogs, opening things in other apps.
//!
//! Everything is off until the app's manifest allows it (`permissions`), and
//! a refused call says which entry would allow it. The checks are here, in
//! the host: JavaScript cannot widen what it was given.
//!
//! Window and clipboard calls are not in this module. They need the window's
//! shell and run on the UI thread; see `SvelteDocument::host_work`.

mod dialog;
mod fs;
mod http;
mod open;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;

pub use fs::named_dir;
pub use http::url_allowed;
pub use open::open_target;

/// Work for a background thread: returns JSON, or the message to reject with.
pub type Work = Box<dyn FnOnce() -> Result<String, String> + Send>;

/// A list of what is allowed, or all of it, or none.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Scope {
    #[default]
    None,
    All,
    Some(Vec<String>),
}

/// The `permissions` block of the app manifest.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Permissions {
    /// URLs `fetch` may reach: origins, optionally with a path prefix.
    pub http: Scope,
    /// Directories the file APIs may touch: names (`data`, `downloads`, …) or absolute paths.
    pub fs: Scope,
    pub dialog: bool,
    pub clipboard: bool,
    pub open: bool,
    pub window: bool,
}

impl Permissions {
    /// Read the block. A value of the wrong type is an error: a typo here must
    /// not quietly turn into "allowed" or "not allowed".
    pub fn parse(block: &Value) -> Result<Self, String> {
        let scope = |name: &str| -> Result<Scope, String> {
            match &block[name] {
                Value::Null | Value::Bool(false) => Ok(Scope::None),
                Value::Bool(true) => Ok(Scope::All),
                Value::Array(entries) => entries
                    .iter()
                    .map(|entry| entry.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .map(Scope::Some)
                    .ok_or_else(|| {
                        format!("app.json: permissions.{name} must be a list of strings")
                    }),
                _ => Err(format!(
                    "app.json: permissions.{name} must be true, false or a list"
                )),
            }
        };
        let flag = |name: &str| -> Result<bool, String> {
            match &block[name] {
                Value::Null => Ok(false),
                Value::Bool(allowed) => Ok(*allowed),
                _ => Err(format!(
                    "app.json: permissions.{name} must be true or false"
                )),
            }
        };
        if !block.is_null() && !block.is_object() {
            return Err("app.json: permissions must be an object".into());
        }
        Ok(Self {
            http: scope("http")?,
            fs: scope("fs")?,
            dialog: flag("dialog")?,
            clipboard: flag("clipboard")?,
            open: flag("open")?,
            window: flag("window")?,
        })
    }
}

/// The message for a call the manifest does not allow.
pub fn denied(what: &str, permission: &str) -> String {
    format!("{what} is not allowed: add it to `permissions.{permission}` in sepal.config")
}

pub struct Api {
    pub permissions: Permissions,
    /// Names the app's data directory.
    app_id: Option<String>,
    /// Paths the user chose in a dialog. Choosing a file is consent to use
    /// it, for as long as the app runs.
    picked: Mutex<Vec<PathBuf>>,
    /// Response bodies that JavaScript has not read yet, by request id.
    bodies: Mutex<HashMap<u64, Vec<u8>>>,
    next_body: Mutex<u64>,
}

impl Api {
    pub fn new(permissions: Permissions, app_id: Option<String>) -> Self {
        Self {
            permissions,
            app_id,
            picked: Mutex::default(),
            bodies: Mutex::default(),
            next_body: Mutex::new(1),
        }
    }

    /// The work behind a host call, or the reason it is refused. Refusals that
    /// need no I/O happen here, before a thread is started.
    pub fn call(self: &Arc<Self>, name: &str, args: &str) -> Result<Work, String> {
        let args: Value =
            serde_json::from_str(args).map_err(|e| format!("{name}: bad arguments: {e}"))?;
        let api = self.clone();
        let (group, action) = name.split_once('.').unwrap_or((name, ""));
        let action = action.to_owned();
        match group {
            "http" => Ok(Box::new(move || http::call(&api, &action, &args))),
            "fs" => Ok(Box::new(move || fs::call(&api, &action, &args))),
            "dialog" => {
                if !self.permissions.dialog {
                    return Err(denied("showing a dialog", "dialog"));
                }
                Ok(Box::new(move || dialog::call(&api, &action, &args)))
            }
            "open" => {
                // Checked now; starting the other app can wait for the thread.
                let command = open::command(self, &args)?;
                Ok(Box::new(move || open::run(command)))
            }
            _ => Err(format!("this host has no call named {name}")),
        }
    }

    /// Remember a path the user picked in a dialog.
    fn pick(&self, path: PathBuf) {
        self.picked.lock().unwrap().push(path);
    }
}
