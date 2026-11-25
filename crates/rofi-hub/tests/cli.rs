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
            format!(
                "#!{}\nprintf '%s\\n' \"$*\" >>\"${{NOTIFY_LOG:-/dev/null}}\"\n",
                shell_path().display()
            ),
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
        let body = body.replacen("#!/bin/sh", &format!("#!{}", shell_path().display()), 1);
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
    env.provider("alpha", "Alpha", &[shell_path().to_str().unwrap()]);

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
    env.provider("alpha", "Alpha", &[shell_path().to_str().unwrap()]);

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
    env.provider("alpha", "Alpha", &[shell_path().to_str().unwrap()]);
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
    env.provider("alpha", "Alpha", &[shell_path().to_str().unwrap()]);
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

fn fake_rofi(env: &TestEnv, log: &Path) -> PathBuf {
    env.script(
        "bin/rofi",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexit 1\n",
            log.display()
        ),
    )
}

#[test]
fn manager_modes_dispatch_raw_rofi_modes() {
    let env = TestEnv::new();
    let log = env.root.path().join("rofi-args");
    let rofi = fake_rofi(&env, &log);

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["launcher"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("-show\ndrun\n"), "{args}");
    assert!(args.contains("-theme\n"), "{args}");
    assert!(args.contains("rofi-hub/theme.rasi"), "{args}");

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["show", "window", "-p", "Windows"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("-show\nwindow\n"), "{args}");
    assert!(args.contains("-p\nWindows\n"), "{args}");
}

#[test]
fn calc_and_emoji_use_their_rofi_modes() {
    let env = TestEnv::new();
    let log = env.root.path().join("rofi-args");
    let rofi = fake_rofi(&env, &log);

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["calc"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("-show\ncalc\n"), "{args}");
    assert!(args.contains("-modi\ncalc\n"), "{args}");

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["emoji"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("-show\nemoji\n"), "{args}");
    assert!(args.contains("-modi\nemoji\n"), "{args}");
}

#[test]
fn provider_aliases_open_the_same_provider() {
    let env = TestEnv::new();
    let marker = env.root.path().join("alias-marker");
    let script = env.script(
        "provider.sh",
        &format!(
            "#!/bin/sh\nprintf '%s' \"$ROFI_HUB_PROVIDER\" > '{}'\n",
            marker.display()
        ),
    );
    env.provider("clipboard", "Clipboard", &[script.to_str().unwrap()]);

    let output = env.run(&["clipboard"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(wait_for(&marker), "clipboard");
}

#[test]
fn missing_power_menu_and_nmcli_fail_cleanly() {
    let env = TestEnv::new();
    let log = env.root.path().join("rofi-args");
    let rofi = fake_rofi(&env, &log);

    let isolated_path = env.root.path().join("bin");
    let output = env
        .command()
        .env("PATH", &isolated_path)
        .env("ROFI_BIN", &rofi)
        .args(["power"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("rofi-power-menu"));

    let output = env
        .command()
        .env("PATH", &isolated_path)
        .args(["network"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("nmcli"));
}

#[test]
fn network_menu_shows_details_and_connects_to_wifi() {
    let env = TestEnv::new();
    let nmcli_log = env.root.path().join("nmcli.log");
    let rofi_args = env.root.path().join("rofi-args");
    let rofi_stdin = env.root.path().join("rofi-stdin");
    let flag = env.root.path().join("rofi-first");
    env.script(
        "bin/nmcli",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in\n\
             'radio wifi') echo enabled ;;\n\
             *DEVICE,TYPE,STATE*device*) printf 'wlan0:wifi:connected\\n' ;;\n\
             *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) printf '*:Home\\\\:5G:82:WPA2:5180\\n :Open:41:--:2412\\n' ;;\n\
             *GENERAL.CONNECTION*device*show*) printf 'GENERAL.CONNECTION:Home\\nGENERAL.HWADDR:20:BD:1D:78:96:D7\\nIP4.ADDRESS[1]:192.168.100.75/24\\nIP4.GATEWAY:192.168.100.1\\nIP4.DNS[1]:192.168.100.1\\n' ;;\n\
             *NAME,TYPE*connection*show*) printf 'Home:802-11-wireless\\n' ;;\n\
             esac\nexit 0\n",
            nmcli_log.display()
        ),
    );
    let rofi = env.script(
        "bin/rofi",
        &format!(
            "#!/bin/sh\ninput=$(cat)\nprintf '%s\\n' \"$input\" >> '{}'\n\
             printf '%s\\n' \"$@\" >> '{}'\n\
             if [ -f '{}' ]; then exit 1; fi\n\
             : > '{}'\nprintf '7\\n'\n",
            rofi_stdin.display(),
            rofi_args.display(),
            flag.display(),
            flag.display()
        ),
    );

    // 7 is the open `Open` access point: QUICK ACTIONS occupies 0-4,
    // NEARBY WI-FI is 5, Home:5G 6, Open 7.
    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["network"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let calls = fs::read_to_string(&nmcli_log).unwrap();
    assert!(
        calls.contains("device wifi connect Open ifname wlan0"),
        "{calls}"
    );
    let args = format!(
        "{}\n{}",
        fs::read_to_string(&rofi_args).unwrap(),
        fs::read_to_string(&rofi_stdin).unwrap()
    );
    for expected in [
        "Wi-Fi: Home:5G",
        "192.168.100.75/24",
        "192.168.100.1",
        "192.168.100.0",
        "192.168.100.255",
        "20:BD:1D:78:96:D7",
        "WPA2",
        "5 GHz",
        "QUICK ACTIONS",
        "NEARBY WI-FI",
        "MANAGE",
        "Edit saved network",
    ] {
        assert!(args.contains(expected), "missing {expected} in {args}");
    }
}

