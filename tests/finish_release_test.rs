mod common;

use common::MockGit;
use bflow::flows::finish_release::{bump_version, sync_with_develop, finish_release};

#[test]
fn bump_version_finds_latest_and_bumps_patch() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["1.1.0".to_string(), "1.1.1".to_string()];

    bump_version(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1",
        "create_tag:1.1.2:chore: bump version to 1.1.2",
        "push_tag:1.1.2",
    ]);
}

#[test]
fn bump_version_single_tag() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["2.0.0".to_string()];

    bump_version(&git, 2, 0).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/2.0",
        "create_tag:2.0.1:chore: bump version to 2.0.1",
        "push_tag:2.0.1",
    ]);
}

#[test]
fn bump_version_errors_when_no_matching_tags() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec![];

    let result = bump_version(&git, 1, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No tags found"));
}

#[test]
fn sync_with_develop_merges_and_returns_to_current() {
    let mut git = MockGit::new();
    git.current_branch = "release/1.1".to_string();

    sync_with_develop(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "checkout:develop",
        "pull:origin/develop",
        "merge:release/1.1:chore: sync release 1.1 with develop",
        "push:develop",
        "checkout:release/1.1",
    ]);
}

#[test]
fn finish_release_no_commits_since_last_tag() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["1.1.0".to_string(), "1.1.1".to_string()];
    git.rev_list_count_result = 0; // no commits since last tag

    finish_release(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1",
        "rev_list_count:1.1.1:release/1.1",
        "checkout:main",
        "pull:origin/main",
        "merge:release/1.1:chore: finish release 1.1",
        "push:main",
        "checkout:develop",
        "pull:origin/develop",
        "merge:release/1.1:chore: merge release 1.1 into develop",
        "push:develop",
        "delete_branch_local:release/1.1",
        "delete_branch_remote:release/1.1",
    ]);
}

#[test]
fn finish_release_with_commits_since_last_tag_auto_bumps() {
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["1.1.0".to_string()];
    git.rev_list_count_result = 3; // commits since last tag

    finish_release(&git, 1, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1",
        "rev_list_count:1.1.0:release/1.1",
        "create_tag:1.1.1:chore: bump version to 1.1.1",
        "push_tag:1.1.1",
        "checkout:main",
        "pull:origin/main",
        "merge:release/1.1:chore: finish release 1.1",
        "push:main",
        "checkout:develop",
        "pull:origin/develop",
        "merge:release/1.1:chore: merge release 1.1 into develop",
        "push:develop",
        "delete_branch_local:release/1.1",
        "delete_branch_remote:release/1.1",
    ]);
}
