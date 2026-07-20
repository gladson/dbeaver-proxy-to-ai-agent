//! Optional lightweight metrics collector for OmniRoute integration.
//!
//! Provides atomic counters and latency histograms that can be
//! exposed via the `/health` endpoint when `ENABLE_METRICS=true`.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Latency histogram buckets (in milliseconds).
const BUCKETS_MS: &[u64] = &[50, 200, 500, 2_000, 10_000];

/// Metrics collector for request tracking.
///
/// Uses atomic operations so it's lock-free and safe to share
/// across concurrent requests.
pub struct Metrics {
    /// Total requests received
    requests_total: AtomicU64,
    /// Currently in-flight requests
    requests_in_flight: AtomicU64,
    /// Total error responses
    errors_total: AtomicU64,
    /// Latency histogram counters (indexed by bucket)
    latency_buckets: [AtomicU64; 5],
    /// Start time of the server
    start_time: Instant,
}

/// A point-in-time snapshot of the metrics.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub status: String,
    pub uptime_seconds: u64,
    pub requests_total: u64,
    pub requests_in_flight: u64,
    pub errors_total: u64,
    pub avg_latency_ms: u64,
}

impl Metrics {
    /// Create a new Metrics collector.
    pub fn new() -> Self {
        Metrics {
            requests_total: AtomicU64::new(0),
            requests_in_flight: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            latency_buckets: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            start_time: Instant::now(),
        }
    }

    /// Record a completed request with its duration and status.
    pub fn record(&self, duration_ms: u64, is_error: bool) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.requests_in_flight.fetch_sub(1, Ordering::Relaxed);

        if is_error {
            self.errors_total.fetch_add(1, Ordering::Relaxed);
        }

        // Record in the appropriate latency bucket
        for (i, bucket) in BUCKETS_MS.iter().enumerate() {
            if duration_ms <= *bucket {
                self.latency_buckets[i].fetch_add(1, Ordering::Relaxed);
                break;
            }
        }
    }

    /// Increment in-flight counter (call when request starts).
    pub fn request_started(&self) {
        self.requests_in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Take a snapshot of current metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total = self.requests_total.load(Ordering::Relaxed);
        let uptime = self.start_time.elapsed().as_secs();

        // Calculate average latency from histogram
        let avg = if total > 0 {
            // Weighted average across buckets using bucket midpoints
            let bucket_midpoints = [25u64, 125, 350, 1250, 6000];
            let mut weighted_sum: u64 = 0;

            for (i, bucket) in BUCKETS_MS.iter().enumerate() {
                let count = self.latency_buckets[i].load(Ordering::Relaxed);
                if *bucket == 10_000 {
                    // Last bucket: use the bucket value itself as estimate
                    weighted_sum += count * *bucket;
                } else {
                    weighted_sum += count * bucket_midpoints[i];
                }
            }

            weighted_sum.checked_div(total).unwrap_or(0)
        } else {
            0
        };

        MetricsSnapshot {
            status: "ok".to_string(),
            uptime_seconds: uptime,
            requests_total: total,
            requests_in_flight: self.requests_in_flight.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            avg_latency_ms: avg,
        }
    }

    /// Check if metrics are enabled via environment variable.
    pub fn is_enabled() -> bool {
        std::env::var("ENABLE_METRICS")
            .map(|v| v == "true" || v == "1" || v == "yes")
            .unwrap_or(false)
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 0);
        assert_eq!(snapshot.errors_total, 0);
        assert_eq!(snapshot.requests_in_flight, 0);
        assert_eq!(snapshot.status, "ok");
    }

    #[test]
    fn test_metrics_record_request() {
        let metrics = Metrics::new();
        metrics.request_started();
        metrics.record(100, false);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 1);
        assert_eq!(snapshot.errors_total, 0);
        assert_eq!(snapshot.requests_in_flight, 0); // decremented by record()
    }

    #[test]
    fn test_metrics_record_error() {
        let metrics = Metrics::new();
        metrics.request_started();
        metrics.record(50, true);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 1);
        assert_eq!(snapshot.errors_total, 1);
    }

    #[test]
    fn test_metrics_in_flight() {
        let metrics = Metrics::new();
        metrics.request_started();
        metrics.request_started();
        metrics.record(30, false);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 1);
        assert_eq!(snapshot.requests_in_flight, 1); // one still in flight
    }

    #[test]
    fn test_metrics_uptime_increases() {
        let metrics = Metrics::new();
        let snapshot1 = metrics.snapshot();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let snapshot2 = metrics.snapshot();
        assert!(snapshot2.uptime_seconds >= snapshot1.uptime_seconds);
    }

    #[test]
    fn test_metrics_is_enabled() {
        // Should default to false (env var not set)
        assert!(!Metrics::is_enabled());

        // Set env var and test (Rust 2024: set_var/remove_var are unsafe)
        unsafe {
            std::env::set_var("ENABLE_METRICS", "true");
        }
        assert!(Metrics::is_enabled());

        // Clean up
        unsafe {
            std::env::remove_var("ENABLE_METRICS");
        }
        assert!(!Metrics::is_enabled());
    }

    #[test]
    fn test_metrics_snapshot_serialization() {
        let metrics = Metrics::new();
        // Small sleep to ensure uptime is measurable
        std::thread::sleep(std::time::Duration::from_millis(1));
        metrics.request_started();
        metrics.record(150, false);

        let snapshot = metrics.snapshot();
        let json = serde_json::to_value(&snapshot).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["requests_total"], 1);
        assert_eq!(json["uptime_seconds"].as_u64().unwrap(), 0); // < 1 second
        assert!(json["avg_latency_ms"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_metrics_multiple_records() {
        let metrics = Metrics::new();

        // Record several requests with different latencies
        for i in 0..10 {
            metrics.request_started();
            metrics.record(30 + i * 20, i % 3 == 0); // every 3rd is error
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.requests_total, 10);
        // Errors: indices 0, 3, 6, 9 = 4 errors
        assert_eq!(snapshot.errors_total, 4);
    }
}
