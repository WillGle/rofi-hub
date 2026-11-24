//! Small wrapper around `rofi -dmenu`.

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::{Error, Item, Result, paths, which};

/// What the user did in the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The `id` of the selected row (the text before the first NUL).
    Selected(String),
    Cancelled,
    /// A `-kb-custom-N` binding. The value is zero-based (`Custom(0)` is
    /// `kb-custom-1`, exit status 10).
    Custom(u8),
}

#[derive(Debug, Clone, Default)]
pub struct DmenuOptions {
    pub prompt: Option<String>,
    pub theme: Option<PathBuf>,
    /// Rofi configuration passed with `-config` (for example `ROFI_CONFIG`).
    pub config: Option<PathBuf>,
    /// Per-launcher pidfile passed with `-pid`. Keeping this separate from
    /// the default rofi.pid avoids interfering with other Rofi prompts.
    pub pid: Option<PathBuf>,
    /// Allow free-form input instead of only the supplied rows. The calendar
    /// uses this for its date search box.
    pub allow_custom: bool,
    pub extra_args: Vec<String>,
}

impl DmenuOptions {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rofi {
    binary: PathBuf,
}

impl Rofi {
    /// Resolve `ROFI_BIN` first, then `PATH`. The override is a path or a
    /// command name; an invalid override is an error rather than a silent
    /// fallback, so a broken desktop shortcut is visible.
    pub fn find() -> Result<Self> {
        if let Some(raw) = env::var_os("ROFI_BIN") {
            let value = raw.to_string_lossy();
            let value = value.trim();
            if !value.is_empty() {
                let binary = if value.contains('/') {
                    let path = paths::expand_tilde(value);
                    which::is_executable(&path).then_some(path)
                } else {
                    which::find(value)
                };
                return binary
                    .map(|binary| Self { binary })
                    .ok_or(Error::RofiMissing);
            }
        }
        let binary = which::find("rofi").ok_or(Error::RofiMissing)?;
        Ok(Self { binary })
    }

    pub fn from_path(binary: PathBuf) -> Self {
        Self { binary }
    }

    pub fn binary(&self) -> &std::path::Path {
        &self.binary
    }

    pub fn version(&self) -> Option<String> {
        let output = Command::new(&self.binary)
            .arg("-version")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let line = text.lines().find(|line| !line.trim().is_empty())?;
        Some(
            line.split_once(':')
                .map(|(_, value)| value.trim().to_owned())
                .unwrap_or_else(|| line.trim().to_owned()),
        )
    }

    /// Build the dmenu command. Kept separate from `dmenu` so argument
    /// handling can be tested without a display.
    pub fn dmenu_command(&self, options: &DmenuOptions, icons: bool) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .arg("-dmenu")
            .arg("-i")
            .arg("-format")
            .arg("s")
            .arg("-disable-history")
            .arg("-markup-rows");

        if !options.allow_custom {
            command.arg("-no-custom");
        }

        if icons {
            command.arg("-show-icons");
        }
        if let Some(prompt) = &options.prompt {
            command.arg("-p").arg(prompt);
        }
        if let Some(config) = &options.config {
            command.arg("-config").arg(config);
        }
        if let Some(pid) = &options.pid {
            command.arg("-pid").arg(pid);
        }
        if let Some(theme) = &options.theme {
            command.arg("-theme").arg(theme);
        }
        for arg in &options.extra_args {
            command.arg(arg);
        }
        command
    }

    /// Show the rows and return the selected row id. Rofi's `-format s`
    /// returns the text before the first NUL, which is exactly `Item::id`;
    /// filtering in Rofi does not change it.
    pub fn dmenu(&self, items: &[Item], options: &DmenuOptions) -> Result<Outcome> {
        if items.is_empty() {
            return Ok(Outcome::Cancelled);
        }
        let icons = items.iter().any(|item| item.icon.is_some());
        let mut command = self.dmenu_command(options, icons);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = command.spawn().map_err(|source| Error::Spawn {
            command: self.binary.display().to_string(),
            source,
        })?;

        if let Some(stdin) = child.stdin.as_mut() {
            for item in items {
                writeln!(stdin, "{}", item.to_line())?;
            }
        }
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
        let status = output.status.code().unwrap_or(-1);
        match status {
            0 => {
                let text = String::from_utf8_lossy(&output.stdout);
                let id = text.trim_end_matches(['\r', '\n']);
                if id.is_empty() {
                    Ok(Outcome::Cancelled)
                } else {
                    Ok(Outcome::Selected(id.to_owned()))
                }
            }
            1 => Ok(Outcome::Cancelled),
            10..=28 => Ok(Outcome::Custom((status - 10) as u8)),
            other => Err(Error::RofiStatus(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(options: &DmenuOptions, icons: bool) -> Vec<String> {
        Rofi::from_path(PathBuf::from("/usr/bin/rofi"))
            .dmenu_command(options, icons)
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn builds_a_predictable_dmenu_command() {
        let options = DmenuOptions {
            prompt: Some("Hub".to_owned()),
            theme: Some(PathBuf::from("/tmp/theme.rasi")),
            config: Some(PathBuf::from("/tmp/config.rasi")),
            pid: Some(PathBuf::from("/tmp/rofi-hub.pid")),
            allow_custom: false,
            extra_args: vec!["-kb-custom-1".to_owned(), "Alt+1".to_owned()],
        };
        let args = args(&options, true);
        assert_eq!(args[0], "-dmenu");
        assert!(args.contains(&"-no-custom".to_owned()));
        assert!(args.contains(&"-format".to_owned()));
        assert!(args.contains(&"s".to_owned()));
        assert!(args.contains(&"-show-icons".to_owned()));
        assert!(args.contains(&"/tmp/theme.rasi".to_owned()));
        assert!(args.contains(&"/tmp/config.rasi".to_owned()));
        assert!(args.contains(&"/tmp/rofi-hub.pid".to_owned()));
        assert_eq!(args[args.len() - 2], "-kb-custom-1");
    }

    #[test]
    fn hides_icons_when_no_row_has_one() {
        let args = args(&DmenuOptions::default(), false);
        assert!(!args.contains(&"-show-icons".to_owned()));
    }
}
