mod common;

use common::MockGit;
use bflow::flows::start::{start_work_branch, start_release};

#[test]
fn start_work_branch_creates_and_pushes() {
    let git = MockGit::new();
    start_work_branch(&git, "feature", "login-page", "develop").unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch:feature/login-page:develop",
        "push:feature/login-page",
    ]);
}

#[test]
fn start_work_branch_with_fix_prefix() {
    let git = MockGit::new();
    start_work_branch(&git, "fix", "broken-auth", "main").unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch:fix/broken-auth:main",
        "push:fix/broken-auth",
    ]);
}

#[test]
fn start_release_creates_new_when_no_release_exists_with_tags() {
    let mut git = MockGit::new();
    git.branches_matching = vec![]; // no existing release branches
    git.tags = vec!["1.0.0".to_string()];

    start_release(&git).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/1.1:develop",
        "push:release/1.1",
        "create_tag:1.1.0:chore: create release branch 1.1",
        "push_tag:1.1.0",
    ]);
}

#[test]
fn start_release_creates_new_when_no_release_exists_no_tags() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec![];

    start_release(&git).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "list_tags",
        "checkout:develop",
        "create_branch:release/0.1:develop",
        "push:release/0.1",
        "create_tag:0.1.0:chore: create release branch 0.1",
        "push_tag:0.1.0",
    ]);
}

#[test]
fn start_release_checks_out_existing_release_branch() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["release/1.1".to_string()];

    start_release(&git).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "checkout:release/1.1",
    ]);
}
