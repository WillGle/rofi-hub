//! A month sheet with a date search box, rendered in Rofi dmenu mode.
//!
//! The month and grid are one message widget, the date entry is the input bar,
//! and Prev/Today/Next are three compact buttons in one list row. Opening the
//! app shows the current month. Typing a date jumps to it: `15/3`, `3/2027`,
//! `15/3/2027`, `2027-03-15`, or just `15`. Left/Right browse months and
//! Ctrl+t returns to today.

use std::env;
use std::process::ExitCode;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, Weekday};
use rofi_dmenu::{Item, Level, MenuInput, Outcome, Rofi, Theme, app, escape_markup, ui};

const BTN_PREV: &str = "\u{2190}";
const BTN_TODAY: &str = "Today Ctrl+t";
const BTN_NEXT: &str = "\u{2192}";
const WEEKDAY_SHORT: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const DAY_GAP: &str = "   ";
#[cfg(test)]
const GRID_WIDTH: usize = 7 * 2 + 6 * DAY_GAP.len();

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rofi-calendar: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct View {
    year: i32,
    month: u32,
    focus: Option<u32>,
}

impl View {
    fn today(today: NaiveDate) -> Self {
        Self {
            year: today.year(),
            month: today.month(),
            focus: None,
        }
    }

    fn shift(&mut self, months: i32) {
        let zero_based = self.year * 12 + self.month as i32 - 1 + months;
        self.year = zero_based.div_euclid(12);
        self.month = zero_based.rem_euclid(12) as u32 + 1;
        self.focus = None;
    }
}

/// The visible "today". `ROFI_CALENDAR_TODAY` pins it for deterministic tests.
fn local_today() -> NaiveDate {
    env::var("ROFI_CALENDAR_TODAY")
        .ok()
        .and_then(|value| NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok())
        .unwrap_or_else(|| Local::now().date_naive())
}

fn run() -> Result<()> {
    let rofi = Rofi::find()?;
    let today = local_today();
    let mut view = View::today(today);
    let mut note: Option<String> = None;
    let mut input = MenuInput::default();
    let (mut palette, theme_file) = app::initial_theme("calendar");
    let mut options = app::dmenu_options("calendar", "Go to date");
    options.theme = theme_file;
    options.allow_custom = true;
    options.allow_empty = true;

    loop {
        // Navigation lives in RASI buttons outside the filterable list. An
        // empty list is intentional: typing a date must never hide the three
        // controls, and Enter accepts the current date string.
        let items: Vec<Item> = Vec::new();

        let mut mesg = render_sheet(&view, today, note.as_deref(), &palette);
        mesg.truncate(mesg.trim_end_matches('\n').len());
        mesg.push('\n');
        mesg.push_str(&format!(
            "<span size=\"12000\">{}</span>",
            ui::hint(&[("Ctrl+1–4", "Theme")], &palette)
        ));
        mesg.push('\n');
        options.extra_args = vec![
            "-matching".to_owned(),
            "normal".to_owned(),
            "-mesg".to_owned(),
            mesg,
            // Reserve Left/Right for month navigation.
            "-kb-move-char-back".to_owned(),
            String::new(),
            "-kb-move-char-forward".to_owned(),
            String::new(),
            // Ctrl+t is `kb-transpose-chars` by default; free it so it can
            // jump back to today without Rofi rejecting the custom binding.
            "-kb-transpose-chars".to_owned(),
            String::new(),
            "-kb-custom-1".to_owned(),
            "Left".to_owned(),
            "-kb-custom-2".to_owned(),
            "Right".to_owned(),
            "-kb-custom-3".to_owned(),
            "Control+t".to_owned(),
        ];
        options.theme_str.push(calendar_theme(&palette));

        input.restore(&mut options, 0);
        app::bind_theme_shortcuts(&mut options, 3);

        let (outcome, current_input) = rofi.dmenu_with_input(&items, &options)?;
        input = MenuInput::default();
        match outcome {
            Outcome::Cancelled => return Ok(()),
            Outcome::Custom(0) => {
                view.shift(-1);
                note = None;
                input = current_input;
            }
            Outcome::Custom(1) => {
                view.shift(1);
                note = None;
                input = current_input;
            }
            Outcome::Custom(2) => {
                view = View::today(today);
                note = None;
                input = current_input;
            }
            Outcome::Custom(binding) => {
                let Some(choice) = app::session_theme("calendar", binding, 3) else {
                    return Ok(());
                };
                (palette, options.theme) = choice;
                input = current_input;
            }
            Outcome::Selected(query) => {
                let query = query.trim();
                if query.is_empty() {
                    return Ok(());
                }
                match parse_query(query, &view, today) {
                    Some((year, month, day)) => {
                        view = View {
                            year,
                            month,
                            focus: day,
                        };
                        note = None;
                    }
                    None => {
                        // Keep the visible month; only the error line changes.
                        view.focus = None;
                        note = Some(query.to_owned());
                    }
                }
            }
        }
    }
}

