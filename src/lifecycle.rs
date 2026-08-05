//! Cross-cutting lifecycle of a bflow invocation: resume lookup, action
//! resolution, the reject → stash → write-state → dispatch ordering contract,
//! state clearing, and the three-way stash-pop policy (see decisions.md,
//! State & Crash-Safety). Lives in the library — not `main.rs` — so the
//! crash-safety ordering is enforced by tests (`tests/lifecycle_test.rs`),
//! not by a comment. `main.rs` keeps only adapter construction and preflight.

use crate::action::Action;
use crate::cli::{resolve_action, Commands};
use crate::editor::Editor;
use crate::flows::{finish_hotfix, finish_release, finish_work, start};
use crate::git::branch::BranchType;
use crate::git::Git;
use crate::hosting::HostingPlatform;
use crate::mainline::resolve_main_branch;
use crate::menu;
use crate::prompt::Prompter;
use crate::repo_config::RepoConfig;
use crate::state::{current_timestamp, FinishKind, FinishState};
use crate::worktree::{WorktreeConfig, WorktreeContext};

pub fn run(
    git: &dyn Git,
    hosting: &dyn HostingPlatform,
    prompter: &dyn Prompter,
    editor: &dyn Editor,
    wt_config: &WorktreeConfig,
    command: Option<Commands>,
) -> Result<(), String> {
    let branch_name = git.current_branch()?;
    let git_dir = git.git_dir()?;

    // One-time upgrade of any pre-2.4 global state file into the per-branch folder.
    FinishState::migrate_legacy(&git_dir)?;

    let branch_type = BranchType::parse(&branch_name);

    let main_branch = resolve_main_branch(git)?;

    let identity = finish_identity(&branch_type);

    // Resume context: an in-progress finish only resumes when you are standing on
    // the source branch that started it. From develop/main/feature branches there
    // is no resume — bflow behaves normally — so a stalled finish never hijacks
    // other work. To continue after a conflict you switch back to the source
    // branch and re-run 'bflow finish'.
    let resume_state = match identity {
        Some((kind, major, minor, patch)) => FinishState::load(&git_dir, kind, major, minor, patch)?,
        None => None,
    };

    // Resolve the action up-front so we can decide whether to fetch / stash / etc.
    let action = resolve_action_with_state(command, prompter, &branch_type, &branch_name, resume_state.as_ref(), wt_config.enabled, &main_branch)?;

    // --abort short-circuits before any state-changing operation, even if the repo
    // is mid-merge — abort is itself a recovery action.
    if matches!(action, Action::AbortFinish) {
        return handle_abort(&git_dir, resume_state);
    }

    // Mid-merge / unmerged-paths preflight (all other paths).
    if git.is_mid_merge()? || git.has_unmerged_paths()? {
        return Err(unresolved_merge_message(resume_state.as_ref()));
    }

    println!("Fetching latest...");
    git.fetch()?;

    // Optional worktree flow: when enabled (and not opted out) for an eligible start,
    // treat it like --no-checkout so the current working tree is left untouched and the
    // new branch is free to be checked out in its own worktree.
    let worktree_active = wt_config.enabled && !action.no_worktree() && action.worktree_eligible();

    let no_checkout = action.no_checkout() || worktree_active;
    let is_finish_with_state = matches!(action, Action::FinishRelease | Action::FinishHotfix);
    let needs_stash = !no_checkout && branch_type != BranchType::Other && !git.is_working_tree_clean()?;

    // Reject dirty-tree finishes BEFORE any side effects (stash, state write).
    // Start actions and resumes get to stash/inherit; everything else must be clean.
    if needs_stash && !action.is_start() && resume_state.is_none() {
        return Err("Working tree is not clean. Commit your changes before finishing.".to_string());
    }

    // Stash if needed. On resume, inherit the prior stash ref from state.
    let stash_msg = if resume_state.is_some() {
        resume_state.as_ref().and_then(|s| s.stash_message.clone())
    } else if needs_stash {
        println!("Stashing uncommitted changes...");
        let msg = format!("bflow-finish:{branch_name}:{}", current_timestamp());
        git.stash_push_with_message(&msg)?;
        Some(msg)
    } else {
        None
    };

    // Write state file BEFORE the first side effect of a release/hotfix finish.
    if is_finish_with_state && resume_state.is_none() {
        let Some((kind, major, minor, patch)) = identity else {
            unreachable!("FinishRelease/FinishHotfix are only ever dispatched from their own release/hotfix branch, which always yields a finish identity");
        };
        FinishState {
            kind, major, minor, patch,
            started_at: current_timestamp(),
            stash_message: stash_msg.clone(),
        }.save(&git_dir)?;
    }

    let worktree = if worktree_active { Some(WorktreeContext { config: wt_config, editor }) } else { None };

    let result = run_flow(git, hosting, prompter, &branch_type, &branch_name, &action, no_checkout, worktree, resume_state.as_ref(), &main_branch);

    // Lifecycle: clear state on success of a release/hotfix finish. Both a fresh
    // finish and a resume run on the source branch, so its identity is available.
    if result.is_ok() && (is_finish_with_state || resume_state.is_some()) {
        if let Some((kind, major, minor, patch)) = identity {
            FinishState::clear(&git_dir, kind, major, minor, patch)?;
        }
    }

    // Stash pop policy:
    //   - On success: always pop (changes restored).
    //   - On failure of a release/hotfix finish: leave stash for resume.
    //   - On failure of any other action: pop (preserves prior bflow behavior of
    //     restoring the user's working tree even on errors).
    let keep_stash_for_resume = result.is_err() && (is_finish_with_state || resume_state.is_some());
    if let Some(msg) = &stash_msg {
        if keep_stash_for_resume {
            eprintln!("Your uncommitted changes remain stashed as '{msg}'. They will be restored on a successful 'bflow finish' resume (or after 'bflow finish --abort').");
        } else {
            println!("Restoring uncommitted changes...");
            match git.find_stash_by_message(msg) {
                Ok(Some(stash_ref)) => {
                    if let Err(e) = git.stash_pop_ref(&stash_ref) {
                        eprintln!("Warning: Failed to restore stashed changes: {e}");
                        eprintln!("Your changes are saved in git stash ({stash_ref}). Run 'git stash pop {stash_ref}' to restore them.");
                    }
                }
                Ok(None) => {} // already gone
                Err(e) => eprintln!("Warning: Could not look up stash by message: {e}"),
            }
        }
    }

    result
}

