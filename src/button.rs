use crate::debounce::Debounce;
use embedded_hal::digital::InputPin;

/// Minimum time a raw reading must hold steady before it is trusted.
const DEBOUNCE_STABLE_MS: u32 = 20;

/// Polled, debounced button. Wired active-high with an external pull-down on
/// the board, so a floating-input pin reads high only while pressed.
pub struct Button<PIN> {
    pin: PIN,
    debounce: Debounce,
}

impl<PIN: InputPin> Button<PIN> {
    pub fn new(pin: PIN) -> Self {
        Self {
            pin,
            debounce: Debounce::new(false),
        }
    }

    /// Sample the pin and advance the debounce state machine. Call every
    /// main-loop iteration.
    pub fn update(&mut self, now_ms: u32) {
        let raw = self.pin.is_high().unwrap_or(false);
        self.debounce.update(raw, now_ms, DEBOUNCE_STABLE_MS);
    }

    #[allow(dead_code)]
    pub fn is_pressed(&self) -> bool {
        self.debounce.is_high()
    }
}
