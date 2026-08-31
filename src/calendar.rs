use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

pub fn week_end(date: NaiveDate) -> NaiveDate {
    week_start(date) + Duration::days(6)
}

pub fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid date components")
}

pub fn month_end(date: NaiveDate) -> NaiveDate {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1).expect("valid next month") - Duration::days(1)
}

pub fn year_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), 1, 1).expect("valid date year")
}

pub fn year_end(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), 12, 31).expect("valid date year")
}

pub fn move_month(date: NaiveDate, delta: i32) -> NaiveDate {
    let zero_based = date.year() * 12 + date.month0() as i32 + delta;
    let year = zero_based.div_euclid(12);
    let month = zero_based.rem_euclid(12) as u32 + 1;
    let day = date
        .day()
        .min(month_end(NaiveDate::from_ymd_opt(year, month, 1).unwrap()).day());
    NaiveDate::from_ymd_opt(year, month, day).expect("clamped month date")
}

pub fn weekday_number(day: Weekday) -> u32 {
    day.num_days_from_monday()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_crosses_year_boundaries() {
        let dec_31 = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        assert_eq!(
            dec_31 + Duration::days(1),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()
        );
        let jan_1 = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_eq!(
            jan_1 - Duration::days(1),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()
        );
    }

    #[test]
    fn month_lengths_and_six_row_month_are_correct() {
        assert_eq!(
            month_end(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()).day(),
            29
        );
        assert_eq!(
            month_end(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()).day(),
            28
        );

        let august_2026 = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        let cells = month_end(august_2026).day()
            + month_start(august_2026).weekday().num_days_from_monday();
        assert_eq!(cells.div_ceil(7), 6);
    }

    #[test]
    fn week_starts_on_monday_across_month_boundary() {
        let sunday = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();
        assert_eq!(
            week_start(sunday),
            NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
        );
        assert_eq!(week_end(sunday), sunday);
    }
}
