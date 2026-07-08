use std::collections::hash_map::DefaultHasher;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// How long to suppress duplicate processing of the same clipboard content.
const DEDUP_SECS: u64 = 10;

fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ah")
}

/// Cross-process exclusive lock via atomic create.
pub struct ExclusiveLock(PathBuf);

impl ExclusiveLock {
    pub fn try_acquire(name: &str) -> Option<Self> {
        let dir = data_dir();
        fs::create_dir_all(&dir).ok()?;
        let path = dir.join(format!("{name}.lock"));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .ok()?;
        Some(Self(path))
    }
}

impl Drop for ExclusiveLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Ensures only one ah daemon runs; reclaims stale PID files after crashes.
pub struct DaemonGuard {
    pid_path: PathBuf,
    pid: u32,
}

impl DaemonGuard {
    pub fn acquire() -> anyhow::Result<Self> {
        let dir = data_dir();
        fs::create_dir_all(&dir)?;
        let pid_path = dir.join("daemon.pid");
        let pid = process::id();

        if let Ok(old) = fs::read_to_string(&pid_path) {
            if let Ok(old_pid) = old.trim().parse::<u32>() {
                if old_pid != pid && process_alive(old_pid) {
                    anyhow::bail!(
                        "ah daemon already running (pid {old_pid}). Stop it with: pkill -f 'ah daemon'"
                    );
                }
            }
        }

        fs::write(&pid_path, pid.to_string())?;
        Ok(Self { pid_path, pid })
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        if let Ok(current) = fs::read_to_string(&self.pid_path) {
            if current.trim() == self.pid.to_string() {
                let _ = fs::remove_file(&self.pid_path);
            }
        }
    }
}

fn process_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

pub fn try_grab_lock() -> Option<ExclusiveLock> {
    ExclusiveLock::try_acquire("grab")
}

pub fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    normalize_for_dedup(content).hash(&mut hasher);
    hasher.finish()
}

fn normalize_for_dedup(content: &str) -> String {
    crate::selection::normalize(content)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn is_recent_duplicate(stored_hash: u64, stored_ts: u64, hash: u64, now: u64) -> bool {
    stored_hash == hash && now.saturating_sub(stored_ts) < DEDUP_SECS
}

/// Returns true if this caller should process `content`.
pub fn try_claim(content: &str) -> bool {
    let Some(_guard) = ExclusiveLock::try_acquire("claim-dedup") else {
        return false;
    };

    let hash = content_hash(content);
    let now = now_secs();
    let path = data_dir().join("last_claim");

    if let Ok(record) = fs::read_to_string(&path) {
        let mut lines = record.lines();
        if let (Some(last_hash), Some(last_ts)) = (lines.next(), lines.next()) {
            if let (Ok(stored_hash), Ok(stored_ts)) =
                (last_hash.parse::<u64>(), last_ts.parse::<u64>())
            {
                if is_recent_duplicate(stored_hash, stored_ts, hash, now) {
                    return false;
                }
            }
        }
    }

    fs::write(&path, format!("{hash}\n{now}")).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_recent_duplicate() {
        let hash = 12345_u64;
        assert!(is_recent_duplicate(hash, 100, hash, 102));
        assert!(!is_recent_duplicate(hash, 100, hash, 111));
        assert!(!is_recent_duplicate(hash, 100, 99999, 101));
    }

    #[test]
    fn test_content_hash_stable() {
        assert_eq!(content_hash("map"), content_hash("map"));
        assert_eq!(content_hash(" map "), content_hash("map"));
        assert_ne!(content_hash("map"), content_hash("filter"));
    }
}
