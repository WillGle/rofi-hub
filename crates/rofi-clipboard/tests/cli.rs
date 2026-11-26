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

    env.set_reply("1");
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
    assert!(args.contains("ARG:-no-custom"), "{args}");
    assert!(args.contains("Alt+d"), "{args}");
    let stdin = env.stdin_log();
    assert!(stdin.contains("one"));
    assert!(stdin.contains("two"));
    assert!(stdin.contains("HISTORY"), "{stdin}");
    assert!(args.contains("2 text"), "{args}");
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

    // Alt+d opens the delete picker; row 1 is the second entry.
    env.set_replies(&["@custom:2", "1"]);
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
        "delete picked the wrong entry"
    );
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
