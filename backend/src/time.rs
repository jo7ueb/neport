use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

pub fn is_valid_timezone(name: &str) -> bool {
    name.parse::<Tz>().is_ok()
}

// Falls back to UTC for a stored name that somehow fails to parse (should
// never happen since writes are validated via is_valid_timezone), rather
// than panicking on a live request.
fn parse_or_utc(name: &str) -> Tz {
    name.parse().unwrap_or(chrono_tz::UTC)
}

pub fn local_date(dt: DateTime<Utc>, tz_name: &str) -> NaiveDate {
    dt.with_timezone(&parse_or_utc(tz_name)).date_naive()
}

pub fn today(tz_name: &str) -> NaiveDate {
    local_date(Utc::now(), tz_name)
}

// Converts local midnight on `date` (in `tz_name`) to the equivalent UTC
// instant - used to build correct month-boundary range queries against
// TIMESTAMPTZ columns without truncating to date on the SQL side.
pub fn local_midnight_utc(date: NaiveDate, tz_name: &str) -> DateTime<Utc> {
    let tz = parse_or_utc(tz_name);
    date.and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(tz)
        .single()
        .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).unwrap().and_utc().with_timezone(&tz))
        .with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn is_valid_timezone_accepts_iana_names_and_rejects_garbage() {
        assert!(is_valid_timezone("Asia/Tokyo"));
        assert!(is_valid_timezone("UTC"));
        assert!(!is_valid_timezone("Not/AZone"));
        assert!(!is_valid_timezone(""));
    }

    #[test]
    fn local_date_buckets_a_late_night_jst_session_into_the_next_utc_day() {
        // 00:06 JST on the 19th is 15:06 UTC on the 18th - the exact bug
        // report: this must bucket as the 19th in Asia/Tokyo, not the 18th.
        let dt = Utc.with_ymd_and_hms(2026, 7, 18, 15, 6, 0).unwrap();
        assert_eq!(local_date(dt, "Asia/Tokyo"), NaiveDate::from_ymd_opt(2026, 7, 19).unwrap());
        assert_eq!(local_date(dt, "UTC"), NaiveDate::from_ymd_opt(2026, 7, 18).unwrap());
    }

    #[test]
    fn local_midnight_utc_converts_jst_midnight_to_the_previous_utc_afternoon() {
        let expected = Utc.with_ymd_and_hms(2026, 6, 30, 15, 0, 0).unwrap();
        assert_eq!(local_midnight_utc(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(), "Asia/Tokyo"), expected);
    }
}
