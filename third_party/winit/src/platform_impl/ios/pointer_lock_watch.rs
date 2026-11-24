//! Verifies that UIKit honoured a pointer-lock request, and re-asks when it did not.
//!
//! `prefersPointerLocked` is only a preference: UIKit grants it later, or never (the window is
//! not yet key, a swipe-up suspended the scene, another scene owns the pointer). Reporting
//! success at request time leaves the game believing it holds a pointer it does not. The watch
//! is polled once per frame with the scene's real lock state, so it needs no timer. Plain data,
//! no UIKit types, so the host test build can exercise it.
use std::time::{Duration, Instant};

/// Time between re-requests while a wanted lock is still not granted.
pub(crate) const RETRY_INTERVAL: Duration = Duration::from_millis(250);
/// Re-requests before the watch stops nudging and reports the lock as refused.
pub(crate) const MAX_RETRIES: u32 = 8;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Nothing to do: no lock wanted, the lock is held, or the next retry is not due.
    Idle,
    /// The lock is wanted but not held: ask UIKit again.
    Retry,
    /// The retries ran out. Reported once per request.
    Refused,
}

#[derive(Default)]
pub(crate) struct LockWatch {
    wanted: bool,
    retries: u32,
    last_ask: Option<Instant>,
    refused: bool,
}

impl LockWatch {
    /// The game asked for (or released) the lock. Every new request starts a fresh count.
    pub(crate) fn request(&mut self, wanted: bool, now: Instant) {
        self.wanted = wanted;
        self.retries = 0;
        self.refused = false;
        self.last_ask = wanted.then_some(now);
    }

    /// One poll with the scene's lock state (`None`: the device has no pointer lock).
    pub(crate) fn poll(&mut self, held: Option<bool>, now: Instant) -> Verdict {
        if !self.wanted || held.is_none() {
            return Verdict::Idle;
        }
        if held == Some(true) {
            self.retries = 0;
            self.refused = false;
            return Verdict::Idle;
        }
        if self.refused {
            return Verdict::Idle;
        }
        let due = self
            .last_ask
            .is_none_or(|t| now.duration_since(t) >= RETRY_INTERVAL);
        if !due {
            return Verdict::Idle;
        }
        if self.retries >= MAX_RETRIES {
            self.refused = true;
            return Verdict::Refused;
        }
        self.retries += 1;
        self.last_ask = Some(now);
        Verdict::Retry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64, base: Instant) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn a_granted_lock_is_never_re_requested() {
        let t0 = Instant::now();
        let mut w = LockWatch::default();
        w.request(true, t0);
        assert_eq!(w.poll(Some(true), at(300, t0)), Verdict::Idle);
        assert_eq!(w.poll(Some(true), at(900, t0)), Verdict::Idle);
    }

    #[test]
    fn a_missing_lock_is_re_asked_once_per_interval() {
        let t0 = Instant::now();
        let mut w = LockWatch::default();
        w.request(true, t0);
        assert_eq!(w.poll(Some(false), at(100, t0)), Verdict::Idle);
        assert_eq!(w.poll(Some(false), at(250, t0)), Verdict::Retry);
        assert_eq!(w.poll(Some(false), at(300, t0)), Verdict::Idle);
        assert_eq!(w.poll(Some(false), at(500, t0)), Verdict::Retry);
    }

    #[test]
    fn exhausted_retries_report_refusal_once() {
        let t0 = Instant::now();
        let mut w = LockWatch::default();
        w.request(true, t0);
        let mut verdicts = Vec::new();
        for i in 1..=12u64 {
            verdicts.push(w.poll(Some(false), at(250 * i, t0)));
        }
        assert_eq!(verdicts.iter().filter(|v| **v == Verdict::Retry).count(), 8);
        assert_eq!(
            verdicts.iter().filter(|v| **v == Verdict::Refused).count(),
            1
        );
        assert_eq!(verdicts[8], Verdict::Refused);
        assert_eq!(verdicts[11], Verdict::Idle);
    }

    #[test]
    fn a_new_request_resets_the_count_and_a_release_stops_the_watch() {
        let t0 = Instant::now();
        let mut w = LockWatch::default();
        w.request(true, t0);
        for i in 1..=9u64 {
            w.poll(Some(false), at(250 * i, t0));
        }
        w.request(true, at(3000, t0));
        assert_eq!(w.poll(Some(false), at(3250, t0)), Verdict::Retry);
        w.request(false, at(4000, t0));
        assert_eq!(w.poll(Some(false), at(5000, t0)), Verdict::Idle);
    }

    #[test]
    fn a_device_without_pointer_lock_is_left_alone() {
        let t0 = Instant::now();
        let mut w = LockWatch::default();
        w.request(true, t0);
        assert_eq!(w.poll(None, at(1000, t0)), Verdict::Idle);
    }
}