/// Decide which Action to run given the parsed command, current branch, and
/// any resume state. Resume state takes precedence over branch-based dispatch
/// for `bflow finish` (and the default interactive path) — a develop-merge
/// conflict leaves HEAD on develop, where the branch-eligibility check would
/// otherwise reject the resume with "Nothing to finish on this branch."
pub fn resolve_action_with_state(
    command: Option<Commands>,
    prompter: &dyn Prompter,
    branch_type: &BranchType,
    branch_name: &str,
    resume_state: Option<&FinishState>,
    worktree_enabled: bool,
    main_branch: &str,
) -> Result<Action, String> {
    // `--abort` wins unconditionally and never errors based on branch type.
    if let Some(Commands::Finish { abort: true, .. }) = &command {
        return Ok(Action::AbortFinish);
    }

    // For `bflow finish` (or the default interactive path), an in-progress finish
    // state takes precedence over branch-based dispatch. This state is only ever
    // present when standing on the source branch (resume is branch-scoped), so it
    // covers the case where a develop-merge conflict was resolved and the user has
    // switched back to the release/hotfix branch to continue.
    // An explicit --base never applies here: resume state only exists on
    // release/hotfix source branches (see finish_identity), whose finishes have a
    // fixed target. Skip the resume shortcut so resolve_action rejects the flag
    // instead of silently ignoring it.
    let has_explicit_base = matches!(&command, Some(Commands::Finish { base: Some(_), .. }));
    let is_finish_or_default = matches!(command, Some(Commands::Finish { .. }) | None);
    if is_finish_or_default && !has_explicit_base {
        if let Some(state) = resume_state {
            eprintln!(
                "↻ Resuming in-progress {} finish for {} (started_at={}). Use 'bflow finish --abort' to discard.",
                state.kind.as_str(),
                state.source_branch(),
                state.started_at,
            );
            return Ok(match state.kind {
                FinishKind::Release => Action::FinishRelease,
                FinishKind::Hotfix => Action::FinishHotfix,
            });
        }
    }

    // No resume state (or a non-finish command): fall through to normal dispatch.
    match command {
        None => menu::show_menu(prompter, branch_type, branch_name, main_branch),
        Some(cmd) => resolve_action(cmd, branch_type, worktree_enabled, main_branch),
    }
}

/// Map a source branch to the (kind, version) identity of its finish state.
/// Only release and hotfix branches carry finish state; everything else (develop,
/// main, feature/*) yields None and therefore never resumes.
fn finish_identity(branch_type: &BranchType) -> Option<(FinishKind, u32, u32, u32)> {
    match branch_type {
        BranchType::Release { major, minor, patch } => {
            Some((FinishKind::Release, *major, *minor, *patch))
        }
        BranchType::Hotfix { major, minor, patch } => {
            Some((FinishKind::Hotfix, *major, *minor, *patch))
        }
        _ => None,
    }
}

fn handle_abort(git_dir: &std::path::Path, state: Option<FinishState>) -> Result<(), String> {
    match state {
        None => {
            println!("No in-progress finish to abort.");
            Ok(())
        }
        Some(s) => {
            println!("Aborting in-progress {} finish for {} (started_at={}).",
                s.kind.as_str(), s.source_branch(), s.started_at);
            FinishState::clear(git_dir, s.kind, s.major, s.minor, s.patch)?;
            if let Some(msg) = &s.stash_message {
                println!("Your original uncommitted changes are still stashed as '{msg}'.");
                println!("Run 'git stash list' to find it, then 'git stash pop <ref>' to restore.");
            }
            Ok(())
        }
    }
}

