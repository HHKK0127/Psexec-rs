//! Performance Optimization Module - Phase 4.6
//! GPU optimization, memory pooling, and frame rate measurement

pub mod damage_tracking;
pub mod memory_pool;

pub use damage_tracking::{DamageTracker, DamagedRegion};
pub use memory_pool::{MemoryPool, ByteBufferPool, StringPool, PoolStats};

use std::time::{Instant, Duration};

/// Frame rate monitor for measuring GUI rendering performance
#[derive(Debug, Clone)]
pub struct FrameRateMonitor {
    frame_times: Vec<Duration>,
    max_samples: usize,
    current_frame_start: Option<Instant>,
    total_frames: u64,
}

impl FrameRateMonitor {
    /// Create new frame rate monitor
    pub fn new(max_samples: usize) -> Self {
        Self {
            frame_times: Vec::with_capacity(max_samples),
            max_samples,
            current_frame_start: None,
            total_frames: 0,
        }
    }

    /// Mark start of frame rendering
    pub fn start_frame(&mut self) {
        self.current_frame_start = Some(Instant::now());
    }

    /// Mark end of frame rendering
    pub fn end_frame(&mut self) {
        if let Some(start) = self.current_frame_start {
            let elapsed = start.elapsed();

            if self.frame_times.len() >= self.max_samples {
                self.frame_times.remove(0);
            }
            self.frame_times.push(elapsed);
            self.total_frames += 1;
        }
    }

    /// Get average frame time
    pub fn average_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            Duration::from_secs(0)
        } else {
            let sum: Duration = self.frame_times.iter().sum();
            sum / self.frame_times.len() as u32
        }
    }

    /// Get average frame rate (FPS)
    pub fn average_fps(&self) -> f64 {
        let avg_frame_time = self.average_frame_time();
        if avg_frame_time.as_secs_f64() == 0.0 {
            0.0
        } else {
            1.0 / avg_frame_time.as_secs_f64()
        }
    }

    /// Get minimum frame time
    pub fn min_frame_time(&self) -> Duration {
        self.frame_times.iter().copied().min().unwrap_or_default()
    }

    /// Get maximum frame time
    pub fn max_frame_time(&self) -> Duration {
        self.frame_times.iter().copied().max().unwrap_or_default()
    }

    /// Get number of frames recorded
    pub fn frame_count(&self) -> usize {
        self.frame_times.len()
    }

    /// Get total frames processed
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Get performance metrics
    pub fn metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            average_fps: self.average_fps(),
            min_frame_time_ms: self.min_frame_time().as_secs_f64() * 1000.0,
            max_frame_time_ms: self.max_frame_time().as_secs_f64() * 1000.0,
            average_frame_time_ms: self.average_frame_time().as_secs_f64() * 1000.0,
            frames_measured: self.frame_count(),
        }
    }
}

/// Performance metrics snapshot
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub average_fps: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub average_frame_time_ms: f64,
    pub frames_measured: usize,
}

impl PerformanceMetrics {
    /// Check if performance is acceptable (>30 FPS)
    pub fn is_acceptable(&self) -> bool {
        self.average_fps >= 30.0
    }

    /// Check if performance is good (>60 FPS)
    pub fn is_good(&self) -> bool {
        self.average_fps >= 60.0
    }

    /// Get performance rating
    pub fn rating(&self) -> &'static str {
        if self.average_fps >= 60.0 {
            "Excellent"
        } else if self.average_fps >= 30.0 {
            "Good"
        } else if self.average_fps >= 15.0 {
            "Acceptable"
        } else {
            "Poor"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_frame_rate_monitor_creation() {
        let monitor = FrameRateMonitor::new(60);
        assert_eq!(monitor.frame_count(), 0);
        assert_eq!(monitor.total_frames(), 0);
    }

    #[test]
    fn test_frame_timing() {
        let mut monitor = FrameRateMonitor::new(10);

        monitor.start_frame();
        thread::sleep(Duration::from_millis(16)); // ~60 FPS
        monitor.end_frame();

        assert_eq!(monitor.frame_count(), 1);
        assert_eq!(monitor.total_frames(), 1);
        assert!(monitor.average_frame_time() >= Duration::from_millis(16));
    }

    #[test]
    fn test_multiple_frames() {
        let mut monitor = FrameRateMonitor::new(10);

        for _ in 0..5 {
            monitor.start_frame();
            thread::sleep(Duration::from_millis(10));
            monitor.end_frame();
        }

        assert_eq!(monitor.frame_count(), 5);
        assert_eq!(monitor.total_frames(), 5);
    }

    #[test]
    fn test_performance_metrics() {
        let mut monitor = FrameRateMonitor::new(10);

        monitor.start_frame();
        thread::sleep(Duration::from_millis(16));
        monitor.end_frame();

        let metrics = monitor.metrics();
        assert!(metrics.average_fps > 0.0);
        assert!(metrics.average_frame_time_ms >= 16.0);
    }

    #[test]
    fn test_performance_rating() {
        let metrics_60fps = PerformanceMetrics {
            average_fps: 60.0,
            min_frame_time_ms: 16.0,
            max_frame_time_ms: 20.0,
            average_frame_time_ms: 16.67,
            frames_measured: 100,
        };

        assert_eq!(metrics_60fps.rating(), "Excellent");
        assert!(metrics_60fps.is_good());
        assert!(metrics_60fps.is_acceptable());
    }

    #[test]
    fn test_performance_rating_low() {
        let metrics_10fps = PerformanceMetrics {
            average_fps: 10.0,
            min_frame_time_ms: 100.0,
            max_frame_time_ms: 120.0,
            average_frame_time_ms: 100.0,
            frames_measured: 100,
        };

        assert_eq!(metrics_10fps.rating(), "Poor");
        assert!(!metrics_10fps.is_acceptable());
        assert!(!metrics_10fps.is_good());
    }
}
