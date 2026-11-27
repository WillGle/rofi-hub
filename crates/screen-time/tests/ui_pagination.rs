mod common;

use common::Fixture;

fn list_lines(payload: &str) -> usize {
    payload
        .lines()
        .find_map(|line| {
            line.strip_prefix("\0theme\x1flistview { lines: ")
                .and_then(|value| value.strip_suffix("; }"))
                .and_then(|value| value.parse().ok())
        })
        .expect("missing listview lines metadata")
}

#[test]
fn main_view_pages_apps_and_survives_detail_round_trip() {
    let fixture = Fixture::new();

    let first = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "0")]);
    assert!(first.contains("apps 1-5 of 8"));
    assert!(first.contains("Code"));
    assert_eq!(first.matches("\0info\x1f").count(), 5);
    assert!(!first.contains("apps 6-8"));
    assert_eq!(list_lines(&first), 12);

    let second = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "21")]);
    assert!(second.contains("apps 6-8 of 8"));
    assert!(
        !second.contains("\0info\x1fCode"),
        "page 2 starts at item 6"
    );
    assert_eq!(second.matches("\0info\x1f").count(), 3);
    assert_eq!(list_lines(&second), 12, "page height stays stable");

    let detail = fixture.render_env(
        &["render", "--format", "rofi"],
        &[("ROFI_RETV", "1"), ("ROFI_INFO", "Code")],
    );
    assert!(detail.contains("APP TIMELINE"));
    assert!(detail.contains("apps 6-8 of 8"));
    assert!(detail.contains("APP DETAILS"));
    assert!(detail.contains("APP ACTIVITY"));
    assert!(detail.contains("BURSTS"));

    let back = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "14")]);
    assert!(back.contains("APP TIMELINE"));
    assert!(
        back.contains("apps 6-8 of 8"),
        "page is preserved after detail"
    );
    assert_eq!(
        list_lines(&back),
        12,
        "returning from detail keeps main height"
    );
}

#[test]
fn page_down_and_up_stay_within_range() {
    let fixture = Fixture::new();
    fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "0")]);

    let last = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "21")]);
    let clamped = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "21")]);
    assert_eq!(last, clamped, "extra page-down is clamped to the last page");

    let back = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "22")]);
    assert!(back.contains("apps 1-5 of 8"));
    let back_again = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "22")]);
    assert_eq!(
        back, back_again,
        "extra page-up is clamped to the first page"
    );
}

#[test]
fn next_page_on_empty_list_keeps_page_zero() {
    let fixture = Fixture::empty();
    fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "0")]);
    let output = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "21")]);
    assert!(output.contains("No activity recorded today"), "{output}");
    assert_eq!(list_lines(&output), 8);
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.paths.ui_state_path).unwrap()).unwrap();
    assert_eq!(state["page"], 0);
}

#[test]
fn theme_change_keeps_page_cursor_and_detail_state() {
    let fixture = Fixture::new();
    fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "0")]);
    fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "21")]);

    let themed = fixture.render_env(
        &["render", "--format", "rofi"],
        &[("ROFI_RETV", "17"), ("ROFI_INFO", "Zoom")],
    );
    assert!(themed.contains("apps 6-8 of 8"), "{themed}");
    assert!(themed.contains("\0new-selection\x1f8"), "{themed}");
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.paths.ui_state_path).unwrap()).unwrap();
    assert_eq!(state["page"], 1);
    assert_eq!(state["cursor"], "Zoom");
    assert_eq!(
        std::fs::read_to_string(fixture.paths.runtime_dir.join("rofi-selection")).unwrap(),
        "8\n"
    );

    let detail = fixture.render_env(
        &["render", "--format", "rofi"],
        &[("ROFI_RETV", "1"), ("ROFI_INFO", "Zoom")],
    );
    assert!(detail.contains("APP DETAILS"), "{detail}");
    assert!(detail.contains("\0new-selection\x1f8"), "{detail}");

    let reopened = fixture.render_env(
        &["render", "--format", "rofi"],
        &[("SCREEN_TIME_ROFI_REOPEN", "1"), ("ROFI_RETV", "0")],
    );
    assert!(reopened.contains("apps 6-8 of 8"), "{reopened}");
    assert!(reopened.contains("APP DETAILS"), "{reopened}");
    assert!(reopened.contains("\0new-selection\x1f8"), "{reopened}");
}

#[test]
fn theme_change_falls_back_to_the_first_row_when_the_cursor_is_gone() {
    let fixture = Fixture::new();
    fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "0")]);
    let themed = fixture.render_env(
        &["render", "--format", "rofi"],
        &[("ROFI_RETV", "18"), ("ROFI_INFO", "Removed App")],
    );
    assert!(themed.contains("\0new-selection\x1f6"), "{themed}");
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.paths.ui_state_path).unwrap()).unwrap();
    assert_eq!(state["cursor"], "Removed App");
    assert_eq!(
        std::fs::read_to_string(fixture.paths.runtime_dir.join("rofi-selection")).unwrap(),
        "6\n"
    );
}

#[test]
fn rofi_next_day_from_today_stays_on_today() {
    let fixture = Fixture::new();
    let output = fixture.render_env(&["render", "--format", "rofi"], &[("ROFI_RETV", "11")]);
    assert!(
        output.contains("Today · Saturday, 21 November 2026"),
        "{output}"
    );
    assert!(!output.contains("22 November"), "{output}");
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.paths.ui_state_path).unwrap()).unwrap();
    assert_eq!(state["date"], "2026-11-21");
}
