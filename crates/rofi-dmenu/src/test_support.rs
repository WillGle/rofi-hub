//! Integration-test harness for the app binaries.
//!
//! Enabled with the `test-support` feature. It creates an isolated XDG
//! environment and a fake `rofi` that records arguments and stdin, then replies
//! with a queued selection when one is available.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

pub struct TestEnv {
    root: TempDir,
    home: PathBuf,
    config: PathBuf,
    data: PathBuf,
    runtime: PathBuf,
    bin: PathBuf,
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEnv {
    pub fn new() -> Self {
        let root = TempDir::new().expect("temporary test directory");
        let home = root.path().join("home");
        let config = root.path().join("config");
        let data = root.path().join("data");
        let runtime = root.path().join("run");
        let bin = root.path().join("bin");
        for dir in [&home, &config, &data, &runtime, &bin] {
            fs::create_dir_all(dir).expect("create test directory");
        }

        let env = Self {
            root,
            home,
            config,
            data,
            runtime,
            bin,
        };
        env.write_fake_rofi();
        env
    }

    /// A command with the isolated environment already applied.
    pub fn command(&self, program: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(program);
        let path = format!(
            "{}:{}",
            self.bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command
            .env("PATH", path)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("ROFI_BIN", self.bin.join("rofi"))
            .env("ROFI_ARGS_LOG", self.root.path().join("rofi-args.log"))
            .env("ROFI_STDIN_LOG", self.root.path().join("rofi-stdin.log"))
            .env("ROFI_REPLY", self.root.path().join("rofi-reply"))
            .env_remove("ROFI_CONFIG")
            .env_remove("ROFI_THEME")
            .env_remove("ROFI_HUB_THEME")
            .env_remove("ROFI_HUB_THEME_FILE")
            .env_remove("ROFI_CALENDAR_TODAY");
        command
    }

    pub fn run(&self, program: impl AsRef<OsStr>, args: &[&str]) -> Output {
        self.command(program)
            .args(args)
            .output()
            .expect("run binary")
    }

    /// Queue one fake-rofi reply. The next invocation consumes it and exits 0;
    /// later invocations exit 1 (cancel).
    pub fn set_reply(&self, text: &str) {
        self.set_replies(&[text]);
    }

    /// Queue several replies. `@custom:N` exits with Rofi's custom-key status
    /// `10 + N`; `@cancel` exits 1; anything else is printed as a selection.
    pub fn set_replies(&self, replies: &[&str]) {
        let body: String = replies.iter().map(|reply| format!("{reply}\n")).collect();
        fs::write(self.root.path().join("rofi-reply"), body).expect("write fake rofi replies");
    }

    pub fn args_log(&self) -> String {
        fs::read_to_string(self.root.path().join("rofi-args.log")).unwrap_or_default()
    }

    pub fn stdin_log(&self) -> String {
        fs::read_to_string(self.root.path().join("rofi-stdin.log")).unwrap_or_default()
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn config_home(&self) -> &Path {
        &self.config
    }

    pub fn bin_path(&self) -> &Path {
        &self.bin
    }

    /// Write a file under `XDG_CONFIG_HOME`.
    pub fn write_config(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.config.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create config directory");
        }
        fs::write(&path, text).expect("write config file");
        path
    }

    /// Write an executable stub into the test `PATH`. A shebang is added when
    /// the body does not already start with one.
    pub fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.bin.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create script directory");
        }
        if body.starts_with("#!") {
            fs::write(&path, body).expect("write test script");
        } else {
            fs::write(&path, format!("#!{}\n{body}", shell_path().display()))
                .expect("write test script");
        }
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    fn write_fake_rofi(&self) {
        let body = format!(
            "#!{}\n\
             printf 'INVOCATION\\n' >>\"${{ROFI_ARGS_LOG:-/dev/null}}\"\n\
             printf 'ARG:%s\\n' \"$@\" >>\"${{ROFI_ARGS_LOG:-/dev/null}}\"\n\
             cat >>\"${{ROFI_STDIN_LOG:-/dev/null}}\"\n\
             if [ -f \"${{ROFI_REPLY:-/nonexistent}}\" ]; then\n\
               first=$(sed -n '1p' \"$ROFI_REPLY\")\n\
               if [ \"$(wc -l < \"$ROFI_REPLY\")\" -gt 1 ]; then\n\
                 sed '1d' \"$ROFI_REPLY\" > \"$ROFI_REPLY.tmp\"\n\
                 mv \"$ROFI_REPLY.tmp\" \"$ROFI_REPLY\"\n\
               else\n\
                 rm -f \"$ROFI_REPLY\"\n\
               fi\n\
               case \"$first\" in\n\
                 @custom:*) exit $((10 + ${{first#@custom:}})) ;;\n\
                 @cancel) exit 1 ;;\n\
                 *) printf '%s\\n' \"$first\"; exit 0 ;;\n\
               esac\n\
             fi\n\
             exit 1\n",
            shell_path().display()
        );
        self.script("rofi", &body);
    }
}

/// A shell from PATH. Nix build sandboxes do not provide `/bin/sh`.
fn shell_path() -> PathBuf {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("sh");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from("/bin/sh")
}
