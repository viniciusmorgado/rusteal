use std::ops::{Add, Sub};
use std::time::Duration;

fn engine() -> bool {
    crate::api::is_api_initialized() && !crate::api::api().platform.is_null()
}

pub fn now_seconds() -> f64 {
    if engine() {
        unsafe { crate::ffi_dispatch::platform_seconds() }
    } else {
        crate::std_fallback::seconds()
    }
}

pub fn sleep(duration: Duration) {
    if engine() {
        unsafe { crate::ffi_dispatch::platform_sleep(duration.as_secs_f32()) }
    } else {
        crate::std_fallback::sleep(duration);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Instant(f64);

impl Instant {
    pub fn now() -> Self {
        Self(now_seconds())
    }

    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration::from_secs_f64((self.0 - earlier.0).max(0.0))
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        (self.0 >= earlier.0).then(|| Duration::from_secs_f64(self.0 - earlier.0))
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, duration: Duration) -> Instant {
        Instant(self.0 + duration.as_secs_f64())
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, duration: Duration) -> Instant {
        Instant(self.0 - duration.as_secs_f64())
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instants_measure_like_std() {
        let start = Instant::now();
        sleep(Duration::from_millis(5));
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(5), "{elapsed:?}");
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");

        let later = start + Duration::from_secs(2);
        let apart = later - start;
        assert!(
            apart.abs_diff(Duration::from_secs(2)) < Duration::from_micros(1),
            "{apart:?}"
        );
        assert_eq!(start.duration_since(later), Duration::ZERO);
        assert!(start.checked_duration_since(later).is_none());
        assert!((later - Duration::from_secs(2)).duration_since(start) < Duration::from_micros(1));
    }
}
