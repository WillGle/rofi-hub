//! End-to-end calendar rendering with a fake Rofi.

use rofi_core::test_support::TestEnv;

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
    assert!(args.contains("ARG:-select"), "{args}");
    assert!(args.contains("ARG:Today"), "{args}");
    assert!(args.contains("ARG:-mesg"), "{args}");
    assert!(args.contains("ARG:-kb-custom-1"), "{args}");
    assert!(args.contains("ARG:Left"), "{args}");
    assert!(!args.contains("ARG:-no-custom"), "{args}");

    let stdin = env.stdin_log();
    assert!(stdin.contains("Today"));
    assert!(stdin.contains("◀ Prev"));
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
    assert!(args.contains("underline=\"double\""), "{args}");
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
    assert!(args.contains("Unknown date: not-a-date"), "{args}");
}
