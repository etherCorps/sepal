//! Native dialogs: open, save, message. What the user picks in one is
//! remembered, so the file APIs can use it without a manifest entry.

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde_json::{Value, json};

use super::Api;

fn file_dialog(args: &Value) -> FileDialog {
    let mut dialog = FileDialog::new();
    if let Some(title) = args["title"].as_str() {
        dialog = dialog.set_title(title);
    }
    if let Some(dir) = args["defaultPath"].as_str() {
        dialog = dialog.set_directory(dir);
    }
    if let Some(name) = args["defaultName"].as_str() {
        dialog = dialog.set_file_name(name);
    }
    for filter in args["filters"].as_array().into_iter().flatten() {
        let extensions: Vec<&str> = filter["extensions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        dialog = dialog.add_filter(filter["name"].as_str().unwrap_or(""), &extensions);
    }
    dialog
}

pub(super) fn call(api: &Api, action: &str, args: &Value) -> Result<String, String> {
    let picked = |path: std::path::PathBuf| {
        let text = path.to_string_lossy().into_owned();
        api.pick(path);
        text
    };
    let result = match action {
        "open" => {
            let dialog = file_dialog(args);
            if args["directory"].as_bool() == Some(true) {
                json!(dialog.pick_folder().map(picked))
            } else if args["multiple"].as_bool() == Some(true) {
                json!(
                    dialog
                        .pick_files()
                        .map(|paths| paths.into_iter().map(picked).collect::<Vec<_>>())
                )
            } else {
                json!(dialog.pick_file().map(picked))
            }
        }
        "save" => json!(file_dialog(args).save_file().map(picked)),
        "message" => {
            let confirm = args["confirm"].as_bool() == Some(true);
            let level = match args["kind"].as_str() {
                Some("warning") => MessageLevel::Warning,
                Some("error") => MessageLevel::Error,
                _ => MessageLevel::Info,
            };
            let answer = MessageDialog::new()
                .set_title(args["title"].as_str().unwrap_or(""))
                .set_description(args["message"].as_str().unwrap_or(""))
                .set_level(level)
                .set_buttons(if confirm {
                    MessageButtons::OkCancel
                } else {
                    MessageButtons::Ok
                })
                .show();
            json!(matches!(
                answer,
                MessageDialogResult::Ok | MessageDialogResult::Yes
            ))
        }
        _ => return Err(format!("this host has no call named dialog.{action}")),
    };
    Ok(result.to_string())
}
