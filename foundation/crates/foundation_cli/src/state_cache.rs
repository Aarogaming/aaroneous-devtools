use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Lightweight summary of a record for in-memory indexing.
/// Avoids re-reading and re-parsing the full Record JSON on every tick.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RecordSummary {
    pub job_id: String,
    pub base_key: String,
    pub submitted_at_secs: u64,
    pub input_hash: u64,
    pub rejection_reason: Option<String>,
    pub outcome: String,
}

/// In-memory index of all known records with persistent sled backing.
/// Eliminates the per-tick full archive scan (up to 65536 files).
/// Sled provides crash recovery via WAL and faster startup via persistent index.
pub struct StateCache {
    /// path -> RecordSummary (hot cache)
    records: DashMap<PathBuf, RecordSummary>,
    /// base_job_key -> latest RecordSummary (most recent attempt)
    pub latest_by_base: DashMap<String, RecordSummary>,
    /// Directory mtimes for change detection
    results_dir_mtime: std::sync::atomic::AtomicU64,
    archive_dir_mtime: std::sync::atomic::AtomicU64,
    /// Persistent sled database for crash recovery
    db: sled::Db,
}

impl StateCache {
    /// Create a new StateCache with a sled database at the given path.
    /// The sled DB persists record summaries across process restarts.
    pub fn new(db_path: &Path) -> Self {
        let db = sled::open(db_path).expect("failed to open sled database");
        let cache = Self {
            records: DashMap::new(),
            latest_by_base: DashMap::new(),
            results_dir_mtime: std::sync::atomic::AtomicU64::new(0),
            archive_dir_mtime: std::sync::atomic::AtomicU64::new(0),
            db,
        };
        // Load persisted records from sled into memory
        cache.load_from_db();
        cache
    }

    /// Create a StateCache with an in-memory sled DB (for tests).
    pub fn new_in_memory() -> Self {
        let db = sled::Config::new().temporary(true).open().expect("failed to open temp sled");
        let cache = Self {
            records: DashMap::new(),
            latest_by_base: DashMap::new(),
            results_dir_mtime: std::sync::atomic::AtomicU64::new(0),
            archive_dir_mtime: std::sync::atomic::AtomicU64::new(0),
            db,
        };
        cache.load_from_db();
        cache
    }

    /// Load persisted records from sled into the in-memory DashMap.
    fn load_from_db(&self) {
        for entry in self.db.iter() {
            if let Ok((key, value)) = entry {
                if let Ok(summary) = serde_json::from_slice::<RecordSummary>(&value) {
                    let path = PathBuf::from(String::from_utf8_lossy(&key).to_string());
                    let base_key = summary.base_key.clone();
                    self.records.insert(path, summary.clone());
                    let mut e = self.latest_by_base.entry(base_key).or_insert(summary.clone());
                    if summary.submitted_at_secs > e.submitted_at_secs {
                        *e = summary;
                    }
                }
            }
        }
    }

    /// Get the last attempt time for a base job key.
    pub fn last_attempt_secs(&self, base_key: &str) -> u64 {
        self.latest_by_base
            .get(base_key)
            .map(|r| r.submitted_at_secs)
            .unwrap_or(0)
    }

    /// Get the attempt count for a base job key.
    pub fn attempt_count(&self, base_key: &str) -> u32 {
        self.records
            .iter()
            .filter(|r| r.base_key == base_key)
            .count() as u32
    }

    /// Check if a specific (job_id, input_hash) combination exists as a
    /// non-failed record. Failed records should be retried, not deduped.
    pub fn has_exact_input_non_failed(&self, job_id: &str, input_hash: u64) -> bool {
        self.records
            .iter()
            .any(|r| {
                r.job_id == job_id
                    && r.input_hash == input_hash
                    && r.outcome != "failed"
            })
    }

    /// Check if a job has any rejection reason and return the most recent one.
    pub fn latest_rejection(&self, base_key: &str) -> Option<String> {
        self.latest_by_base
            .get(base_key)
            .and_then(|r| r.rejection_reason.clone())
    }

