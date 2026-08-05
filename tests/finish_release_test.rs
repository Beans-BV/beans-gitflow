mod common;

use common::MockGit;
use bflow::flows::finish_release::{bump_version, sync_with_develop, finish_release};

/// Configure a MockGit for a "fresh start" release finish: nothing yet merged,
/// no tags exist past the latest RC, source branch still exists.
fn fresh_release_mock(major: u32, minor: u32, rc_tags: &[&str]) -> MockGit {
    let mut git = MockGit::new();
    let source = format!("release/{major}.{minor}.0");
    git.existing_local_branches.insert(source.clone());
    git.existing_remote_branches.insert(source);
    git.tags_on_branch = rc_tags.iter().map(|t| t.to_string()).collect();
    git
}

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
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string(), "v1.1.1".to_string()];

    bump_version(&git, 1, 1).unwrap();

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
        "ff_merge:origin/develop",
        "merge:release/1.1.0:chore: sync release 1.1.0 with develop",
        "push:develop",
        "checkout:release/1.1.0",
    ]);
}

#[test]
fn finish_release_creates_clean_tag_from_rc() {
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1", "v1.1.0-rc.2"]);
    git.rev_list_count_result = 0;

    finish_release(&git, 1, 1, "main").unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "is_ancestor:release/1.1.0:main",
        "rev_list_count:v1.1.0-rc.2:release/1.1.0",
        "checkout:main",
        "ff_merge:origin/main",
        "merge:release/1.1.0:chore: merge release 1.1.0 into main",
        "tag_exists:v1.1.0",
        "create_tag:v1.1.0:chore: release 1.1.0",
        "is_pushed:main",
        "push:main",
        "remote_tag_exists:v1.1.0",
        "push_tag:v1.1.0",
        "is_ancestor:release/1.1.0:develop",
        "checkout:develop",
        "ff_merge:origin/develop",
        "merge:release/1.1.0:chore: merge release 1.1.0 into develop",
        "is_pushed:develop",
        "push:develop",
        "current_branch",
        "local_branch_exists:release/1.1.0",
        "delete_branch_local:release/1.1.0",
        "remote_branch_exists:release/1.1.0",
        "delete_branch_remote:release/1.1.0",
    ]);
}

#[test]
fn finish_release_targets_master_when_that_is_the_mainline() {
    // The mainline is data, resolved once from bflow.branch.main. Every
    // checkout/merge/push target the release finish aims at the mainline must
    // follow it — a master repo previously got main-branch menus that then
    // failed on the first checkout.
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    git.rev_list_count_result = 0;

    finish_release(&git, 1, 1, "master").unwrap();

    assert_eq!(git.calls(), vec![
        "tags_on_branch:release/1.1.0",
        "is_ancestor:release/1.1.0:master",
        "rev_list_count:v1.1.0-rc.1:release/1.1.0",
        "checkout:master",
        "ff_merge:origin/master",
        "merge:release/1.1.0:chore: merge release 1.1.0 into master",
        "tag_exists:v1.1.0",
        "create_tag:v1.1.0:chore: release 1.1.0",
        "is_pushed:master",
        "push:master",
        "remote_tag_exists:v1.1.0",
        "push_tag:v1.1.0",
        "is_ancestor:release/1.1.0:develop",
        "checkout:develop",
        "ff_merge:origin/develop",
        "merge:release/1.1.0:chore: merge release 1.1.0 into develop",
        "is_pushed:develop",
        "push:develop",
        "current_branch",
        "local_branch_exists:release/1.1.0",
        "delete_branch_local:release/1.1.0",
        "remote_branch_exists:release/1.1.0",
        "delete_branch_remote:release/1.1.0",
    ]);
}

#[test]
fn the_rc_gate_error_names_the_configured_mainline() {
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    git.rev_list_count_result = 3;

    let err = finish_release(&git, 1, 1, "master").unwrap_err();

    assert!(err.contains("merged to master"), "got: {err}");
}

#[test]
fn finish_release_single_rc() {
    // The full ordering is already pinned above; what is distinct here is that a
    // lone RC is also the latest one, so the gate measures against it and the
    // clean tag is stripped from it.
    let mut git = fresh_release_mock(2, 0, &["v2.0.0-rc.1"]);
    git.rev_list_count_result = 0;

    finish_release(&git, 2, 0, "main").unwrap();

    let calls = git.calls();
    assert!(calls.iter().any(|c| c == "rev_list_count:v2.0.0-rc.1:release/2.0.0"),
        "the only RC is the one the gate measures against; calls: {calls:?}");
    assert!(calls.iter().any(|c| c == "create_tag:v2.0.0:chore: release 2.0.0"),
        "the clean tag is the RC stripped of its pre-release; calls: {calls:?}");
}

