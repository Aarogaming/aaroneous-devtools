use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobState {
    Discovered,
    Drafting,
    Gate1Syntax,
    Gate2Types,
    Gate3Tests,
    PendingReview,
    Accepted,
    Actuated,
    Archived,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileModification {
    pub file_path: String,
    pub search_block: String,
    pub replace_block: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IterationAttempt {
    pub attempt_number: u32,
    pub submitted_at_secs: u64,
    pub draft: Option<Vec<FileModification>>,
    pub compiler_output: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: String,
    pub state: JobState,
    pub input_packet: String,
    pub current_draft: Option<Vec<FileModification>>,
    pub rust_draft: Option<String>,
    pub iteration_history: Vec<IterationAttempt>,
    pub created_at_secs: u64,
    pub updated_at_secs: u64,
    pub rejection_reason: Option<String>,
}

pub struct Ledger {
    db: sled::Db,
}

impl Ledger {
    pub fn new(db_path: &Path) -> Result<Self> {
        let db = sled::open(db_path).context("failed to open sled ledger db")?;
        Ok(Self { db })
    }

    pub fn insert_job(&self, job: &JobRecord) -> Result<()> {
        let data = serde_json::to_vec(&job)?;
        self.db.insert(job.job_id.as_bytes(), data)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<JobRecord>> {
        if let Some(data) = self.db.get(job_id.as_bytes())? {
            let job: JobRecord = serde_json::from_slice(&data)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)] // consumed by the ledger iteration flow still being wired (C56)
    pub fn update_state(&self, job_id: &str, new_state: JobState) -> Result<()> {
        if let Some(mut job) = self.get_job(job_id)? {
            job.state = new_state;
            job.updated_at_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.insert_job(&job)?;
        }
        Ok(())
    }

    pub fn get_jobs_by_state(&self, target_state: JobState) -> Result<Vec<JobRecord>> {
        let mut results = Vec::new();
        for item in self.db.iter() {
            let (_, data) = item?;
            let job: JobRecord = serde_json::from_slice(&data)?;
            if job.state == target_state {
                results.push(job);
            }
        }
        Ok(results)
    }

    #[allow(dead_code)] // consumed by the ledger iteration flow still being wired (C56)
    pub fn add_iteration(&self, job_id: &str, attempt: IterationAttempt) -> Result<()> {
        if let Some(mut job) = self.get_job(job_id)? {
            job.iteration_history.push(attempt);
            job.updated_at_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.insert_job(&job)?;
        }
        Ok(())
    }
}
