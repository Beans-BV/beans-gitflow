use std::process::{Command, ExitCode};

use clap::Parser;

use bflow::git::GitCli;
use bflow::git::Git;
use bflow::git::branch::BranchType;
use bflow::hosting::github::GitHub;
use bflow::hosting::HostingPlatform;
use bflow::menu::{self, Action};
use bflow::flows::{start, finish_work, finish_release, finish_hotfix};

#[derive(Parser)]
#[command(name = "bflow", version, about = "Beans GitFlow - customized gitflow workflow CLI")]
struct Cli {}

fn main() -> ExitCode {
    Cli::parse();

    if let Err(e) = run() {
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), String> {
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

    if branch_type != BranchType::Other && !git.is_working_tree_clean()? {
        return Err("Working tree is not clean. Commit or stash your changes first.".to_string());
    }

    // Pull latest changes into current branch
    git.merge(&format!("origin/{branch_name}"), &format!("chore: pull latest {branch_name}"))?;

    let action = menu::show_menu(&branch_type, &branch_name)?;

    match action {
        Action::StartWorkBranch { prefix, name, from } => {
            start::start_work_branch(&git, &prefix, &name, &from)?;
        }
        Action::StartRelease => {
            start::start_release(&git)?;
        }
        Action::StartReleaseFix => {
            start::start_release_fix(&git)?;
        }
        Action::StartHotfixFix => {
            start::start_hotfix_fix(&git)?;
        }
        Action::FinishWorkBranch => {
            finish_work::finish_work_branch(&git, &hosting, &branch_type)?;
        }
        Action::FinishReleaseFix => {
            let BranchType::ReleaseFix { major, minor, name, .. } = &branch_type else {
                unreachable!("FinishReleaseFix action only from ReleaseFix branch");
            };
            finish_work::finish_release_fix(&git, &hosting, *major, *minor, name)?;
        }
        Action::FinishHotfixFix => {
            let BranchType::HotfixFix { major, minor, patch, name, .. } = &branch_type else {
                unreachable!("FinishHotfixFix action only from HotfixFix branch");
            };
            finish_work::finish_hotfix_fix(&git, &hosting, *major, *minor, *patch, name)?;
        }
        Action::BumpVersion => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("BumpVersion action only from Release branch");
            };
            finish_release::bump_version(&git, major, minor)?;
        }
        Action::SyncWithDevelop => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("SyncWithDevelop action only from Release branch");
            };
            finish_release::sync_with_develop(&git, major, minor)?;
        }
        Action::FinishRelease => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("FinishRelease action only from Release branch");
            };
            finish_release::finish_release(&git, major, minor)?;
        }
        Action::FinishHotfix => {
            let BranchType::Hotfix { major, minor, patch } = branch_type else {
                unreachable!("FinishHotfix action only from Hotfix branch");
            };
            finish_hotfix::finish_hotfix(&git, major, minor, patch)?;
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
