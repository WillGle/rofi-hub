//! End-to-end keybinding parsing and execution with a fake Rofi.

use std::thread::sleep;
use std::time::Duration;

use rofi_dmenu::test_support::TestEnv;

const BIN: &str = env!("CARGO_BIN_EXE_rofi-keybinds");

fn config(env: &TestEnv) -> String {
    format!(
        r#"# --- Basic ---
bind = MOD4, Return, exec, touch "{home}/ran"
bind = MOD4, Q, killactive

# --- Workspaces ---
bind = MOD4, 1, workspace, 1
bind = MOD4, 2, workspace, 2
"#,
        home = env.home().display()
    )
}

#[test]
fn lists_sections_and_runs_exec_bindings() {
    let env = TestEnv::new();
    env.write_config("hypr/hyprland.conf", &config(&env));

    env.set_reply("0");
    let output = env.run(BIN, &[]);
    assert!(
        output.status.success(),
        "rofi-keybinds failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let ran = env.home().join("ran");
    for _ in 0..50 {
        if ran.is_file() {
            break;
        }
        sleep(Duration::from_millis(20));
    }
    assert!(ran.is_file(), "shortcut did not run");
    let args = env.args_log();
    assert!(args.contains("ARG:SHORTCUTS"), "{args}");
    assert!(args.contains("ARG:-no-custom"), "{args}");
    assert!(args.contains("ARG:-mesg"), "{args}");

    let stdin = env.stdin_log();
    for expected in [
        "\u{258c} BASIC",
        "Super+Enter",
        "Super+Q",
        "\u{258c} WORKSPACES",
        "Super+1..2",
        "Workspace 1..2",
    ] {
        assert!(stdin.contains(expected), "missing {expected} in {stdin}");
    }
    assert!(!stdin.contains("[Basic]"), "{stdin}");
}

#[test]
fn missing_hyprland_config_is_reported() {
    let env = TestEnv::new();
    let output = env.run(BIN, &[]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot read"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
