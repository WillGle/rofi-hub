//! End-to-end clipboard picker tests with stubbed `cliphist` and `wl-copy`.

use rofi_dmenu::test_support::TestEnv;

const BIN: &str = env!("CARGO_BIN_EXE_rofi-clipboard");

#[test]
fn selecting_an_entry_decodes_and_copies_it() {
    let env = TestEnv::new();
    let decode_input = env.home().join("decode-input");
    let wl_copy_out = env.home().join("wl-copy.out");

    env.script(
        "cliphist",
        r#"case "$1" in
  list) printf '7\tone\n8\ttwo\n' ;;
  decode) cat > "$DECODE_INPUT"; printf 'decoded:'; cat "$DECODE_INPUT" ;;
esac
"#,
    );
    env.script("wl-copy", r#"cat > "$WL_COPY_OUT""#);

    env.set_reply("1:two");
    let output = env
        .command(BIN)
        .env("DECODE_INPUT", &decode_input)
        .env("WL_COPY_OUT", &wl_copy_out)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "rofi-clipboard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(std::fs::read_to_string(&decode_input).unwrap(), "8\ttwo\n");
    assert_eq!(
        std::fs::read_to_string(&wl_copy_out).unwrap(),
        "decoded:8\ttwo\n"
    );

    let args = env.args_log();
    assert!(args.contains("ARG:FILTER"), "{args}");
    assert!(!args.contains("ARG:-no-custom"), "{args}");
    assert!(args.contains("Alt+d"), "{args}");
    let stdin = env.stdin_log();
    assert!(stdin.contains("one"));
    assert!(stdin.contains("two"));
    assert!(args.contains("HISTORY"), "{args}");
    assert!(args.contains("num-filtered-rows"), "{args}");
    assert!(args.contains("2 text"), "{args}");
    // The fixed HISTORY band is not a row, so the first entry is row 0.
    assert!(args.contains("ARG:-selected-row\nARG:0"), "{args}");
}

#[test]
fn clear_action_wipes_history_after_confirmation() {
    let env = TestEnv::new();
    let calls = env.home().join("cliphist.log");
    env.script(
        "cliphist",
        r#"printf '%s\n' "$*" >> "$CLIPHIST_LOG"
case "$1" in
  list) printf '7\tone\n8\ttwo\n' ;;
esac
"#,
    );
    env.script("wl-copy", "cat >/dev/null\n");

    env.set_replies(&["@custom:0", "yes"]);
    let output = env
        .command(BIN)
        .env("CLIPHIST_LOG", &calls)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "rofi-clipboard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(calls.contains("wipe"), "{calls}");
}

#[test]
fn delete_action_removes_the_selected_entry() {
    let env = TestEnv::new();
    let calls = env.home().join("cliphist.log");
    let deleted = env.home().join("cliphist-deleted");
    env.script(
        "cliphist",
        r#"printf '%s\n' "$*" >> "$CLIPHIST_LOG"
case "$1" in
  list) printf '7\tone\n8\ttwo\n' ;;
  delete) cat >> "$CLIPHIST_DELETED" ;;
esac
"#,
    );
    env.script("wl-copy", "cat >/dev/null\n");

    // The fixed HISTORY band is not a row, so row 1 is the second entry. The
    // typed filter must survive the reopen after deleting.
    env.set_replies(&["@custom:2:1:two", "@cancel"]);
    let output = env
        .command(BIN)
        .env("CLIPHIST_LOG", &calls)
        .env("CLIPHIST_DELETED", &deleted)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "rofi-clipboard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(calls.contains("delete"), "{calls}");
    assert_eq!(
        std::fs::read_to_string(deleted).unwrap(),
        "8\ttwo\n",
        "delete removed the wrong selected entry"
    );
    let args = env.args_log();
    assert!(args.contains("ARG:-filter"), "{args}");
    assert!(args.contains("ARG:two"), "{args}");
}

#[test]
fn filtered_header_keeps_the_total_and_uses_a_live_match_count() {
    let env = TestEnv::new();
    env.script(
        "cliphist",
        "case \"$1\" in list) printf '7\\tone\\n8\\ttwo\\n' ;; esac\n",
    );
    env.script("wl-copy", "cat >/dev/null\n");

    // The fixed header owns the full-history total once. The match count is
    // Rofi's live `num-filtered-rows` widget, so typing updates it without a
    // refresh or a relaunch.
    env.set_replies(&["@custom:1:1:two", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    let args = env.args_log();
    let second = args.rsplit("INVOCATION").next().unwrap();
    assert!(second.contains("2 items"), "{second}");
    assert!(!second.contains("match for"), "stale match text: {second}");
    assert!(second.contains("num-filtered-rows"), "{second}");
    assert!(second.contains("textbox-count-label"), "{second}");
    assert!(second.contains("matching"), "{second}");
    assert!(second.contains("textbox-section-history"), "{second}");
    assert!(second.contains("ARG:-filter"), "{second}");
    assert!(second.contains("ARG:two"), "{second}");
}

#[test]
fn section_band_is_fixed_and_never_copies() {
    let env = TestEnv::new();
    let decode = env.home().join("decode-called");
    let copy = env.home().join("copy-called");
    env.script(
        "cliphist",
        r#"case "$1" in
  list) printf '7\tone\n8\ttwo\n' ;;
  decode) : > "$DECODE_CALLED"; cat ;;
esac
"#,
    );
    env.script("wl-copy", r#": > "$COPY_CALLED"; cat >/dev/null"#);

    env.set_replies(&["@cancel"]);
    let output = env
        .command(BIN)
        .env("DECODE_CALLED", &decode)
        .env("COPY_CALLED", &copy)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!decode.exists(), "header decode must not run");
    assert!(!copy.exists(), "header copy must not run");
    let args = env.args_log();
    assert_eq!(args.matches("INVOCATION").count(), 1, "{args}");
    assert!(
        !args.contains("ARG:-a"),
        "section band must not be a row: {args}"
    );
    assert!(args.contains("textbox-section-history"), "{args}");
    assert!(args.contains("HISTORY"), "{args}");
}

