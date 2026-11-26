//! End-to-end dispatch tests for the standalone network app.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::TempDir;

struct TestEnv {
    root: TempDir,
    home: PathBuf,
    config: PathBuf,
    data: PathBuf,
    runtime: PathBuf,
    bin: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let home = root.path().join("home");
        let config = root.path().join("config");
        let data = root.path().join("data");
        let runtime = root.path().join("run");
        let bin = root.path().join("bin");
        for dir in [&home, &config, &data, &runtime, &bin] {
            fs::create_dir_all(dir).unwrap();
        }
        Self {
            root,
            home,
            config,
            data,
            runtime,
            bin,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rofi-network"));
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
            .env_remove("ROFI_HUB_THEME")
            .env_remove("ROFI_HUB_THEME_FILE")
            .env_remove("ROFI_CONFIG")
            .env_remove("ROFI_BIN");
        command
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
             *GENERAL.CONNECTION*device*show*) printf 'GENERAL.CONNECTION:Home\\nGENERAL.HWADDR:02:00:00:00:00:01\\nIP4.ADDRESS[1]:192.0.2.42/24\\nIP4.GATEWAY:192.0.2.1\\nIP4.DNS[1]:192.0.2.1\\n' ;;\n\
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

    // 7 is the open `Open` access point: QUICK ACTIONS 0-4,
    // NEARBY WI-FI 5, Home:5G 6, Open 7.
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let calls = fs::read_to_string(&nmcli_log).unwrap();
    assert!(
        calls.contains("device wifi connect Open ifname wlan0"),
        "{calls}\nSTDERR:\n{}\nSTDIN:\n{}",
        stderr(&output),
        fs::read_to_string(&rofi_stdin).unwrap_or_default()
    );
    let args = format!(
        "{}\n{}",
        fs::read_to_string(&rofi_args).unwrap(),
        fs::read_to_string(&rofi_stdin).unwrap()
    );
    for expected in [
        "Wi-Fi: Home:5G",
        "192.0.2.42/24",
        "192.0.2.1",
        "192.0.2.0",
        "192.0.2.255",
        "02:00:00:00:00:01",
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
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let calls = fs::read_to_string(&nmcli_log).unwrap();
    assert!(calls.contains("device connect eth0"), "{calls}");
}

#[test]
fn missing_nmcli_fails_cleanly() {
    let env = TestEnv::new();
    let output = env.command().env("PATH", &env.bin).output().unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("nmcli"));
}

#[test]
fn theme_path_is_app_resolved_and_independent() {
    let env = TestEnv::new();
    let nmcli_log = env.root.path().join("nmcli.log");
    let rofi_args = env.root.path().join("rofi-args");
    let flag = env.root.path().join("rofi-first");
    env.script(
        "bin/nmcli",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in\n\
             'radio wifi') echo enabled ;;\n\
             *DEVICE,TYPE,STATE*device*) printf 'wlan0:wifi:connected\\n' ;;\n\
             *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) printf '*:Home\\n' ;;\n\
             *GENERAL.CONNECTION*device*show*) printf 'GENERAL.CONNECTION:Home\\n' ;;\n\
             esac\nexit 0\n",
            nmcli_log.display()
        ),
    );
    let rofi = env.script(
        "bin/rofi",
        &format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' \"$@\" >> '{}'\nif [ -f '{}' ]; then exit 1; fi\n: > '{}'\nprintf '1\\n'\n",
            rofi_args.display(),
            flag.display(),
            flag.display()
        ),
    );
    let app_override = env.config.join("rofi/network.rasi");
    fs::create_dir_all(app_override.parent().unwrap()).unwrap();
    fs::write(&app_override, "/* network override */\n").unwrap();

    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&rofi_args).unwrap();
    assert!(args.contains("-theme"), "{args}");
    assert!(args.contains(app_override.to_str().unwrap()), "{args}");
}

#[test]
fn rofi_theme_override_is_honored() {
    let env = TestEnv::new();
    let nmcli_log = env.root.path().join("nmcli.log");
    let rofi_args = env.root.path().join("rofi-args");
    let flag = env.root.path().join("rofi-first");
    env.script(
        "bin/nmcli",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$*\" in\n\
             'radio wifi') echo enabled ;;\n\
             *DEVICE,TYPE,STATE*device*) printf 'wlan0:wifi:connected\\n' ;;\n\
             *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) printf '*:Home\\n' ;;\n\
             *GENERAL.CONNECTION*device*show*) printf 'GENERAL.CONNECTION:Home\\n' ;;\n\
             esac\nexit 0\n",
            nmcli_log.display()
        ),
    );
    let rofi = env.script(
        "bin/rofi",
        &format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' \"$@\" >> '{}'\nif [ -f '{}' ]; then exit 1; fi\n: > '{}'\nprintf '1\\n'\n",
            rofi_args.display(),
            flag.display(),
            flag.display()
        ),
    );
    let global = env.root.path().join("global.rasi");
    fs::write(&global, "/* global */\n").unwrap();

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .env("ROFI_THEME", &global)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = fs::read_to_string(&rofi_args).unwrap();
    assert!(args.contains(global.to_str().unwrap()), "{args}");
}

#[test]
fn help_exits_zero() {
    let env = TestEnv::new();
    let output = env.command().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("rofi-network"));
}
