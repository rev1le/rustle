//! Progress bars via `indicatif`

use indicatif::{ProgressBar, ProgressStyle};

/// Scan progress bar
///
/// @todo style "scanning: {wide_bar} {pos}/{len}"
pub fn scan_progress(_total: u64) -> ProgressBar {
    let _ = ProgressStyle::default_bar();
    todo!("create the scan progress bar")
}

/// Classification spinner (file count unknown up front)
///
/// @todo style "{spinner} classifying: {pos} files"
pub fn classify_spinner() -> ProgressBar {
    let _ = ProgressStyle::default_spinner();
    todo!("create the classification spinner")
}