/// Three real RASI buttons replace the old list rows. Their actions mirror the
/// Left, Right, and Ctrl+t shortcuts and stay visible while the date entry is
/// filtered or incomplete.
fn calendar_theme(theme: &Theme) -> String {
    format!(
        r#"
window {{
    width: calc( ( ( 46ch + 60px ) * 0.95 ) min 94% );
    location: center;
    anchor: center;
    /* Keep the width unchanged and add roughly 10% to the content height in
       the vertical padding. 1em per side scales with the configured font. */
    padding: calc(20px + 1em) 20px;
}}
mainbox {{
    children: [ message, inputbar, box-calendar-actions ];
    spacing: 16px;
}}
message {{ padding: 12px 14px 8px; }}
textbox {{ horizontal-align: 0.5; }}
listview {{ enabled: false; }}
inputbar {{ padding: 10px 14px; }}
box-calendar-actions {{
    background-color: transparent;
    orientation: horizontal;
    expand: false;
    spacing: 12px;
    /* Bottom padding is part of the outline space: at scaled/fractional text
       heights Rofi can fold the last row of a button's 1px border into the
       box edge. Keep space below the buttons so the full outline renders. */
    padding: 4px 0;
    children: [ button-prev, button-today, button-next ];
}}
button-prev, button-today, button-next {{
    expand: true;
    padding: 9px 12px;
    margin: 0;
    border: 1px;
    border-radius: 6px;
    border-color: {muted};
    background-color: {mantle};
    text-color: {text};
    horizontal-align: 0.5;
}}
button-prev, button-next {{
    expand: false;
    padding: 9px 14px;
}}
button-prev {{
    content: "{prev}";
    action: "kb-custom-1";
}}
button-today {{
    content: "{today}";
    action: "kb-custom-3";
    border-color: {accent};
}}
button-next {{
    content: "{next}";
    action: "kb-custom-2";
}}
button-prev selected, button-prev selected.normal,
button-today selected, button-today selected.normal,
button-next selected, button-next selected.normal {{
    background-color: {accent};
    text-color: {base};
}}
"#,
        muted = theme.muted,
        mantle = theme.mantle,
        text = theme.text,
        accent = theme.accent,
        base = theme.base,
        prev = BTN_PREV,
        today = BTN_TODAY,
        next = BTN_NEXT,
    )
}

/// Turn typed text into `(year, month, optional day)`.
///
/// Numbers may be separated by `/`, `-`, `.`, or spaces. The order is
/// day-first unless the first number is a year:
///
/// `15` → day 15 of the visible month
/// `15/3` → 15 March this year
/// `3/2027` → March 2027
/// `15/3/2027` → a full date
/// `2027-03-15` → ISO order
fn parse_query(input: &str, view: &View, today: NaiveDate) -> Option<(i32, u32, Option<u32>)> {
    let normalized: String = input
        .chars()
        .map(|ch| match ch {
            '/' | '-' | '.' => ' ',
            other => other,
        })
        .collect();
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let numbers: Vec<u32> = parts
        .iter()
        .map(|part| part.parse::<u32>().ok())
        .collect::<Option<_>>()?;

    let (year, month, day) = match numbers.as_slice() {
        [day] => (view.year, view.month, Some(*day)),
        [year, month, day] if *year >= 1000 => (*year as i32, *month, Some(*day)),
        [year, month] if *year >= 1000 => (*year as i32, *month, None),
        [month, year] if *year > 12 => (*year as i32, *month, None),
        [day, month] => (today.year(), *month, Some(*day)),
        [day, month, year] => {
            let mut year = *year as i32;
            if year < 100 {
                year += 2000;
            }
            (year, *month, Some(*day))
        }
        _ => return None,
    };

    if !(1..=12).contains(&month) {
        return None;
    }
    // Validate the date (including leap years) before accepting it.
    NaiveDate::from_ymd_opt(year, month, day.unwrap_or(1))?;
    Some((year, month, day))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .expect("valid first day of the next month");
    first_next
        .pred_opt()
        .expect("the day before the first is valid")
        .day()
}

