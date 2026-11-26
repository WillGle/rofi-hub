//! Best-effort desktop notifications.

use std::process::{Command, Stdio};

use crate::which;

pub const APP_NAME: &str = "Rofi Hub";

/// Show a normal notification. Missing `notify-send` is not an error: the
/// caller still has stderr and its exit status.
pub fn notify(summary: &str, body: &str) {
    send("normal", summary, body);
}

pub fn notify_error(summary: &str, body: &str) {
    send("critical", summary, body);
}

fn send(urgency: &str, summary: &str, body: &str) {
    let Some(program) = which::find("notify-send") else {
        return;
    };
    let _ = Command::new(program)
        .arg("--app-name")
        .arg(APP_NAME)
        .arg("--urgency")
        .arg(urgency)
        .arg(summary)
        .arg(body)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
