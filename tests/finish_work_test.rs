mod common;

use common::{MockGit, MockHosting, MockPrompter};
use bflow::flows::finish_work::{finish_release_fix, finish_hotfix_fix, finish_work_branch};
use bflow::git::branch::BranchType;

#[test]
fn finish_release_fix_pushes_and_creates_pr() {
    let mut git = MockGit::new();
    git.current_branch = "release-fix/1.1.0/login-bug".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::ReleaseFix { major: 1, minor: 1, patch: 0, name: "login-bug".to_string() };

    finish_release_fix(&git, &hosting, &branch_type, None).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "push:release-fix/1.1.0/login-bug",
    ]);

    assert_eq!(hosting.calls(), vec![
        "merged_pr:release-fix/1.1.0/login-bug",
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

    finish_hotfix_fix(&git, &hosting, &branch_type, None).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "push:hotfix-fix/1.0.1/crash-fix",
    ]);

    assert_eq!(hosting.calls(), vec![
        "merged_pr:hotfix-fix/1.0.1/crash-fix",
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

    finish_release_fix(&git, &hosting, &branch_type, None).unwrap();

    assert_eq!(hosting.calls(), vec![
        "merged_pr:release-fix/2.0.0/typo",
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

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].starts_with("create_or_get_pr:feature/login:"));
    assert!(calls[1].ends_with(":feat: login"));
}

#[test]
fn finish_work_branch_feature_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "feature/remove-api".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "remove-api".to_string() };

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(true), None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].ends_with(":feat!: remove-api"),
        "Expected PR title to end with 'feat!: remove-api', got: {}", calls[1]);
}

#[test]
fn finish_work_branch_chore_breaking_honored() {
    let mut git = MockGit::new();
    git.current_branch = "chore/drop-node-16".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Chore { name: "drop-node-16".to_string() };

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(true), None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].ends_with(":chore!: drop-node-16"),
        "Explicit --breaking should be honored on chore, got: {}", calls[1]);
}

#[test]
fn finish_work_branch_docs_defaults_to_non_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "docs/readme".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Docs { name: "readme".to_string() };

    // No flag (None) — docs should NOT prompt, should default to non-breaking
    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, None, None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].ends_with(":docs: readme"),
        "Docs with None should default to non-breaking, got: {}", calls[1]);
}

#[test]
fn finish_work_branch_with_explicit_base_skips_detection() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    git.existing_remote_branches.insert("develop".to_string());
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), Some("develop".to_string()), None).unwrap();

    let git_calls = git.calls();
    assert!(!git_calls.contains(&"list_remote_branches".to_string()),
        "Explicit --base must skip parent detection, got: {git_calls:?}");
    let calls = hosting.calls();
    assert!(calls[1].starts_with("create_or_get_pr:feature/login:develop:"),
        "PR should target the explicit base, got: {}", calls[1]);
}

#[test]
fn finish_work_branch_with_local_only_base_errors() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    // Base exists locally but was never pushed: PR creation would fail on GitHub,
    // so bflow must reject it up-front instead of pushing and then failing.
    git.existing_local_branches.insert("feature/auth".to_string());
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    let err = finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), Some("feature/auth".to_string()), None).unwrap_err();

    assert!(err.contains("feature/auth") && err.contains("origin"),
        "Error should name the branch and origin, got: {err}");
    assert!(!git.calls().iter().any(|c| c.starts_with("push:")),
        "Nothing should be pushed for an invalid base");
    assert!(hosting.calls().is_empty(), "No PR should be created for a local-only base");
}

#[test]
fn finish_work_branch_with_base_equal_to_current_errors() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    // The current branch trivially "exists", so without a dedicated guard this
    // would pass validation and fail later at `gh pr create` with head == base.
    git.existing_remote_branches.insert("feature/login".to_string());
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    let err = finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), Some("feature/login".to_string()), None).unwrap_err();

    assert!(err.contains("feature/login"), "Error should name the branch, got: {err}");
    assert!(hosting.calls().is_empty(), "No PR should be created when base == current");
}

#[test]
fn finish_work_branch_with_unknown_base_errors() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };

    let err = finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), Some("no-such-branch".to_string()), None).unwrap_err();

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
    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].starts_with("create_or_get_pr:feature/login:develop:"),
        "Single candidate should be auto-selected, got: {}", calls[1]);
}

