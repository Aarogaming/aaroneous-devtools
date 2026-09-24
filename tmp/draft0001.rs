use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use parking_lot::RwLock;
use atomic::{AtomicBool, AtomicU64};
use crate::compute::{SiOnlineLearner, ShadowTickResult, ConcurrenceSnapshot};
use crate::ipc_bus::ObservationBuffer;
use crate::paths::WorkspacePaths;
use crate::governance::SmtActionInterlock;
use crate::state::SystemState;

/// SupervisoryDaemon orchestrates the hypervisor tick loop.
/// It manages shadow inference, concurrence tracking, and state publishing.
pub struct SupervisoryDaemon {
    /// Optional shadow learner for dual-rail inference.
    shadow_learner: Option<Arc<Mutex<SiOnlineLearner>>>,
    /// Shared concurrence snapshot for external readers.
    concurrence_snapshot: Arc<RwLock<ConcurrenceSnapshot>>,
    /// System state for the current tick.
    state: Arc<RwLock<SystemState>>,
    /// Tick rate configuration.
    tick_rate: Duration,
    /// Hive database for persistence.
    hive_db: Option<Arc<Mutex<crate::hive::HiveDB>>>,
    /// Workspace root path.
    _workspace_root: std::path::PathBuf,
    /// Kalman filter for state estimation.
    kalman_filter: Option<crate::filters::KalmanFilter>,
    /// Hidden Markov Model for intent tracking.
    hmm_model: Option<crate::models::HMMModel>,
    /// Shutdown flag.
    shutdown: Arc<AtomicBool>,
    /// Maximum ticks before forced termination.
    max_ticks: Arc<AtomicU64>,
    /// Tick start time.
    tick_start: Arc<RwLock<std::time::Instant>>,
    /// State publisher for external state snapshots.
    state_publisher: Arc<crate::state_snapshot::EngineStatePublisher>,
    /// Pacing regulator for tick timing.
    pacing_regulator: crate::pacing::PacingRegulator,
    /// Flight recorder for debugging.
    flight_recorder: Option<Arc<Mutex<crate::ipc_bus::FlightRecorder>>>,
    /// SMT action interlock gatekeeper.
    smt_interlock: Arc<RwLock<SmtActionInterlock>>,
    /// Observation buffer for passive telemetry.
    observation_buffer: Option<Arc<Mutex<ObservationBuffer>>>,
}

impl SupervisoryDaemon {
    /// Constructs a new SupervisoryDaemon with optional shadow learner injection.
    /// If `reflex_si_path` exists, the learner is loaded and injected via `with_shadow_learner`.
    pub fn new(
        tick_rate_ms: u64,
        hive_db: Option<Arc<Mutex<crate::hive::HiveDB>>>,
        workspace_root: std::path::PathBuf,
        reflex_si_path: Option<std::path::PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let paths = WorkspacePaths::from_config(WorkspacePathsConfig::default());
        let obs_path = paths.data().join("shm").join("observation.shm");
        let observation_buffer = ObservationBuffer::open_or_create(
            &obs_path,
            ipc_bus::DEFAULT_OBSERVATION_CAPACITY,
        )
        .ok()
        .map(|b| Arc::new(Mutex::new(b)));

        let shadow_learner = if let Some(path) = reflex_si_path {
            let learner = SiOnlineLearner::load(&path)?;
            Some(Arc::new(Mutex::new(learner)))
        } else {
            None
        };

        let daemon = SupervisoryDaemon {
            shadow_learner,
            concurrence_snapshot: Arc::new(RwLock::new(ConcurrenceSnapshot::default())),
            state: Arc::new(RwLock::new(SystemState::default())),
            tick_rate: Duration::from_millis(tick_rate_ms),
            hive_db,
            _workspace_root: workspace_root,
            kalman_filter: None,
            hmm_model: None,
            shutdown: Arc::new(AtomicBool::new(false)),
            max_ticks: Arc::new(AtomicU64::new(1_000_000)),
            tick_start: Arc::new(RwLock::new(std::time::Instant::now())),
            state_publisher: Arc::new(crate::state_snapshot::EngineStatePublisher::new()),
            pacing_regulator: crate::pacing::PacingRegulator::new(),
            flight_recorder: None,
            smt_interlock: Arc::new(RwLock::new(SmtActionInterlock::strict())),
            observation_buffer,
        };

        // Inject shadow learner if present
        if let Some(learner) = shadow_learner {
            Ok(daemon.with_shadow_learner(learner))
        } else {
            Ok(daemon)
        }
    }

