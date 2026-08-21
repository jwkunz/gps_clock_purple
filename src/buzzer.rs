use arduino_hal::pac::TC0;

const F_CPU: u32 = 16_000_000;

/// Frequency, duration, and stop behavior for the next tone. Applied by
/// `Buzzer::enable`.
pub struct BuzzerConfig {
    pub frequency_hz: u16,
    pub duration_ms: u32,
    /// When true, the tone ignores `duration_ms` and plays until `disable`
    /// is called explicitly.
    pub run_until_stopped: bool,
}

enum State {
    Idle,
    Playing { started_ms: u32 },
}

/// Polled, run-to-completion tone generator. D6 is Timer0's OC0A output;
/// once configured, Timer0 toggles the pin in hardware on every compare
/// match, so the generated tone is sample-accurate and needs no CPU
/// attention between edges. `update` only has to notice when a timed tone's
/// duration has elapsed.
pub struct Buzzer {
    tc0: TC0,
    config: BuzzerConfig,
    state: State,
}

impl Buzzer {
    pub fn new(tc0: TC0) -> Self {
        let buzzer = Self {
            tc0,
            config: BuzzerConfig {
                frequency_hz: 0,
                duration_ms: 0,
                run_until_stopped: false,
            },
            state: State::Idle,
        };
        buzzer.silence();
        buzzer
    }

    /// Set the tone to play on the next `enable`. Has no effect on a tone
    /// already playing.
    pub fn configure(&mut self, config: BuzzerConfig) {
        self.config = config;
    }

    /// Start the configured tone.
    pub fn enable(&mut self, now_ms: u32) {
        self.start_tone();
        self.state = State::Playing { started_ms: now_ms };
    }

    /// Silence the buzzer immediately, regardless of configured duration.
    pub fn disable(&mut self) {
        self.silence();
        self.state = State::Idle;
    }

    /// Advance the run-to-completion state machine. Call every main-loop
    /// iteration.
    pub fn update(&mut self, now_ms: u32) {
        if let State::Playing { started_ms } = self.state {
            let elapsed_ms = now_ms.wrapping_sub(started_ms);
            if !self.config.run_until_stopped && elapsed_ms >= self.config.duration_ms {
                self.disable();
            }
        }
    }

    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        matches!(self.state, State::Playing { .. })
    }

    /// Configure Timer0 for CTC toggle-on-compare-match at the frequency
    /// closest to `self.config.frequency_hz`.
    fn start_tone(&self) {
        let (prescaler, top) = Self::prescaler_and_top(self.config.frequency_hz);

        // Stop the clock while reconfiguring so a partial cycle from a
        // previous tone can't leave the pin in the wrong state.
        self.tc0.tccr0b().write(|w| w.cs0().no_clock());
        self.tc0.tcnt0().write(|w| w.set(0));
        self.tc0.ocr0a().write(|w| w.set(top));
        self.tc0
            .tccr0a()
            .write(|w| w.wgm0().ctc().com0a().match_toggle());
        self.write_prescaler(prescaler);
    }

    fn silence(&self) {
        self.tc0
            .tccr0a()
            .write(|w| w.wgm0().ctc().com0a().disconnected());
        self.tc0.tccr0b().write(|w| w.cs0().no_clock());
    }

    fn write_prescaler(&self, prescaler: u32) {
        match prescaler {
            1 => self.tc0.tccr0b().write(|w| w.cs0().direct()),
            8 => self.tc0.tccr0b().write(|w| w.cs0().prescale_8()),
            64 => self.tc0.tccr0b().write(|w| w.cs0().prescale_64()),
            256 => self.tc0.tccr0b().write(|w| w.cs0().prescale_256()),
            _ => self.tc0.tccr0b().write(|w| w.cs0().prescale_1024()),
        };
    }

    /// Pick the smallest prescaler (and so the finest frequency resolution)
    /// whose 8-bit compare top can reach the requested frequency. CTC toggle
    /// mode divides the prescaled clock by `2 * (1 + top)`.
    fn prescaler_and_top(frequency_hz: u16) -> (u32, u8) {
        const PRESCALERS: [u32; 5] = [1, 8, 64, 256, 1024];

        let freq = (frequency_hz as u32).max(1);
        for &prescaler in PRESCALERS.iter() {
            let divisor = 2 * prescaler * freq;
            let top_plus_1 = (F_CPU + divisor / 2) / divisor;
            if (1..=256).contains(&top_plus_1) {
                return (prescaler, (top_plus_1 - 1) as u8);
            }
        }

        // Frequency too low for any prescaler to reach exactly; use the
        // slowest tone the hardware can produce.
        (1024, 255)
    }
}
