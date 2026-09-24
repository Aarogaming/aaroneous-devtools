use crate::ledger::{JobState, Ledger};
use std::path::Path;

pub fn dump_pending() {
    let db_path = Path::new(
        "d:\\aaroneous-devtools\\artifacts\\local-worker-service\\reservation\\ledger_db",
    );
    if let Ok(ledger) = Ledger::new(db_path) {
        if let Ok(jobs) = ledger.get_jobs_by_state(JobState::PendingReview) {
            for job in jobs {
                let has_plan = job.current_draft.as_ref().map_or(false, |p| !p.is_empty());
                let has_rust_draft = job
                    .rust_draft
                    .as_ref()
                    .map_or(false, |c| !c.trim().is_empty());
                println!(
                    "Job {}: has_plan={}, has_rust_draft={}",
                    job.job_id, has_plan, has_rust_draft
                );
            }
        }
    }
}
