//! End-to-end clipboard picker tests with stubbed `cliphist` and `wl-copy`.

use rofi_core::test_support::TestEnv;

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
    assert!(args.contains("ARG:CLIPBOARD"), "{args}");
    assert!(args.contains("ARG:-no-custom"), "{args}");
    let stdin = env.stdin_log();
    assert!(stdin.contains("one"));
    assert!(stdin.contains("two"));
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