/// Render the whole message widget: centered month title, centered seven-column
/// grid, the focused date, and a one-line error next to the input.
fn render_sheet(view: &View, today: NaiveDate, note: Option<&str>, palette: &Theme) -> String {
    let month = NaiveDate::from_ymd_opt(view.year, view.month, 1)
        .expect("the view always holds a valid month")
        .format("%B %Y")
        .to_string();
    let title_span = format!(
        "<span foreground=\"{}\" weight=\"bold\" size=\"18000\">{}</span>",
        palette.text,
        escape_markup(&month)
    );
    let body = format!(
        "{}\n{}",
        render_grid(view, today, palette),
        render_note(view, note, palette)
    );
    format!("{title_span}\n<span size=\"12000\">{body}</span>\n")
}

fn render_grid(view: &View, today: NaiveDate, palette: &Theme) -> String {
    let first = NaiveDate::from_ymd_opt(view.year, view.month, 1)
        .expect("the view always holds a valid month");
    let offset = first.weekday().num_days_from_monday() as usize;
    let days = days_in_month(view.year, view.month) as usize;
    let rows = (offset + days).div_ceil(7);

    let mut sheet = String::new();
    let header = WEEKDAY_SHORT
        .iter()
        .map(|day| format!("{day:^2}"))
        .collect::<Vec<_>>()
        .join(DAY_GAP);
    sheet.push_str(&format!(
        "<span foreground=\"{}\">{header}</span>\n",
        palette.muted
    ));

    for row in 0..rows {
        let mut line = String::new();
        for column in 0..7 {
            let index = row * 7 + column;
            if index < offset || index >= offset + days {
                line.push_str("  ");
            } else {
                let day = (index - offset + 1) as u32;
                let cell = format!("{day:>2}");
                let is_today =
                    view.year == today.year() && view.month == today.month() && day == today.day();
                if is_today {
                    // The accent cell keeps a background even when the same
                    // day is also the search result.
                    let underline = if view.focus == Some(day) {
                        " underline=\"single\""
                    } else {
                        ""
                    };
                    line.push_str(&format!(
                        "<span background=\"{}\" foreground=\"{}\"{underline}><b>{cell}</b></span>",
                        palette.accent, palette.base
                    ));
                } else if view.focus == Some(day) {
                    line.push_str(&format!(
                        "<span foreground=\"{}\" underline=\"single\"><b>{cell}</b></span>",
                        palette.accent
                    ));
                } else if column >= 5 {
                    line.push_str(&format!(
                        "<span foreground=\"{}\">{cell}</span>",
                        palette.muted
                    ));
                } else {
                    line.push_str(&cell);
                }
            }
            if column < 6 {
                line.push_str(DAY_GAP);
            }
        }
        sheet.push_str(&line);
        if row + 1 < rows {
            sheet.push('\n');
        }
    }

    sheet
}

fn render_note(view: &View, note: Option<&str>, palette: &Theme) -> String {
    if let Some(query) = note {
        let text = format!("Invalid date: {query}");
        return ui::status(&text, Level::Danger, palette);
    }
    let Some(day) = view.focus else {
        return String::new();
    };
    let Some(date) = NaiveDate::from_ymd_opt(view.year, view.month, day) else {
        return String::new();
    };
    let text = format!(
        "{}, {:02}/{:02}/{:04}",
        weekday_name(date.weekday()),
        day,
        view.month,
        view.year
    );
    format!(
        "<span foreground=\"{}\" size=\"small\">{}</span>",
        palette.accent,
        escape_markup(&text)
    )
}

