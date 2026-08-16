//! Optional change adapters. Prune (directory mtime) is the portable baseline
//! and lives in the walker. `notify` / USN are not compiled in the default build.

/// Placeholder for `feature = "watch"` (`notify` crate).
#[cfg(feature = "watch")]
pub mod notify_watch {}

/// Placeholder for `feature = "usn"` (`usn-journal-rs`, Windows).
#[cfg(feature = "usn")]
pub mod usn_watch {}
