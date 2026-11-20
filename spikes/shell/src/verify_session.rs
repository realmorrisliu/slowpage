//! File commands for one explicit verify session.
//!
//! A normal launch has no session. It does not resign the key window for a
//! leftover blur file, and it does not append a status log.

use std::io::Write;
use std::path::{Path, PathBuf};

pub const SESSION_ENV: &str = "SUPERREADER_VERIFY_SESSION";
pub const LOG_CAP_BYTES: u64 = 262_144;
const SPIKE_ROOT: &str = "/tmp/superreader-spike";

pub fn session_id(raw: Option<&str>) -> Option<String> {
    let text = raw?.trim();
    if !(8..=64).contains(&text.len()) {
        return None;
    }
    if !text
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return None;
    }
    Some(text.to_string())
}

pub struct VerifyHooks {
    base: PathBuf,
    root: Option<PathBuf>,
}

impl VerifyHooks {
    pub fn open(raw: Option<&str>, base: &Path) -> Self {
        let root = session_id(raw).map(|id| base.join(format!("verify-{id}")));
        Self {
            base: base.to_path_buf(),
            root,
        }
    }

    pub fn from_env() -> Self {
        let raw = std::env::var(SESSION_ENV).ok();
        Self::open(raw.as_deref(), Path::new(SPIKE_ROOT))
    }

    pub fn scoped(&self, name: &str) -> PathBuf {
        match &self.root {
            Some(root) => root.join(name),
            None => self.base.join(name),
        }
    }

    pub fn should_resign(&self) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| root.join("chrome-blur").is_file())
    }

    pub fn append_status(&self, line: &str) -> bool {
        let Some(root) = &self.root else {
            return false;
        };
        let extra = u64::try_from(line.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        if extra > LOG_CAP_BYTES {
            return false;
        }
        let path = root.join("chrome-status.jsonl");
        let existing = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        if existing.saturating_add(extra) > LOG_CAP_BYTES {
            return false;
        }
        if std::fs::create_dir_all(root).is_err() {
            return false;
        }
        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(file) => file,
            Err(_) => return false,
        };
        writeln!(file, "{line}").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "sr-verify-session-{}-{}-{}",
            std::process::id(),
            label,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn a_plain_launch_ignores_a_leftover_blur_file_and_writes_no_log() {
        let base = scratch("plain");
        std::fs::write(base.join("chrome-blur"), b"1\n").unwrap();
        let plain = VerifyHooks::open(None, &base);
        assert!(!plain.should_resign());
        assert!(!plain.append_status("{\"reading\":true}"));
        assert!(!base.join("chrome-status.jsonl").is_file());
        assert!(base.join("chrome-blur").is_file());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn a_killed_verify_session_does_not_reach_the_next_plain_launch() {
        let base = scratch("killed");
        let killed = VerifyHooks::open(Some("session-aaa1"), &base);
        assert!(killed.append_status("{\"reading\":true}"));
        let blur = killed.scoped("chrome-blur");
        std::fs::write(&blur, b"1\n").unwrap();
        let pin = killed.scoped("shell-frame");
        std::fs::write(&pin, b"578 528\n").unwrap();
        assert!(killed.should_resign());
        let log = killed.scoped("chrome-status.jsonl");
        let before = std::fs::metadata(&log).unwrap().len();

        let plain = VerifyHooks::open(None, &base);
        assert!(!plain.should_resign());
        assert!(!plain.append_status("{\"reading\":true}"));
        assert_eq!(std::fs::metadata(&log).unwrap().len(), before);
        assert_ne!(plain.scoped("shell-frame"), pin);
        assert!(!plain.scoped("shell-frame").is_file());
        assert!(!plain.scoped("chrome-blur").is_file());

        let next = VerifyHooks::open(Some("session-bbb2"), &base);
        assert!(!next.should_resign());
        assert!(next.append_status("{\"reading\":false}"));
        assert_eq!(std::fs::metadata(&log).unwrap().len(), before);
        assert!(!next.scoped("shell-frame").is_file());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn the_status_log_stops_at_the_cap_and_stays_there() {
        let base = scratch("cap");
        let hooks = VerifyHooks::open(Some("session-cap1"), &base);
        let line = "0123456789abcdef";
        let mut wrote = 0u32;
        while hooks.append_status(line) {
            wrote += 1;
            assert!(wrote < 100_000, "the cap did not stop the log");
        }
        assert!(wrote > 0);
        let path = hooks.scoped("chrome-status.jsonl");
        let capped = std::fs::metadata(&path).unwrap().len();
        assert!(capped <= LOG_CAP_BYTES);
        assert!(!hooks.append_status(line));
        assert_eq!(std::fs::metadata(&path).unwrap().len(), capped);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn a_session_id_with_a_path_is_not_a_session() {
        let base = scratch("path");
        let hooks = VerifyHooks::open(Some("../session"), &base);
        assert!(!hooks.should_resign());
        assert!(!hooks.append_status("{\"reading\":true}"));
        assert!(!base.join("chrome-status.jsonl").is_file());
        let _ = std::fs::remove_dir_all(base);
    }
}