fn weekday_name(weekday: Weekday) -> &'static str {
    WEEKDAYS[weekday.num_days_from_monday() as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        View {
            year: 2026,
            month: 11,
            focus: None,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 24).unwrap()
    }

    #[test]
    fn parses_the_documented_query_forms() {
        assert_eq!(
            parse_query("15", &view(), today()),
            Some((2026, 11, Some(15)))
        );
        assert_eq!(
            parse_query("15/3", &view(), today()),
            Some((2026, 3, Some(15)))
        );
        assert_eq!(
            parse_query("3/2027", &view(), today()),
            Some((2027, 3, None))
        );
        assert_eq!(
            parse_query("15/3/2027", &view(), today()),
            Some((2027, 3, Some(15)))
        );
        assert_eq!(
            parse_query("2027-03-15", &view(), today()),
            Some((2027, 3, Some(15)))
        );
        assert_eq!(
            parse_query("15.3.2027", &view(), today()),
            Some((2027, 3, Some(15)))
        );
        assert_eq!(
            parse_query("15/3/27", &view(), today()),
            Some((2027, 3, Some(15)))
        );
        assert_eq!(
            parse_query("2027-03", &view(), today()),
            Some((2027, 3, None))
        );
    }

    #[test]
    fn rejects_impossible_dates_and_text() {
        assert_eq!(parse_query("30/2", &view(), today()), None);
        assert_eq!(parse_query("2026-02-30", &view(), today()), None);
        assert_eq!(parse_query("13/13", &view(), today()), None);
        assert_eq!(parse_query("tomorrow", &view(), today()), None);
        assert_eq!(parse_query("", &view(), today()), None);
        assert_eq!(parse_query("1/2/3/4", &view(), today()), None);
    }

    #[test]
    fn month_navigation_wraps_years() {
        let mut view = View {
            year: 2026,
            month: 12,
            focus: Some(4),
        };
        view.shift(1);
        assert_eq!((view.year, view.month, view.focus), (2027, 1, None));
        view.shift(-1);
        assert_eq!((view.year, view.month), (2026, 12));
    }

    #[test]
    fn knows_month_lengths() {
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn sheet_has_seven_columns_and_marks_today() {
        let palette = Theme::gruvbox();
        let mesg = render_sheet(&view(), today(), None, &palette);
        assert!(mesg.contains("November 2026"));
        assert!(mesg.contains(&WEEKDAY_SHORT.join(DAY_GAP)));
        assert!(mesg.contains("background=\"#fabd2f\""));
        assert!(mesg.contains("30"));
        for line in render_grid(&view(), today(), &palette).lines() {
            assert_eq!(ui::display_width(&strip_markup(line)), GRID_WIDTH);
        }
    }

    #[test]
    fn focus_and_note_are_rendered() {
        let palette = Theme::nord();
        let focused = View {
            focus: Some(17),
            ..view()
        };
        assert!(render_sheet(&focused, today(), None, &palette).contains("underline=\"single\""));
        let note = render_note(&focused, None, &palette);
        assert!(note.contains("Tuesday, 17/11/2026"));
        let error = render_note(&focused, Some("a < b"), &palette);
        assert!(error.contains("Invalid date: a &lt; b"));
        assert!(error.contains(&palette.danger));
    }

    #[test]
    fn focus_and_today_share_the_accent_cell() {
        let palette = Theme::gruvbox();
        let both = View {
            focus: Some(24),
            ..view()
        };
        let mesg = render_sheet(&both, today(), None, &palette);
        assert!(mesg.contains("underline=\"single\""));
        assert!(mesg.contains("background=\"#fabd2f\""));
    }

    #[test]
    fn six_week_months_render_every_row() {
        let palette = Theme::gruvbox();
        // 1 August 2026 is a Saturday; the month needs six week rows.
        let august = View {
            year: 2026,
            month: 8,
            focus: None,
        };
        let grid = render_grid(&august, today(), &palette);
        assert_eq!(grid.lines().count(), 1 + 6);
        assert!(grid.contains("31"));
    }

    #[test]
    fn leap_february_and_december_to_january_render() {
        let palette = Theme::gruvbox();
        let leap = View {
            year: 2028,
            month: 2,
            focus: Some(29),
        };
        let grid = render_grid(&leap, today(), &palette);
        assert!(grid.contains("29"));
        let mut december = View {
            year: 2026,
            month: 12,
            focus: None,
        };
        december.shift(1);
        assert_eq!((december.year, december.month), (2027, 1));
        let mesg = render_sheet(&december, today(), None, &palette);
        assert!(mesg.contains("January 2027"));
    }

    #[test]
    fn side_buttons_are_compact_icons() {
        let theme = calendar_theme(&Theme::gruvbox());
        assert!(
            theme.contains("width: calc( ( ( 46ch + 60px ) * 0.95 ) min 94% );"),
            "the calendar should be about 10% narrower: {theme}"
        );
        assert!(theme.contains("content: \"←\";"), "{theme}");
        assert!(theme.contains("content: \"→\";"), "{theme}");
        assert!(
            theme.contains("button-prev, button-next {\n    expand: false;"),
            "Prev and Next should shrink to icon buttons: {theme}"
        );
    }

    #[test]
    fn window_adds_about_five_percent_height() {
        let theme = calendar_theme(&Theme::gruvbox());
        assert!(
            theme.contains("padding: calc(20px + 1em) 20px;"),
            "the calendar should keep its width but reserve a little more height: {theme}"
        );
    }

    #[test]
    fn action_row_keeps_space_for_the_button_outline() {
        let theme = calendar_theme(&Theme::gruvbox());
        assert!(
            theme.contains("padding: 4px 0;"),
            "the action box needs bottom padding so Rofi does not clip the button borders: {theme}"
        );
    }

    #[test]
    fn today_is_not_highlighted_in_another_month() {
        let palette = Theme::gruvbox();
        let other = View {
            year: 2026,
            month: 12,
            focus: None,
        };
        let grid = render_grid(&other, today(), &palette);
        assert!(!grid.contains("background=\"#fabd2f\""));
    }

    /// Test helper: remove Pango tags so visible widths can be compared.
    fn strip_markup(text: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for ch in text.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        out
    }
}
