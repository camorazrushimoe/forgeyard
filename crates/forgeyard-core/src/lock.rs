use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;

use crate::error::{ForgeError, Result};

const LOCK_EX: i32 = 2;
const LOCK_NB: i32 = 4;
const LOCK_UN: i32 = 8;

extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

/// Advisory exclusive lock on `path` (created if missing).
#[derive(Debug)]
pub struct FileLock {
    file: File,
}

impl FileLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;
        let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX) };
        if rc != 0 {
            return Err(ForgeError::Io(std::io::Error::last_os_error()));
        }
        Ok(FileLock { file })
    }

    pub fn try_acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;
        let rc = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
        if rc != 0 {
            return Err(ForgeError::Busy(format!(
                "lock busy: {}",
                path.display()
            )));
        }
        Ok(FileLock { file })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        unsafe {
            let _ = flock(self.file.as_raw_fd(), LOCK_UN);
        }
    }
}

pub fn state_lock_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("state.json.lock")
}

pub fn events_lock_path(project_dir: &Path) -> std::path::PathBuf {
    project_dir.join("events.jsonl.lock")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("fy-lock-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn exclusive_try_acquire_fails_while_held() {
        let dir = tmp();
        let path = dir.join("x.lock");
        let held = FileLock::acquire(&path).unwrap();
        let err = FileLock::try_acquire(&path).unwrap_err();
        assert_eq!(err.exit(), crate::Exit::Busy);
        drop(held);
        assert!(FileLock::try_acquire(&path).is_ok());
    }
}
