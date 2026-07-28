mod common;

use common::{MockGit, MockHosting};
use bflow::flows::finish_work::{finish_release_fix, finish_hotfix_fix, finish_work_branch};
use bflow::git::branch::BranchType;

#[test]
fn finish_release_fix_pushes_and_creates_pr() {
    let mut git = MockGit::new();
    git.current_branch = "release-fix/1.1.0/login-bug".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::ReleaseFix { major: 1, minor: 1, patch: 0, name: "login-bug".to_string() };

    finish_release_fix(&git, &hosting, &branch_type).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "push:release-fix/1.1.0/login-bug",
    ]);

    assert_eq!(hosting.calls(), vec![
        "create_or_get_pr:release-fix/1.1.0/login-bug:release/1.1.0:fix: login bug",
        "open_url:https://github.com/org/repo/pull/1",
    ]);
}

#[test]
fn finish_hotfix_fix_pushes_and_creates_pr() {
    let mut git = MockGit::new();
    git.current_branch = "hotfix-fix/1.0.1/crash-fix".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::HotfixFix { major: 1, minor: 0, patch: 1, name: "crash-fix".to_string() };

    finish_hotfix_fix(&git, &hosting, &branch_type).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "push:hotfix-fix/1.0.1/crash-fix",
    ]);

    assert_eq!(hosting.calls(), vec![
        "create_or_get_pr:hotfix-fix/1.0.1/crash-fix:hotfix/1.0.1:fix: crash fix",
        "open_url:https://github.com/org/repo/pull/1",
    ]);
}

#[test]
fn finish_release_fix_with_custom_pr_url() {
    let mut git = MockGit::new();
    git.current_branch = "release-fix/2.0.0/typo".to_string();
    let mut hosting = MockHosting::new();
    hosting.pr_url = "https://github.com/org/repo/pull/42".to_string();
    let branch_type = BranchType::ReleaseFix { major: 2, minor: 0, patch: 0, name: "typo".to_string() };

    finish_release_fix(&git, &hosting, &branch_type).unwrap();

    assert_eq!(hosting.calls(), vec![
        "create_or_get_pr:release-fix/2.0.0/typo:release/2.0.0:fix: typo",
        "open_url:https://github.com/org/repo/pull/42",
    ]);
}

// --- finish_work_branch tests ---

#[test]
fn finish_work_branch_feature_non_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    finish_work_branch(&git, &hosting, &branch_type, Some(false), None).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].starts_with("create_or_get_pr:feature/login:"));
    assert!(calls[0].ends_with(":feat: login"));
}

#[test]
fn finish_work_branch_feature_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "feature/remove-api".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "remove-api".to_string() };

    finish_work_branch(&git, &hosting, &branch_type, Some(true), None).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].ends_with(":feat!: remove-api"),
        "Expected PR title to end with 'feat!: remove-api', got: {}", calls[0]);
}

#[test]
fn finish_work_branch_chore_breaking_honored() {
    let mut git = MockGit::new();
    git.current_branch = "chore/drop-node-16".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Chore { name: "drop-node-16".to_string() };

    finish_work_branch(&git, &hosting, &branch_type, Some(true), None).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].ends_with(":chore!: drop-node-16"),
        "Explicit --breaking should be honored on chore, got: {}", calls[0]);
}

#[test]
fn finish_work_branch_docs_defaults_to_non_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "docs/readme".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Docs { name: "readme".to_string() };

    // No flag (None) — docs should NOT prompt, should default to non-breaking
    finish_work_branch(&git, &hosting, &branch_type, None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].ends_with(":docs: readme"),
        "Docs with None should default to non-breaking, got: {}", calls[0]);
}

#[test]
fn finish_work_branch_with_explicit_base_skips_detection() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    git.existing_remote_branches.insert("develop".to_string());
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    finish_work_branch(&git, &hosting, &branch_type, Some(false), Some("develop".to_string())).unwrap();

    let git_calls = git.calls();
    assert!(!git_calls.contains(&"list_remote_branches".to_string()),
        "Explicit --base must skip parent detection, got: {git_calls:?}");
    let calls = hosting.calls();
    assert!(calls[0].starts_with("create_or_get_pr:feature/login:develop:"),
        "PR should target the explicit base, got: {}", calls[0]);
}

#[test]
fn finish_work_branch_with_local_only_base_accepted() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    git.existing_local_branches.insert("feature/auth".to_string());
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    finish_work_branch(&git, &hosting, &branch_type, Some(false), Some("feature/auth".to_string())).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].starts_with("create_or_get_pr:feature/login:feature/auth:"),
        "PR should target the local-only base, got: {}", calls[0]);
}

#[test]
fn finish_work_branch_with_unknown_base_errors() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    let err = finish_work_branch(&git, &hosting, &branch_type, Some(false), Some("no-such-branch".to_string())).unwrap_err();

    assert!(err.contains("no-such-branch"), "Error should name the missing branch, got: {err}");
    assert!(hosting.calls().is_empty(), "No PR should be created for an unknown base");
}

#[test]
fn finish_work_branch_single_candidate_finishes_without_menu() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    // Exactly one candidate parent on the remote; equal rev-list counts keep it.
    git.remote_branches = vec!["develop".to_string()];
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    // Passing proves show_select was never reached: it has no TTY here.
    finish_work_branch(&git, &hosting, &branch_type, Some(false), None).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].starts_with("create_or_get_pr:feature/login:develop:"),
        "Single candidate should be auto-selected, got: {}", calls[0]);
}

#[test]
fn finish_work_branch_fix_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "fix/auth".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Fix { name: "auth".to_string() };

    finish_work_branch(&git, &hosting, &branch_type, Some(true), None).unwrap();

    let calls = hosting.calls();
    assert!(calls[0].ends_with(":fix!: auth"),
        "Expected 'fix!: auth', got: {}", calls[0]);
}
