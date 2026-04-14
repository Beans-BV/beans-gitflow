mod common;

use common::MockGit;
use bflow::flows::finish_release::{bump_version, sync_with_develop, finish_release};

#[test]
fn bump_version_increments_rc() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string()];

    bump_version(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "create_tag:v1.1.0-rc.2:chore: bump version to v1.1.0-rc.2",
        "push_tag:v1.1.0-rc.2",
    ]);
}

#[test]
fn bump_version_multiple_rcs() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string(), "v1.1.0-rc.2".to_string()];

    bump_version(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "create_tag:v1.1.0-rc.3:chore: bump version to v1.1.0-rc.3",
        "push_tag:v1.1.0-rc.3",
    ]);
}

#[test]
fn bump_version_ignores_mismatched_patch_tags() {
    let mut git = MockGit::new();
    // Simulate a stray v1.1.1 tag reachable from the release branch
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string(), "v1.1.1".to_string()];

    bump_version(&git, 1, 1).unwrap();

    // Should bump from rc.1, ignoring the v1.1.1 tag (wrong patch)
    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "create_tag:v1.1.0-rc.2:chore: bump version to v1.1.0-rc.2",
        "push_tag:v1.1.0-rc.2",
    ]);
}

#[test]
fn bump_version_ignores_non_rc_pre_release_tags() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string(), "v1.1.0-beta.5".to_string()];

    bump_version(&git, 1, 1).unwrap();

    // Should bump from rc.1, ignoring the beta tag
    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "create_tag:v1.1.0-rc.2:chore: bump version to v1.1.0-rc.2",
        "push_tag:v1.1.0-rc.2",
    ]);
}

#[test]
fn bump_version_errors_when_no_matching_tags() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec![];

    let result = bump_version(&git, 1, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No RC tags found"));
}

#[test]
fn sync_with_develop_merges_and_returns_to_current() {
    let mut git = MockGit::new();
    git.current_branch = "release/1.1.0".to_string();

    sync_with_develop(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "checkout:develop",
        "pull:origin/develop",
        "merge:release/1.1.0:chore: sync release 1.1.0 with develop",
        "push:develop",
        "checkout:release/1.1.0",
    ]);
}

#[test]
fn finish_release_creates_clean_tag_from_rc() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string(), "v1.1.0-rc.2".to_string()];
    git.rev_list_count_result = 0;

    finish_release(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "rev_list_count:v1.1.0-rc.2:release/1.1.0",
        "checkout:main",
        "pull:origin/main",
        "merge:release/1.1.0:chore: merge release 1.1.0 into main",
        "create_tag:v1.1.0:chore: release 1.1.0",
        "push:main",
        "push_tag:v1.1.0",
        "checkout:develop",
        "pull:origin/develop",
        "merge:release/1.1.0:chore: merge release 1.1.0 into develop",
        "push:develop",
        "delete_branch_local:release/1.1.0",
        "delete_branch_remote:release/1.1.0",
    ]);
}

#[test]
fn finish_release_single_rc() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v2.0.0-rc.1".to_string()];
    git.rev_list_count_result = 0;

    finish_release(&git, 2, 0).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/2.0.0",
        "rev_list_count:v2.0.0-rc.1:release/2.0.0",
        "checkout:main",
        "pull:origin/main",
        "merge:release/2.0.0:chore: merge release 2.0.0 into main",
        "create_tag:v2.0.0:chore: release 2.0.0",
        "push:main",
        "push_tag:v2.0.0",
        "checkout:develop",
        "pull:origin/develop",
        "merge:release/2.0.0:chore: merge release 2.0.0 into develop",
        "push:develop",
        "delete_branch_local:release/2.0.0",
        "delete_branch_remote:release/2.0.0",
    ]);
}

#[test]
fn finish_release_fails_when_head_past_latest_rc() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string(), "v1.1.0-rc.2".to_string()];
    git.rev_list_count_result = 2; // 2 commits on release/1.1.0 past v1.1.0-rc.2

    let result = finish_release(&git, 1, 1);

    assert!(result.is_err(), "expected guard to reject finish when HEAD is past latest RC");
    let err = result.unwrap_err();
    assert!(err.contains("v1.1.0-rc.2"), "error should name the latest RC tag; got: {err}");
    assert!(err.contains("bflow bump"), "error should tell user to bump; got: {err}");
    assert!(err.contains("2 commit"), "error should state how many commits past the RC; got: {err}");

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("checkout:main")),
        "guard must abort before touching main; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("create_tag:")),
        "guard must abort before tagging; calls: {calls:?}");
}

#[test]
fn finish_release_error_message_uses_singular_for_one_commit() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string()];
    git.rev_list_count_result = 1;

    let err = finish_release(&git, 1, 1).unwrap_err();
    assert!(err.contains("1 commit past"), "expected singular 'commit'; got: {err}");
    assert!(!err.contains("1 commits"), "should not use plural for 1; got: {err}");
}
