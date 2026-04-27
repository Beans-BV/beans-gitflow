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
        "create_tag:v1.0.1:chore: hotfix 1.0.1",
        "push:main",
        "push_tag:v1.0.1",
        "checkout:develop",
        "pull:origin/develop",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into develop",
        "push:develop",
        "list_branches_matching:release/*",
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
        "create_tag:v2.3.4:chore: hotfix 2.3.4",
        "push:main",
        "push_tag:v2.3.4",
        "checkout:develop",
        "pull:origin/develop",
        "merge:hotfix/2.3.4:chore: merge hotfix 2.3.4 into develop",
        "push:develop",
        "list_branches_matching:release/*",
        "delete_branch_local:hotfix/2.3.4",
        "delete_branch_remote:hotfix/2.3.4",
    ]);
}

#[test]
fn finish_hotfix_propagates_to_open_release_branch() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["release/1.2.0".to_string()];

    finish_hotfix(&git, 1, 0, 1).unwrap();

    assert_eq!(git.calls(), vec![
        "checkout:main",
        "pull:origin/main",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into main",
        "create_tag:v1.0.1:chore: hotfix 1.0.1",
        "push:main",
        "push_tag:v1.0.1",
        "checkout:develop",
        "pull:origin/develop",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into develop",
        "push:develop",
        "list_branches_matching:release/*",
        "checkout:release/1.2.0",
        "pull:origin/release/1.2.0",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/1.2.0",
        "push:release/1.2.0",
        "delete_branch_local:hotfix/1.0.1",
        "delete_branch_remote:hotfix/1.0.1",
    ]);
}

#[test]
fn finish_hotfix_propagates_to_multiple_release_branches_in_sorted_order() {
    let mut git = MockGit::new();
    // Provide unsorted to verify the implementation sorts deterministically.
    git.branches_matching = vec![
        "release/2.0.0".to_string(),
        "release/1.5.0".to_string(),
    ];

    finish_hotfix(&git, 1, 0, 1).unwrap();

    let calls = git.calls();
    let expected_tail = vec![
        "list_branches_matching:release/*",
        "checkout:release/1.5.0",
        "pull:origin/release/1.5.0",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/1.5.0",
        "push:release/1.5.0",
        "checkout:release/2.0.0",
        "pull:origin/release/2.0.0",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/2.0.0",
        "push:release/2.0.0",
        "delete_branch_local:hotfix/1.0.1",
        "delete_branch_remote:hotfix/1.0.1",
    ];
    let tail_start = calls.len() - expected_tail.len();
    assert_eq!(&calls[tail_start..], &expected_tail[..]);
}

#[test]
fn finish_hotfix_excludes_release_fix_branches() {
    let mut git = MockGit::new();
    // list_branches_matching("release/*") may return both release/* and release-fix/*
    // because the latter starts with "release". The propagation must skip release-fix.
    git.branches_matching = vec![
        "release/1.2.0".to_string(),
        "release-fix/1.2.0/foo".to_string(),
    ];

    finish_hotfix(&git, 1, 0, 1).unwrap();

    let calls = git.calls();
    assert!(
        calls.iter().any(|c| c == "checkout:release/1.2.0"),
        "expected propagation to release/1.2.0; got: {calls:?}"
    );
    assert!(
        !calls.iter().any(|c| c.contains("release-fix/")),
        "must not propagate into release-fix branches; got: {calls:?}"
    );
}

#[test]
fn finish_hotfix_aborts_on_release_merge_conflict_without_deleting_hotfix() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["release/1.2.0".to_string()];
    // 1st merge: into main (ok), 2nd: into develop (ok), 3rd: into release (fail).
    git.fail_nth_merge = Some(3);

    let result = finish_hotfix(&git, 1, 0, 1);

    assert!(result.is_err(), "expected merge conflict to surface");
    let err = result.unwrap_err();
    assert!(err.contains("release/1.2.0"), "error should name the conflicting branch; got: {err}");

    let calls = git.calls();
    // Main + develop + tag must already be done before the conflict.
    assert!(calls.iter().any(|c| c == "create_tag:v1.0.1:chore: hotfix 1.0.1"));
    assert!(calls.iter().any(|c| c == "push_tag:v1.0.1"));
    assert!(calls.iter().any(|c| c == "push:develop"));
    // Hotfix branch must NOT be deleted — user needs it for retry.
    assert!(
        !calls.iter().any(|c| c.starts_with("delete_branch_local:hotfix/")),
        "hotfix branch must survive for retry; calls: {calls:?}"
    );
    assert!(
        !calls.iter().any(|c| c.starts_with("delete_branch_remote:hotfix/")),
        "hotfix branch must survive for retry; calls: {calls:?}"
    );
}