#[test]
fn main_screen_uses_ras_buttons_for_every_action() {
    let env = TestEnv::new();
    env.script(
        "cliphist",
        "case \"$1\" in list) printf '7\\tone\\n8\\ttwo\\n' ;; esac\n",
    );
    env.script("wl-copy", "cat >/dev/null\n");

    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    let args = env.args_log();
    for button in [
        "button-copy",
        "button-delete",
        "button-clear",
        "button-refresh",
    ] {
        assert!(args.contains(button), "missing {button}: {args}");
    }
    for pair in [
        ("button-copy {", "kb-accept-entry"),
        ("button-delete {", "kb-custom-3"),
        ("button-clear {", "kb-custom-1"),
        ("button-refresh {", "kb-custom-2"),
    ] {
        assert!(args.contains(pair.0), "missing {}: {args}", pair.0);
        assert!(args.contains(pair.1), "missing {}: {args}", pair.1);
    }
    assert!(
        !args.contains("Copy  \u{00b7}"),
        "text chips remain: {args}"
    );
    assert!(
        args.contains("num-filtered-rows"),
        "missing live count: {args}"
    );
    assert!(args.contains("textbox-section-history"), "{args}");
}

#[test]
fn refresh_action_reloads_history() {
    let env = TestEnv::new();
    let calls = env.home().join("cliphist.log");
    env.script(
        "cliphist",
        r#"printf '%s\n' "$*" >> "$CLIPHIST_LOG"
case "$1" in
  list) printf '7\tone\n' ;;
esac
"#,
    );
    env.script("wl-copy", "cat >/dev/null\n");

    env.set_replies(&["@custom:1"]);
    let output = env
        .command(BIN)
        .env("CLIPHIST_LOG", &calls)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "rofi-clipboard failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert_eq!(calls.matches("list").count(), 2, "{calls}");
}

#[test]
fn empty_history_shows_an_empty_state() {
    let env = TestEnv::new();
    env.script("cliphist", "case \"$1\" in list) : ;; esac\n");
    env.script("wl-copy", "cat >/dev/null\n");

    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    let args = env.args_log();
    assert!(args.contains("ARG:-mesg"), "{args}");
    assert!(args.contains("ARG:-mesg"), "{args}");
    assert!(
        env.stdin_log().contains("Clipboard history is empty"),
        "{}",
        env.stdin_log()
    );
    assert!(
        !args.contains("num-filtered-rows"),
        "empty history has no match count: {args}"
    );
}

#[test]
fn theme_choice_is_locked_and_ctrl4_restores_the_configuration() {
    let env = TestEnv::new();
    env.script(
        "cliphist",
        "case \"$1\" in list) printf '7\\tone\\n8\\ttwo\\n' ;; esac\n",
    );
    env.script("wl-copy", "cat >/dev/null\n");
    env.write_config(
        "rofi/theme.rasi",
        "* { font: \"JetBrainsMono Nerd Font 12\"; }\n",
    );
    let choice = env.state_home().join("rofi-apps/clipboard/theme-choice");

    // Ctrl+1 is the fourth custom binding: Alt+x, Alt+r and Alt+d come first.
    env.set_replies(&["@custom:3", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert_eq!(std::fs::read_to_string(&choice).unwrap(), "nord\n");

    env.set_reply("@cancel");
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert!(
        env.args_log().contains("theme-clipboard-nord.rasi"),
        "the saved choice was not used on the next launch: {}",
        env.args_log()
    );

    env.set_replies(&["@custom:6", "@cancel"]);
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    assert!(!choice.exists(), "Ctrl+4 must remove the theme lock");

    let args_log = env.home().parent().unwrap().join("rofi-args.log");
    let _ = std::fs::remove_file(&args_log);
    env.set_reply("@cancel");
    let output = env.run(BIN, &[]);
    assert!(output.status.success());
    let args = env.args_log();
    assert!(args.contains("theme-clipboard.rasi"), "{args}");
    assert!(!args.contains("theme-clipboard-nord.rasi"), "{args}");
}

#[test]
fn missing_cliphist_is_a_clean_error() {
    let env = TestEnv::new();
    env.script("wl-copy", "cat >/dev/null\n");

    let output = env
        .command(BIN)
        .env("PATH", env.bin_path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cliphist"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
