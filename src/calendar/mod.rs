use chrono::{Datelike, NaiveDate};

/// Returns weeks for a given month. Each week is 7 Option<NaiveDate> slots
/// (None = padding day outside the month).
pub fn month_weeks(year: i32, month: u32) -> Vec<Vec<Option<NaiveDate>>> {
    let first         = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let start_offset  = first.weekday().num_days_from_monday() as i64;
    let days_in_month = days_in_month(year, month) as i64;

    let mut weeks: Vec<Vec<Option<NaiveDate>>> = Vec::new();
    let mut week: Vec<Option<NaiveDate>> = Vec::new();

    for _ in 0..start_offset { week.push(None); }

    for d in 1..=days_in_month {
        week.push(NaiveDate::from_ymd_opt(year, month, d as u32));
        if week.len() == 7 {
            weeks.push(week.clone());
            week.clear();
        }
    }
    if !week.is_empty() {
        while week.len() < 7 { week.push(None); }
        weeks.push(week);
    }
    weeks
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    (next.unwrap() - first).num_days() as u32
}
