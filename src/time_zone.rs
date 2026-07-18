use crate::nmea::UtcDateTime;

/// Select the clock's U.S. time zone.  The default is Eastern Time.
pub const TIME_ZONE: UsTimeZone = UsTimeZone::Eastern;

/*
U.S. time-zone reference.  All four selections automatically use the current
U.S. daylight-saving rule: DST starts at 02:00 local standard time on the
second Sunday in March and ends at 02:00 local daylight time on the first
Sunday in November.

 Eastern: EST (UTC-5) / EDT (UTC-4)
          New York, Boston, Miami, Atlanta, Detroit, Washington DC, Toronto

 Central: CST (UTC-6) / CDT (UTC-5)
          Chicago, Dallas, Houston, Minneapolis, New Orleans, Winnipeg

 Mountain: MST (UTC-7) / MDT (UTC-6)
          Denver, Salt Lake City, Boise, Albuquerque, Calgary
          Arizona is an exception: most of it stays on MST all year.

 Pacific: PST (UTC-8) / PDT (UTC-7)
          Los Angeles, Seattle, San Francisco, Las Vegas, Portland, Vancouver

Not covered by this selector: Hawaii (UTC-10, no DST), Alaska's multiple
zones, Arizona's no-DST rule, Indiana's historical exceptions, and locations
outside the U.S./Canada DST rule.  Those need a separate fixed-offset or
regional rule implementation.
*/
#[derive(Clone, Copy)]
#[allow(dead_code)] // The selected compile-time zone uses one variant at a time.
pub enum UsTimeZone {
    Eastern,
    Central,
    Mountain,
    Pacific,
}

impl UsTimeZone {
    const fn standard_offset_hours(self) -> i8 {
        match self {
            Self::Eastern => -5,
            Self::Central => -6,
            Self::Mountain => -7,
            Self::Pacific => -8,
        }
    }
}

/// Convert GPS UTC to the selected U.S. local time.  The display has no date,
/// so only the local clock fields are returned; hour wrapping is intentional.
pub fn from_utc(utc: UtcDateTime) -> (u8, u8, u8) {
    let offset = TIME_ZONE.standard_offset_hours() + if is_us_dst(utc, TIME_ZONE) { 1 } else { 0 };
    let local_hours = (utc.hours as i8 + offset).rem_euclid(24) as u8;
    (local_hours, utc.minutes, utc.seconds)
}

fn is_us_dst(utc: UtcDateTime, zone: UsTimeZone) -> bool {
    let year = 2000 + utc.year as u16;
    let dst_start_day = nth_sunday(year, 3, 2);
    let dst_end_day = nth_sunday(year, 11, 1);

    match utc.month {
        1 | 2 | 12 => false,
        4..=10 => true,
        3 => utc.day > dst_start_day
            || (utc.day == dst_start_day
                && utc.hours >= (2 - zone.standard_offset_hours()) as u8),
        11 => utc.day < dst_end_day
            || (utc.day == dst_end_day
                && utc.hours < (1 - zone.standard_offset_hours()) as u8),
        _ => false,
    }
}

/// Day of month for the nth Sunday, using Sunday = 0.
fn nth_sunday(year: u16, month: u8, nth: u8) -> u8 {
    let first_sunday = 1 + (7 - weekday(year, month, 1)) % 7;
    first_sunday + (nth - 1) * 7
}

/// Gregorian calendar weekday, where Sunday is zero.
fn weekday(mut year: u16, month: u8, day: u8) -> u8 {
    const MONTH_OFFSETS: [u16; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];

    if month < 3 {
        year -= 1;
    }

    ((year + year / 4 - year / 100 + year / 400 + MONTH_OFFSETS[(month - 1) as usize] + day as u16)
        % 7) as u8
}

#[cfg(test)]
mod tests {
    use super::{from_utc, TIME_ZONE};
    use crate::nmea::UtcDateTime;

    #[test]
    fn eastern_dst_changes_at_the_correct_utc_hours() {
        assert!(matches!(TIME_ZONE, super::UsTimeZone::Eastern));
        assert_eq!(from_utc(utc(26, 3, 8, 6)), (1, 0, 0));
        assert_eq!(from_utc(utc(26, 3, 8, 7)), (3, 0, 0));
        assert_eq!(from_utc(utc(26, 11, 1, 5)), (1, 0, 0));
        assert_eq!(from_utc(utc(26, 11, 1, 6)), (1, 0, 0));
    }

    fn utc(year: u8, month: u8, day: u8, hours: u8) -> UtcDateTime {
        UtcDateTime {
            year,
            month,
            day,
            hours,
            minutes: 0,
            seconds: 0,
        }
    }
}