fn unresolved_merge_message(resume_state: Option<&FinishState>) -> String {
    let mut msg = String::from(
        "Unresolved merge in progress. Resolve conflicts, run 'git commit', then re-run 'bflow finish'."
    );
    if let Some(s) = resume_state {
        msg.push_str(&format!(
            "\n(In-progress {} finish for {} is waiting for resume.)",
            s.kind.as_str(), s.source_branch(),
        ));
    }
    msg
}

#[allow(clippy::too_many_arguments)]
fn run_flow(
    git: &dyn Git,
    hosting: &dyn HostingPlatform,
    prompter: &dyn Prompter,
    branch_type: &BranchType,
    branch_name: &str,
    action: &Action,
    skip_current_branch_sync: bool,
    worktree: Option<WorktreeContext<'_>>,
    resume_state: Option<&FinishState>,
    main_branch: &str,
) -> Result<(), String> {
    // Fast-forward the current branch to origin when the flow will operate on
    // this checkout and we're not resuming (on resume the user may be on
    // main/develop after conflict resolution — syncing that branch is harmless
    // but produces noisy output).
    if !skip_current_branch_sync && resume_state.is_none() {
        if let Err(e) = git.ff_merge(&format!("origin/{branch_name}")) {
            if !e.contains("not something we can merge") {
                return Err(e);
            }
        }
    }

    match action {
        Action::StartWorkBranch { prefix, name, from, no_checkout, .. } => {
            start::start_work_branch(git, prefix, name, from, *no_checkout, worktree)?;
        }
        Action::StartRelease(release_type) => {
            // Task 16 threads the real RepoConfig read from .bflow/config; until
            // then every run behaves as free mode (today's behavior).
            start::start_release(git, prompter, hosting, None, &RepoConfig::default(), *release_type)?;
        }
        Action::StartReleaseFix { name, no_checkout, .. } => {
            start::start_release_fix(git, name, *no_checkout, worktree)?;
        }
        Action::StartHotfixFix { name, no_checkout, .. } => {
            start::start_hotfix_fix(git, name, *no_checkout, worktree, main_branch, None)?;
        }
        Action::FinishWorkBranch { breaking, base } => {
            let template = resolve_pr_template(git, branch_type)?;
            finish_work::finish_work_branch(git, hosting, prompter, branch_type, *breaking, base.clone(), template.as_deref())?;
        }
        Action::FinishReleaseFix => {
            let template = resolve_pr_template(git, branch_type)?;
            finish_work::finish_release_fix(git, hosting, branch_type, template.as_deref())?;
        }
        Action::FinishHotfixFix => {
            let template = resolve_pr_template(git, branch_type)?;
            finish_work::finish_hotfix_fix(git, hosting, branch_type, template.as_deref())?;
        }
        Action::FinishReleaseChore => {
            let template = resolve_pr_template(git, branch_type)?;
            finish_work::finish_release_chore(git, hosting, branch_type, template.as_deref())?;
        }
        Action::BumpVersion => {
            let BranchType::Release { major, minor, .. } = branch_type else {
                unreachable!("BumpVersion action only from Release branch");
            };
            // Task 16 wires the real hosting/script/config values through the
            // lifecycle; for now bump only ever runs the free-mode, no-script path.
            finish_release::bump_version(git, hosting, None, &RepoConfig::default(), *major, *minor)?;
        }
        Action::SyncWithDevelop => {
            let BranchType::Release { major, minor, .. } = branch_type else {
                unreachable!("SyncWithDevelop action only from Release branch");
            };
            finish_release::sync_with_develop(git, *major, *minor)?;
        }
        Action::FinishRelease => {
            // On resume, prefer the state's version (we may not be on the release branch).
            let (major, minor) = if let Some(s) = resume_state {
                (s.major, s.minor)
            } else {
                let BranchType::Release { major, minor, .. } = branch_type else {
                    unreachable!("FinishRelease action only from Release branch");
                };
                (*major, *minor)
            };
            finish_release::finish_release(git, major, minor, main_branch)?;
        }
        Action::FinishHotfix => {
            let (major, minor, patch) = if let Some(s) = resume_state {
                (s.major, s.minor, s.patch)
            } else {
                let BranchType::Hotfix { major, minor, patch } = branch_type else {
                    unreachable!("FinishHotfix action only from Hotfix branch");
                };
                (*major, *minor, *patch)
            };
            finish_hotfix::finish_hotfix(git, major, minor, patch, main_branch)?;
        }
        Action::AbortFinish => {
            unreachable!("AbortFinish is handled before run_flow");
        }
    }

    Ok(())
}

/// Resolve the PR template at the composition boundary, anchored to the repo
/// root — resolution keeps working from subdirectories, and flows never probe
/// the filesystem themselves (they receive the resolved path as a parameter).
fn resolve_pr_template(git: &dyn Git, branch_type: &BranchType) -> Result<Option<std::path::PathBuf>, String> {
    Ok(crate::hosting::template::resolve(&git.repo_root()?, branch_type))
}
