//! Management Component Lifecycle Tier
//!
//! Owns continuous maintenance runners, local model workers, and system telemetry
//! monitoring across development stations.

use serde::{Deserialize, Serialize};

/// Measured operational telemetry for an agent station.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostTelemetry {
    pub seat_id: String,
    pub os: String,
    pub cpu_cores: usize,
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub disk_free_gb: u64,
    pub gpu_model: Option<String>,
    pub gpu_vram_mb: Option<u64>,
    pub thermal_throttle: bool,
}

impl HostTelemetry {
    pub fn new(seat_id: impl Into<String>, os: impl Into<String>) -> Self {
        Self {
            seat_id: seat_id.into(),
            os: os.into(),
            cpu_cores: 0,
            ram_total_mb: 0,
            ram_available_mb: 0,
            disk_free_gb: 0,
            gpu_model: None,
            gpu_vram_mb: None,
            thermal_throttle: false,
        }
    }

    /// Evaluates whether the host is under resource backpressure.
    pub fn is_under_backpressure(&self) -> bool {
        if self.thermal_throttle {
            return true;
        }
        if self.disk_free_gb < 5 {
            return true;
        }
        if self.ram_total_mb > 0 && self.ram_available_mb < (self.ram_total_mb / 10) {
            return true;
        }
        false
    }
}

/// Operational status of the local model worker service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalWorkerState {
    pub active_model: String,
    pub max_pending: u32,
    pub pending_count: u32,
    pub quiet_window_active: bool,
    pub reservation_held_by: Option<String>,
}

impl LocalWorkerState {
    pub fn new(model: impl Into<String>, max_pending: u32) -> Self {
        Self {
            active_model: model.into(),
            max_pending,
            pending_count: 0,
            quiet_window_active: false,
            reservation_held_by: None,
        }
    }

    pub fn can_accept_job(&self) -> bool {
        !self.quiet_window_active
            && self.reservation_held_by.is_none()
            && self.pending_count < self.max_pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_telemetry_backpressure() {
        let mut telemetry = HostTelemetry::new("ubuntu-worker-01", "linux");
        telemetry.disk_free_gb = 50;
        telemetry.ram_total_mb = 16384;
        telemetry.ram_available_mb = 8192;
        assert!(!telemetry.is_under_backpressure());

        telemetry.thermal_throttle = true;
        assert!(telemetry.is_under_backpressure());
    }

    #[test]
    fn test_worker_can_accept_job() {
        let mut worker = LocalWorkerState::new("qwen3.5:9b-q6", 16);
        assert!(worker.can_accept_job());

        worker.pending_count = 16;
        assert!(!worker.can_accept_job());

        worker.pending_count = 5;
        worker.reservation_held_by = Some("c67_review".into());
        assert!(!worker.can_accept_job());
    }
}
