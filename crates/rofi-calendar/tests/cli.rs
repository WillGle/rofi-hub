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
    assert!(args.contains("ARG:1"), "{args}");
    assert!(args.contains("November 2026"), "{args}");
    assert!(args.contains("ARG:Go to date"), "{args}");
    assert!(args.contains("ARG:-kb-custom-1"), "{args}");
    assert!(args.contains("ARG:Left"), "{args}");
    assert!(!args.contains("ARG:-no-custom"), "{args}");

    let stdin = env.stdin_log();
    assert!(stdin.contains("prev"), "{stdin}");
    assert!(stdin.contains("today"), "{stdin}");
    assert!(stdin.contains("next"), "{stdin}");
    assert!(stdin.contains("◀ Prev"), "{stdin}");
    assert!(stdin.contains("Next ▶"), "{stdin}");
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
fn button_ids_navigate_without_a_date_query() {
    let env = TestEnv::new();
    env.set_replies(&["prev", "@cancel"]);
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
    env.set_replies(&["next", "@cancel"]);
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
    env.set_replies(&["prev", "@custom:2", "@cancel"]);
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
