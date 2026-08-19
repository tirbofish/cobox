use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(30);

pub struct Button {
    pressed: bool,
    raw: bool,
    changed: Instant,
}

impl Button {
    pub fn new(pressed: bool) -> Self {
        Self {
            pressed,
            raw: pressed,
            changed: Instant::now(),
        }
    }

    pub fn update(&mut self, raw: bool) -> Option<bool> {
        if raw != self.raw {
            self.raw = raw;
            self.changed = Instant::now();
        }
        if self.changed.elapsed() >= DEBOUNCE && raw != self.pressed {
            self.pressed = raw;
            Some(raw)
        } else {
            None
        }
    }
}
