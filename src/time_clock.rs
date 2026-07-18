use core::fmt::Write;
use heapless::String;

/// A whole-second UTC clock.  PPS is the sole source of elapsed time.
pub struct TimeClock {
    seconds: u8,
    minutes: u8,
    hours: u8,
}

impl TimeClock {
    pub fn new(hours: u8, minutes: u8, seconds: u8) -> Self {
        Self {
            hours,
            minutes,
            seconds,
        }
    }

    pub fn set_time(&mut self, hours: u8, minutes: u8, seconds: u8) {
        self.hours = hours;
        self.minutes = minutes;
        self.seconds = seconds;
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

    pub fn format<'a>(&self, s: &'a mut String<16>, tenths: u8) -> &'a str {
        s.clear();

        // Decimal points light the MAX7221's decimal-point segments, leaving
        // seven digits for HH.MM.SS.T.  The final display position is blank.
        let _ = write!(
            s,
            "{}{}.{}{}.{}{}.{}",
            self.hours / 10,
            self.hours % 10,
            self.minutes / 10,
            self.minutes % 10,
            self.seconds / 10,
            self.seconds % 10,
            tenths,
        );

        s.as_str()
    }
}
