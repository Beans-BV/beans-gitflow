use std::fs;
use std::path::{Path, PathBuf};

pub const STATE_FILE_NAME: &str = "bflow-finish.state";
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishKind {
    Release,
    Hotfix,
}

impl FinishKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Hotfix => "hotfix",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "release" => Some(Self::Release),
            "hotfix" => Some(Self::Hotfix),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishState {
    pub kind: FinishKind,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub started_at: String,
    pub stash_ref: Option<String>,
}

impl FinishState {
    pub fn source_branch(&self) -> String {
        match self.kind {
            FinishKind::Release => format!("release/{}.{}.{}", self.major, self.minor, self.patch),
            FinishKind::Hotfix => format!("hotfix/{}.{}.{}", self.major, self.minor, self.patch),
        }
    }

    pub fn path(git_dir: &Path) -> PathBuf {
        git_dir.join(STATE_FILE_NAME)
    }

    pub fn load(git_dir: &Path) -> Result<Option<Self>, String> {
        let path = Self::path(git_dir);
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        Self::parse(&contents).map(Some)
    }

    pub fn save(&self, git_dir: &Path) -> Result<(), String> {
        let path = Self::path(git_dir);
        fs::write(&path, self.serialize())
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
    }

    pub fn clear(git_dir: &Path) -> Result<(), String> {
        let path = Self::path(git_dir);
        if !path.exists() {
            return Ok(());
        }
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))
    }

    fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("version={}\n", SCHEMA_VERSION));
        out.push_str(&format!("kind={}\n", self.kind.as_str()));
        out.push_str(&format!("major={}\n", self.major));
        out.push_str(&format!("minor={}\n", self.minor));
        out.push_str(&format!("patch={}\n", self.patch));
        out.push_str(&format!("started_at={}\n", self.started_at));
        if let Some(stash_ref) = &self.stash_ref {
            out.push_str(&format!("stash_ref={stash_ref}\n"));
        }
        out
    }

    fn parse(contents: &str) -> Result<Self, String> {
        let mut version: Option<u32> = None;
        let mut kind: Option<FinishKind> = None;
        let mut major: Option<u32> = None;
        let mut minor: Option<u32> = None;
        let mut patch: Option<u32> = None;
        let mut started_at: Option<String> = None;
        let mut stash_ref: Option<String> = None;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')
                .ok_or_else(|| format!("Malformed state line: {line}"))?;
            match key.trim() {
                "version" => version = value.trim().parse().ok(),
                "kind" => kind = FinishKind::parse(value.trim()),
                "major" => major = value.trim().parse().ok(),
                "minor" => minor = value.trim().parse().ok(),
                "patch" => patch = value.trim().parse().ok(),
                "started_at" => started_at = Some(value.trim().to_string()),
                "stash_ref" => stash_ref = Some(value.trim().to_string()),
                _ => {}
            }
        }

        let version = version.ok_or("Missing version in state file")?;
        if version != SCHEMA_VERSION {
            return Err(format!(
                "Unsupported state file version {version} (expected {SCHEMA_VERSION}). \
                 Run 'bflow finish --abort' to discard."
            ));
        }
        Ok(Self {
            kind: kind.ok_or("Missing/invalid kind in state file")?,
            major: major.ok_or("Missing major in state file")?,
            minor: minor.ok_or("Missing minor in state file")?,
            patch: patch.ok_or("Missing patch in state file")?,
            started_at: started_at.ok_or("Missing started_at in state file")?,
            stash_ref,
        })
    }
}

pub fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir() -> PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = env::temp_dir().join(format!(
            "bflow-state-test-{}-{}-{n}",
            std::process::id(),
            current_timestamp(),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips_release_state() {
        let dir = tmp_dir();
        let s = FinishState {
            kind: FinishKind::Release,
            major: 1, minor: 3, patch: 0,
            started_at: "1234".to_string(),
            stash_ref: None,
        };
        s.save(&dir).unwrap();
        let loaded = FinishState::load(&dir).unwrap().unwrap();
        assert_eq!(loaded, s);
        assert_eq!(loaded.source_branch(), "release/1.3.0");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn round_trips_hotfix_state_with_stash() {
        let dir = tmp_dir();
        let s = FinishState {
            kind: FinishKind::Hotfix,
            major: 2, minor: 0, patch: 4,
            started_at: "5678".to_string(),
            stash_ref: Some("stash@{0}".to_string()),
        };
        s.save(&dir).unwrap();
        let loaded = FinishState::load(&dir).unwrap().unwrap();
        assert_eq!(loaded, s);
        assert_eq!(loaded.source_branch(), "hotfix/2.0.4");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_returns_none_when_missing() {
        let dir = tmp_dir();
        assert_eq!(FinishState::load(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clear_is_idempotent() {
        let dir = tmp_dir();
        FinishState::clear(&dir).unwrap();
        FinishState::clear(&dir).unwrap();
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let dir = tmp_dir();
        let path = FinishState::path(&dir);
        fs::write(&path, "version=99\nkind=release\nmajor=1\nminor=0\npatch=0\nstarted_at=0\n").unwrap();
        let err = FinishState::load(&dir).unwrap_err();
        assert!(err.contains("Unsupported state file version"));
        fs::remove_dir_all(&dir).ok();
    }
}
