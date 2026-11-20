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
    root: Option<PathBuf>,
}

impl VerifyHooks {
    pub fn open(raw: Option<&str>, base: &Path) -> Self {
        let root = session_id(raw).map(|id| base.join(format!("verify-{id}")));
        Self { root }
    }

    pub fn from_env() -> Self {
        let raw = std::env::var(SESSION_ENV).ok();
        Self::open(raw.as_deref(), Path::new(SPIKE_ROOT))
    }

    /// A hook path exists only for a valid session. No session has no path.
    pub fn command(&self, name: &str) -> Option<PathBuf> {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
            return None;
        }
        self.root.as_ref().map(|root| root.join(name))
    }

    pub fn read_text(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.command(name)?).ok()
    }

    pub fn should_resign(&self) -> bool {
        self.command("chrome-blur")
            .is_some_and(|path| path.is_file())
    }

    pub fn append_status(&self, line: &str) -> bool {
        let Some(path) = self.command("chrome-status.jsonl") else {
            return false;
        };
        let Some(root) = path.parent() else {
            return false;
        };
        let extra = u64::try_from(line.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        if extra > LOG_CAP_BYTES {
            return false;
        }
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
        let blur = killed.command("chrome-blur").unwrap();
        std::fs::write(&blur, b"1\n").unwrap();
        let pin = killed.command("shell-frame").unwrap();
        std::fs::write(&pin, b"578 528\n").unwrap();
        assert!(killed.should_resign());
        let log = killed.command("chrome-status.jsonl").unwrap();
        let before = std::fs::metadata(&log).unwrap().len();

        let plain = VerifyHooks::open(None, &base);
        assert!(!plain.should_resign());
        assert!(!plain.append_status("{\"reading\":true}"));
        assert_eq!(std::fs::metadata(&log).unwrap().len(), before);
        assert!(plain.command("shell-frame").is_none());
        assert!(plain.read_text("shell-frame").is_none());

        let next = VerifyHooks::open(Some("session-bbb2"), &base);
        assert!(!next.should_resign());
        assert!(next.append_status("{\"reading\":false}"));
        assert_eq!(std::fs::metadata(&log).unwrap().len(), before);
        assert!(next.read_text("shell-frame").is_none());
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
        let path = hooks.command("chrome-status.jsonl").unwrap();
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

    #[test]
    fn a_root_control_file_does_not_steer_a_launch_without_a_session() {
        let base = scratch("root");
        std::fs::write(base.join("shell-frame"), b"1100 800\n").unwrap();
        std::fs::write(base.join("bar-fullscreen"), b"1\n").unwrap();
        std::fs::write(base.join("chrome-blur"), b"1\n").unwrap();
        for raw in [
            None,
            Some(""),
            Some("short"),
            Some("../session"),
            Some("1"),
            Some("bad id"),
        ] {
            let hooks = VerifyHooks::open(raw, &base);
            assert!(hooks.command("shell-frame").is_none());
            assert!(hooks.read_text("shell-frame").is_none());
            assert!(hooks.read_text("bar-fullscreen").is_none());
            assert!(!hooks.should_resign());
            assert!(!hooks.append_status("{\"reading\":true}"));
        }
        assert_eq!(
            std::fs::read_to_string(base.join("shell-frame")).unwrap(),
            "1100 800\n"
        );
        assert!(!base.join("chrome-status.jsonl").is_file());

        let session = VerifyHooks::open(Some("session-own1"), &base);
        let frame = session.command("shell-frame").unwrap();
        let full = session.command("bar-fullscreen").unwrap();
        assert_ne!(frame, base.join("shell-frame"));
        assert_ne!(full, base.join("bar-fullscreen"));
        std::fs::create_dir_all(frame.parent().unwrap()).unwrap();
        std::fs::write(&frame, b"900 700\n").unwrap();
        std::fs::write(&full, b"0\n").unwrap();
        assert_eq!(
            session.read_text("shell-frame").as_deref(),
            Some("900 700\n")
        );
        assert_eq!(session.read_text("bar-fullscreen").as_deref(), Some("0\n"));
        assert_eq!(
            std::fs::read_to_string(base.join("shell-frame")).unwrap(),
            "1100 800\n"
        );
        assert_eq!(
            std::fs::read_to_string(base.join("bar-fullscreen")).unwrap(),
            "1\n"
        );
        let _ = std::fs::remove_dir_all(base);
    }
}
