use std::path::PathBuf;

/// RAII guard for filesystem-based reservation locks. Removes the lock
/// file on drop, ensuring cleanup regardless of how the scope exits.
/// Used by local_worker, agentic_registrar, and multithreader to
/// coordinate exclusive access to shared resources.
pub(crate) struct Reservation(pub PathBuf);

impl Drop for Reservation {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
