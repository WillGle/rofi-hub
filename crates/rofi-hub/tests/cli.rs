//! End-to-end dispatch tests with a temporary XDG environment.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::sleep;
use std::time::{Duration, Instant};

use tempfile::TempDir;

struct TestEnv {
    root: TempDir,
    home: PathBuf,
    config: PathBuf,
    data: PathBuf,
    runtime: PathBuf,
    providers: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let home = root.path().join("home");
        let config = root.path().join("config");
        let data = root.path().join("data");
        let runtime = root.path().join("run");
        let providers = data.join("rofi-hub/providers");
        for dir in [&home, &config, &data, &runtime, &providers] {
            fs::create_dir_all(dir).unwrap();
        }
        let bin = root.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let notify = bin.join("notify-send");
        fs::write(
            &notify,
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >>\"${NOTIFY_LOG:-/dev/null}\"\n",
        )
        .unwrap();
        let mut perms = fs::metadata(&notify).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&notify, perms).unwrap();
        Self {
            root,
            home,
            config,
            data,
            runtime,
            providers,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rofi-hub"));
        let path = format!(
            "{}:{}",
            self.root.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        command
            .env("PATH", path)
            .env("NOTIFY_LOG", self.root.path().join("notify.log"))
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("ROFI_HUB_PROVIDER_DIR", &self.providers)
            .env_remove("ROFI_HUB_THEME")
            .env_remove("ROFI_CONFIG")
            .env_remove("ROFI_BIN");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn provider(&self, id: &str, name: &str, exec: &[&str]) -> PathBuf {
        let exec = exec
            .iter()
            .map(|value| format!("\"{value}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let text = format!(
            "id = \"{id}\"\nname = \"{name}\"\ndescription = \"Test provider\"\nexec = [{exec}]\n"
        );
        let path = self.providers.join(format!("{id}.toml"));
        fs::write(&path, text).unwrap();
        path
    }

    fn script(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, body).unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn wait_for(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if let Ok(text) = fs::read_to_string(path) {
            return text;
        }
        sleep(Duration::from_millis(20));
    }
    panic!("provider did not write {}", path.display());
}

#[test]
fn list_prints_providers_and_json() {
    let env = TestEnv::new();
    env.provider("alpha", "Alpha", &["/bin/sh"]);

    let output = env.run(&["list"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("alpha"), "{text}");
    assert!(text.contains("ok"), "{text}");

    let output = env.run(&["list", "--json"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value[0]["id"], "alpha");
    assert_eq!(value[0]["available"], true);
    assert_eq!(value[0]["category"], "Apps");
}

#[test]
fn theme_choice_is_persisted_and_reported() {
    let env = TestEnv::new();

    let output = env.run(&["theme", "set", "nord"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let saved = fs::read_to_string(env.config.join("rofi-hub/theme")).unwrap();
    assert_eq!(saved, "nord\n");

    let output = env.run(&["theme", "show"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("mode:   nord"), "{text}");
    assert!(text.contains("source: saved choice"), "{text}");
}

#[test]
fn unknown_provider_and_theme_fail_cleanly() {
    let env = TestEnv::new();
    env.provider("alpha", "Alpha", &["/bin/sh"]);

    let output = env.run(&["definitely-missing"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown provider `definitely-missing`"));

    let output = env.run(&["theme", "set", "sepia"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown theme `sepia`"));
    assert!(
        !env.root.path().join("notify.log").exists(),
        "CLI errors must not reach the desktop notifier"
    );
}

#[test]
fn selecting_a_provider_passes_its_environment_and_args() {
    let env = TestEnv::new();
    let marker = env.root.path().join("marker");
    let script = env.script(
        "provider.sh",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$ROFI_HUB_PROVIDER\" \"$ROFI_HUB_THEME\" \"$ROFI_HUB_THEME_FILE\" \"$1\" > '{}'\n",
            marker.display()
        ),
    );
    env.provider(
        "alpha",
        "Alpha",
        &[script.to_str().unwrap(), "first argument"],
    );

    let output = env.run(&["--theme", "gruvbox", "alpha"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = wait_for(&marker);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "alpha");
    assert_eq!(lines[1], "gruvbox");
    assert!(lines[2].ends_with("rofi-hub/theme.rasi"), "{text}");
    assert_eq!(lines[3], "first argument");
}

#[test]
fn doctor_reports_a_healthy_install() {
    let env = TestEnv::new();
    env.provider("alpha", "Alpha", &["/bin/sh"]);
    let bin = env.root.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    env.script("bin/rofi", "#!/bin/sh\nprintf 'Version: 2.0.0\\n'\n");
    let path = format!(
        "{}:{}:{}",
        env.root.path().join("bin").display(),
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = env
        .command()
        .env("PATH", &path)
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["rofi"]["version"], "2.0.0");
    assert_eq!(value["providers"][0]["available"], true);
}

#[test]
fn doctor_honors_rofi_bin() {
    let env = TestEnv::new();
    env.provider("alpha", "Alpha", &["/bin/sh"]);
    let rofi = env.script("fake-rofi", "#!/bin/sh\nprintf 'Version: 2.0.0\\n'\n");

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["doctor"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("status         ok"));
}

#[test]
fn help_exits_zero() {
    let env = TestEnv::new();
    let output = env.run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("rofi-hub"));
}
