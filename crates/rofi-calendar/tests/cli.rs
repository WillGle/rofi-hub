//! End-to-end calendar rendering with a fake Rofi.

use rofi_dmenu::test_support::TestEnv;

const BIN: &str = env!("CARGO_BIN_EXE_rofi-calendar");

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "rofi-calendar failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn renders_the_month_sheet_and_accepts_free_text() {
    let env = TestEnv::new();
    let output = env.run(BIN, &[]);
    assert_success(&output);

    let args = env.args_log();
    assert!(args.contains("ARG:-dmenu"), "{args}");
    assert!(args.contains("ARG:-selected-row"), "{args}");
    assert!(args.contains("ARG:0"), "{args}");
    assert!(args.contains("November 2026"), "{args}");
    assert!(args.contains("ARG:Go to date"), "{args}");
    assert!(args.contains("ARG:-kb-custom-1"), "{args}");
    assert!(args.contains("ARG:Left"), "{args}");
    assert!(!args.contains("ARG:-no-custom"), "{args}");
    assert!(args.contains("button-prev"), "{args}");
    assert!(args.contains("button-today"), "{args}");
    assert!(args.contains("button-next"), "{args}");
    assert!(args.contains("action: \"kb-custom-1\""), "{args}");
    assert!(args.contains("action: \"kb-custom-3\""), "{args}");
    assert!(args.contains("action: \"kb-custom-2\""), "{args}");
    assert!(args.contains("Prev ←"), "{args}");
    assert!(args.contains("Today Ctrl+t"), "{args}");
    assert!(args.contains("Next →"), "{args}");
    assert!(args.contains("listview { enabled: false; }"), "{args}");
    assert!(
        args.contains("textbox { horizontal-align: 0.5; }"),
        "{args}"
    );
    assert!(
        args.contains("message { padding: 12px 14px 8px; }"),
        "{args}"
    );
    assert!(
        args.contains("padding: 4px 0;"),
        "the action row must reserve its bottom outline: {args}"
    );
    assert!(
        args.contains("padding: calc(20px + 0.5em) 20px;"),
        "the calendar window must reserve about 5% more height: {args}"
    );
    assert!(args.contains("location: center;"), "{args}");
    assert!(args.contains("anchor: center;"), "{args}");
    assert!(args.contains("51ch"), "{args}");
    assert!(env.stdin_log().is_empty(), "{}", env.stdin_log());
}

#[test]
fn typed_date_redraws_the_requested_month() {
    let env = TestEnv::new();
    env.set_reply("15/3");
    let output = env
        .command(BIN)
        .env("ROFI_CALENDAR_TODAY", "2026-11-24")
        .output()
        .unwrap();
    assert_success(&output);

    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 2, "{args}");
    assert!(args.contains("March 2026"), "{args}");
    assert!(args.contains("underline=\"single\""), "{args}");
    assert!(args.contains("Sunday, 15/03/2026"), "{args}");
}

#[test]
fn invalid_date_is_reported_in_the_sheet() {
    let env = TestEnv::new();
    env.set_reply("not-a-date");
    let output = env
        .command(BIN)
        .env("ROFI_CALENDAR_TODAY", "2026-11-24")
        .output()
        .unwrap();
    assert_success(&output);

    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 2, "{args}");
    assert!(args.contains("Invalid date: not-a-date"), "{args}");
}

#[test]
fn button_actions_navigate_without_a_date_query() {
    let env = TestEnv::new();
    env.set_replies(&["@custom:0", "@cancel"]);
    let output = env
        .command(BIN)
        .env("ROFI_CALENDAR_TODAY", "2026-11-24")
        .output()
        .unwrap();
    assert_success(&output);

    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 2, "{args}");
    assert_eq!(args.matches("October 2026").count(), 1, "{args}");
    assert_eq!(args.matches("November 2026").count(), 1, "{args}");
}

#[test]
fn next_wraps_december_into_january() {
    let env = TestEnv::new();
    env.set_replies(&["@custom:1", "@cancel"]);
    let output = env
        .command(BIN)
        .env("ROFI_CALENDAR_TODAY", "2026-12-15")
        .output()
        .unwrap();
    assert_success(&output);

    let args = env.args_log();
    assert!(args.contains("January 2027"), "{args}");
}

#[test]
fn ctrl_t_returns_to_today() {
    let env = TestEnv::new();
    env.set_replies(&["@custom:0", "@custom:2", "@cancel"]);
    let output = env
        .command(BIN)
        .env("ROFI_CALENDAR_TODAY", "2026-11-24")
        .output()
        .unwrap();
    assert_success(&output);

    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 3, "{args}");
    assert!(args.contains("November 2026"), "{args}");
}

#[test]
fn partial_date_survives_a_theme_change_and_enter_accepts_once() {
    let env = TestEnv::new();
    // Ctrl+1 is kb-custom-4 (theme bindings start at custom 4).
    env.set_replies(&["@custom:3::19", "19", "@cancel"]);
    let output = env
        .command(BIN)
        .env("ROFI_CALENDAR_TODAY", "2026-11-24")
        .output()
        .unwrap();
    assert_success(&output);

    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 3, "{args}");
    assert!(args.contains("ARG:-filter"), "{args}");
    assert!(args.contains("ARG:19"), "{args}");
    assert!(args.contains("Thursday, 19/11/2026"), "{args}");
    assert_eq!(args.matches("underline=\"single\"").count(), 1, "{args}");
}

#[test]
fn theme_choice_is_locked_and_ctrl4_restores_the_configuration() {
    let env = TestEnv::new();
    env.write_config(
        "rofi/theme.rasi",
        "* { font: \"JetBrainsMono Nerd Font 12\"; }\n",
    );
    let choice = env.state_home().join("rofi-apps/calendar/theme-choice");
    env.set_replies(&["@custom:3", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert_success(&output);
    assert_eq!(std::fs::read_to_string(&choice).unwrap(), "nord\n");

    env.set_reply("@cancel");
    let output = env.run(BIN, &[]);
    assert_success(&output);
    assert!(
        env.args_log().contains("theme-calendar-nord.rasi"),
        "the saved choice was not used on the next launch: {}",
        env.args_log()
    );

    // Ctrl+4 is kb-custom-7; it clears the lock and returns to the configured
    // theme on the next launch.
    let args_log = env.home().parent().unwrap().join("rofi-args.log");
    let _ = std::fs::remove_file(&args_log);
    env.set_replies(&["@custom:6", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert_success(&output);
    assert!(!choice.exists(), "Ctrl+4 must remove the theme lock");

    let _ = std::fs::remove_file(&args_log);
    env.set_reply("@cancel");
    let output = env.run(BIN, &[]);
    assert_success(&output);
    let args = env.args_log();
    assert!(args.contains("theme-calendar.rasi"), "{args}");
    assert!(!args.contains("theme-calendar-nord.rasi"), "{args}");
}
