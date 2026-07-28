mod common;

use common::{MockEditor, MockGit};
use bflow::flows::start::{start_work_branch, start_release, start_release_fix, start_hotfix_fix, ReleaseType, detect_breaking_changes};
use bflow::version::SemVer;
use bflow::worktree::{WorktreeConfig, WorktreeContext};

/// Build a worktree config whose base path is an existing temp dir, so the
/// flow's `create_dir_all` is a harmless no-op during tests.
fn test_worktree_config(editor: &str) -> WorktreeConfig {
    let base = std::env::temp_dir();
    WorktreeConfig {
        enabled: true,
        editor: editor.to_string(),
        base_path: Some(base.to_string_lossy().to_string()),
    }
}

#[test]
fn start_work_branch_creates_and_pushes() {
    let git = MockGit::new();
    start_work_branch(&git, "feature", "login-page", "develop", false, None).unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch:feature/login-page:develop",
        "push:feature/login-page",
    ]);
}

#[test]
fn start_work_branch_with_fix_prefix() {
    let git = MockGit::new();
    start_work_branch(&git, "fix", "broken-auth", "main", false, None).unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch:fix/broken-auth:main",
        "push:fix/broken-auth",
    ]);
}

#[test]
fn start_release_creates_new_when_no_release_exists_with_tags() {
    let mut git = MockGit::new();
    git.branches_matching = vec![]; // no existing release branches
    git.tags = vec!["v1.0.0".to_string()];

    start_release(&git, Some(ReleaseType::Minor)).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/1.1.0:develop",
        "push:release/1.1.0",
        "create_tag:v1.1.0-rc.1:chore: create release branch 1.1.0",
        "push_tag:v1.1.0-rc.1",
    ]);
}

#[test]
fn start_release_creates_new_when_no_release_exists_no_tags() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec![];

    start_release(&git, Some(ReleaseType::Minor)).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/0.1.0:develop",
        "push:release/0.1.0",
        "create_tag:v0.1.0-rc.1:chore: create release branch 0.1.0",
        "push_tag:v0.1.0-rc.1",
    ]);
}

#[test]
fn start_release_checks_out_existing_release_branch() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["release/1.1.0".to_string()];

    start_release(&git, Some(ReleaseType::Minor)).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "checkout:release/1.1.0",
    ]);
}

#[test]
fn start_release_fix_creates_and_pushes() {
    let mut git = MockGit::new();
    git.current_branch = "release/1.2.0".to_string();

    start_release_fix(&git, "broken-login", false, None).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "create_branch:release-fix/1.2.0/broken-login:release/1.2.0",
        "push:release-fix/1.2.0/broken-login",
    ]);
}