#[test]
fn finish_work_branch_fix_breaking() {
    let mut git = MockGit::new();
    git.current_branch = "fix/auth".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Fix { name: "auth".to_string() };

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(true), None, None).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].ends_with(":fix!: auth"),
        "Expected 'fix!: auth', got: {}", calls[1]);
}

#[test]
fn finish_work_branch_passes_resolved_template_to_hosting() {
    let mut git = MockGit::new();
    git.current_branch = "feature/login".to_string();
    let hosting = MockHosting::new();
    let branch_type = BranchType::Feature { name: "login".to_string() };
    let template = std::path::Path::new(".github/pr-templates/bflow-feature.md");

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), None, Some(template)).unwrap();

    let calls = hosting.calls();
    assert!(calls[1].ends_with(":template=.github/pr-templates/bflow-feature.md"),
        "template path must reach the hosting platform verbatim, got: {}", calls[1]);
}

// --- Parent-branch candidate ordering (reachable now that prompting goes
// --- through the Prompter port; previously required a TTY) ---

/// Wire up a candidate: merge base with `current`, our distance since
/// divergence, and the candidate's own commit count since divergence.
fn add_candidate(git: &mut MockGit, current: &str, branch: &str, base: &str, ours: u32, theirs: u32) {
    git.merge_bases.insert((current.to_string(), branch.to_string()), base.to_string());
    git.rev_list_counts.insert((base.to_string(), current.to_string()), ours);
    git.rev_list_counts.insert((base.to_string(), branch.to_string()), theirs);
}

#[test]
fn parent_candidates_sorted_by_merge_distance_ascending() {
    let mut git = MockGit::new();
    git.current_branch = "feature/child".to_string();
    git.remote_branches = vec!["develop".to_string(), "feature/near".to_string()];
    // develop diverged 5 commits ago, feature/near only 2 — nearest first.
    add_candidate(&mut git, "feature/child", "develop", "base-d", 5, 0);
    add_candidate(&mut git, "feature/child", "feature/near", "base-n", 2, 0);
    let hosting = MockHosting::new();
    let prompter = MockPrompter::scripted(&[0]);
    let branch_type = BranchType::Feature { name: "child".to_string() };

    finish_work_branch(&git, &hosting, &prompter, &branch_type, Some(false), None, None).unwrap();

    assert_eq!(prompter.calls(), vec!["select:PR target branch:[feature/near, develop]"]);
    assert!(hosting.calls()[1].starts_with("create_or_get_pr:feature/child:feature/near:"),
        "choosing index 0 must target the nearest candidate, got: {}", hosting.calls()[1]);
}

#[test]
fn parent_candidates_tie_prefers_develop_then_alphabetical() {
    let mut git = MockGit::new();
    git.current_branch = "feature/child".to_string();
    git.remote_branches = vec![
        "feature/bbb".to_string(),
        "develop".to_string(),
        "feature/aaa".to_string(),
    ];
    // All three candidates at the same distance.
    add_candidate(&mut git, "feature/child", "feature/bbb", "base-b", 3, 0);
    add_candidate(&mut git, "feature/child", "develop", "base-d", 3, 0);
    add_candidate(&mut git, "feature/child", "feature/aaa", "base-a", 3, 0);
    let hosting = MockHosting::new();
    let prompter = MockPrompter::scripted(&[0]);
    let branch_type = BranchType::Feature { name: "child".to_string() };

    finish_work_branch(&git, &hosting, &prompter, &branch_type, Some(false), None, None).unwrap();

    assert_eq!(prompter.calls(),
        vec!["select:PR target branch:[develop, feature/aaa, feature/bbb]"]);
}

#[test]
fn parent_detection_excludes_child_branches_and_skips_menu_for_single_candidate() {
    let mut git = MockGit::new();
    git.current_branch = "feature/parent".to_string();
    git.remote_branches = vec!["develop".to_string(), "feature/stacked".to_string()];
    add_candidate(&mut git, "feature/parent", "develop", "base-d", 4, 0);
    // feature/stacked has MORE commits since divergence than we do — it
    // branched from us, so it must not be offered as a PR target.
    add_candidate(&mut git, "feature/parent", "feature/stacked", "base-s", 1, 6);
    let hosting = MockHosting::new();
    let prompter = MockPrompter::new(); // unscripted: any select would error
    let branch_type = BranchType::Feature { name: "parent".to_string() };

    finish_work_branch(&git, &hosting, &prompter, &branch_type, Some(false), None, None).unwrap();

    assert!(prompter.calls().is_empty(), "single surviving candidate must be auto-selected");
    assert!(hosting.calls()[1].starts_with("create_or_get_pr:feature/parent:develop:"),
        "child branch must be excluded, got: {}", hosting.calls()[1]);
}

