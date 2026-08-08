mod common;

use common::{MockGit, MockHosting};
use bflow::flows::finish_hotfix::finish_hotfix;
use bflow::repo_config::{Mode, RepoConfig};

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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
    assert!(hosting.calls().is_empty(), "free mode must make zero hosting calls; calls: {:?}", hosting.calls());
}

#[test]
fn finish_hotfix_targets_master_when_that_is_the_mainline() {
    let git = fresh_hotfix_mock(2, 3, 4);

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 2, 3, 4, "master", None).unwrap();

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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
        "tag_exists:v1.2.0",
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
fn finish_hotfix_excludes_shipped_release_branch() {
    // Trap 1: release/1.2.0 already shipped (tagged v1.2.0) — fan-out must
    // not merge or push into it, only into the still-open release/1.3.0.
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.branches_matching = vec!["release/1.2.0".to_string(), "release/1.3.0".to_string()];
    git.existing_tags.insert("v1.2.0".to_string());

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

    let calls = git.calls();
    assert!(calls.contains(&"tag_exists:v1.2.0".to_string()));
    assert!(calls.contains(&"tag_exists:v1.3.0".to_string()));
    assert!(
        !calls.iter().any(|c| c.contains("release/1.2.0")),
        "shipped release/1.2.0 must receive no merge/push; calls: {calls:?}"
    );
    assert!(
        calls.contains(&"merge:hotfix/1.0.1:chore: merge hotfix 1.0.1 into release/1.3.0".to_string()),
        "open release/1.3.0 must still be merged into; calls: {calls:?}"
    );
    assert!(calls.contains(&"push:release/1.3.0".to_string()));
}

#[test]
fn finish_hotfix_propagates_to_multiple_release_branches_in_sorted_order() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    // Provide unsorted to verify the implementation sorts deterministically.
    git.branches_matching = vec![
        "release/2.0.0".to_string(),
        "release/1.5.0".to_string(),
    ];

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

    let calls = git.calls();
    let expected_tail = vec![
        "list_branches_matching:release/*",
        "tag_exists:v2.0.0",
        "tag_exists:v1.5.0",
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
    // The `release/*` pattern does the excluding; the flow has no filter of its own.
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.branches_matching = vec![
        "release/1.2.0".to_string(),
        "release-fix/1.2.0/foo".to_string(),
    ];

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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

    let hosting = MockHosting::new();
    let result = finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None);

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

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

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_")), "deletions should be skipped; calls: {calls:?}");
}

#[test]
fn finish_hotfix_keeps_branch_when_configured() {
    // Same fully-completed world as finish_hotfix_resume_when_branch_already_deleted_is_idempotent,
    // except the source branch still exists and HEAD is still on it — proving
    // keep-release-branches=true skips delete_source_branch entirely, not just
    // its individual deletion calls.
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

    let hosting = MockHosting::new();
    let cfg = RepoConfig { mode: Mode::Free, keep_release_branches: true, ..RepoConfig::default() };
    finish_hotfix(&git, &hosting, &cfg, 1, 0, 1, "main", None).unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_")), "keep must skip deletion; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c == "checkout:main"), "keep must skip delete_source_branch's checkout; calls: {calls:?}");
}

// --- Conflict guidance: every merge step must tell the user to switch back ---

#[test]
fn finish_hotfix_main_merge_conflict_names_source_branch_to_switch_back() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.fail_nth_merge = Some(1); // main merge

    let hosting = MockHosting::new();
    let err = finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap_err();
    assert!(err.contains("git add . && git commit --no-edit"),
        "the commit step must come before git switch, which fails mid-merge; got: {err}");
    assert!(err.contains("git switch hotfix/1.0.1"),
        "main conflict should tell user to switch back to the hotfix branch; got: {err}");
    assert!(err.contains("bflow finish"), "should mention re-running bflow finish; got: {err}");
}

#[test]
fn finish_hotfix_develop_merge_conflict_names_source_branch_to_switch_back() {
    let mut git = fresh_hotfix_mock(1, 0, 1);
    git.fail_nth_merge = Some(2); // develop merge

    let hosting = MockHosting::new();
    let err = finish_hotfix(&git, &hosting, &RepoConfig::default(), 1, 0, 1, "main", None).unwrap_err();
    assert!(err.contains("git switch hotfix/1.0.1"),
        "develop conflict should tell user to switch back to the hotfix branch; got: {err}");
}

// --- Protected mode: sequential landing PRs (hotfixes have no RC gate) ---

fn protected_cfg(keep: bool) -> RepoConfig {
    RepoConfig { mode: Mode::Protected, keep_release_branches: keep, ..RepoConfig::default() }
}

