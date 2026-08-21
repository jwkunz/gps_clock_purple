use core::fmt::Write;
use heapless::String;

/// Alarm time (HH:MM) and its field-editing cursor. Cursor 0 selects the
/// hour, 1 selects the minute; incrementing/decrementing always bumps the
/// whole selected field by one, wrapping, so every state the editor can
/// reach is a valid time.
pub struct Alarm {
    hours: u8,
    minutes: u8,
    editing: bool,
    cursor: u8,
}

impl Alarm {
    pub fn new() -> Self {
        Self {
            hours: 0,
            minutes: 0,
            editing: false,
            cursor: 0,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn start_editing(&mut self) {
        self.editing = true;
        self.cursor = 0;
    }

    pub fn stop_editing(&mut self) {
        self.editing = false;
    }

    pub fn cursor(&self) -> u8 {
        self.cursor
    }

    pub fn scroll_left(&mut self) {
        self.cursor = 0;
    }

    pub fn scroll_right(&mut self) {
        self.cursor = 1;
    }

    pub fn increment(&mut self) {
        if self.cursor == 0 {
            self.hours = (self.hours + 1) % 24;
        } else {
            self.minutes = (self.minutes + 1) % 60;
        }
    }

    pub fn decrement(&mut self) {
        if self.cursor == 0 {
            self.hours = (self.hours + 23) % 24;
        } else {
            self.minutes = (self.minutes + 59) % 60;
        }
    }

    pub fn hours(&self) -> u8 {
        self.hours
    }

    pub fn minutes(&self) -> u8 {
        self.minutes
    }

    pub fn matches(&self, hours: u8, minutes: u8) -> bool {
        self.hours == hours && self.minutes == minutes
    }

    /// Render the alarm as HH.MM, blanking both digits of the selected
    /// field together when `blink_on` is false so the caller can flash a
    /// cursor over time.
    pub fn format_edit<'a>(&self, s: &'a mut String<16>, blink_on: bool) -> &'a str {
        s.clear();

        let hours_selected = self.cursor == 0;
        let minutes_selected = self.cursor == 1;

        let digit_char = |selected: bool, value: u8| -> char {
            if selected && !blink_on {
                ' '
            } else {
                (b'0' + value) as char
            }
        };

        let _ = write!(
            s,
            "{}{}.{}{}",
            digit_char(hours_selected, self.hours / 10),
            digit_char(hours_selected, self.hours % 10),
            digit_char(minutes_selected, self.minutes / 10),
            digit_char(minutes_selected, self.minutes % 10),
        );

        s.as_str()
    }
}
