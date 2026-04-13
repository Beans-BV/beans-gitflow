#[derive(Debug, PartialEq)]
pub enum BranchType {
    Main,
    Develop,
    Feature { name: String },
    Fix { name: String },
    Chore { name: String },
    Docs { name: String },
    Refactor { name: String },
    Release { major: u32, minor: u32, patch: u32 },
    ReleaseFix { major: u32, minor: u32, patch: u32, name: String },
    Hotfix { major: u32, minor: u32, patch: u32 },
    HotfixFix { major: u32, minor: u32, patch: u32, name: String },
    Other,
}

impl BranchType {
    pub fn parse(branch: &str) -> Self {
        match branch {
            "main" | "master" => return Self::Main,
            "develop" => return Self::Develop,
            _ => {}
        }

        for (prefix, constructor) in [
            ("feature/", Self::new_feature as fn(String) -> Self),
            ("fix/", Self::new_fix as fn(String) -> Self),
            ("chore/", Self::new_chore as fn(String) -> Self),
            ("docs/", Self::new_docs as fn(String) -> Self),
            ("refactor/", Self::new_refactor as fn(String) -> Self),
        ] {
            if let Some(name) = branch.strip_prefix(prefix) {
                if !name.is_empty() {
                    return constructor(name.to_string());
                }
            }
        }

        if let Some(version) = branch.strip_prefix("release/") {
            if let Some((major, minor, patch)) = Self::parse_major_minor_patch(version) {
                return Self::Release { major, minor, patch };
            }
        }

        if let Some(rest) = branch.strip_prefix("release-fix/") {
            if let Some((version, name)) = rest.split_once('/') {
                if let Some((major, minor, patch)) = Self::parse_major_minor_patch(version) {
                    if !name.is_empty() {
                        return Self::ReleaseFix { major, minor, patch, name: name.to_string() };
                    }
                }
            }
        }

        if let Some(version) = branch.strip_prefix("hotfix/") {
            if let Some((major, minor, patch)) = Self::parse_major_minor_patch(version) {
                return Self::Hotfix { major, minor, patch };
            }
        }

        if let Some(rest) = branch.strip_prefix("hotfix-fix/") {
            if let Some((version, name)) = rest.split_once('/') {
                if let Some((major, minor, patch)) = Self::parse_major_minor_patch(version) {
                    if !name.is_empty() {
                        return Self::HotfixFix { major, minor, patch, name: name.to_string() };
                    }
                }
            }
        }

        Self::Other
    }

    pub fn commit_type(&self) -> Option<&'static str> {
        match self {
            Self::Feature { .. } => Some("feat"),
            Self::Fix { .. } => Some("fix"),
            Self::Chore { .. } => Some("chore"),
            Self::Docs { .. } => Some("docs"),
            Self::Refactor { .. } => Some("refactor"),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Feature { name } | Self::Fix { name } | Self::Chore { name }
            | Self::Docs { name } | Self::Refactor { name }
            | Self::ReleaseFix { name, .. } | Self::HotfixFix { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn is_work_branch(&self) -> bool {
        matches!(self, Self::Feature { .. } | Self::Fix { .. } | Self::Chore { .. } | Self::Docs { .. } | Self::Refactor { .. })
    }

    fn new_feature(name: String) -> Self { Self::Feature { name } }
    fn new_fix(name: String) -> Self { Self::Fix { name } }
    fn new_chore(name: String) -> Self { Self::Chore { name } }
    fn new_docs(name: String) -> Self { Self::Docs { name } }
    fn new_refactor(name: String) -> Self { Self::Refactor { name } }

    fn parse_major_minor(s: &str) -> Option<(u32, u32)> {
        let (major, minor) = s.split_once('.')?;
        Some((major.parse().ok()?, minor.parse().ok()?))
    }

    fn parse_major_minor_patch(s: &str) -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        if parts.len() != 3 { return None; }
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
    }
}
