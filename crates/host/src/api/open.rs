//! Open a link or a file in whatever the system uses for it.

use std::path::Path;
use std::process::Command;

use blitz_traits::net::Url;
use serde_json::Value;

use super::{Api, denied};

/// What may be opened, as the argument for the system's opener: a web or mail
/// link, or a file the app is allowed to read.
///
/// Anything else is refused. In particular no other URL scheme: on every
/// platform schemes are how one app makes another do things.
pub fn open_target(api: &Api, target: &str) -> Result<String, String> {
    if !api.permissions.open {
        return Err(denied("opening things in other apps", "open"));
    }
    if let Ok(url) = Url::parse(target)
        && url.scheme().len() > 1
    {
        return match url.scheme() {
            "http" | "https" | "mailto" => Ok(url.into()),
            scheme => Err(format!(
                "{scheme}: links cannot be opened, only http, https and mailto"
            )),
        };
    }
    if !Path::new(target).is_absolute() {
        return Err(format!("{target:?} is neither a link nor an absolute path"));
    }
    let path = api.allowed_path(Path::new(target))?;
    Ok(path.to_string_lossy().into_owned())
}

pub(super) fn command(api: &Api, args: &Value) -> Result<Command, String> {
    let target = open_target(
        api,
        args["target"]
            .as_str()
            .ok_or("open needs a link or a path")?,
    )?;
    let mut command;
    if cfg!(target_os = "macos") {
        command = Command::new("open");
    } else if cfg!(windows) {
        // `start` is a shell builtin; the empty string is the window title it expects first.
        command = Command::new("cmd");
        command.args(["/C", "start", ""]);
    } else {
        command = Command::new("xdg-open");
    }
    command.arg(target);
    Ok(command)
}

pub(super) fn run(mut command: Command) -> Result<String, String> {
    let status = command
        .status()
        .map_err(|e| format!("cannot open it: {e}"))?;
    if status.success() {
        Ok("null".into())
    } else {
        Err(format!("the system could not open it ({status})"))
    }
}