#[test]
fn network_quick_action_connects_wired() {
    let env = TestEnv::new();
    let nmcli_log = env.root.path().join("nmcli.log");
    let flag = env.root.path().join("rofi-first");
    env.script(
        "bin/nmcli",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in\n\
             'radio wifi') echo enabled ;;\n\
             *DEVICE,TYPE,STATE*device*) printf 'wlan0:wifi:connected\\neth0:ethernet:disconnected\\n' ;;\n\
             *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) printf '*:Home\\n' ;;\n\
             *GENERAL.CONNECTION*device*show*) printf 'GENERAL.CONNECTION:Home\\n' ;;\n\
             esac\nexit 0\n",
            nmcli_log.display()
        ),
    );
    let rofi = env.script(
        "bin/rofi",
        &format!(
            "#!/bin/sh\ncat >/dev/null\nif [ -f '{}' ]; then exit 1; fi\n: > '{}'\nprintf '1\\n'\n",
            flag.display(),
            flag.display()
        ),
    );
    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["network"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let calls = fs::read_to_string(&nmcli_log).unwrap();
    assert!(calls.contains("device connect eth0"), "{calls}");
}

#[test]
fn theme_dump_uses_app_profiles() {
    let env = TestEnv::new();
    let hub = env
        .command()
        .args(["theme", "dump", "--app", "hub"])
        .output()
        .unwrap();
    assert!(hub.status.success(), "{}", stderr(&hub));
    let hub = stdout(&hub);
    assert!(hub.contains("96ch"), "{hub}");
    assert!(hub.contains("show-icons: false"), "{hub}");

    let calendar = env
        .command()
        .args(["theme", "dump", "--app", "calendar"])
        .output()
        .unwrap();
    assert!(calendar.status.success(), "{}", stderr(&calendar));
    let calendar = stdout(&calendar);
    assert!(calendar.contains("64ch"), "{calendar}");
    assert!(calendar.contains("lines: 12;"), "{calendar}");
    assert!(calendar.contains("show-icons: false"), "{calendar}");
}

#[test]
fn help_exits_zero() {
    let env = TestEnv::new();
    let output = env.run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("rofi-hub"));
}