fn landed(head_sha: &str, merge_commit_sha: &str) -> bflow::hosting::LandedPr {
    bflow::hosting::LandedPr {
        url: "https://github.com/org/repo/pull/1".to_string(),
        head_sha: head_sha.to_string(),
        merge_commit_sha: merge_commit_sha.to_string(),
    }
}

#[test]
fn protected_hotfix_opens_main_pr_and_stops() {
    let mut git = MockGit::new();
    git.existing_remote_branches.insert("hotfix/1.1.1".to_string());
    git.pushed_branches.insert("hotfix/1.1.1".to_string());

    let hosting = MockHosting::new();
    finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 1, 1, "main", None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "tag_exists:v1.1.1",
        "remote_branch_exists:hotfix/1.1.1",
        "is_pushed:hotfix/1.1.1",
    ]);
    assert_eq!(hosting.calls(), vec![
        "merged_pr_to:hotfix/1.1.1:main",
        "create_or_get_pr:hotfix/1.1.1:main:chore: merge hotfix 1.1.1 into main",
    ]);
    let calls = git.calls();
    assert!(!calls.iter().any(|c| c == "checkout:main"), "must not merge locally; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("merge:")), "must not merge locally; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("create_tag")), "must not tag before main lands; calls: {calls:?}");
}

#[test]
fn protected_hotfix_tags_merge_commit_then_opens_develop_pr() {
    let mut git = MockGit::new();
    git.branch_shas.insert("hotfix/1.1.1".to_string(), "hfsha".to_string());
    git.existing_remote_branches.insert("hotfix/1.1.1".to_string());
    git.pushed_branches.insert("hotfix/1.1.1".to_string());
    git.ancestors.insert(("mc1".to_string(), "origin/main".to_string()));
    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(
        ("hotfix/1.1.1".to_string(), "main".to_string()),
        landed("hfsha", "mc1"),
    );

    finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 1, 1, "main", None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "is_ancestor:mc1:origin/main",
        "tag_exists:v1.1.1",
        "tag_exists:v1.1.1",
        "create_tag_at:v1.1.1:chore: hotfix 1.1.1:mc1",
        "remote_tag_exists:v1.1.1",
        "push_tag:v1.1.1",
        "branch_sha:hotfix/1.1.1",
        "remote_branch_exists:hotfix/1.1.1",
        "is_pushed:hotfix/1.1.1",
    ]);
    assert_eq!(hosting.calls(), vec![
        "merged_pr_to:hotfix/1.1.1:main",
        "merged_pr_to:hotfix/1.1.1:develop",
        "create_or_get_pr:hotfix/1.1.1:develop:chore: merge hotfix 1.1.1 into develop",
    ]);
    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("rev_list_count")), "hotfixes carry no RC gate; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("tags_on_branch")), "hotfixes carry no RC gate; calls: {calls:?}");
}

#[test]
fn protected_hotfix_opens_one_release_pr_per_run() {
    let mut git = MockGit::new();
    git.branch_shas.insert("hotfix/1.1.1".to_string(), "hfsha".to_string());
    git.branches_matching = vec!["release/1.3.0".to_string(), "release/1.2.0".to_string()];
    git.existing_remote_branches.insert("hotfix/1.1.1".to_string());
    git.pushed_branches.insert("hotfix/1.1.1".to_string());
    git.existing_tags.insert("v1.1.1".to_string());
    git.tag_commits.insert("v1.1.1".to_string(), "mc1".to_string());
    git.existing_remote_tags.insert("v1.1.1".to_string());
    git.ancestors.insert(("mc1".to_string(), "origin/main".to_string()));
    git.ancestors.insert(("mc2".to_string(), "origin/develop".to_string()));
    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "main".to_string()), landed("hfsha", "mc1"));
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "develop".to_string()), landed("hfsha", "mc2"));

    finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 1, 1, "main", None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "tag_exists:v1.3.0",
        "tag_exists:v1.2.0",
        "is_ancestor:mc1:origin/main",
        "tag_exists:v1.1.1",
        "tag_commit_sha:v1.1.1",
        "is_ancestor:mc1:origin/main",
        "remote_tag_exists:v1.1.1",
        "branch_sha:hotfix/1.1.1",
        "is_ancestor:mc2:origin/develop",
        "remote_branch_exists:hotfix/1.1.1",
        "is_pushed:hotfix/1.1.1",
    ]);
    assert_eq!(hosting.calls(), vec![
        "merged_pr_to:hotfix/1.1.1:main",
        "merged_pr_to:hotfix/1.1.1:develop",
        "merged_pr_to:hotfix/1.1.1:release/1.2.0",
        "create_or_get_pr:hotfix/1.1.1:release/1.2.0:chore: merge hotfix 1.1.1 into release/1.2.0",
    ]);
}

