use backon::{BackoffBuilder, ExponentialBackoff, ExponentialBuilder};
use std::time::{Duration, Instant};

/// Resettable backoff policy.
pub struct ResettableBackoff {
    backoff: ExponentialBackoff,
    builder: ExponentialBuilder,
    start_time: Instant,
}

impl Default for ResettableBackoff {
    /// Creates default resettable backoff policy adjusted for Kubernetes API.
    fn default() -> Self {
        let builder = ExponentialBuilder::default()
            .with_min_delay(Duration::from_millis(800))
            .with_max_delay(Duration::from_secs(30))
            .with_factor(2.0)
            .without_max_times()
            .with_jitter();
        let backoff = builder.build();

        Self {
            backoff,
            builder,
            start_time: Instant::now(),
        }
    }
}

impl ResettableBackoff {
    /// Gets next backoff duration.
    pub fn next_backoff(&mut self) -> Option<Duration> {
        if self.start_time.elapsed().as_secs() > 120 {
            self.reset();
        }

        self.backoff.next()
    }

    /// Resets backoff.
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.backoff = self.builder.build();
    }
}
