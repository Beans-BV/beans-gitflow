mod common;

use common::{MockGit, MockHosting};
use bflow::flows::finish_work::{finish_release_fix, finish_hotfix_fix};

#[test]
fn finish_release_fix_pushes_and_creates_pr() {
    let mut git = MockGit::new();
    git.current_branch = "release-fix/1.1/login-bug".to_string();
    let hosting = MockHosting::new();

    finish_release_fix(&git, &hosting, 1, 1, "login-bug").unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "push:release-fix/1.1/login-bug",
    ]);

    assert_eq!(hosting.calls(), vec![
        "create_or_get_pr:release-fix/1.1/login-bug:release/1.1:fix: login-bug",
        "open_url:https://github.com/org/repo/pull/1",
    ]);
}

#[test]
fn finish_hotfix_fix_pushes_and_creates_pr() {
    let mut git = MockGit::new();
    git.current_branch = "hotfix-fix/1.0.1/crash-fix".to_string();
    let hosting = MockHosting::new();

    finish_hotfix_fix(&git, &hosting, 1, 0, 1, "crash-fix").unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "push:hotfix-fix/1.0.1/crash-fix",
    ]);

    assert_eq!(hosting.calls(), vec![
        "create_or_get_pr:hotfix-fix/1.0.1/crash-fix:hotfix/1.0.1:fix: crash-fix",
        "open_url:https://github.com/org/repo/pull/1",
    ]);
}

#[test]
fn finish_release_fix_with_custom_pr_url() {
    let mut git = MockGit::new();
    git.current_branch = "release-fix/2.0/typo".to_string();
    let mut hosting = MockHosting::new();
    hosting.pr_url = "https://github.com/org/repo/pull/42".to_string();

    finish_release_fix(&git, &hosting, 2, 0, "typo").unwrap();

    assert_eq!(hosting.calls(), vec![
        "create_or_get_pr:release-fix/2.0/typo:release/2.0:fix: typo",
        "open_url:https://github.com/org/repo/pull/42",
    ]);
}
