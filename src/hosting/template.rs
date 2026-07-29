//! Branch-aware PR template resolution.
//!
//! bflow looks for templates in `.github/pr-templates/` named `bflow-<key>.md` and
//! resolves them most-specific first:
//!   1. `bflow-<specific>.md`  (e.g. `bflow-release-fix.md`)
//!   2. `bflow-<group>.md`     (e.g. `bflow-fix.md` for the fix family)
//!   3. `bflow-default.md`
//!
//! When none of these exist this returns `None` and the hosting layer falls back to the
//! repository's git/GitHub default template (or an empty body).

use std::path::{Path, PathBuf};
use crate::git::branch::BranchType;

const DIR: &str = ".github/pr-templates";

/// Resolve the PR template for `branch_type` against the conventional location
/// inside `repo_root`. Anchoring to the repo root (not the process CWD) keeps
/// resolution working from subdirectories; called from the composition root so
/// flows never probe the filesystem themselves.
pub fn resolve(repo_root: &Path, branch_type: &BranchType) -> Option<PathBuf> {
    resolve_in(&repo_root.join(DIR), branch_type)
}

/// Resolution against an explicit directory — kept separate so tests can point at a
/// scratch dir without touching the process working directory.
fn resolve_in(dir: &Path, branch_type: &BranchType) -> Option<PathBuf> {
    let (specific, group) = branch_type.pr_template_keys()?;
    let mut keys = vec![specific];
    if group != specific {
        keys.push(group);
    }
    keys.push("default");
    keys.into_iter()
        .map(|k| dir.join(format!("bflow-{k}.md")))
        .find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir() -> PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = env::temp_dir().join(format!("bflow-template-test-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(dir: &Path, name: &str) {
        fs::write(dir.join(name), "body").unwrap();
    }

    #[test]
    fn resolve_is_anchored_to_the_repo_root() {
        let root = tmp_dir();
        let dir = root.join(".github/pr-templates");
        fs::create_dir_all(&dir).unwrap();
        touch(&dir, "bflow-default.md");
        let bt = BranchType::parse("feature/foo");
        assert_eq!(resolve(&root, &bt), Some(dir.join("bflow-default.md")));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn specific_wins_over_group_and_default() {
        let dir = tmp_dir();
        touch(&dir, "bflow-release-fix.md");
        touch(&dir, "bflow-fix.md");
        touch(&dir, "bflow-default.md");
        let bt = BranchType::parse("release-fix/1.2.0/foo");
        assert_eq!(resolve_in(&dir, &bt), Some(dir.join("bflow-release-fix.md")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn group_wins_over_default_when_no_specific() {
        let dir = tmp_dir();
        touch(&dir, "bflow-fix.md");
        touch(&dir, "bflow-default.md");
        // release-fix has no specific file, so it falls back to the fix group.
        let bt = BranchType::parse("release-fix/1.2.0/foo");
        assert_eq!(resolve_in(&dir, &bt), Some(dir.join("bflow-fix.md")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fix_family_maps_to_fix_group() {
        let dir = tmp_dir();
        touch(&dir, "bflow-fix.md");
        for branch in ["fix/foo", "release-fix/1.2.0/foo", "hotfix-fix/1.2.0/foo"] {
            let bt = BranchType::parse(branch);
            assert_eq!(resolve_in(&dir, &bt), Some(dir.join("bflow-fix.md")), "branch={branch}");
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn falls_back_to_default() {
        let dir = tmp_dir();
        touch(&dir, "bflow-default.md");
        let bt = BranchType::parse("feature/foo");
        assert_eq!(resolve_in(&dir, &bt), Some(dir.join("bflow-default.md")));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn none_when_no_files() {
        let dir = tmp_dir();
        let bt = BranchType::parse("feature/foo");
        assert_eq!(resolve_in(&dir, &bt), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn none_for_non_pr_branch_even_with_default() {
        let dir = tmp_dir();
        touch(&dir, "bflow-default.md");
        // release branches never open a PR — no template key, so no resolution.
        let bt = BranchType::parse("release/1.2.0");
        assert_eq!(resolve_in(&dir, &bt), None);
        fs::remove_dir_all(&dir).ok();
    }
}
