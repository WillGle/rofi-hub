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
            .env_remove("ROFI_BIN")
            .env_remove("TERMINAL");
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

    fn nmcli(&self, body: &str) -> PathBuf {
        let log = self.root.path().join("nmcli.log");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{}\nexit 0\n",
            log.display(),
            body
        );
        self.script("bin/nmcli", &script)
    }

    /// Fake Rofi with a queue of replies. `@cancel` exits 1, `@custom:N` exits
    /// with Rofi's custom-key status, and anything else is a selected id.
    fn rofi(&self, replies: &[&str]) -> PathBuf {
        let reply_path = self.root.path().join("rofi-replies");
        let args_path = self.root.path().join("rofi-args");
        let stdin_path = self.root.path().join("rofi-stdin");
        fs::write(&reply_path, replies.join("\n") + "\n").unwrap();
        let body = format!(
            "#!/bin/sh\n\
             input=$(cat)\n\
             printf '%s\\n' \"$input\" >> '{}'\n\
             printf '%s\\n' \"$@\" >> '{}'\n\
             reply=$(sed -n '1p' '{}')\n\
             if [ -z \"$reply\" ]; then exit 1; fi\n\
             sed '1d' '{}' > '{}.tmp' && mv '{}.tmp' '{}'\n\
             case \"$reply\" in\n\
               @custom:*) exit $((10 + ${{reply#@custom:}})) ;;\n\
               @cancel) exit 1 ;;\n\
               *) printf '%s\\n' \"$reply\"; exit 0 ;;\n\
             esac\n",
            stdin_path.display(),
            args_path.display(),
            reply_path.display(),
            reply_path.display(),
            reply_path.display(),
            reply_path.display(),
            reply_path.display()
        );
        self.script("bin/rofi", &body)
    }

    fn args(&self) -> String {
        fs::read_to_string(self.root.path().join("rofi-args")).unwrap_or_default()
    }

    fn stdin(&self) -> String {
        fs::read_to_string(self.root.path().join("rofi-stdin")).unwrap_or_default()
    }

    fn nmcli_log(&self) -> String {
        fs::read_to_string(self.root.path().join("nmcli.log")).unwrap_or_default()
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

const DEVICES: &str = "printf 'wlan0:wifi:connected\\neth0:ethernet:unavailable\\n'";

fn wifi_fixture(extra: &str) -> String {
    format!(
        "case \"$*\" in\n\
         'radio wifi') echo enabled ;;\n\
         *DEVICE,TYPE,STATE*device*) {DEVICES} ;;\n\
         *IN-USE,SSID,SIGNAL,SECURITY,FREQ*) {extra} ;;\n\
         *GENERAL.CONNECTION*device*show*)\n\
           printf 'GENERAL.CONNECTION:Example WiFi\\n';\n\
           printf 'GENERAL.HWADDR:02:00:00:00:00:01\\n';\n\
           printf 'IP4.ADDRESS[1]:192.0.2.42/24\\n';\n\
           printf 'IP4.GATEWAY:192.0.2.1\\n';\n\
           printf 'IP4.DNS[1]:192.0.2.53\\n';\n\
           printf 'IP4.DNS[2]:192.0.2.54' ;;\n\
         *NAME,TYPE*connection*show*)\n\
           printf 'Example WiFi:802-11-wireless\\nOffice WiFi:802-11-wireless\\nWired:802-3-ethernet\\n' ;;\n\
         esac"
    )
}

#[test]
fn main_screen_shows_connection_and_sorted_networks() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture(
        "printf '*:Example WiFi:77:WPA2:5180\\n';\
         printf ' :Cafe Open:45:--:2462\\n';\
         printf ' :Neighbor 5G:92:WPA1 WPA2:5180\\n';\
         printf ' :Alpha:92:WPA2:2412\\n';\
         printf ' :Office WiFi:55:WPA2:5745\\n';\
         printf ' :IoT 2.4G:69:WPA2:2412'",
    ));
    let rofi = env.rofi(&["@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let args = env.args();
    let stdin = env.stdin();
    for expected in ["Connected", "Example WiFi", "Alt+r Scan", "Alt+m Manage"] {
        assert!(args.contains(expected), "missing {expected} in {args}");
    }
    for row in [
        "Alpha",
        "Neighbor 5G",
        "Cafe Open",
        "Office WiFi",
        "IoT 2.4G",
    ] {
        assert!(stdin.contains(row), "missing {row} in {stdin}");
    }
    assert!(
        !stdin.contains("ap:Example WiFi"),
        "active SSID is listed again: {stdin}"
    );
    let alpha = stdin.find("Alpha").unwrap();
    let neighbor = stdin.find("Neighbor").unwrap();
    assert!(alpha < neighbor, "equal signals sort by name: {stdin}");
    assert!(stdin.contains("92%"), "{stdin}");
    assert!(stdin.contains("Secured"), "{stdin}");
    assert!(stdin.contains("Open"), "{stdin}");
    for old in ["QUICK ACTIONS", "NEARBY WI-FI", "MANAGE", "192.0.2.42/24"] {
        assert!(!args.contains(old), "old top-card content `{old}`: {args}");
    }
}

#[test]
fn open_network_connects_without_a_password_prompt() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture(
        "printf '*:Example WiFi:77:WPA2:5180\\n';\
         printf ' :Cafe Open:45:--:2462'",
    ));
    let rofi = env.rofi(&["ap:Cafe Open", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let log = env.nmcli_log();
    assert!(
        log.contains("device wifi connect Cafe Open ifname wlan0"),
        "{log}"
    );
    assert!(!log.contains("password"), "{log}");
    assert!(!env.args().contains("-password"), "{}", env.args());
}

#[test]
fn secured_network_prompts_and_cancel_sends_no_connect() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture(
        "printf '*:Example WiFi:77:WPA2:5180\\n';\
         printf ' :Neighbor 5G:92:WPA1 WPA2:5180'",
    ));
    let rofi = env.rofi(&["ap:Neighbor 5G", "@cancel", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    assert!(
        !env.nmcli_log().contains("device wifi connect"),
        "{}",
        env.nmcli_log()
    );
    assert!(env.args().contains("-password"), "{}", env.args());
}

#[test]
fn connect_failure_is_shown_and_the_list_stays_usable() {
    let env = TestEnv::new();
    env.nmcli(&format!(
        "{}\ncase \"$*\" in\n  *'wifi connect'*) printf 'Error: wrong password\\n' >&2; exit 4 ;;\nesac",
        wifi_fixture(
            "printf '*:Example WiFi:77:WPA2:5180\\n';\
             printf ' :Neighbor 5G:92:WPA1 WPA2:5180'",
        )
    ));
    let rofi = env.rofi(&["ap:Neighbor 5G", "bad-password", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(env.args().contains("Could not connect"), "{}", env.args());
}

#[test]
fn alt_r_scans_and_alt_m_opens_manage() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture(
        "printf '*:Example WiFi:77:WPA2:5180\\n';\
         printf ' :Office WiFi:55:WPA2:5745'",
    ));
    let rofi = env.rofi(&["@custom:0", "@custom:1", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    assert!(
        env.nmcli_log().contains("device wifi rescan ifname wlan0"),
        "{}",
        env.nmcli_log()
    );
    let args = env.args();
    assert!(args.contains("Manage network"), "{args}");
    for expected in [
        "Connection details",
        "Turn Wi-Fi off",
        "Connect to hidden network",
        "Edit saved connection",
        "Disconnect current Wi-Fi",
        "Connect Ethernet eth0",
        "Back",
    ] {
        assert!(
            env.stdin().contains(expected),
            "missing {expected}: {}",
            env.stdin()
        );
    }
}

#[test]
fn details_are_queried_only_when_the_screen_opens() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture(
        "printf '*:Example WiFi:77:WPA2:5180\\n';\
         printf ' :Office WiFi:55:WPA2:5745'",
    ));
    let rofi = env.rofi(&[
        "@custom:1",
        "manage:details",
        "@cancel",
        "@cancel",
        "@cancel",
    ]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let details_calls = env
        .nmcli_log()
        .lines()
        .filter(|line| line.contains("device show"))
        .count();
    assert_eq!(details_calls, 1, "{}", env.nmcli_log());
    let args = env.args();
    for expected in [
        "Example WiFi",
        "Interface",
        "wlan0",
        "Band",
        "5 GHz",
        "WPA2",
        "IP/prefix",
        "192.0.2.42/24",
        "Gateway",
        "192.0.2.1",
        "Subnet",
        "192.0.2.0",
        "Broadcast",
        "192.0.2.255",
        "DNS",
        "192.0.2.53, 192.0.2.54",
        "MAC",
        "02:00:00:00:00:01",
    ] {
        assert!(args.contains(expected), "missing {expected} in {args}");
    }
}

#[test]
fn ethernet_connect_and_disconnect_use_the_named_interface() {
    let env = TestEnv::new();
    env.nmcli(&format!(
        "{}\ncase \"$*\" in\n  *'device connect eth0'*) printf 'connected\\n' ;;\nesac",
        wifi_fixture("printf '*:Example WiFi:77:WPA2:5180'")
    ));
    let rofi = env.rofi(&["@custom:1", "ethernet:connect:eth0", "@cancel", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        env.nmcli_log().contains("device connect eth0"),
        "{}",
        env.nmcli_log()
    );
}

#[test]
fn hidden_network_cancel_returns_to_manage_without_connecting() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture("printf '*:Example WiFi:77:WPA2:5180'"));
    let rofi = env.rofi(&["@custom:1", "manage:hidden", "@cancel", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !env.nmcli_log().contains("hidden yes"),
        "{}",
        env.nmcli_log()
    );
}

#[test]
fn no_adapter_lists_ethernet_and_wifi_off_lists_turn_on() {
    let env = TestEnv::new();
    env.nmcli(
        "case \"$*\" in\n\
         'radio wifi') echo enabled ;;\n\
         *DEVICE,TYPE,STATE*device*) printf 'eth0:ethernet:disconnected\\n' ;;\n\
         *NAME,TYPE*connection*show*) printf 'Wired:802-3-ethernet\\n' ;;\n\
         esac",
    );
    let rofi = env.rofi(&["@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(env.args().contains("No Wi-Fi adapter"), "{}", env.args());
    assert!(
        env.stdin().contains("Connect Ethernet eth0"),
        "{}",
        env.stdin()
    );

    let env = TestEnv::new();
    env.nmcli(
        "case \"$*\" in\n\
         'radio wifi') echo disabled ;;\n\
         *DEVICE,TYPE,STATE*device*) printf 'wlan0:wifi:disconnected\\n' ;;\n\
         esac",
    );
    let rofi = env.rofi(&["action:wifi-on", "@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(env.args().contains("Wi-Fi off"), "{}", env.args());
    assert!(env.stdin().contains("Turn Wi-Fi on"), "{}", env.stdin());
    assert!(
        env.nmcli_log().contains("radio wifi on"),
        "{}",
        env.nmcli_log()
    );
}

#[test]
fn only_the_active_network_reports_no_other_networks() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture("printf '*:Example WiFi:77:WPA2:5180'"));
    let rofi = env.rofi(&["@cancel"]);
    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = env.args();
    assert!(args.contains("-selected-row\n1"), "{args}");
    assert!(
        env.stdin().contains("No other networks found"),
        "{}",
        env.stdin()
    );
    assert!(env.stdin().contains("Scan for networks"), "{}", env.stdin());
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
    env.nmcli(&wifi_fixture("printf '*:Home:82:WPA2:5180'"));
    let rofi = env.rofi(&["@cancel"]);
    let app_override = env.config.join("rofi/network.rasi");
    fs::create_dir_all(app_override.parent().unwrap()).unwrap();
    fs::write(&app_override, "/* network override */\n").unwrap();

    let output = env.command().env("ROFI_BIN", &rofi).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let args = env.args();
    assert!(args.contains("-theme"), "{args}");
    assert!(args.contains(app_override.to_str().unwrap()), "{args}");
}

#[test]
fn rofi_theme_override_is_honored() {
    let env = TestEnv::new();
    env.nmcli(&wifi_fixture("printf '*:Home:82:WPA2:5180'"));
    let rofi = env.rofi(&["@cancel"]);
    let global = env.root.path().join("global.rasi");
    fs::write(&global, "/* global */\n").unwrap();

    let output = env
        .command()
        .env("ROFI_BIN", &rofi)
        .env("ROFI_THEME", &global)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        env.args().contains(global.to_str().unwrap()),
        "{}",
        env.args()
    );
}

#[test]
fn help_exits_zero() {
    let env = TestEnv::new();
    let output = env.command().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("rofi-network"));
}