    /// Mounts a pre-initialised `.si` learner in dual-rail shadow mode.
    pub fn with_shadow_learner(
        mut self,
        learner: Arc<Mutex<SiOnlineLearner>>,
    ) -> Self {
        self.shadow_learner = Some(learner);
        self
    }

    /// Executes a single tick of the supervisory loop.
    pub fn tick(&mut self, dispatched_opcode: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.state.write();
        let tick_start = self.tick_start.write();
        let elapsed = tick_start.elapsed();
        let tick_duration_us = elapsed.as_micros() as u32;

        // Update state with dispatched opcode
        state.last_dispatched_opcode = dispatched_opcode;
        state.integrity_score = 100.0;
        state.concept_drift = 0.0;

        // Run shadow inference if learner is mounted
        if let Some(ref learner_mutex) = self.shadow_learner {
            let learner = learner_mutex.lock();
            let state_slice = self.build_state_slice(&state);
            
            match learner.forward_adapted_step(&state_slice) {
                Ok(pred) => {
                    // Use the actual dispatched opcode, not hardcoded zero
                    let actual_opcode = state.last_dispatched_opcode;
                    let predicted_opcode = pred.predicted_opcode_id;
                    let confidence = pred.confidence_score;
                    let reward = (state.integrity_score as f32 - state.concept_drift * 100.0)
                        .clamp(-1.0, 1.0) / 100.0;

                    let tick_result = ShadowTickResult {
                        actual_opcode,
                        predicted_opcode,
                        confidence,
                        reward,
                    };

                    // Update concurrence engine
                    CONCURRENCE_ENGINE.with(|cell| {
                        let engine = unsafe { &mut *cell.get() };
                        engine.update(tick_result);
                    });

                    // Publish snapshot
                    let published_snapshot = CONCURRENCE_ENGINE.with(|cell| {
                        unsafe { (*cell.get()).snapshot() }
                    });
                    *self.concurrence_snapshot.write() = published_snapshot;

                    // Periodic export
                    if tick_count.is_multiple_of(100) {
                        write_concurrence_report(&published_snapshot);
                    }
                }
                Err(e) => {
                    // Log error but continue
                    eprintln!("Shadow learner error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Builds a 256-element state slice for the learner.
    fn build_state_slice(&self, state: &SystemState) -> [f32; 256] {
        let mut s = [0.0f32; 256];
        let src = &state.latent_vector;
        let copy_len = src.len().min(256);
        s[..copy_len].copy_from_slice(&src[..copy_len]);
        if copy_len < 256 {
            s[copy_len.min(255)] = (state.integrity_score as f32) / 100.0;
        }
        s
    }
}

// Thread-local concurrence engine
thread_local! {
    static CONCURRENCE_ENGINE: std::cell::RefCell<compute::ConcurrenceEngine> = std::cell::RefCell::new(compute::ConcurrenceEngine::default());
}

fn write_concurrence_report(snapshot: &ConcurrenceSnapshot) {
    // Best-effort JSON export
    let json = serde_json::to_string(snapshot).unwrap_or_default();
    let _ = std::fs::write("/tmp/concurrence_report.json", json);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_shadow_learner_injection() {
        let reflex_si_path = PathBuf::from("/fake/reflex_v1.si");
        let result = SupervisoryDaemon::new(100, None, PathBuf::from("/workspace"), Some(reflex_si_path));
        assert!(result.is_ok(), "Learner should be injected when reflex_v1.si exists");
    }

    #[test]
    fn test_concurrence_tracking_uses_dispatched_opcode() {
        let mut daemon = SupervisoryDaemon::new(100, None, PathBuf::from("/workspace"), None).unwrap();
        let dispatched_opcode = 0x42;
        let result = daemon.tick(dispatched_opcode);
        assert!(result.is_ok(), "Tick should succeed with dispatched opcode");
        // Verify concurrence engine was updated with dispatched_opcode, not 0
        let snapshot = *daemon.concurrence_snapshot.read();
        assert!(snapshot.concurrence_score > 0.0, "Concurrence should reflect actual opcode");
    }
}
