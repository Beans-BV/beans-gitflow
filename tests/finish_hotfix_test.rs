mod common;

use common::MockGit;
use bflow::flows::finish_hotfix::finish_hotfix;

#[test]
fn finish_hotfix_full_sequence() {
    let git = MockGit::new();

    finish_hotfix(&git, 1, 0, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "checkout:main",
        "pull:origin/main",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into main",
        "create_tag:1.0.1:chore: hotfix 1.0.1",
        "push:main",
        "push_tag:1.0.1",
        "checkout:develop",
        "pull:origin/develop",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into develop",
        "push:develop",
        "delete_branch_local:hotfix/1.0.1",
        "delete_branch_remote:hotfix/1.0.1",
    ]);
}

#[test]
fn finish_hotfix_different_version() {
    let git = MockGit::new();

    finish_hotfix(&git, 2, 3, 4).unwrap();

    assert_eq!(git.calls(), vec![
        "checkout:main",
        "pull:origin/main",
        "merge:hotfix/2.3.4:chore: merge hotfix 2.3.4 into main",
        "create_tag:2.3.4:chore: hotfix 2.3.4",
        "push:main",
        "push_tag:2.3.4",
        "checkout:develop",
        "pull:origin/develop",
        "merge:hotfix/2.3.4:chore: merge hotfix 2.3.4 into develop",
        "push:develop",
        "delete_branch_local:hotfix/2.3.4",
        "delete_branch_remote:hotfix/2.3.4",
    ]);
}
