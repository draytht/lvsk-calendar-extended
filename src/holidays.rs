/// US + Vietnam holidays for the LifeManager calendar.
///
/// Fixed-date holidays are stored as (month, day) pairs.
/// Floating holidays (MLK, Thanksgiving, etc.) are computed per year.
/// Lunar calendar holidays (Tết, Mid-Autumn) are hardcoded for 2024-2030.
use chrono::{Datelike, NaiveDate, Weekday};

use crate::calendar::days_in_month;

// ─── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Holiday {
    pub name:    &'static str,
    pub country: &'static str, // "US" | "VN" | "US+VN"
    pub emoji:   &'static str,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Returns all holidays that fall on the given date.
pub fn holidays_on(date: NaiveDate) -> Vec<Holiday> {
    let mut out = Vec::new();
    let y = date.year();
    let m = date.month();
    let d = date.day();

    // ── Fixed US holidays ─────────────────────────────────────────────────────
    const US_FIXED: &[(u32, u32, &str, &str)] = &[
        (1,  1,  "New Year's Day",       "🎆"),
        (2,  14, "Valentine's Day",      "💝"),
        (3,  17, "St. Patrick's Day",    "🍀"),
        (6,  19, "Juneteenth",           "✊"),
        (7,  4,  "Independence Day",     "🎇"),
        (10, 31, "Halloween",            "🎃"),
        (11, 11, "Veterans Day",         "🎖"),
        (12, 25, "Christmas Day",        "🎄"),
        (12, 31, "New Year's Eve",       "🥂"),
    ];
    for &(hm, hd, name, emoji) in US_FIXED {
        if m == hm && d == hd {
            out.push(Holiday { name, country: "US", emoji });
        }
    }

    // ── Floating US holidays ──────────────────────────────────────────────────
    let floating_us: &[(&str, &str, u32, Weekday, u32)] = &[
        // (name, emoji, month, weekday, n)
        ("MLK Day",         "✊",  1,  Weekday::Mon, 3),
        ("Presidents' Day", "🏛",  2,  Weekday::Mon, 3),
        ("Labor Day",       "⚒",  9,  Weekday::Mon, 1),
        ("Columbus Day",    "⛵", 10, Weekday::Mon, 2),
        ("Thanksgiving",    "🦃", 11, Weekday::Thu, 4),
    ];
    for &(name, emoji, month, weekday, n) in floating_us {
        if m == month {
            if let Some(h) = nth_weekday(y, month, weekday, n) {
                if h == date { out.push(Holiday { name, country: "US", emoji }); }
            }
        }
    }
    // Memorial Day — last Monday of May
    if m == 5 {
        if let Some(h) = last_weekday(y, 5, Weekday::Mon) {
            if h == date { out.push(Holiday { name: "Memorial Day", country: "US", emoji: "🪖" }); }
        }
    }

    // ── Fixed Vietnam public holidays ─────────────────────────────────────────
    const VN_FIXED: &[(u32, u32, &str, &str)] = &[
        (1,  1,  "New Year (Dương lịch)",    "🎊"),
        (4,  30, "Reunification Day",         "🇻🇳"),
        (5,  1,  "International Labour Day",  "✊"),
        (9,  2,  "National Day",              "🇻🇳"),
    ];
    for &(hm, hd, name, emoji) in VN_FIXED {
        if m == hm && d == hd {
            out.push(Holiday { name, country: "VN", emoji });
        }
    }

    // ── Lunar calendar holidays (hardcoded Gregorian, 2024-2030) ─────────────
    const LUNAR: &[(i32, u32, u32, &str, &str, &str)] = &[
        // Tết Nguyên Đán (Lunar New Year) — first day
        (2024, 2, 10,  "Tết Nguyên Đán",      "VN", "🧧"),
        (2025, 1, 29,  "Tết Nguyên Đán",      "VN", "🧧"),
        (2026, 2, 17,  "Tết Nguyên Đán",      "VN", "🧧"),
        (2027, 2,  6,  "Tết Nguyên Đán",      "VN", "🧧"),
        (2028, 1, 26,  "Tết Nguyên Đán",      "VN", "🧧"),
        (2029, 2, 13,  "Tết Nguyên Đán",      "VN", "🧧"),
        (2030, 2,  3,  "Tết Nguyên Đán",      "VN", "🧧"),
        // Giỗ Tổ Hùng Vương — 10th of 3rd lunar month
        (2024, 4, 18,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        (2025, 4,  7,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        (2026, 3, 28,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        (2027, 4, 16,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        (2028, 4,  5,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        (2029, 4, 25,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        (2030, 4, 14,  "Giỗ Tổ Hùng Vương",  "VN", "🏯"),
        // Tết Trung Thu — 15th of 8th lunar month
        (2024, 9, 17,  "Tết Trung Thu",       "VN", "🥮"),
        (2025, 10, 6,  "Tết Trung Thu",       "VN", "🥮"),
        (2026, 9, 25,  "Tết Trung Thu",       "VN", "🥮"),
        (2027, 9, 15,  "Tết Trung Thu",       "VN", "🥮"),
        (2028, 10, 3,  "Tết Trung Thu",       "VN", "🥮"),
        (2029, 9, 22,  "Tết Trung Thu",       "VN", "🥮"),
        (2030, 9, 12,  "Tết Trung Thu",       "VN", "🥮"),
        // Vu Lan (Ghost Festival) — 15th of 7th lunar month
        (2024, 8, 18,  "Vu Lan",              "VN", "🕯"),
        (2025, 9,  8,  "Vu Lan",              "VN", "🕯"),
        (2026, 8, 28,  "Vu Lan",              "VN", "🕯"),
        (2027, 8, 17,  "Vu Lan",              "VN", "🕯"),
        (2028, 9,  5,  "Vu Lan",              "VN", "🕯"),
        (2029, 8, 25,  "Vu Lan",              "VN", "🕯"),
        (2030, 8, 14,  "Vu Lan",              "VN", "🕯"),
    ];
    for &(hy, hm, hd, name, country, emoji) in LUNAR {
        if y == hy && m == hm && d == hd {
            out.push(Holiday { name, country, emoji });
        }
    }

    out
}

/// Returns `(day_of_month, Holiday)` pairs for every holiday in the given month.
pub fn holidays_in_month(year: i32, month: u32) -> Vec<(u32, Holiday)> {
    let mut out = Vec::new();
    for day in 1..=days_in_month(year, month) {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            for h in holidays_on(date) {
                out.push((day, h));
            }
        }
    }
    out
}

/// True if the given date is a holiday (any country).
pub fn is_holiday(date: NaiveDate) -> bool {
    !holidays_on(date).is_empty()
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// nth occurrence of `weekday` in the given month (1-indexed).
fn nth_weekday(year: i32, month: u32, weekday: Weekday, n: u32) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let days_until =
        (weekday.num_days_from_monday() + 7 - first.weekday().num_days_from_monday()) % 7;
    let day = 1 + days_until + (n - 1) * 7;
    if day > days_in_month(year, month) { return None; }
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Last occurrence of `weekday` in the given month.
fn last_weekday(year: i32, month: u32, weekday: Weekday) -> Option<NaiveDate> {
    let last_day = days_in_month(year, month);
    let last = NaiveDate::from_ymd_opt(year, month, last_day)?;
    let days_back =
        (last.weekday().num_days_from_monday() + 7 - weekday.num_days_from_monday()) % 7;
    NaiveDate::from_ymd_opt(year, month, last_day - days_back)
}
