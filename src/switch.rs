use crate::debounce::Debounce;
use arduino_hal::adc::channel;
use arduino_hal::Adc;

const DEBOUNCE_STABLE_MS: u32 = 20;
/// The switch ties A6 directly to VCC or GND, so any threshold near
/// mid-scale cleanly separates the two states.
const HIGH_THRESHOLD: u16 = 512;

/// Polled, debounced two-position switch on A6. A6 has no digital input
/// buffer on the ATmega328P, so it is sampled through the ADC instead.
pub struct Switch {
    debounce: Debounce,
}

impl Switch {
    pub fn new() -> Self {
        Self {
            debounce: Debounce::new(false),
        }
    }

    /// Sample the switch and advance the debounce state machine. Call every
    /// main-loop iteration.
    pub fn update(&mut self, adc: &mut Adc, now_ms: u32) {
        let raw = adc.read_blocking(&channel::ADC6) >= HIGH_THRESHOLD;
        self.debounce.update(raw, now_ms, DEBOUNCE_STABLE_MS);
    }

    #[allow(dead_code)]
    pub fn is_high(&self) -> bool {
        self.debounce.is_high()
    }
}