#[test]
fn start_hotfix_fix_creates_and_pushes_existing_hotfix() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["hotfix/1.0.1".to_string()];

    start_hotfix_fix(&git, "urgent-crash", false, None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:hotfix/*",
        "checkout:hotfix/1.0.1",
        "create_branch:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
        "push:hotfix-fix/1.0.1/urgent-crash",
    ]);
}

#[test]
fn start_hotfix_fix_creates_hotfix_branch_when_none_exists() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec!["1.0.0".to_string()];

    start_hotfix_fix(&git, "urgent-crash", false, None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:hotfix/*",
        "list_tags",
        "checkout:main",
        "create_branch:hotfix/1.0.1:main",
        "push:hotfix/1.0.1",
        "create_branch:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
        "push:hotfix-fix/1.0.1/urgent-crash",
    ]);
}

#[test]
fn start_work_branch_no_checkout_creates_without_switching() {
    let git = MockGit::new();
    start_work_branch(&git, "feature", "login-page", "develop", true, None).unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch_no_checkout:feature/login-page:develop",
        "push:feature/login-page",
    ]);
}

#[test]
fn start_release_fix_no_checkout_discovers_release_branch() {
    let mut git = MockGit::new();
    git.current_branch = "develop".to_string();
    git.branches_matching = vec!["release/1.2.0".to_string()];

    start_release_fix(&git, "broken-login", true, None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "create_branch_no_checkout:release-fix/1.2.0/broken-login:release/1.2.0",
        "push:release-fix/1.2.0/broken-login",
    ]);
}

#[test]
fn start_release_fix_no_checkout_errors_when_no_release_branch() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];

    let result = start_release_fix(&git, "broken-login", true, None);
    assert!(result.is_err());
}

#[test]
fn start_hotfix_fix_no_checkout_existing_hotfix() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["hotfix/1.0.1".to_string()];

    start_hotfix_fix(&git, "urgent-crash", true, None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:hotfix/*",
        "create_branch_no_checkout:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
        "push:hotfix-fix/1.0.1/urgent-crash",
    ]);
}

#[test]
fn start_hotfix_fix_no_checkout_creates_hotfix_branch_when_none_exists() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec!["1.0.0".to_string()];

    start_hotfix_fix(&git, "urgent-crash", true, None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:hotfix/*",
        "list_tags",
        "create_branch_no_checkout:hotfix/1.0.1:main",
        "push:hotfix/1.0.1",
        "create_branch_no_checkout:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
        "push:hotfix-fix/1.0.1/urgent-crash",
    ]);
}

#[test]
fn start_release_falls_back_to_rc_tags_when_no_clean_tags() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec!["v1.1.0-rc.1".to_string(), "v1.1.0-rc.2".to_string()];

    start_release(&git, Some(ReleaseType::Minor)).unwrap();

    // Should use 1.1.0 (from RC tags) as base, bump to 1.2
    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/1.2.0:develop",
        "push:release/1.2.0",
        "create_tag:v1.2.0-rc.1:chore: create release branch 1.2.0",
        "push_tag:v1.2.0-rc.1",
    ]);
}

#[test]
fn start_release_major_bumps_major_version() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec!["v1.5.0".to_string()];

    start_release(&git, Some(ReleaseType::Major)).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/2.0.0:develop",
        "push:release/2.0.0",
        "create_tag:v2.0.0-rc.1:chore: create release branch 2.0.0",
        "push_tag:v2.0.0-rc.1",
    ]);
}

#[test]
fn start_release_ignores_rc_tags_when_determining_next_version() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec!["v1.0.0".to_string(), "v1.1.0-rc.1".to_string(), "v1.1.0-rc.2".to_string()];

    start_release(&git, Some(ReleaseType::Minor)).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/1.1.0:develop",
        "push:release/1.1.0",
        "create_tag:v1.1.0-rc.1:chore: create release branch 1.1.0",
        "push_tag:v1.1.0-rc.1",
    ]);
}

// --- Worktree flow tests ---

#[test]
fn start_work_branch_worktree_active_forces_no_checkout_and_opens() {
    let git = MockGit::new(); // repo_root default "/repos/beans-gitflow"
    let config = test_worktree_config("code");
    let editor = MockEditor::new();
    let ctx = WorktreeContext { config: &config, editor: &editor };

    // Pass no_checkout=false: worktree mode must still force the no-checkout path.
    start_work_branch(&git, "feature", "login-page", "develop", false, Some(ctx)).unwrap();

    let expected = std::env::temp_dir().join("beans-gitflow-feature-login-page");
    let expected = expected.display().to_string();
    assert_eq!(git.calls(), vec![
        "create_branch_no_checkout:feature/login-page:develop".to_string(),
        "push:feature/login-page".to_string(),
        "repo_root".to_string(),
        format!("add_worktree:{expected}:feature/login-page"),
    ]);
    assert_eq!(editor.calls(), vec![format!("open:{expected}")]);
}

#[test]
fn start_work_branch_without_worktree_uses_legacy_sequence() {
    let git = MockGit::new();
    start_work_branch(&git, "feature", "login-page", "develop", false, None).unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch:feature/login-page:develop",
        "push:feature/login-page",
    ]);
}

#[test]
fn start_work_branch_worktree_editor_failure_is_not_fatal() {
    let git = MockGit::new();
    let config = test_worktree_config("code");
    let mut editor = MockEditor::new();
    editor.fail = true;
    let ctx = WorktreeContext { config: &config, editor: &editor };

    let result = start_work_branch(&git, "feature", "login-page", "develop", false, Some(ctx));
    assert!(result.is_ok(), "editor failure should be a warning, not fatal");
    assert!(git.calls().iter().any(|c| c.starts_with("add_worktree:")), "worktree should still be created");
}

#[test]
fn start_work_branch_worktree_editor_none_skips_open() {
    let git = MockGit::new();
    let config = test_worktree_config("none");
    let editor = MockEditor::new();
    let ctx = WorktreeContext { config: &config, editor: &editor };

    start_work_branch(&git, "feature", "login-page", "develop", false, Some(ctx)).unwrap();

    assert!(editor.calls().is_empty(), "editor 'none' should not open anything");
    assert!(git.calls().iter().any(|c| c.starts_with("add_worktree:")), "worktree should still be created");
}

#[test]
fn start_release_fix_worktree_active_discovers_and_opens() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["release/1.2.0".to_string()];
    let config = test_worktree_config("code");
    let editor = MockEditor::new();
    let ctx = WorktreeContext { config: &config, editor: &editor };

    // worktree mode forces the no-checkout discovery path even from develop.
    start_release_fix(&git, "broken-login", false, Some(ctx)).unwrap();

    let expected = std::env::temp_dir().join("beans-gitflow-release-fix-1.2.0-broken-login");
    let expected = expected.display().to_string();
    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*".to_string(),
        "create_branch_no_checkout:release-fix/1.2.0/broken-login:release/1.2.0".to_string(),
        "push:release-fix/1.2.0/broken-login".to_string(),
        "repo_root".to_string(),
        format!("add_worktree:{expected}:release-fix/1.2.0/broken-login"),
    ]);
    assert_eq!(editor.calls(), vec![format!("open:{expected}")]);
}

// Note: the pure `message_is_breaking` string-matching logic is tested
// as unit tests in src/flows/start.rs. These integration tests cover the
// git interaction — which ref is queried, and the develop → origin/develop
// fallback.

#[test]
fn detect_breaking_queries_develop_not_head() {
    let mut git = MockGit::new();
    git.commit_messages = vec!["feat!: remove API".to_string()];

    let result = detect_breaking_changes(&git, &SemVer::new(1, 0, 0));

    assert!(result);
    // Must query develop, not HEAD — so start release works from any branch
    assert!(git.calls().iter().any(|c| c == "commit_messages:v1.0.0:develop"),
        "Expected commit_messages to be called with 'develop', got: {:?}", git.calls());
}

#[test]
fn detect_breaking_falls_back_to_origin_develop_when_develop_missing() {
    let mut git = MockGit::new();
    // Simulate a fresh clone / CI environment where local 'develop' doesn't exist
    git.fail_commit_messages_for = vec!["develop".to_string()];
    git.commit_messages = vec!["feat!: remove API".to_string()];

    let result = detect_breaking_changes(&git, &SemVer::new(1, 0, 0));

    assert!(result, "Fallback to origin/develop should detect the breaking change");
    assert_eq!(git.calls(), vec![
        "commit_messages:v1.0.0:develop",         // first attempt
        "commit_messages:v1.0.0:origin/develop",  // fallback
    ]);
}

#[test]
fn detect_breaking_returns_false_when_neither_develop_nor_origin_exist() {
    let mut git = MockGit::new();
    git.fail_commit_messages_for = vec!["develop".to_string(), "origin/develop".to_string()];

    let result = detect_breaking_changes(&git, &SemVer::new(1, 0, 0));

    assert!(!result, "Should return false when no refs are accessible");
    assert_eq!(git.calls(), vec![
        "commit_messages:v1.0.0:develop",
        "commit_messages:v1.0.0:origin/develop",
    ]);
}