    /// Insert or update a record summary. Persists to sled for crash recovery.
    pub fn upsert(&self, summary: RecordSummary) {
        let path_key = summary.job_id.clone();
        let base_key = summary.base_key.clone();
        // Persist to sled
        if let Ok(data) = serde_json::to_vec(&summary) {
            let _ = self.db.insert(path_key.as_bytes(), data);
        }
        // Update in-memory index
        self.records.insert(
            PathBuf::from(&path_key),
            summary.clone(),
        );
        // Update latest_by_base if this is newer
        let mut entry = self.latest_by_base.entry(base_key).or_insert(summary.clone());
        if summary.submitted_at_secs > entry.submitted_at_secs {
            *entry = summary;
        }
    }

    /// Check if a directory has changed since last check.
    pub fn dir_changed(&self, dir: &Path, is_archive: bool) -> bool {
        let mtime = std::fs::metadata(dir)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let atomic = if is_archive {
            &self.archive_dir_mtime
        } else {
            &self.results_dir_mtime
        };
        let prev = atomic.load(std::sync::atomic::Ordering::Relaxed);
        if mtime != prev {
            atomic.store(mtime, std::sync::atomic::Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Scan a directory for new/changed record files and update the cache.
    /// Returns the number of new records added.
    pub fn scan_directory(&self, dir: &Path) -> usize {
        let mut added = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name.ends_with("ledger.json")
                    || name.ends_with("marker_retained.json")
                    || name.contains("exploration-summary")
                {
                    continue;
                }
                // Skip if already cached
                if self.records.contains_key(&path) {
                    continue;
                }
                // Read and parse
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(rec) = serde_json::from_str::<crate::local_worker::Record>(&text) {
                        let input_hash = hash_input(&rec.input);
                        let base_key = crate::local_worker::base_job_key(&rec.job_id).to_string();
                        let summary = RecordSummary {
                            job_id: rec.job_id,
                            base_key,
                            submitted_at_secs: rec.submitted_at_secs,
                            input_hash,
                            rejection_reason: rec.rejection_reason,
                            outcome: rec.outcome,
                        };
                        self.upsert(summary);
                        added += 1;
                    }
                }
            }
        }
        added
    }

    /// Full rescan of both results and archive directories.
    /// Only rescans directories whose mtime has changed.
    pub fn rescan_if_needed(&self, results_dir: &Path, archive_dir: &Path) -> usize {
        let mut total_added = 0;
        if self.dir_changed(results_dir, false) {
            total_added += self.scan_directory(results_dir);
        }
        if archive_dir.exists() && self.dir_changed(archive_dir, true) {
            total_added += self.scan_directory(archive_dir);
        }
        total_added
    }

    /// Number of cached records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Simple hash function for input strings (for dedup checking).
/// Not cryptographic - just needs to be fast and collision-resistant enough.
pub fn hash_input(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_cache_basic() {
        let cache = StateCache::new_in_memory();
        assert!(cache.is_empty());

        let summary = RecordSummary {
            job_id: "TEST-1".into(),
            base_key: "TEST".into(),
            submitted_at_secs: 100,
            input_hash: 42,
            rejection_reason: None,
            outcome: "pending-owner-review".into(),
        };
        cache.upsert(summary);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.last_attempt_secs("TEST"), 100);
        assert_eq!(cache.attempt_count("TEST"), 1);
        assert!(!cache.has_exact_input_non_failed("TEST-1", 99));
        assert!(cache.has_exact_input_non_failed("TEST-1", 42));
    }

    #[test]
    fn test_state_cache_dedup() {
        let cache = StateCache::new_in_memory();

        // Two records with same base key
        cache.upsert(RecordSummary {
            job_id: "TEST-1".into(),
            base_key: "TEST".into(),
            submitted_at_secs: 100,
            input_hash: 42,
            rejection_reason: None,
            outcome: "pending-owner-review".into(),
        });
        cache.upsert(RecordSummary {
            job_id: "TEST-2".into(),
            base_key: "TEST".into(),
            submitted_at_secs: 200,
            input_hash: 99,
            rejection_reason: Some("fix the overflow check".into()),
            outcome: "failed".into(),
        });

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.last_attempt_secs("TEST"), 200); // newer wins
        assert_eq!(cache.attempt_count("TEST"), 2);
        assert_eq!(
            cache.latest_rejection("TEST"),
            Some("fix the overflow check".into())
        );
    }
}
