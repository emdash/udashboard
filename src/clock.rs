use std::time::Instant;

// Wrapper around somewhat obnoxious system time api.
pub struct Clock {
    instant: Instant,
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            instant: Instant::now(),
        }
    }

    // Return system time as floating point value.
    pub fn seconds(&self) -> f64 {
        let e = self.instant.elapsed();
        (e.as_secs() as f64) + (0.001 * e.subsec_millis() as f64)
    }
}
