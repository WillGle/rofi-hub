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
    assert!(
        args.contains("Enter run"),
        "fixed keyboard hint missing while filtering: {args}"
    );

    let stdin = env.stdin_log();
    assert!(
        !stdin.contains("Enter run"),
        "hint must not be filtered: {stdin}"
    );
    for expected in [
        "\u{258c} BASIC",
        "Super+Enter",
        "Super+Q",
        "\u{258c} WORKSPACES",
        "Super+1..2",
        "Workspace 1..2",
        "0\tSuper+Enter\t",
        "section-0\tBasic",
    ] {
        assert!(stdin.contains(expected), "missing {expected} in {stdin}");
    }
    assert!(!stdin.contains("[Basic]"), "{stdin}");
}

#[test]
fn selecting_a_header_keeps_the_menu_open_and_runs_nothing() {
    let env = TestEnv::new();
    env.write_config("hypr/hyprland.conf", &config(&env));
    env.set_replies(&["section-0\tBasic", "@cancel"]);

    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert!(!env.home().join("ran").exists(), "header ran a command");
    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 2, "{args}");
    assert!(args.contains("ARG:-a"), "{args}");
    assert!(args.contains("ARG:0"), "{args}");
    assert!(args.contains("element selected.active"), "{args}");
}

#[test]
fn theme_choice_is_locked_and_ctrl4_restores_the_configuration() {
    let env = TestEnv::new();
    env.write_config("hypr/hyprland.conf", &config(&env));
    env.write_config(
        "rofi/theme.rasi",
        "* { font: \"JetBrainsMono Nerd Font 12\"; }\n",
    );
    let choice = env.state_home().join("rofi-apps/keybinds/theme-choice");

    // Ctrl+1 is kb-custom-1.
    env.set_replies(&["@custom:0", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&choice).unwrap(), "nord\n");

    env.set_reply("@cancel");
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert!(
        env.args_log().contains("theme-keybinds-nord.rasi"),
        "the saved choice was not used on the next launch: {}",
        env.args_log()
    );

    env.set_replies(&["@custom:3", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert!(!choice.exists(), "Ctrl+4 must remove the theme lock");

    let args_log = env.home().parent().unwrap().join("rofi-args.log");
    let _ = std::fs::remove_file(&args_log);
    env.set_reply("@cancel");
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    let args = env.args_log();
    assert!(args.contains("theme-keybinds.rasi"), "{args}");
    assert!(!args.contains("theme-keybinds-nord.rasi"), "{args}");
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
