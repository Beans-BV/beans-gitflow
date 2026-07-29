mod common;

use std::path::PathBuf;

use common::{tmp_dir, MockEditor, MockGit, MockHosting, MockPrompter};
use bflow::action::Action;
use bflow::cli::Commands;
use bflow::git::branch::BranchType;
use bflow::lifecycle::{resolve_action_with_state, run};
use bflow::state::{FinishKind, FinishState};
use bflow::worktree::WorktreeConfig;

// The lifecycle (reject → stash → write-state → dispatch → clear/pop) used to
// live in main.rs, where tests/ could not link it — the system's most
// safety-critical ordering was enforced by a comment. These tests pin it.

fn wt_config() -> WorktreeConfig {
    WorktreeConfig { enabled: false, editor: "code".to_string(), base_path: None }
}

fn finish_cmd() -> Option<Commands> {
    Some(Commands::Finish { breaking: None, base: None, abort: false })
}

/// A MockGit standing on release/2.5.0 with one RC tag and a fake `.git` dir,
/// ready for a `bflow finish`.
fn release_git() -> MockGit {
    let mut git = MockGit::new();
    git.current_branch = "release/2.5.0".to_string();
    git.git_dir = tmp_dir("bflow-lifecycle-test");
    git.tags_on_branch = vec!["v2.5.0-rc.1".to_string()];
    git.existing_local_branches.insert("release/2.5.0".to_string());
    git.existing_remote_branches.insert("release/2.5.0".to_string());
    git
}

fn state_path(git_dir: &PathBuf) -> PathBuf {
    FinishState::path(git_dir, FinishKind::Release, 2, 5, 0)
}

fn run_lifecycle(git: &MockGit, command: Option<Commands>) -> Result<(), String> {
    let hosting = MockHosting::new();
    let prompter = MockPrompter::new();
    let editor = MockEditor::new();
    run(git, &hosting, &prompter, &editor, &wt_config(), command)
}

// --- State-before-mutation ordering ---

