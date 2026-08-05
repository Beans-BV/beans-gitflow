mod common;

use common::MockGit;
use bflow::flows::finish_hotfix::finish_hotfix;

/// Configure a MockGit for a "fresh start" hotfix finish: nothing is yet merged,
/// no tags exist, source branch still exists locally and remotely.
fn fresh_hotfix_mock(major: u32, minor: u32, patch: u32) -> MockGit {
    let mut git = MockGit::new();
    let source = format!("hotfix/{major}.{minor}.{patch}");
    git.existing_local_branches.insert(source.clone());
    git.existing_remote_branches.insert(source);
    git
}

#[test]
fn finish_hotfix_full_sequence() {
    let git = fresh_hotfix_mock(1, 0, 1);

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    assert_eq!(git.calls(), vec![
        "is_ancestor:hotfix/1.0.1:main",
        "checkout:main",
        "ff_merge:origin/main",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into main",
        "tag_exists:v1.0.1",
        "create_tag:v1.0.1:chore: hotfix 1.0.1",
        "is_pushed:main",
        "push:main",
        "remote_tag_exists:v1.0.1",
        "push_tag:v1.0.1",
        "is_ancestor:hotfix/1.0.1:develop",
        "checkout:develop",
        "ff_merge:origin/develop",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into develop",
        "is_pushed:develop",
        "push:develop",
        "list_branches_matching:release/*",
        "current_branch",
        "local_branch_exists:hotfix/1.0.1",
        "delete_branch_local:hotfix/1.0.1",
        "remote_branch_exists:hotfix/1.0.1",
        "delete_branch_remote:hotfix/1.0.1",
    ]);
}

#[test]
fn finish_hotfix_targets_master_when_that_is_the_mainline() {
    // Also the suite's second version: 2.3.4 rather than 1.0.1, so one script
    // proves both that the version threads through every call and that the
    // mainline does.
    let git = fresh_hotfix_mock(2, 3, 4);

    finish_hotfix(&git, 2, 3, 4, "master").unwrap();

    assert_eq!(git.calls(), vec![
        "is_ancestor:hotfix/2.3.4:master",
        "checkout:master",
        "ff_merge:origin/master",
        "merge:hotfix/2.3.4:chore: merge hotfix 2.3.4 into master",
        "tag_exists:v2.3.4",
        "create_tag:v2.3.4:chore: hotfix 2.3.4",
        "is_pushed:master",
        "push:master",
        "remote_tag_exists:v2.3.4",
        "push_tag:v2.3.4",
        "is_ancestor:hotfix/2.3.4:develop",
        "checkout:develop",
        "ff_merge:origin/develop",
        "merge:hotfix/2.3.4:chore: merge hotfix 2.3.4 into develop",
        "is_pushed:develop",
        "push:develop",
        "list_branches_matching:release/*",
        "current_branch",
        "local_branch_exists:hotfix/2.3.4",
        "delete_branch_local:hotfix/2.3.4",
        "remote_branch_exists:hotfix/2.3.4",
        "delete_branch_remote:hotfix/2.3.4",
    ]);
}

#[test]
fn finish_hotfix_propagates_to_open_release_branch() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.branches_matching = vec!["release/1.2.0".to_string()];

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    assert_eq!(git.calls(), vec![
        "is_ancestor:hotfix/1.0.1:main",
        "checkout:main",
        "ff_merge:origin/main",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into main",
        "tag_exists:v1.0.1",
        "create_tag:v1.0.1:chore: hotfix 1.0.1",
        "is_pushed:main",
        "push:main",
        "remote_tag_exists:v1.0.1",
        "push_tag:v1.0.1",
        "is_ancestor:hotfix/1.0.1:develop",
        "checkout:develop",
        "ff_merge:origin/develop",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into develop",
        "is_pushed:develop",
        "push:develop",
        "list_branches_matching:release/*",
        "is_ancestor:hotfix/1.0.1:release/1.2.0",
        "checkout:release/1.2.0",
        "ff_merge:origin/release/1.2.0",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/1.2.0",
        "is_pushed:release/1.2.0",
        "push:release/1.2.0",
        "current_branch",
        "local_branch_exists:hotfix/1.0.1",
        "delete_branch_local:hotfix/1.0.1",
        "remote_branch_exists:hotfix/1.0.1",
        "delete_branch_remote:hotfix/1.0.1",
    ]);
}

