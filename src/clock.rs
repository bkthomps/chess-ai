//! Monotonic, server-authoritative Fischer clocks. White is index 0.
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Clock {
    pub base: Duration,
    pub increment: Duration,
    pub remaining: [Duration; 2],
    pub active: Option<bool>,
    last: Instant,
}
impl Clock {
    pub fn new(base: Duration, increment: Duration, now: Instant) -> Self {
        Self {
            base,
            increment,
            remaining: [base; 2],
            active: None,
            last: now,
        }
    }
    pub fn start(&mut self, white: bool, now: Instant) {
        self.active = Some(white);
        self.last = now;
    }
    /// Charge elapsed time and return the side that flagged. Repeated checks
    /// cannot charge the same interval twice, and exactly zero is a flag.
    pub fn update(&mut self, now: Instant) -> Option<bool> {
        let white = self.active?;
        let i = usize::from(!white);
        self.remaining[i] =
            self.remaining[i].saturating_sub(now.saturating_duration_since(self.last));
        self.last = now;
        if self.remaining[i].is_zero() {
            self.active = None;
            Some(white)
        } else {
            None
        }
    }
    /// Called only after a legal move is accepted before the flag falls.
    pub fn complete_move(&mut self, now: Instant) -> Result<(), bool> {
        if let Some(flagged) = self.update(now) {
            return Err(flagged);
        }
        if let Some(white) = self.active {
            let i = usize::from(!white);
            self.remaining[i] += self.increment;
            self.start(!white, now);
        }
        Ok(())
    }
    pub fn stop(&mut self, now: Instant) {
        self.update(now);
        self.active = None;
    }
    pub fn restore(&mut self, remaining: [Duration; 2], white: bool, now: Instant) {
        self.remaining = remaining;
        self.start(white, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn charge_active_side_and_add_increment_once() {
        let now = Instant::now();
        let mut c = Clock::new(Duration::from_secs(120), Duration::from_secs(1), now);
        c.start(true, now);
        assert_eq!(c.update(now + Duration::from_secs(3)), None);
        assert_eq!(c.update(now + Duration::from_secs(3)), None);
        c.complete_move(now + Duration::from_secs(4)).unwrap();
        assert_eq!(
            c.remaining,
            [Duration::from_secs(117), Duration::from_secs(120)]
        );
        assert_eq!(c.active, Some(false));
        c.complete_move(now + Duration::from_secs(6)).unwrap();
        assert_eq!(
            c.remaining,
            [Duration::from_secs(117), Duration::from_secs(119)]
        );
    }
    #[test]
    fn a_late_move_cannot_be_rescued_by_increment() {
        let now = Instant::now();
        let mut c = Clock::new(Duration::from_secs(1), Duration::from_secs(5), now);
        c.start(true, now);
        assert_eq!(c.complete_move(now + Duration::from_secs(1)), Err(true));
        assert_eq!(c.remaining[0], Duration::ZERO);
        assert_eq!(c.active, None);
    }
    #[test]
    fn stopped_and_restored_clocks() {
        let now = Instant::now();
        let mut c = Clock::new(Duration::from_secs(60), Duration::ZERO, now);
        c.start(true, now);
        c.stop(now + Duration::from_secs(5));
        c.update(now + Duration::from_secs(50));
        assert_eq!(c.remaining[0], Duration::from_secs(55));
        c.restore(
            [Duration::from_secs(60); 2],
            true,
            now + Duration::from_secs(60),
        );
        c.update(now + Duration::from_secs(62));
        assert_eq!(c.remaining[0], Duration::from_secs(58));
    }
}
