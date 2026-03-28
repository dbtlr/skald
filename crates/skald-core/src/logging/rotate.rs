use crate::config::paths;
use std::time::{Duration, SystemTime};

pub fn prune_old_logs(retention_days: u64) -> std::io::Result<usize> {
    let log_dir = paths::log_dir();
    if !log_dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now() - Duration::from_secs(retention_days * 86400);
    let mut pruned = 0;

    for entry in std::fs::read_dir(&log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if let Ok(modified) = metadata.modified()
            && modified < cutoff
            && std::fs::remove_file(entry.path()).is_ok()
        {
            pruned += 1;
        }
    }

    Ok(pruned)
}
