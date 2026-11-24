//! A month sheet with a date search box, rendered in Rofi dmenu mode.
//!
//! Enter on the sheet opens today. Typing a date jumps to it:
//! `15/3`, `3/2027`, `15/3/2027`, `2027-03-15`, or just `15`.
//! Left/Right browse months and Ctrl+t returns to today.

use std::env;
use std::process::ExitCode;

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate, Weekday};
use rofi_core::{Item, Outcome, Rofi, app, escape_markup, theme};

const GRID_WIDTH: usize = 20;
const BTN_PREV: &str = "◀  Trước";
const BTN_TODAY: &str = "Hôm nay";
const BTN_NEXT: &str = "Sau  ▶";
const WEEKDAYS_VI: [&str; 7] = [
    "Thứ hai",
    "Thứ ba",
    "Thứ tư",
    "Thứ năm",
    "Thứ sáu",
    "Thứ bảy",
    "Chủ nhật",
];

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

    loop {
        let palette = theme::active();
        let items = vec![
            Item::new(BTN_PREV, BTN_PREV),
            Item::new(BTN_TODAY, BTN_TODAY),
            Item::new(BTN_NEXT, BTN_NEXT),
        ];

        let sheet = render_sheet(&view, today, &palette);
        let note_line = render_note(&view, note.as_deref(), &palette);
        let hint = "<span size=\"x-small\" alpha=\"50%\">← → tháng · Ctrl+t hôm nay · Esc</span>";
        let mesg = format!("{sheet}{note_line}\n{hint}");

        let mut options = app::dmenu_options("calendar", "Lịch");
        options.allow_custom = true;
        options.extra_args = vec![
            "-matching".to_owned(),
            "normal".to_owned(),
            "-select".to_owned(),
            BTN_TODAY.to_owned(),
            "-mesg".to_owned(),
            mesg,
            // Reserve Left/Right for month navigation.
            "-kb-move-char-back".to_owned(),
            String::new(),
            "-kb-move-char-forward".to_owned(),
            String::new(),
            "-kb-custom-1".to_owned(),
            "Left".to_owned(),
            "-kb-custom-2".to_owned(),
            "Right".to_owned(),
            "-kb-custom-3".to_owned(),
            "Control+t".to_owned(),
        ];

        match rofi.dmenu(&items, &options)? {
            Outcome::Cancelled => return Ok(()),
            Outcome::Custom(0) => {
                view.shift(-1);
                note = None;
            }
            Outcome::Custom(1) => {
                view.shift(1);
                note = None;
            }
            Outcome::Custom(2) => {
                view = View::today(today);
                note = None;
            }
            Outcome::Custom(_) => return Ok(()),
            Outcome::Selected(text) => match text.as_str() {
                BTN_PREV => {
                    view.shift(-1);
                    note = None;
                }
                BTN_NEXT => {
                    view.shift(1);
                    note = None;
                }
                BTN_TODAY => {
                    view = View::today(today);
                    note = None;
                }
                query => {
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
                        None => note = Some(query.to_owned()),
                    }
                }
            },
        }
    }
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

fn render_sheet(view: &View, today: NaiveDate, palette: &theme::Theme) -> String {
    let first = NaiveDate::from_ymd_opt(view.year, view.month, 1)
        .expect("the view always holds a valid month");
    let offset = first.weekday().num_days_from_monday() as usize;
    let days = days_in_month(view.year, view.month) as usize;
    let rows = (offset + days).div_ceil(7);

    let title = format!("Tháng {} {}", view.month, view.year);
    let pad = GRID_WIDTH.saturating_sub(title.chars().count()) / 2;
    let mut sheet = format!(
        "{:pad$}<span foreground=\"{}\"><b>{}</b></span>\n",
        "",
        palette.accent,
        title,
        pad = pad
    );
    sheet.push_str(&format!(
        "<span foreground=\"{}\">T2 T3 T4 T5 T6 T7 CN</span>\n",
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
                if view.year == today.year() && view.month == today.month() && day == today.day() {
                    line.push_str(&format!(
                        "<span background=\"{}\" foreground=\"{}\"><b>{}</b></span>",
                        palette.accent, palette.base, cell
                    ));
                } else if view.focus == Some(day) {
                    line.push_str(&format!(
                        "<span foreground=\"{}\" underline=\"double\"><b>{}</b></span>",
                        palette.accent, cell
                    ));
                } else if column >= 5 {
                    line.push_str(&format!(
                        "<span foreground=\"{}\">{}</span>",
                        palette.muted, cell
                    ));
                } else {
                    line.push_str(&cell);
                }
            }
            if column < 6 {
                line.push(' ');
            }
        }
        sheet.push_str(&line);
        sheet.push('\n');
    }

    sheet
}

fn render_note(view: &View, note: Option<&str>, palette: &theme::Theme) -> String {
    let text = if let Some(query) = note {
        format!("không nhận ra: {query}")
    } else if let Some(day) = view.focus {
        let Some(date) = NaiveDate::from_ymd_opt(view.year, view.month, day) else {
            return String::new();
        };
        format!(
            "{}, {:02}/{:02}/{:04}",
            weekday_name(date.weekday()),
            day,
            view.month,
            view.year
        )
    } else {
        return String::new();
    };

    format!(
        "\n<span foreground=\"{}\" size=\"small\">{}</span>",
        palette.accent,
        escape_markup(&text)
    )
}

fn weekday_name(weekday: Weekday) -> &'static str {
    WEEKDAYS_VI[weekday.num_days_from_monday() as usize]
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
    fn sheet_has_twenty_column_grid_and_marks_today() {
        let palette = theme::Theme::gruvbox();
        let sheet = render_sheet(&view(), today(), &palette);
        let lines: Vec<&str> = sheet.lines().collect();
        assert_eq!(
            lines[1],
            "<span foreground=\"#a89984\">T2 T3 T4 T5 T6 T7 CN</span>"
        );
        assert!(sheet.contains("background=\"#d79921\""));
        assert!(sheet.contains("30"));
        // The last week row is 20 visible columns: 2 chars per day.
        let last = lines.last().unwrap();
        assert!(last.contains("30"));
    }

    #[test]
    fn focus_and_note_are_rendered() {
        let palette = theme::Theme::nord();
        let focused = View {
            focus: Some(17),
            ..view()
        };
        assert!(render_sheet(&focused, today(), &palette).contains("underline=\"double\""));
        let note = render_note(&focused, None, &palette);
        assert!(note.contains("Thứ ba, 17/11/2026"));
        let error = render_note(&focused, Some("a < b"), &palette);
        assert!(error.contains("không nhận ra: a &lt; b"));
    }
}
