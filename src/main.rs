use std::process::{Command, ExitCode};

use clap::Parser;

use bflow::cli::{Commands, resolve_action};
use bflow::git::GitCli;
use bflow::git::Git;
use bflow::git::branch::BranchType;
use bflow::hosting::github::GitHub;
use bflow::hosting::HostingPlatform;
use bflow::menu::{self, Action};
use bflow::flows::{start, finish_work, finish_release, finish_hotfix};

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
    check_command_exists("gh")?;

    let git = GitCli::new();
    let hosting = GitHub::new();

    hosting.check_auth().map_err(|e| {
        format!("GitHub CLI is not authenticated. Run 'gh auth login' first.\n{e}")
    })?;

    let branch_name = git.current_branch().map_err(|_| {
        "Not in a git repository.".to_string()
    })?;

    println!("Fetching latest...");
    git.fetch()?;

    let branch_type = BranchType::parse(&branch_name);

    let stashed = if branch_type != BranchType::Other && !git.is_working_tree_clean()? {
        println!("Stashing uncommitted changes...");
        git.stash_push()?;
        true
    } else {
        false
    };

    let result = run_flow(&git, &hosting, &branch_type, &branch_name, command, stashed);

    if stashed {
        println!("Restoring uncommitted changes...");
        if let Err(e) = git.stash_pop() {
            eprintln!("Warning: Failed to restore stashed changes: {e}");
            eprintln!("Your changes are saved in git stash. Run 'git stash pop' to restore them.");
        }
    }

    result
}

fn run_flow(
    git: &GitCli,
    hosting: &GitHub,
    branch_type: &BranchType,
    branch_name: &str,
    command: Option<Commands>,
    stashed: bool,
) -> Result<(), String> {
    // Pull latest changes into current branch
    git.merge(&format!("origin/{branch_name}"), &format!("chore: pull latest {branch_name}"))?;

    let action = match command {
        None => menu::show_menu(branch_type, branch_name)?,
        Some(cmd) => resolve_action(cmd, branch_type)?,
    };

    if stashed && !action.is_start() {
        return Err("Working tree is not clean. Commit your changes before finishing.".to_string());
    }

    match action {
        Action::StartWorkBranch { prefix, name, from, no_checkout } => {
            start::start_work_branch(git, &prefix, &name, &from, no_checkout)?;
        }
        Action::StartRelease => {
            start::start_release(git)?;
        }
        Action::StartReleaseFix { name, no_checkout } => {
            start::start_release_fix(git, &name, no_checkout)?;
        }
        Action::StartHotfixFix { name, no_checkout } => {
            start::start_hotfix_fix(git, &name, no_checkout)?;
        }
        Action::FinishWorkBranch => {
            finish_work::finish_work_branch(git, hosting, branch_type)?;
        }
        Action::FinishReleaseFix => {
            let BranchType::ReleaseFix { major, minor, name, .. } = branch_type else {
                unreachable!("FinishReleaseFix action only from ReleaseFix branch");
            };
            finish_work::finish_release_fix(git, hosting, *major, *minor, name)?;
        }
        Action::FinishHotfixFix => {
            let BranchType::HotfixFix { major, minor, patch, name, .. } = branch_type else {
                unreachable!("FinishHotfixFix action only from HotfixFix branch");
            };
            finish_work::finish_hotfix_fix(git, hosting, *major, *minor, *patch, name)?;
        }
        Action::BumpVersion => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("BumpVersion action only from Release branch");
            };
            finish_release::bump_version(git, *major, *minor)?;
        }
        Action::SyncWithDevelop => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("SyncWithDevelop action only from Release branch");
            };
            finish_release::sync_with_develop(git, *major, *minor)?;
        }
        Action::FinishRelease => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("FinishRelease action only from Release branch");
            };
            finish_release::finish_release(git, *major, *minor)?;
        }
        Action::FinishHotfix => {
            let BranchType::Hotfix { major, minor, patch } = branch_type else {
                unreachable!("FinishHotfix action only from Hotfix branch");
            };
            finish_hotfix::finish_hotfix(git, *major, *minor, *patch)?;
        }
    }

    Ok(())
}

fn check_command_exists(cmd: &str) -> Result<(), String> {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map_err(|_| format!("'{cmd}' is not installed or not in PATH."))?;
    Ok(())
}