#[test]
fn protected_hotfix_next_run_opens_pr_for_the_remaining_release() {
    let mut git = MockGit::new();
    git.branch_shas.insert("hotfix/1.1.1".to_string(), "hfsha".to_string());
    git.branches_matching = vec!["release/1.3.0".to_string(), "release/1.2.0".to_string()];
    git.existing_remote_branches.insert("hotfix/1.1.1".to_string());
    git.pushed_branches.insert("hotfix/1.1.1".to_string());
    git.existing_tags.insert("v1.1.1".to_string());
    git.tag_commits.insert("v1.1.1".to_string(), "mc1".to_string());
    git.existing_remote_tags.insert("v1.1.1".to_string());
    git.ancestors.insert(("mc1".to_string(), "origin/main".to_string()));
    git.ancestors.insert(("mc2".to_string(), "origin/develop".to_string()));
    git.ancestors.insert(("mc3".to_string(), "origin/release/1.2.0".to_string()));
    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "main".to_string()), landed("hfsha", "mc1"));
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "develop".to_string()), landed("hfsha", "mc2"));
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "release/1.2.0".to_string()), landed("hfsha", "mc3"));

    finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 1, 1, "main", None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "tag_exists:v1.3.0",
        "tag_exists:v1.2.0",
        "is_ancestor:mc1:origin/main",
        "tag_exists:v1.1.1",
        "tag_commit_sha:v1.1.1",
        "is_ancestor:mc1:origin/main",
        "remote_tag_exists:v1.1.1",
        "branch_sha:hotfix/1.1.1",
        "is_ancestor:mc2:origin/develop",
        "is_ancestor:mc3:origin/release/1.2.0",
        "remote_branch_exists:hotfix/1.1.1",
        "is_pushed:hotfix/1.1.1",
    ]);
    assert_eq!(hosting.calls(), vec![
        "merged_pr_to:hotfix/1.1.1:main",
        "merged_pr_to:hotfix/1.1.1:develop",
        "merged_pr_to:hotfix/1.1.1:release/1.2.0",
        "merged_pr_to:hotfix/1.1.1:release/1.3.0",
        "create_or_get_pr:hotfix/1.1.1:release/1.3.0:chore: merge hotfix 1.1.1 into release/1.3.0",
    ]);
}

#[test]
fn protected_hotfix_completes_after_all_landed() {
    let mut git = MockGit::new();
    git.current_branch = "hotfix/1.1.1".to_string();
    git.branch_shas.insert("hotfix/1.1.1".to_string(), "hfsha".to_string());
    git.existing_tags.insert("v1.1.1".to_string());
    git.tag_commits.insert("v1.1.1".to_string(), "mc1".to_string());
    git.existing_remote_tags.insert("v1.1.1".to_string());
    git.existing_local_branches.insert("hotfix/1.1.1".to_string());
    git.existing_remote_branches.insert("hotfix/1.1.1".to_string());
    git.branches_matching = vec!["release/1.2.0".to_string()];
    git.ancestors.insert(("mc1".to_string(), "origin/main".to_string()));
    git.ancestors.insert(("mc2".to_string(), "origin/develop".to_string()));
    git.ancestors.insert(("mc3".to_string(), "origin/release/1.2.0".to_string()));

    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "main".to_string()), landed("hfsha", "mc1"));
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "develop".to_string()), landed("hfsha", "mc2"));
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "release/1.2.0".to_string()), landed("hfsha", "mc3"));

    finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 1, 1, "main", None).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "tag_exists:v1.2.0",
        "is_ancestor:mc1:origin/main",
        "tag_exists:v1.1.1",
        "tag_commit_sha:v1.1.1",
        "is_ancestor:mc1:origin/main",
        "remote_tag_exists:v1.1.1",
        "branch_sha:hotfix/1.1.1",
        "is_ancestor:mc2:origin/develop",
        "is_ancestor:mc3:origin/release/1.2.0",
        "branch_sha:hotfix/1.1.1",
        "current_branch",
        "checkout:main",
        "local_branch_exists:hotfix/1.1.1",
        "delete_branch_local:hotfix/1.1.1",
        "remote_branch_exists:hotfix/1.1.1",
        "delete_branch_remote:hotfix/1.1.1",
    ]);
    assert_eq!(hosting.calls(), vec![
        "merged_pr_to:hotfix/1.1.1:main",
        "merged_pr_to:hotfix/1.1.1:develop",
        "merged_pr_to:hotfix/1.1.1:release/1.2.0",
    ]);
}