#[test]
fn finish_hotfix_propagates_to_multiple_release_branches_in_sorted_order() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    // Provide unsorted to verify the implementation sorts deterministically.
    git.branches_matching = vec![
        "release/2.0.0".to_string(),
        "release/1.5.0".to_string(),
    ];

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    let expected_tail = vec![
        "list_branches_matching:release/*",
        "is_ancestor:hotfix/1.0.1:release/1.5.0",
        "checkout:release/1.5.0",
        "ff_merge:origin/release/1.5.0",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/1.5.0",
        "is_pushed:release/1.5.0",
        "push:release/1.5.0",
        "is_ancestor:hotfix/1.0.1:release/2.0.0",
        "checkout:release/2.0.0",
        "ff_merge:origin/release/2.0.0",
        "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/2.0.0",
        "is_pushed:release/2.0.0",
        "push:release/2.0.0",
        "current_branch",
        "local_branch_exists:hotfix/1.0.1",
        "delete_branch_local:hotfix/1.0.1",
        "remote_branch_exists:hotfix/1.0.1",
        "delete_branch_remote:hotfix/1.0.1",
    ];
    let tail_start = calls.len() - expected_tail.len();
    assert_eq!(&calls[tail_start..], &expected_tail[..]);
}

#[test]
fn finish_hotfix_excludes_release_fix_branches() {
    // A hotfix propagates into open *release* branches, never into someone's
    // in-flight release-fix work. That exclusion is the ref pattern's job:
    // `release/*` cannot match `release-fix/…` (`release-` != `release/`), so
    // the flow needs no filter of its own. The repo below has both kinds.
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.branches_matching = vec![
        "release/1.2.0".to_string(),
        "release-fix/1.2.0/foo".to_string(),
    ];

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    assert!(
        calls.iter().any(|c| c == "list_branches_matching:release/*"),
        "the pattern is what does the excluding; got: {calls:?}"
    );
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
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.branches_matching = vec!["release/1.2.0".to_string()];
    // 1st merge: into main (ok), 2nd: into develop (ok), 3rd: into release (fail).
    git.fail_nth_merge = Some(3);

    let result = finish_hotfix(&git, 1, 0, 1, "main");

    assert!(result.is_err(), "expected merge conflict to surface");
    let err = result.unwrap_err();
    assert!(err.contains("release/1.2.0"), "error should name the conflicting branch; got: {err}");
    assert!(err.contains("bflow finish"), "error should tell user to re-run bflow finish; got: {err}");

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

// --- Idempotent resume tests ---

#[test]
fn finish_hotfix_resume_skips_already_merged_main_and_develop() {
    // Scenario: previous finish merged into main + develop + tagged + pushed,
    // but failed during release propagation. User resolves, re-runs.
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.branches_matching = vec!["release/1.2.0".to_string()];
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "main".to_string()));
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "develop".to_string()));
    git.existing_tags.insert("v1.0.1".to_string());
    git.existing_remote_tags.insert("v1.0.1".to_string());
    git.pushed_branches.insert("main".to_string());
    git.pushed_branches.insert("develop".to_string());

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    // No re-merges of main or develop
    assert!(!calls.iter().any(|c| c == "checkout:main"), "must not re-checkout main; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c == "checkout:develop"), "must not re-checkout develop; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("merge:hotfix/1.0.1:") && c.contains("into main")), "must not re-merge main");
    assert!(!calls.iter().any(|c| c.starts_with("merge:hotfix/1.0.1:") && c.contains("into develop")), "must not re-merge develop");
    // No re-tag, no re-push
    assert!(!calls.iter().any(|c| c.starts_with("create_tag:")), "must not re-create tag");
    assert!(!calls.iter().any(|c| c == "push:main"), "must not re-push main");
    assert!(!calls.iter().any(|c| c == "push_tag:v1.0.1"), "must not re-push tag");
    // Release propagation still runs
    assert!(calls.iter().any(|c| c == "merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/1.2.0"), "must merge into release; calls: {calls:?}");
    // Hotfix cleanup runs
    assert!(calls.iter().any(|c| c == "delete_branch_local:hotfix/1.0.1"));
    assert!(calls.iter().any(|c| c == "delete_branch_remote:hotfix/1.0.1"));
}