#[test]
fn crashed_finish_leaves_resumable_state_on_disk() {
    let mut git = release_git();
    git.fail_nth_merge = Some(1); // conflict on the main merge, mid-flow

    let result = run_lifecycle(&git, finish_cmd());

    assert!(result.is_err(), "merge conflict must surface");
    let state = FinishState::load(&git.git_dir, FinishKind::Release, 2, 5, 0)
        .unwrap()
        .expect("state must have been written BEFORE the first mutating git call, so a crash mid-flow leaves a resumable record");
    assert_eq!(state.source_branch(), "release/2.5.0");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

#[test]
fn successful_finish_clears_state() {
    let git = release_git();

    run_lifecycle(&git, finish_cmd()).unwrap();

    assert!(!state_path(&git.git_dir).exists(),
        "state must be cleared after a successful finish");
    assert!(git.calls().iter().any(|c| c.starts_with("merge:release/2.5.0:")),
        "the finish flow must actually have run");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

#[test]
fn dirty_tree_finish_rejected_before_any_side_effect() {
    let mut git = release_git();
    git.working_tree_clean = false;

    let err = run_lifecycle(&git, finish_cmd()).unwrap_err();

    assert!(err.contains("Working tree is not clean"), "got: {err}");
    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("stash_push_with_message")),
        "nothing may be stashed before the reject; calls: {calls:?}");
    assert!(!state_path(&git.git_dir).exists(),
        "no state may be written before the reject");
    assert!(!calls.iter().any(|c| c.starts_with("merge:")),
        "no mutation may run; calls: {calls:?}");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

#[test]
fn mid_merge_preflight_blocks_and_names_pending_resume() {
    let mut git = release_git();
    git.mid_merge = true;
    // An in-progress finish is waiting.
    FinishState {
        kind: FinishKind::Release, major: 2, minor: 5, patch: 0,
        started_at: "1".to_string(), stash_ref: None,
    }.save(&git.git_dir).unwrap();

    let err = run_lifecycle(&git, finish_cmd()).unwrap_err();

    assert!(err.contains("Unresolved merge in progress"), "got: {err}");
    assert!(err.contains("release/2.5.0"), "must name the waiting finish; got: {err}");
    assert!(!git.calls().iter().any(|c| c == "fetch"),
        "preflight must block before fetch");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

// --- Stash policy (dirty start: stash before mutation, pop on success) ---

#[test]
fn dirty_start_stashes_before_mutating_and_pops_on_success() {
    let mut git = MockGit::new();
    git.current_branch = "develop".to_string();
    git.git_dir = tmp_dir("bflow-lifecycle-test");
    git.working_tree_clean = false;
    let start = {
        use bflow::cli::{StartKind, StartOptions};
        Some(Commands::Start { kind: StartKind::Feature {
            name: "login".to_string(),
            base: "develop".to_string(),
            opts: StartOptions { no_checkout: false, no_worktree: false },
        }})
    };

    run_lifecycle(&git, start).unwrap();

    let calls = git.calls();
    let stash_idx = calls.iter().position(|c| c.starts_with("stash_push_with_message:bflow-finish:develop:"))
        .expect("dirty start must stash");
    let create_idx = calls.iter().position(|c| c.starts_with("create_branch:feature/login:"))
        .expect("branch must be created");
    let pop_idx = calls.iter().position(|c| c.starts_with("stash_pop_ref:"))
        .expect("stash must be popped on success");
    assert!(stash_idx < create_idx, "stash must precede the first mutation; calls: {calls:?}");
    assert!(pop_idx > create_idx, "pop must follow the flow; calls: {calls:?}");
    assert!(!state_path(&git.git_dir).exists(),
        "start actions never write finish state");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

// --- Resume ---

#[test]
fn resume_runs_flow_from_state_and_clears_it_on_success() {
    // Fully-completed world: resume should re-drive the flow (all steps skip),
    // then clear the state file.
    let mut git = release_git();
    git.ancestors.insert(("release/2.5.0".to_string(), "main".to_string()));
    git.ancestors.insert(("release/2.5.0".to_string(), "develop".to_string()));
    git.existing_tags.insert("v2.5.0".to_string());
    git.existing_remote_tags.insert("v2.5.0".to_string());
    git.pushed_branches.insert("main".to_string());
    git.pushed_branches.insert("develop".to_string());
    FinishState {
        kind: FinishKind::Release, major: 2, minor: 5, patch: 0,
        started_at: "1".to_string(), stash_ref: None,
    }.save(&git.git_dir).unwrap();

    run_lifecycle(&git, finish_cmd()).unwrap();

    let calls = git.calls();
    assert!(!calls.iter().any(|c| c.starts_with("merge:")),
        "resume of a completed finish must not re-merge; calls: {calls:?}");
    assert!(!state_path(&git.git_dir).exists(),
        "state must be cleared after a successful resume");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

#[test]
fn failed_resume_keeps_state_for_the_next_attempt() {
    let mut git = release_git();
    git.fail_nth_merge = Some(1);
    FinishState {
        kind: FinishKind::Release, major: 2, minor: 5, patch: 0,
        started_at: "1".to_string(), stash_ref: None,
    }.save(&git.git_dir).unwrap();

    let result = run_lifecycle(&git, finish_cmd());

    assert!(result.is_err());
    assert!(state_path(&git.git_dir).exists(),
        "a failed resume must keep the state file for the next attempt");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

// --- Abort ---

#[test]
fn abort_clears_state_without_touching_the_repo() {
    let mut git = release_git();
    git.mid_merge = true; // abort must work even mid-merge
    FinishState {
        kind: FinishKind::Release, major: 2, minor: 5, patch: 0,
        started_at: "1".to_string(), stash_ref: Some("bflow-finish:release/2.5.0:1".to_string()),
    }.save(&git.git_dir).unwrap();

    run_lifecycle(&git, Some(Commands::Finish { breaking: None, base: None, abort: true })).unwrap();

    assert!(!state_path(&git.git_dir).exists(), "abort must discard the state file");
    let calls = git.calls();
    assert!(!calls.iter().any(|c| c == "fetch"), "abort must not fetch");
    assert!(!calls.iter().any(|c| c.starts_with("merge:") || c.starts_with("checkout:")
            || c.starts_with("stash_pop_ref:")),
        "abort must not mutate the repo or auto-pop the stash; calls: {calls:?}");
    std::fs::remove_dir_all(&git.git_dir).ok();
}

// --- Action resolution (moved from main.rs with the lifecycle) ---

fn release_state() -> FinishState {
    FinishState {
        kind: FinishKind::Release,
        major: 2,
        minor: 5,
        patch: 0,
        started_at: "0".to_string(),
        stash_ref: None,
    }
}

#[test]
fn resume_state_resumes_finish_without_base() {
    let branch_type = BranchType::Release { major: 2, minor: 5, patch: 0 };
    let state = release_state();
    let cmd = Some(Commands::Finish { breaking: None, base: None, abort: false });

    let action = resolve_action_with_state(cmd, &branch_type, "release/2.5.0", Some(&state), false).unwrap();

    assert!(matches!(action, Action::FinishRelease));
}

#[test]
fn explicit_base_rejected_even_when_resume_state_exists() {
    // Regression: the fixed-target --base guard lives in resolve_action, which the
    // resume early-return used to skip — silently swallowing an invalid --base.
    let branch_type = BranchType::Release { major: 2, minor: 5, patch: 0 };
    let state = release_state();
    let cmd = Some(Commands::Finish { breaking: None, base: Some("develop".to_string()), abort: false });

    let err = resolve_action_with_state(cmd, &branch_type, "release/2.5.0", Some(&state), false).unwrap_err();

    assert!(err.contains("--base"), "Expected the fixed-target --base error, got: {err}");
}
