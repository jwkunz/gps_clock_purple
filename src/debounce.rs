/// Debounces a boolean input by requiring a new raw value to hold steady for
/// a minimum duration before it is accepted.
pub struct Debounce {
    debounced: bool,
    candidate: bool,
    candidate_since_ms: u32,
}

impl Debounce {
    pub const fn new(initial: bool) -> Self {
        Self {
            debounced: initial,
            candidate: initial,
            candidate_since_ms: 0,
        }
    }

    /// Feed a new raw sample. Call on every poll with the current time base.
    pub fn update(&mut self, raw: bool, now_ms: u32, stable_ms: u32) {
        if raw != self.candidate {
            self.candidate = raw;
            self.candidate_since_ms = now_ms;
        } else if self.candidate != self.debounced
            && now_ms.wrapping_sub(self.candidate_since_ms) >= stable_ms
        {
            self.debounced = self.candidate;
        }
    }

    pub fn is_high(&self) -> bool {
        self.debounced
    }
}
