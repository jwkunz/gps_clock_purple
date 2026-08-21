use core::fmt::Write;
use heapless::String;

use crate::nmea::UtcDateTime;

/// A whole-second UTC clock.  PPS is the sole source of elapsed time.  The
/// date fields only change when a GPS capture applies a new one; there is no
/// autonomous midnight rollover between captures, since a single missed
/// capture is rare and the date is only ever a display convenience here.
pub struct TimeClock {
    seconds: u8,
    minutes: u8,
    hours: u8,
    year: u8,
    month: u8,
    day: u8,
}

impl TimeClock {
    pub fn new(hours: u8, minutes: u8, seconds: u8) -> Self {
        Self {
            hours,
            minutes,
            seconds,
            year: 0,
            month: 0,
            day: 0,
        }
    }

    pub fn set_time(&mut self, hours: u8, minutes: u8, seconds: u8) {
        self.hours = hours;
        self.minutes = minutes;
        self.seconds = seconds;
    }

    pub fn set_date(&mut self, year: u8, month: u8, day: u8) {
        self.year = year;
        self.month = month;
        self.day = day;
    }

    pub fn tick_second(&mut self) {
        self.seconds += 1;

        if self.seconds == 60 {
            self.seconds = 0;
            self.minutes += 1;

            if self.minutes == 60 {
                self.minutes = 0;
                self.hours += 1;

                if self.hours == 24 {
                    self.hours = 0;
                }
            }
        }
    }

    /// Shift the displayed time by a signed number of hours, rolling the
    /// date forward or backward a day at a time as needed. Used to apply a
    /// UTC-offset change to the currently displayed time immediately,
    /// rather than waiting for the next GPS capture to re-derive it.
    pub fn shift_hours(&mut self, delta_hours: i8) {
        let mut hours = self.hours as i16 + delta_hours as i16;

        while hours < 0 || hours >= 24 {
            let date = UtcDateTime {
                year: self.year,
                month: self.month,
                day: self.day,
                hours: 0,
                minutes: 0,
                seconds: 0,
            };
            let date = if hours < 0 {
                hours += 24;
                date.previous_day()
            } else {
                hours -= 24;
                date.next_day()
            };
            self.year = date.year;
            self.month = date.month;
            self.day = date.day;
        }

        self.hours = hours as u8;
    }

    pub fn is_minute_boundary(&self) -> bool {
        self.seconds == 0
    }

    /// Current whole second within the minute, for scheduling the next PPS.
    pub fn seconds(&self) -> u8 {
        self.seconds
    }

    pub fn hours(&self) -> u8 {
        self.hours
    }

    pub fn minutes(&self) -> u8 {
        self.minutes
    }

    pub fn format<'a>(&self, s: &'a mut String<16>, centiseconds: u8) -> &'a str {
        s.clear();

        // Decimal points light the MAX7221's decimal-point segments, leaving
        // eight digits for HH.MM.SS.CC.
        let _ = write!(
            s,
            "{}{}.{}{}.{}{}.{}{}",
            self.hours / 10,
            self.hours % 10,
            self.minutes / 10,
            self.minutes % 10,
            self.seconds / 10,
            self.seconds % 10,
            centiseconds / 10,
            centiseconds % 10,
        );

        s.as_str()
    }

    /// Format the most recently applied local date as month followed by
    /// day, each zero-padded to two digits. The year is intentionally
    /// omitted: it's the field an unresolved GPS parsing issue occasionally
    /// corrupts, while month/day have always been reliable.
    pub fn format_date<'a>(&self, s: &'a mut String<16>) -> &'a str {
        s.clear();

        let _ = write!(s, "{:02}{:02}", self.month, self.day);

        s.as_str()
    }
}