#[test]
fn finish_hotfix_resume_skips_already_propagated_release() {
    // Scenario: previous finish completed everything except cleanup.
    let mut git = MockGit::new();
    git.branches_matching = vec!["release/1.2.0".to_string()];
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "main".to_string()));
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "develop".to_string()));
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "release/1.2.0".to_string()));
    git.existing_tags.insert("v1.0.1".to_string());
    git.existing_remote_tags.insert("v1.0.1".to_string());
    git.pushed_branches.insert("main".to_string());
    git.pushed_branches.insert("develop".to_string());
    git.pushed_branches.insert("release/1.2.0".to_string());
    git.existing_local_branches.insert("hotfix/1.0.1".to_string());
    git.existing_remote_branches.insert("hotfix/1.0.1".to_string());

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("merge:")), "no merges should run; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("push:")), "no pushes should run; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("create_tag:")), "no tag creation");
    assert!(calls.iter().any(|c| c == "delete_branch_local:hotfix/1.0.1"));
    assert!(calls.iter().any(|c| c == "delete_branch_remote:hotfix/1.0.1"));
}

#[test]
fn finish_hotfix_switches_off_source_before_deleting_when_currently_on_it() {
    // Scenario: resume after develop-merge conflict resolved on develop.
    // User switches back to hotfix and re-runs. The develop merge is detected
    // as already done, so the flow never checks out develop — HEAD stays on
    // the hotfix branch. The cleanup step must switch off it before deleting.
    let mut git = MockGit::new();
    git.current_branch = "hotfix/1.0.1".to_string();
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "main".to_string()));
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "develop".to_string()));
    git.existing_tags.insert("v1.0.1".to_string());
    git.existing_remote_tags.insert("v1.0.1".to_string());
    git.pushed_branches.insert("main".to_string());
    git.pushed_branches.insert("develop".to_string());
    git.existing_local_branches.insert("hotfix/1.0.1".to_string());
    git.existing_remote_branches.insert("hotfix/1.0.1".to_string());

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    let checkout_main_idx = calls.iter().position(|c| c == "checkout:main")
        .unwrap_or_else(|| panic!("expected a checkout:main before delete; calls: {calls:?}"));
    let delete_idx = calls.iter().position(|c| c == "delete_branch_local:hotfix/1.0.1")
        .expect("expected delete_branch_local");
    assert!(checkout_main_idx < delete_idx,
        "checkout:main must come before delete_branch_local; calls: {calls:?}");
}

#[test]
fn finish_hotfix_skips_cleanup_checkout_when_already_off_source() {
    // Sanity: when HEAD is not on the source branch (the happy path, where
    // the develop merge moved us to develop), no extra checkout fires.
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.current_branch = "develop".to_string();

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    // The only checkouts should be the ones the flow explicitly drives (main, develop).
    let checkouts: Vec<&String> = calls.iter().filter(|c| c.starts_with("checkout:")).collect();
    assert_eq!(checkouts, vec![
        &"checkout:main".to_string(),
        &"checkout:develop".to_string(),
    ], "unexpected extra checkout; calls: {calls:?}");
}

#[test]
fn finish_hotfix_resume_when_branch_already_deleted_is_idempotent() {
    // Scenario: everything done; user runs again. All deletions skipped.
    let mut git = MockGit::new();
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "main".to_string()));
    git.ancestors.insert(("hotfix/1.0.1".to_string(), "develop".to_string()));
    git.existing_tags.insert("v1.0.1".to_string());
    git.existing_remote_tags.insert("v1.0.1".to_string());
    git.pushed_branches.insert("main".to_string());
    git.pushed_branches.insert("develop".to_string());
    // Note: no entries in existing_local_branches / existing_remote_branches → already deleted

    finish_hotfix(&git, 1, 0, 1, "main").unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_")), "deletions should be skipped; calls: {calls:?}");
}

// --- Conflict guidance: every merge step must tell the user to switch back ---

#[test]
fn finish_hotfix_main_merge_conflict_names_source_branch_to_switch_back() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.fail_nth_merge = Some(1); // main merge

    let err = finish_hotfix(&git, 1, 0, 1, "main").unwrap_err();
    assert!(err.contains("git switch hotfix/1.0.1"),
        "main conflict should tell user to switch back to the hotfix branch; got: {err}");
    assert!(err.contains("bflow finish"), "should mention re-running bflow finish; got: {err}");
}

#[test]
fn finish_hotfix_develop_merge_conflict_names_source_branch_to_switch_back() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.fail_nth_merge = Some(2); // develop merge

    let err = finish_hotfix(&git, 1, 0, 1, "main").unwrap_err();
    assert!(err.contains("git switch hotfix/1.0.1"),
        "develop conflict should tell user to switch back to the hotfix branch; got: {err}");
}
