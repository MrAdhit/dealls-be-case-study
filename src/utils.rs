use chrono::{DateTime, Datelike as _, Days, FixedOffset, Timelike as _, Weekday};

pub fn get_today_range(time: &DateTime<FixedOffset>) -> (DateTime<FixedOffset>, DateTime<FixedOffset>) {
    let start_of_day = time.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap();
    let end_of_day = time.with_hour(23).unwrap().with_minute(59).unwrap().with_second(59).unwrap();
    
    (start_of_day, end_of_day)
}

pub fn count_working_days(mut start: DateTime<FixedOffset>, end: DateTime<FixedOffset>) -> i64 {
    let mut working_days = 0;

    while start <= end {
        if start.weekday() != Weekday::Sat && start.weekday() != Weekday::Sun {
            working_days += 1;
        }

        start = start.checked_add_days(Days::new(1)).unwrap();
    }
    
    working_days
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{Local, TimeZone as _};

    #[test]
    fn test_get_today_range() {
        let time = Local.with_ymd_and_hms(2023, 10, 10, 8, 30, 0).unwrap().fixed_offset();
        
        let (start, end) = get_today_range(&time);
        
        assert_eq!(start, Local.with_ymd_and_hms(2023, 10, 10, 0, 0, 0).unwrap().fixed_offset());
        assert_eq!(end, Local.with_ymd_and_hms(2023, 10, 10, 23, 59, 59).unwrap().fixed_offset());
    }
    
    #[test]
    fn test_count_working_days() {
        let period_start = Local.with_ymd_and_hms(2024, 6, 1, 8, 30, 0).unwrap().fixed_offset();
        let period_end = Local.with_ymd_and_hms(2024, 6, 30, 8, 30, 0).unwrap().fixed_offset();
        
        assert_eq!(count_working_days(period_start, period_end), 20);
    }
}