// --- Already-merged PR: finish is complete, clean up instead of a new PR ---

use bflow::hosting::MergedPr;

fn merged(url: &str, sha: &str, base: &str) -> Option<MergedPr> {
    Some(MergedPr { url: url.to_string(), head_sha: sha.to_string(), base: base.to_string() })
}

#[test]
fn merged_pr_in_worktree_cleans_up_branch_and_worktree() {
    let mut git = MockGit::new();
    git.current_branch = "feature/task-a".to_string();
    git.head_sha = "abc123".to_string();
    git.linked_worktree = true;
    git.existing_remote_branches.insert("feature/task-a".to_string());
    let mut hosting = MockHosting::new();
    hosting.merged_pr = merged("https://github.com/org/repo/pull/49", "abc123", "develop");
    let branch_type = BranchType::Feature { name: "task-a".to_string() };

    // breaking=None + unscripted prompter: passing proves the breaking-changes
    // prompt (and parent detection) never ran on an already-finished branch.
    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, None, None, None).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "head_sha",
        // Remote deletion comes first: after remove_current_worktree the process
        // cwd is gone, so it must be the last git call.
        "remote_branch_exists:feature/task-a",
        "delete_branch_remote:feature/task-a",
        "is_linked_worktree",
        "detach_head",
        "delete_branch_local:feature/task-a",
        "remove_current_worktree",
    ]);
    assert_eq!(hosting.calls(), vec!["merged_pr:feature/task-a"], "no new PR may be created");
}

#[test]
fn merged_pr_in_plain_checkout_returns_to_base_and_deletes_branch() {
    let mut git = MockGit::new();
    git.current_branch = "feature/task-a".to_string();
    git.head_sha = "abc123".to_string();
    // Remote branch already auto-deleted by the platform after merge.
    let mut hosting = MockHosting::new();
    hosting.merged_pr = merged("https://github.com/org/repo/pull/49", "abc123", "develop");
    let branch_type = BranchType::Feature { name: "task-a".to_string() };

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, None, None, None).unwrap();

    assert_eq!(git.calls(), vec![
        "current_branch",
        "head_sha",
        "remote_branch_exists:feature/task-a",
        "is_linked_worktree",
        "checkout:develop",
        "ff_merge:origin/develop",
        "delete_branch_local:feature/task-a",
    ]);
    assert_eq!(hosting.calls(), vec!["merged_pr:feature/task-a"]);
}

#[test]
fn merged_pr_with_new_commits_since_merge_creates_a_new_pr() {
    let mut git = MockGit::new();
    git.current_branch = "feature/task-a".to_string();
    git.head_sha = "newwork456".to_string();
    let mut hosting = MockHosting::new();
    hosting.merged_pr = merged("https://github.com/org/repo/pull/49", "abc123", "develop");
    let branch_type = BranchType::Feature { name: "task-a".to_string() };

    finish_work_branch(&git, &hosting, &MockPrompter::new(), &branch_type, Some(false), None, None).unwrap();

    // New commits after the merge = new work: nothing may be deleted...
    assert!(!git.calls().iter().any(|c| c.starts_with("delete_") || c == "remove_current_worktree"),
        "diverged branch must not be cleaned up, got: {:?}", git.calls());
    // ...and the flow continues into a fresh PR.
    assert!(hosting.calls().iter().any(|c| c.starts_with("create_or_get_pr:feature/task-a:")),
        "a new PR should be created, got: {:?}", hosting.calls());
}

#[test]
fn merged_pr_cleans_up_release_fix_too() {
    let mut git = MockGit::new();
    git.current_branch = "release-fix/1.1.0/login-bug".to_string();
    git.head_sha = "abc123".to_string();
    git.linked_worktree = true;
    let mut hosting = MockHosting::new();
    hosting.merged_pr = merged("https://github.com/org/repo/pull/50", "abc123", "release/1.1.0");
    let branch_type = BranchType::ReleaseFix { major: 1, minor: 1, patch: 0, name: "login-bug".to_string() };

    finish_release_fix(&git, &hosting, &branch_type, None).unwrap();

    assert!(!git.calls().iter().any(|c| c.starts_with("push:")), "nothing to push on a finished branch");
    assert!(git.calls().contains(&"remove_current_worktree".to_string()));
    assert_eq!(hosting.calls(), vec!["merged_pr:release-fix/1.1.0/login-bug"]);
}
