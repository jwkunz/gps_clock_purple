use heapless::String;
use core::fmt::Write;
pub struct TimeClock {
    milliseconds: u16,
    seconds: u8,
    minutes: u8,
    hours: u8,
    twelve_hour: bool,
}

impl TimeClock {
    pub fn new(
        hours: u8,
        minutes: u8,
        seconds: u8,
        twelve_hour: bool,
    ) -> Self {
        Self {
            milliseconds: 0,
            seconds,
            minutes,
            hours,
            twelve_hour,
        }
    }

    pub fn tick_fwd(
        &mut self,
        milliseconds: u16,
        seconds: u8,
        minutes: u8,
        hours: u8,
    ) {
        self.milliseconds += milliseconds;

        if self.milliseconds >= 1000 {
            self.milliseconds %= 1000;

            self.seconds += seconds;

            if self.seconds >= 60 {
                self.seconds %= 60;

                self.minutes += minutes;

                if self.minutes >= 60 {
                    self.minutes %= 60;

                    self.hours += hours;

                    if self.hours >= 24 {
                        self.hours %= 24;
                    }
                }
            }
        }
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

    pub fn format<'a>(
        &self,
        s: &'a mut String<16>,
    ) -> &'a str {

        s.clear();

        let _ = write!(
            s,
            "{}{}.{}{}.{}{}.{}{}",
            self.hours / 10,
            self.hours % 10,
            self.minutes / 10,
            self.minutes % 10,
            self.seconds / 10,
            self.seconds % 10,
            self.milliseconds / 100,
            (self.milliseconds % 100) / 10
        );

        s.as_str()
    }

}