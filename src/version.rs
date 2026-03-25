use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self { Self { major, minor, patch } }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        if parts.len() != 3 { return None; }
        Some(Self { major: parts[0].parse().ok()?, minor: parts[1].parse().ok()?, patch: parts[2].parse().ok()? })
    }

    pub fn bump_minor(&self) -> Self { Self::new(self.major, self.minor + 1, 0) }
    pub fn bump_patch(&self) -> Self { Self::new(self.major, self.minor, self.patch + 1) }
    pub fn release_branch(&self) -> String { format!("release/{}.{}", self.major, self.minor) }
    pub fn hotfix_branch(&self) -> String { format!("hotfix/{}", self) }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
