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

    /// Branch types whose finish has a fixed merge/PR target, so `--base` never applies.
    pub fn has_fixed_finish_target(&self) -> bool {
        matches!(self, Self::Release { .. } | Self::ReleaseFix { .. } | Self::Hotfix { .. } | Self::HotfixFix { .. })
    }

    /// PR-template lookup keys as `(specific, group)` for branch types that open a PR.
    /// The fix family (`fix`, `release-fix`, `hotfix-fix`) shares the `fix` group; for
    /// every other type the group equals the specific key. Returns `None` for branches
    /// that never open a PR (main, develop, release, hotfix, other).
    pub fn pr_template_keys(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Feature { .. } => Some(("feature", "feature")),
            Self::Fix { .. } => Some(("fix", "fix")),
            Self::Chore { .. } => Some(("chore", "chore")),
            Self::Docs { .. } => Some(("docs", "docs")),
            Self::Refactor { .. } => Some(("refactor", "refactor")),
            Self::ReleaseFix { .. } => Some(("release-fix", "fix")),
            Self::HotfixFix { .. } => Some(("hotfix-fix", "fix")),
            _ => None,
        }
    }

    fn new_feature(name: String) -> Self { Self::Feature { name } }
    fn new_fix(name: String) -> Self { Self::Fix { name } }
    fn new_chore(name: String) -> Self { Self::Chore { name } }
    fn new_docs(name: String) -> Self { Self::Docs { name } }
    fn new_refactor(name: String) -> Self { Self::Refactor { name } }

    fn parse_major_minor_patch(s: &str) -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        if parts.len() != 3 { return None; }
        Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_template_keys_for_work_branches() {
        assert_eq!(BranchType::parse("feature/x").pr_template_keys(), Some(("feature", "feature")));
        assert_eq!(BranchType::parse("fix/x").pr_template_keys(), Some(("fix", "fix")));
        assert_eq!(BranchType::parse("chore/x").pr_template_keys(), Some(("chore", "chore")));
        assert_eq!(BranchType::parse("docs/x").pr_template_keys(), Some(("docs", "docs")));
        assert_eq!(BranchType::parse("refactor/x").pr_template_keys(), Some(("refactor", "refactor")));
    }

    #[test]
    fn fix_family_shares_fix_group() {
        assert_eq!(BranchType::parse("release-fix/1.2.0/x").pr_template_keys(), Some(("release-fix", "fix")));
        assert_eq!(BranchType::parse("hotfix-fix/1.2.0/x").pr_template_keys(), Some(("hotfix-fix", "fix")));
    }

    #[test]
    fn fixed_finish_target_only_for_release_and_hotfix_families() {
        assert!(BranchType::parse("release/1.2.0").has_fixed_finish_target());
        assert!(BranchType::parse("release-fix/1.2.0/x").has_fixed_finish_target());
        assert!(BranchType::parse("hotfix/1.2.1").has_fixed_finish_target());
        assert!(BranchType::parse("hotfix-fix/1.2.1/x").has_fixed_finish_target());
        assert!(!BranchType::parse("feature/x").has_fixed_finish_target());
        assert!(!BranchType::parse("develop").has_fixed_finish_target());
        assert!(!BranchType::parse("main").has_fixed_finish_target());
    }

    #[test]
    fn pr_template_keys_none_for_non_pr_branches() {
        assert_eq!(BranchType::parse("main").pr_template_keys(), None);
        assert_eq!(BranchType::parse("develop").pr_template_keys(), None);
        assert_eq!(BranchType::parse("release/1.2.0").pr_template_keys(), None);
        assert_eq!(BranchType::parse("hotfix/1.2.0").pr_template_keys(), None);
        assert_eq!(BranchType::parse("whatever").pr_template_keys(), None);
    }
}
