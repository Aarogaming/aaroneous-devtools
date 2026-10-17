//! devtools_ops: Aaroneous Devtools Operational Superstructure & Add-On Components
//!
//! Provides the operational component lifecycle framework for Aaroneous development:
//! - `pending`: Canary tools and shadow execution sandbox awaiting graduation into Aaroneous.
//! - `management`: Operational runners, local model worker services, and host telemetry.
//! - `coordination`: Multi-agent consensus, seat attestation, and queue arbitration.

#![forbid(unsafe_code)]

pub mod coordination;
pub mod management;
pub mod pending;
pub mod pipeline_router;

pub use coordination::{LaneClaim, SeatIdentity};
pub use management::{HostTelemetry, LocalWorkerState};
pub use pending::{CandidateComponent, EvaluationStatus};
pub use pipeline_router::{
    PipelineRouterConfig, RoutingAction, WorkerProposal, WorkerResultPayload, evaluate_result,
    route_all_pending_results,
};
