use std::process::{Command, ExitCode};

use clap::Parser;

use bflow::cli::{Commands, WorktreeAction, resolve_action};
use bflow::git::GitCli;
use bflow::git::Git;
use bflow::git::branch::BranchType;
use bflow::hosting::detect::{self, Provider};
use bflow::hosting::devops::AzureDevOps;
use bflow::hosting::github::GitHub;
use bflow::hosting::HostingPlatform;
use bflow::action::Action;
use bflow::menu::{self, MenuPrompter};
use bflow::prompt::Prompter;
use bflow::flows::{start, finish_work, finish_release, finish_hotfix};
use bflow::state::{FinishState, FinishKind, current_timestamp};
use bflow::editor::{CommandEditor, Editor};
use bflow::worktree::{self, WorktreeConfig, WorktreeContext};

#[derive(Parser)]
#[command(name = "bflow", version, about = "Beans GitFlow - customized gitflow workflow CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(command: Option<Commands>) -> Result<(), String> {
    check_command_exists("git")?;
    let git = GitCli::new();

    // `bflow worktree` only reads/writes git config — no gh, auth, fetch, or branch
    // context needed. Dispatch it here and return before the branch-flow machinery.
    let command = match command {
        Some(Commands::Worktree { action, local }) => {
            return run_worktree_config(&git, action, local);
        }
        other => other,
    };

    // Provider detection reads the origin remote, so the repo check comes first.
    let branch_name = git.current_branch().map_err(|_| {
        "Not in a git repository.".to_string()
    })?;

    let hosting = create_hosting(&git)?;
    let git_dir = git.git_dir()?;

    // One-time upgrade of any pre-2.4 global state file into the per-branch folder.
    FinishState::migrate_legacy(&git_dir)?;

    let branch_type = BranchType::parse(&branch_name);

    // Resume context: an in-progress finish only resumes when you are standing on
    // the source branch that started it. From develop/main/feature branches there
    // is no resume — bflow behaves normally — so a stalled finish never hijacks
    // other work. To continue after a conflict you switch back to the source
    // branch and re-run 'bflow finish'.
    let resume_state = match finish_identity(&branch_type) {
        Some((kind, major, minor, patch)) => FinishState::load(&git_dir, kind, major, minor, patch)?,
        None => None,
    };

    // Load the worktree config BEFORE resolving the action: the release-fix/
    // hotfix-fix branch-type gate must know whether the worktree flow (which
    // auto-discovers the target branch, like --no-checkout) will apply.
    let wt_config = WorktreeConfig::load(&git)?;

    // Resolve the action up-front so we can decide whether to fetch / stash / etc.
    let action = resolve_action_with_state(command, &branch_type, &branch_name, resume_state.as_ref(), wt_config.enabled)?;

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
    let editor = CommandEditor::new(wt_config.editor.clone());
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
        resume_state.as_ref().and_then(|s| s.stash_ref.clone())
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
        write_state_for_action(&action, &branch_type, &git_dir, stash_msg.clone())?;
    }

    let prompter = MenuPrompter;
    let result = run_flow(&git, &*hosting, &prompter, &branch_type, &branch_name, &action, no_checkout, worktree_active, &wt_config, &editor, resume_state.as_ref());

    // Lifecycle: clear state on success of a release/hotfix finish. Both a fresh
    // finish and a resume run on the source branch, so its identity is available.
    if result.is_ok() && (is_finish_with_state || resume_state.is_some()) {
        if let Some((kind, major, minor, patch)) = finish_identity(&branch_type) {
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
fn resolve_action_with_state(
    command: Option<Commands>,
    branch_type: &BranchType,
    branch_name: &str,
    resume_state: Option<&FinishState>,
    worktree_enabled: bool,
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
        None => menu::show_menu(branch_type, branch_name),
        Some(cmd) => resolve_action(cmd, branch_type, worktree_enabled),
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

fn write_state_for_action(
    action: &Action,
    branch_type: &BranchType,
    git_dir: &std::path::Path,
    stash_ref: Option<String>,
) -> Result<(), String> {
    // The action decides *whether* state is written; the branch supplies the
    // identity via the same finish_identity mapping the resume lookup uses,
    // so the two can never encode the branch→identity rule differently.
    let expected_kind = match action {
        Action::FinishRelease => FinishKind::Release,
        Action::FinishHotfix => FinishKind::Hotfix,
        _ => return Ok(()),
    };
    let Some((kind, major, minor, patch)) = finish_identity(branch_type) else {
        return Ok(());
    };
    if kind != expected_kind {
        return Ok(());
    }
    FinishState { kind, major, minor, patch, started_at: current_timestamp(), stash_ref }.save(git_dir)
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
            if let Some(msg) = &s.stash_ref {
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
    git: &GitCli,
    hosting: &dyn HostingPlatform,
    prompter: &dyn Prompter,
    branch_type: &BranchType,
    branch_name: &str,
    action: &Action,
    skip_current_branch_sync: bool,
    worktree_active: bool,
    wt_config: &WorktreeConfig,
    editor: &dyn Editor,
    resume_state: Option<&FinishState>,
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
            let wt = if worktree_active { Some(WorktreeContext { config: wt_config, editor }) } else { None };
            start::start_work_branch(git, prefix, name, from, *no_checkout, wt)?;
        }
        Action::StartRelease(release_type) => {
            start::start_release(git, prompter, *release_type)?;
        }
        Action::StartReleaseFix { name, no_checkout, .. } => {
            let wt = if worktree_active { Some(WorktreeContext { config: wt_config, editor }) } else { None };
            start::start_release_fix(git, name, *no_checkout, wt)?;
        }
        Action::StartHotfixFix { name, no_checkout, .. } => {
            let wt = if worktree_active { Some(WorktreeContext { config: wt_config, editor }) } else { None };
            start::start_hotfix_fix(git, name, *no_checkout, wt)?;
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
        Action::BumpVersion => {
            let BranchType::Release { major, minor, .. } = branch_type else {
                unreachable!("BumpVersion action only from Release branch");
            };
            finish_release::bump_version(git, *major, *minor)?;
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
            finish_release::finish_release(git, major, minor)?;
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
            finish_hotfix::finish_hotfix(git, major, minor, patch)?;
        }
        Action::AbortFinish => {
            unreachable!("AbortFinish is handled before run_flow");
        }
    }

    Ok(())
}

/// Resolve the PR template at the composition root, anchored to the repo root —
/// resolution keeps working from subdirectories, and flows never probe the
/// filesystem themselves (they receive the resolved path as a parameter).
fn resolve_pr_template(git: &dyn Git, branch_type: &BranchType) -> Result<Option<std::path::PathBuf>, String> {
    Ok(bflow::hosting::template::resolve(&git.repo_root()?, branch_type))
}

/// Detect the hosting provider for this repo and return a ready-to-use,
/// preflighted (CLI installed + authenticated) hosting backend.
fn create_hosting(git: &dyn Git) -> Result<Box<dyn HostingPlatform>, String> {
    match detect::detect(git)? {
        Provider::GitHub => {
            check_command_exists("gh")?;
            let hosting = GitHub::new();
            hosting.check_auth().map_err(|e| {
                format!("GitHub CLI is not authenticated. Run 'gh auth login' first.\n{e}")
            })?;
            Ok(Box::new(hosting))
        }
        Provider::AzureDevOps { org, project, repo } => {
            check_command_exists("az")?;
            let hosting = AzureDevOps::new(org, project, repo);
            hosting.check_auth().map_err(|e| {
                format!("Azure CLI is not ready for Azure DevOps. Run 'az login' (or 'az devops login' with a PAT).\n{e}")
            })?;
            Ok(Box::new(hosting))
        }
    }
}

fn run_worktree_config(git: &GitCli, action: Option<WorktreeAction>, local: bool) -> Result<(), String> {
    match action {
        None => worktree::wizard(git, local),
        Some(WorktreeAction::Enable) => worktree::set_enabled(git, true, local),
        Some(WorktreeAction::Disable) => worktree::set_enabled(git, false, local),
        Some(WorktreeAction::Editor { value }) => worktree::set_editor(git, &value, local),
        Some(WorktreeAction::Path { value }) => worktree::set_path(git, &value, local),
        Some(WorktreeAction::Status) => worktree::show_status(git),
    }
}

fn check_command_exists(cmd: &str) -> Result<(), String> {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map_err(|_| format!("'{cmd}' is not installed or not in PATH."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
