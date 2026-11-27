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
        let providers = data.join("rofi-apps/providers");
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

    fn write_config(&self, relative: &str, text: &str) {
        let path = self.config.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rofi-hub-menu"));
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
            .env_remove("ROFI_HUB_THEME")
            .env_remove("ROFI_HUB_THEME_FILE")
            .env_remove("ROFI_HUB_PROVIDER")
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
    let saved = fs::read_to_string(env.config.join("rofi-hub-menu/theme")).unwrap();
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
fn auto_clears_the_menu_override_and_restores_the_configured_theme() {
    let env = TestEnv::new();
    env.write_config(
        "rofi-hub-menu/config.toml",
        "[theme]\nmode = \"solarized\"\n",
    );

    let output = env.run(&["theme", "set", "nord"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        fs::read_to_string(env.config.join("rofi-hub-menu/theme")).unwrap(),
        "nord\n"
    );

    let output = env.run(&["theme", "set", "auto"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !env.config.join("rofi-hub-menu/theme").exists(),
        "Ctrl+4/auto must remove the menu override"
    );

    let output = env.run(&["theme", "show"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("mode:   solarized"), "{text}");
    assert!(text.contains("source: config.toml"), "{text}");
}

#[test]
fn theme_change_keeps_filter_and_selected_provider_id() {
    let env = TestEnv::new();
    let marker = env.root.path().join("provider-marker");
    let provider = env.script(
        "provider.sh",
        &format!(
            "#!{}\nprintf '%s' \"$1\" > '{}'\n",
            shell_path().display(),
            marker.display()
        ),
    );
    env.provider(
        "alpha-one",
        "Alpha One",
        &[provider.to_str().unwrap(), "first"],
    );
    env.provider(
        "alpha-two",
        "Alpha Two",
        &[provider.to_str().unwrap(), "second"],
    );

    let args_log = env.root.path().join("rofi-replies-args");
    let reply_path = env.root.path().join("rofi-replies");
    fs::write(&reply_path, "@custom:0:2:Alpha\nalpha-two\n").unwrap();
    let rofi = env.script(
        "bin/rofi",
        &format!(
            "#!/bin/sh\n\
             printf 'INVOCATION\\n' >> '{}'\n\
             printf '%s\\n' \"$@\" >> '{}'\n\
             cat >/dev/null\n\
             reply=$(sed -n '1p' '{}')\n\
             sed '1d' '{}' > '{}.tmp' && mv '{}.tmp' '{}'\n\
             case \"$reply\" in\n\
               @custom:*:*:*)\n\
                 rest=${{reply#@custom:}}\n\
                 code=${{rest%%:*}}\n\
                 rest=${{rest#*:}}\n\
                 row=${{rest%%:*}}\n\
                 filter=${{rest#*:}}\n\
                 printf '%s\\n%s\\n' \"$row\" \"$filter\"\n\
                 exit $((10 + code)) ;;\n\
               *) printf '%s\\n' \"$reply\"; exit 0 ;;\n\
             esac\n",
            args_log.display(),
            args_log.display(),
            reply_path.display(),
            reply_path.display(),
            reply_path.display(),
            reply_path.display(),
            reply_path.display(),
        ),
    );

    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(wait_for(&marker), "second");
    let args = fs::read_to_string(&args_log).unwrap();
    assert_eq!(args.matches("INVOCATION").count(), 2, "{args}");
    assert!(args.contains("-filter\nAlpha\n"), "{args}");
    assert!(args.contains("-selected-row\n2\n"), "{args}");
    let second = args.rsplit("INVOCATION").next().unwrap();
    assert!(second.contains("rofi/theme-hub.rasi"), "{second}");
    assert!(!second.contains("theme-hub-menu.rasi"), "{second}");
}

#[test]
fn launched_providers_get_a_clean_environment_and_their_args() {
    let env = TestEnv::new();
    let marker = env.root.path().join("marker");
    let script = env.script(
        "provider.sh",
        &format!(
            "#!/bin/sh\n{{ env | grep -E '^ROFI_HUB_' || true; }} > '{}.hub'\nprintf '%s' \"$1\" > '{}'\n",
            marker.display(),
            marker.display()
        ),
    );
    env.provider(
        "alpha",
        "Alpha",
        &[script.to_str().unwrap(), "first argument"],
    );

    let output = env.command().args(["alpha"]).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(wait_for(&marker), "first argument");
    assert_eq!(
        fs::read_to_string(format!("{}.hub", marker.display())).unwrap(),
        "",
        "the menu must not inject ROFI_HUB_* into app processes"
    );
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
    // Keep the test independent of a packaged fallback installed on the host:
    // a user base theme must take precedence and produce a runtime theme.
    fs::create_dir_all(env.config.join("rofi")).unwrap();
    fs::write(
        env.config.join("rofi/theme.rasi"),
        "* { background-color: #101010; }\n",
    )
    .unwrap();
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
    assert!(args.contains("rofi/theme-hub-menu.rasi"), "{args}");
    assert!(args.contains("columns: 3; lines: 10;"), "{args}");

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
    assert!(args.contains("columns: 4; lines: 15;"), "{args}");
}

#[test]
fn power_menu_uses_a_three_column_grid() {
    let env = TestEnv::new();
    let log = env.root.path().join("rofi-args");
    let rofi = fake_rofi(&env, &log);
    env.script(
        "bin/rofi-power-menu",
        "#!/bin/sh\nprintf 'Shut down\\nReboot\\nSuspend\\n'",
    );

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["power"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("-show\npower-menu\n"), "{args}");
    assert!(args.contains("-modi\npower-menu:"), "{args}");
    assert!(args.contains("columns: 3; lines: 2;"), "{args}");
}

#[test]
fn missing_power_menu_fails_cleanly() {
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
    assert!(calendar.contains("46ch"), "{calendar}");
    assert!(calendar.contains("lines: 1;"), "{calendar}");
    assert!(calendar.contains("columns: 3;"), "{calendar}");
    assert!(
        calendar.contains("placeholder: \"DD/MM/YYYY\";"),
        "{calendar}"
    );
    assert!(calendar.contains("show-icons: false"), "{calendar}");

    let keybinds = env
        .command()
        .args(["theme", "dump", "--app", "keybinds"])
        .output()
        .unwrap();
    assert!(keybinds.status.success(), "{}", stderr(&keybinds));
    let keybinds = stdout(&keybinds);
    assert!(keybinds.contains("columns: 3;"), "{keybinds}");
    assert!(keybinds.contains("lines: 10;"), "{keybinds}");
}

#[test]
fn launcher_columns_are_applied_from_config() {
    let env = TestEnv::new();
    let log = env.root.path().join("rofi-args");
    let rofi = fake_rofi(&env, &log);
    env.write_config(
        "rofi-hub-menu/config.toml",
        "[launcher]\ncolumns = 3\nlines = 10\n",
    );

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .args(["launcher"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&log).unwrap();
    assert!(args.contains("-theme-str"), "{args}");
    assert!(
        args.contains("listview { columns: 3; lines: 10; }"),
        "{args}"
    );
}

#[test]
fn help_exits_zero() {
    let env = TestEnv::new();
    let output = env.run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("rofi-hub-menu"));
}