#[test]
fn protected_hotfix_keeps_branch_when_configured() {
    let mut git = MockGit::new();
    git.current_branch = "hotfix/1.1.1".to_string();
    git.branch_shas.insert("hotfix/1.1.1".to_string(), "hfsha".to_string());
    git.existing_tags.insert("v1.1.1".to_string());
    git.tag_commits.insert("v1.1.1".to_string(), "mc1".to_string());
    git.existing_remote_tags.insert("v1.1.1".to_string());
    git.ancestors.insert(("mc1".to_string(), "origin/main".to_string()));
    git.ancestors.insert(("mc2".to_string(), "origin/develop".to_string()));

    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "main".to_string()), landed("hfsha", "mc1"));
    hosting.merged_prs_to.insert(("hotfix/1.1.1".to_string(), "develop".to_string()), landed("hfsha", "mc2"));

    finish_hotfix(&git, &hosting, &protected_cfg(true), 1, 1, 1, "main", None).unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c == "checkout:main"), "keep must skip delete_source_branch's checkout; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_")), "keep must skip deletion; calls: {calls:?}");
}

#[test]
fn protected_hotfix_advances_to_the_release_leg_when_the_branch_moved() {
    // Main and develop both landed (proven by tag/PR containment, not by a
    // fresh merged_pr_to hit or a head-sha match — the branch tip has since
    // moved). Hotfixes carry no RC gate, so landing must advance straight to
    // the one still-open release branch instead of getting stuck re-opening
    // a main PR.
    let mut git = MockGit::new();
    git.existing_tags.insert("v1.3.1".to_string());
    git.tag_commits.insert("v1.3.1".to_string(), "old-tip-sha".to_string());
    git.ancestors.insert(("old-tip-sha".to_string(), "origin/main".to_string()));
    git.branch_shas.insert("hotfix/1.3.1".to_string(), "moved-tip-sha".to_string());
    git.branches_matching = vec!["release/1.4.0".to_string()];
    git.existing_remote_branches.insert("hotfix/1.3.1".to_string());
    git.pushed_branches.insert("hotfix/1.3.1".to_string());
    git.ancestors.insert(("mc2".to_string(), "origin/develop".to_string()));

    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(("hotfix/1.3.1".to_string(), "develop".to_string()), landed("old-develop-head", "mc2"));

    finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 3, 1, "main", None).unwrap();

    let hosting_calls = hosting.calls();
    assert_eq!(hosting_calls, vec![
        "merged_pr_to:hotfix/1.3.1:main",
        "merged_pr_to:hotfix/1.3.1:develop",
        "merged_pr_to:hotfix/1.3.1:release/1.4.0",
        "create_or_get_pr:hotfix/1.3.1:release/1.4.0:chore: merge hotfix 1.3.1 into release/1.4.0",
    ]);
    assert_eq!(hosting_calls.iter().filter(|c| c.starts_with("create_or_get_pr")).count(), 1,
        "expected exactly one create_or_get_pr call; calls: {hosting_calls:?}");
    assert!(!hosting_calls.iter().any(|c| c.starts_with("create_or_get_pr") && c.contains(":main:")),
        "must not reopen a main PR once main has landed; calls: {hosting_calls:?}");
}

#[test]
fn protected_hotfix_keeps_the_branch_when_its_tip_landed_nowhere() {
    // Every leg has landed, but the branch tip matches no landed PR head —
    // commits sitting on the hotfix branch that never reached any target.
    // Squash merges leave no ancestry to fall back on, so deleting here would
    // lose them: warn and keep, exactly as the release path does.
    let mut git = MockGit::new();
    git.existing_tags.insert("v1.3.1".to_string());
    git.tag_commits.insert("v1.3.1".to_string(), "old-tip-sha".to_string());
    git.ancestors.insert(("old-tip-sha".to_string(), "origin/main".to_string()));
    git.branch_shas.insert("hotfix/1.3.1".to_string(), "unrelated-tip-sha".to_string());
    git.existing_local_branches.insert("hotfix/1.3.1".to_string());
    git.existing_remote_branches.insert("hotfix/1.3.1".to_string());
    git.ancestors.insert(("mc2".to_string(), "origin/develop".to_string()));

    let mut hosting = MockHosting::new();
    hosting.merged_prs_to.insert(("hotfix/1.3.1".to_string(), "develop".to_string()), landed("old-develop-head", "mc2"));

    let result = finish_hotfix(&git, &hosting, &protected_cfg(false), 1, 3, 1, "main", None);
    assert!(result.is_ok(), "the guard must not fail the run; got: {result:?}");

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_local")), "tip landed nowhere must not delete; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c.starts_with("delete_branch_remote")), "tip landed nowhere must not delete; calls: {calls:?}");
    assert!(!calls.iter().any(|c| c == "checkout:main"), "tip landed nowhere must not touch the branch at all; calls: {calls:?}");
}