#[test]
fn finish_release_fails_when_head_past_latest_rc() {
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1", "v1.1.0-rc.2"]);
    git.rev_list_count_result = 2; // 2 commits on release/1.1.0 past v1.1.0-rc.2

    let result = finish_release(&git, 1, 1, "main");

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
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    git.rev_list_count_result = 1;

    let err = finish_release(&git, 1, 1, "main").unwrap_err();
    assert!(err.contains("1 commit past"), "expected singular 'commit'; got: {err}");
    assert!(!err.contains("1 commits"), "should not use plural for 1; got: {err}");
}

// --- Idempotent resume tests ---

#[test]
fn finish_release_resume_after_main_already_merged_and_tagged() {
    // Scenario: previous finish merged into main, tagged, pushed, but failed
    // on the develop merge (conflict). User resolves, re-runs.
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    git.rev_list_count_result = 0;
    git.ancestors.insert(("release/1.1.0".to_string(), "main".to_string()));
    git.existing_tags.insert("v1.1.0".to_string());
    git.existing_remote_tags.insert("v1.1.0".to_string());
    git.pushed_branches.insert("main".to_string());

    finish_release(&git, 1, 1, "main").unwrap();

    let calls = git.calls();
    // No re-merge into main, no re-tag, no re-push
    assert!(!calls.iter().any(|c| c == "checkout:main"), "must not re-checkout main; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("create_tag:")), "must not re-create tag");
    assert!(!calls.iter().any(|c| c == "push:main"), "must not re-push main");
    assert!(!calls.iter().any(|c| c == "push_tag:v1.1.0"), "must not re-push tag");
    // Develop merge still runs
    assert!(calls.iter().any(|c| c == "merge:release/1.1.0:chore: merge release 1.1.0 into develop"), "must merge into develop; calls: {calls:?}");
    // Cleanup runs
    assert!(calls.iter().any(|c| c == "delete_branch_local:release/1.1.0"));
}

#[test]
fn finish_release_resume_skips_rc_gate_when_already_merged_to_main() {
    // Scenario: main is already merged. The rev_list_count gate should be
    // skipped because we're past the merge step.
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    // Set count high so that the gate would fail IF it ran — proving it doesn't.
    git.rev_list_count_result = 99;
    git.ancestors.insert(("release/1.1.0".to_string(), "main".to_string()));

    let result = finish_release(&git, 1, 1, "main");
    assert!(result.is_ok(), "expected resume to succeed past the gate; got: {result:?}");
}

#[test]
fn finish_release_fully_idempotent_no_op_on_second_run() {
    // Everything already done.
    let mut git = MockGit::new();
    git.tags_on_branch = vec!["v1.1.0-rc.1".to_string()];
    git.ancestors.insert(("release/1.1.0".to_string(), "main".to_string()));
    git.ancestors.insert(("release/1.1.0".to_string(), "develop".to_string()));
    git.existing_tags.insert("v1.1.0".to_string());
    git.existing_remote_tags.insert("v1.1.0".to_string());
    git.pushed_branches.insert("main".to_string());
    git.pushed_branches.insert("develop".to_string());
    // No entries in existing_local_branches/existing_remote_branches → already deleted

    finish_release(&git, 1, 1, "main").unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("merge:")), "no merges; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("create_tag:")));
    assert!(!calls.iter().any(|c| c.starts_with("push:")));
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_")));
}

// --- Conflict guidance: every merge step must tell the user to switch back ---

#[test]
fn finish_release_main_merge_conflict_names_source_branch_to_switch_back() {
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    git.fail_nth_merge = Some(1); // main merge

    let err = finish_release(&git, 1, 1, "main").unwrap_err();
    assert!(err.contains("git switch release/1.1.0"),
        "main conflict should tell user to switch back to the release branch; got: {err}");
    assert!(err.contains("bflow finish"), "should mention re-running bflow finish; got: {err}");
}

#[test]
fn finish_release_develop_merge_conflict_names_source_branch_to_switch_back() {
    let mut git = fresh_release_mock(1, 1, &["v1.1.0-rc.1"]);
    git.fail_nth_merge = Some(2); // develop merge

    let err = finish_release(&git, 1, 1, "main").unwrap_err();
    assert!(err.contains("git switch release/1.1.0"),
        "develop conflict should tell user to switch back to the release branch; got: {err}");
}
