use core::fmt::Write;
use heapless::String;

use crate::nmea::UtcDateTime;

const MIN_OFFSET_HOURS: i8 = -12;
const MAX_OFFSET_HOURS: i8 = 14;

/// A whole-hour UTC offset (UTC-12 through UTC+14, the world's whole-hour
/// zones). There is no automatic daylight-saving adjustment: DST rules
/// (whether a region observes it, and when it changes) vary too much
/// worldwide to guess from a UTC offset alone, so the offset is adjusted
/// manually - by the same two steps used to select it - whenever DST
/// starts or ends locally.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UtcOffset(i8);

impl UtcOffset {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub fn next(self) -> Self {
        Self(if self.0 >= MAX_OFFSET_HOURS {
            MIN_OFFSET_HOURS
        } else {
            self.0 + 1
        })
    }

    pub fn previous(self) -> Self {
        Self(if self.0 <= MIN_OFFSET_HOURS {
            MAX_OFFSET_HOURS
        } else {
            self.0 - 1
        })
    }

    /// Render as "UTC", two blanks, then a sign letter (N for negative, P
    /// for positive - clearer at a glance on this display than +/-)
    /// followed by the two-digit offset magnitude.
    pub fn format<'a>(&self, s: &'a mut String<16>) -> &'a str {
        s.clear();

        let sign = if self.0 < 0 { 'N' } else { 'P' };

        let _ = write!(s, "UTC  {}{:02}", sign, self.0.unsigned_abs());

        s.as_str()
    }

    /// The shortest signed hour delta from `other` to `self`, wrapping
    /// across the -12/+14 boundary the same way `next`/`previous` do (going
    /// from +14 to -12 is a delta of +1, not +26). Lets a UI apply an
    /// offset change to an already-displayed local time immediately,
    /// without waiting for a fresh GPS capture to re-derive it from UTC.
    pub fn hours_delta_from(self, other: Self) -> i8 {
        const RANGE_SPAN: i16 = (MAX_OFFSET_HOURS - MIN_OFFSET_HOURS) as i16 + 1;

        let raw = self.0 as i16 - other.0 as i16;
        let wrapped = if raw > RANGE_SPAN / 2 {
            raw - RANGE_SPAN
        } else if raw < -(RANGE_SPAN / 2) {
            raw + RANGE_SPAN
        } else {
            raw
        };

        wrapped as i8
    }
}

/// Local wall-clock date and time after applying the selected UTC offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalDateTime {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

/// Convert GPS UTC to local time under the given UTC offset. Unlike a
/// US-only offset (always negative), a worldwide offset can push the local
/// date either a day forward or a day backward.
pub fn from_utc(utc: UtcDateTime, offset: UtcOffset) -> LocalDateTime {
    let shifted_hours = utc.hours as i8 + offset.0;
    let local_date = if shifted_hours < 0 {
        utc.previous_day()
    } else if shifted_hours >= 24 {
        utc.next_day()
    } else {
        utc
    };

    LocalDateTime {
        year: local_date.year,
        month: local_date.month,
        day: local_date.day,
        hours: shifted_hours.rem_euclid(24) as u8,
        minutes: utc.minutes,
        seconds: utc.seconds,
    }
}

#[cfg(test)]
mod tests {
    use super::{from_utc, LocalDateTime, UtcOffset};
    use crate::nmea::UtcDateTime;

    #[test]
    fn negative_offset_borrows_a_day_when_it_crosses_utc_midnight() {
        // 02:00 UTC at UTC-5 is 21:00 the previous local day.
        assert_eq!(
            from_utc(utc(26, 1, 15, 2), utc_offset_hours(-5)),
            LocalDateTime {
                year: 26,
                month: 1,
                day: 14,
                hours: 21,
                minutes: 0,
                seconds: 0,
            }
        );
    }

    #[test]
    fn positive_offset_advances_a_day_when_it_crosses_utc_midnight() {
        // 22:00 UTC at UTC+5 is 03:00 the next local day.
        assert_eq!(
            from_utc(utc(26, 1, 15, 22), utc_offset_hours(5)),
            LocalDateTime {
                year: 26,
                month: 1,
                day: 16,
                hours: 3,
                minutes: 0,
                seconds: 0,
            }
        );
    }

    #[test]
    fn local_date_matches_utc_date_when_the_offset_does_not_cross_midnight() {
        assert_eq!(
            from_utc(utc(26, 1, 15, 12), utc_offset_hours(-5)),
            LocalDateTime {
                year: 26,
                month: 1,
                day: 15,
                hours: 7,
                minutes: 0,
                seconds: 0,
            }
        );
    }

    #[test]
    fn next_and_previous_wrap_at_the_range_ends() {
        assert_eq!(utc_offset_hours(14).next(), utc_offset_hours(-12));
        assert_eq!(utc_offset_hours(-12).previous(), utc_offset_hours(14));
    }

    #[test]
    fn hours_delta_takes_the_shortest_path_across_the_wrap() {
        // One `previous()` step from -12 lands on +14; the delta the UI
        // needs to apply is -1 hour, not the -26 raw difference.
        assert_eq!(
            utc_offset_hours(14).hours_delta_from(utc_offset_hours(-12)),
            -1
        );
        // One `next()` step from +14 lands on -12; delta is +1, not +26.
        assert_eq!(
            utc_offset_hours(-12).hours_delta_from(utc_offset_hours(14)),
            1
        );
        // A non-wrapping change reports its plain difference.
        assert_eq!(
            utc_offset_hours(-8).hours_delta_from(utc_offset_hours(-5)),
            -3
        );
    }

    fn utc_offset_hours(hours: i8) -> UtcOffset {
        let mut offset = UtcOffset::zero();
        if hours >= 0 {
            for _ in 0..hours {
                offset = offset.next();
            }
        } else {
            for _ in 0..-hours {
                offset = offset.previous();
            }
        }
        offset
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
