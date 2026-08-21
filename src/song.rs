use crate::buzzer::{Buzzer, BuzzerConfig};

/// A pitch, or `Rest` for silence. Two natural-note octaves are enough for a
/// simple alarm melody; extend this table if a song needs sharps/flats or a
/// wider range.
#[derive(Clone, Copy)]
#[allow(dead_code)] // Full two-octave table is available even when a given song only uses some notes.
pub enum Note {
    Rest,
    C4,
    D4,
    E4,
    F4,
    G4,
    A4,
    B4,
    C5,
    D5,
    E5,
    F5,
    G5,
    A5,
    B5,
}

impl Note {
    const fn frequency_hz(self) -> u16 {
        match self {
            Note::Rest => 0,
            Note::C4 => 262,
            Note::D4 => 294,
            Note::E4 => 330,
            Note::F4 => 349,
            Note::G4 => 392,
            Note::A4 => 440,
            Note::B4 => 494,
            Note::C5 => 523,
            Note::D5 => 587,
            Note::E5 => 659,
            Note::F5 => 698,
            Note::G5 => 784,
            Note::A5 => 880,
            Note::B5 => 988,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SongStep {
    pub note: Note,
    /// Duration in beats; 1 is one quarter note at the configured tempo.
    pub beats: u8,
}

/// Notes, tempo, and repeat behavior for the next song. Applied by
/// `Song::enable`.
#[allow(dead_code)]
pub struct SongConfig<'a> {
    pub steps: &'a [SongStep],
    pub beats_per_minute: u16,
    /// When true, the song loops back to its first note after the last
    /// instead of stopping.
    pub repeat: bool,
}

/// Polled, run-to-completion song sequencer. Owns a `Buzzer` and steps it
/// through a note array on beat boundaries derived from the configured
/// tempo.
pub struct Song<'a> {
    buzzer: Buzzer,
    steps: &'a [SongStep],
    beats_per_minute: u16,
    repeat: bool,
    playing: bool,
    index: usize,
    step_started_ms: u32,
}

impl<'a> Song<'a> {
    pub fn new(buzzer: Buzzer) -> Self {
        Self {
            buzzer,
            steps: &[],
            beats_per_minute: 120,
            repeat: false,
            playing: false,
            index: 0,
            step_started_ms: 0,
        }
    }

    /// Set the song to play on the next `enable`. Has no effect on a song
    /// already playing.
    #[allow(dead_code)]
    pub fn configure(&mut self, config: SongConfig<'a>) {
        self.steps = config.steps;
        self.beats_per_minute = config.beats_per_minute.max(1);
        self.repeat = config.repeat;
    }

    /// Start the configured song from its first note.
    #[allow(dead_code)]
    pub fn enable(&mut self, now_ms: u32) {
        if self.steps.is_empty() {
            return;
        }
        self.index = 0;
        self.playing = true;
        self.start_step(now_ms);
    }

    /// Stop the song and silence the buzzer immediately.
    pub fn disable(&mut self) {
        self.playing = false;
        self.buzzer.disable();
    }

    /// Advance the run-to-completion state machine. Call every main-loop
    /// iteration.
    pub fn update(&mut self, now_ms: u32) {
        self.buzzer.update(now_ms);

        if !self.playing {
            return;
        }

        let elapsed_ms = now_ms.wrapping_sub(self.step_started_ms);
        if elapsed_ms < self.step_duration_ms(self.index) {
            return;
        }

        self.index += 1;
        if self.index >= self.steps.len() {
            if !self.repeat {
                self.disable();
                return;
            }
            self.index = 0;
        }
        self.start_step(now_ms);
    }

    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.playing
    }

    fn start_step(&mut self, now_ms: u32) {
        self.step_started_ms = now_ms;
        let frequency_hz = self.steps[self.index].note.frequency_hz();
        if frequency_hz == 0 {
            self.buzzer.disable();
        } else {
            self.buzzer.configure(BuzzerConfig {
                frequency_hz,
                duration_ms: 0,
                run_until_stopped: true,
            });
            self.buzzer.enable(now_ms);
        }
    }

    fn step_duration_ms(&self, index: usize) -> u32 {
        let beat_ms = 60_000u32 / self.beats_per_minute as u32;
        beat_ms * self.steps[index].beats as u32
    }
}
