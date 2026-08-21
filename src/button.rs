use crate::debounce::Debounce;
use embedded_hal::digital::InputPin;

/// Minimum time a raw reading must hold steady before it is trusted.
const DEBOUNCE_STABLE_MS: u32 = 20;

/// How long a press must be held before `take_repeat_edge` starts repeating.
const REPEAT_INITIAL_DELAY_MS: u32 = 500;
/// Steady-state interval between repeats once a hold passes the initial
/// delay.
const REPEAT_INTERVAL_MS: u32 = 150;

/// Polled, debounced button. Wired active-high with an external pull-down on
/// the board, so a floating-input pin reads high only while pressed.
pub struct Button<PIN> {
    pin: PIN,
    debounce: Debounce,
    was_pressed: bool,
    press_started_ms: u32,
    repeats_fired: u32,
}

impl<PIN: InputPin> Button<PIN> {
    pub fn new(pin: PIN) -> Self {
        Self {
            pin,
            debounce: Debounce::new(false),
            was_pressed: false,
            press_started_ms: 0,
            repeats_fired: 0,
        }
    }

    /// Sample the pin and advance the debounce state machine. Call every
    /// main-loop iteration.
    pub fn update(&mut self, now_ms: u32) {
        let raw = self.pin.is_high().unwrap_or(false);
        self.debounce.update(raw, now_ms, DEBOUNCE_STABLE_MS);
    }

    pub fn is_pressed(&self) -> bool {
        self.debounce.is_high()
    }

    /// True exactly once per press. Call once per main-loop iteration even
    /// when the result is discarded, since it consumes the rising edge.
    pub fn take_pressed_edge(&mut self) -> bool {
        let pressed = self.debounce.is_high();
        let edge = pressed && !self.was_pressed;
        self.was_pressed = pressed;
        edge
    }

    /// True on the initial press, then true repeatedly while held: first
    /// after REPEAT_INITIAL_DELAY_MS, then every REPEAT_INTERVAL_MS after
    /// that. The repeat count is derived from total held time rather than a
    /// scheduled next-fire time, so a delayed poll can't cause a burst of
    /// repeats to make up for lost time. Call once per main-loop iteration
    /// even when the result is discarded.
    pub fn take_repeat_edge(&mut self, now_ms: u32) -> bool {
        let pressed = self.debounce.is_high();
        if !pressed {
            self.was_pressed = false;
            return false;
        }

        if !self.was_pressed {
            self.was_pressed = true;
            self.press_started_ms = now_ms;
            self.repeats_fired = 0;
            return true;
        }

        let held_ms = now_ms.wrapping_sub(self.press_started_ms);
        if held_ms < REPEAT_INITIAL_DELAY_MS {
            return false;
        }

        let due_repeats = 1 + (held_ms - REPEAT_INITIAL_DELAY_MS) / REPEAT_INTERVAL_MS;
        if due_repeats > self.repeats_fired {
            self.repeats_fired = due_repeats;
            return true;
        }

        false
    }
}
