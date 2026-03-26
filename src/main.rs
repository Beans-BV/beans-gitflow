use std::process::{Command, ExitCode};

use clap::{Parser, Subcommand};

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

#[derive(Subcommand)]
enum Commands {
    /// Start a new branch
    Start {
        #[command(subcommand)]
        kind: StartKind,
    },
    /// Finish the current branch (infers action from branch type)
    Finish,
    /// Bump the patch version on the current release branch
    Bump,
    /// Sync the current release branch into develop
    Sync,
}

#[derive(Subcommand)]
enum StartKind {
    /// Start a new feature branch
    Feature {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
    },
    /// Start a new fix branch
    Fix {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
    },
    /// Start a new chore branch
    Chore {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
    },
    /// Start a new docs branch
    Docs {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
    },
    /// Start a new refactor branch
    Refactor {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
    },
    /// Start a new release branch (or resume existing)
    Release,
    /// Start a release fix branch (must be on a release branch)
    ReleaseFix {
        #[arg(long)]
        name: String,
    },
    /// Start a hotfix fix branch (must be on main or hotfix branch)
    HotfixFix {
        #[arg(long)]
        name: String,
    },
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

    if branch_type != BranchType::Other && !git.is_working_tree_clean()? {
        return Err("Working tree is not clean. Commit or stash your changes first.".to_string());
    }

    // Pull latest changes into current branch
    git.merge(&format!("origin/{branch_name}"), &format!("chore: pull latest {branch_name}"))?;

    let action = match command {
        None => menu::show_menu(&branch_type, &branch_name)?,
        Some(cmd) => resolve_action(cmd, &branch_type)?,
    };

    match action {
        Action::StartWorkBranch { prefix, name, from } => {
            start::start_work_branch(&git, &prefix, &name, &from)?;
        }
        Action::StartRelease => {
            start::start_release(&git)?;
        }
        Action::StartReleaseFix { name } => {
            start::start_release_fix(&git, &name)?;
        }
        Action::StartHotfixFix { name } => {
            start::start_hotfix_fix(&git, &name)?;
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

fn resolve_action(command: Commands, branch_type: &BranchType) -> Result<Action, String> {
    match command {
        Commands::Start { kind } => match kind {
            StartKind::Feature { name, base } => {
                menu::validate_branch_name(&name)?;
                Ok(Action::StartWorkBranch { prefix: "feature".to_string(), name, from: base })
            }
            StartKind::Fix { name, base } => {
                menu::validate_branch_name(&name)?;
                Ok(Action::StartWorkBranch { prefix: "fix".to_string(), name, from: base })
            }
            StartKind::Chore { name, base } => {
                menu::validate_branch_name(&name)?;
                Ok(Action::StartWorkBranch { prefix: "chore".to_string(), name, from: base })
            }
            StartKind::Docs { name, base } => {
                menu::validate_branch_name(&name)?;
                Ok(Action::StartWorkBranch { prefix: "docs".to_string(), name, from: base })
            }
            StartKind::Refactor { name, base } => {
                menu::validate_branch_name(&name)?;
                Ok(Action::StartWorkBranch { prefix: "refactor".to_string(), name, from: base })
            }
            StartKind::Release => Ok(Action::StartRelease),
            StartKind::ReleaseFix { name } => {
                menu::validate_branch_name(&name)?;
                if !matches!(branch_type, BranchType::Release { .. }) {
                    return Err("This command is only valid on a release branch.".to_string());
                }
                Ok(Action::StartReleaseFix { name })
            }
            StartKind::HotfixFix { name } => {
                menu::validate_branch_name(&name)?;
                if !matches!(branch_type, BranchType::Main | BranchType::Hotfix { .. }) {
                    return Err("This command is only valid on a main or hotfix branch.".to_string());
                }
                Ok(Action::StartHotfixFix { name })
            }
        },
        Commands::Finish => match branch_type {
            BranchType::Main | BranchType::Develop => {
                Err("Nothing to finish on this branch.".to_string())
            }
            BranchType::Feature { .. } | BranchType::Fix { .. } | BranchType::Chore { .. }
            | BranchType::Docs { .. } | BranchType::Refactor { .. } => {
                Ok(Action::FinishWorkBranch)
            }
            BranchType::Release { .. } => Ok(Action::FinishRelease),
            BranchType::ReleaseFix { .. } => Ok(Action::FinishReleaseFix),
            BranchType::Hotfix { .. } => Ok(Action::FinishHotfix),
            BranchType::HotfixFix { .. } => Ok(Action::FinishHotfixFix),
            BranchType::Other => {
                Err("Not on a recognized gitflow branch.".to_string())
            }
        },
        Commands::Bump => {
            if !matches!(branch_type, BranchType::Release { .. }) {
                return Err("This command is only valid on a release branch.".to_string());
            }
            Ok(Action::BumpVersion)
        }
        Commands::Sync => {
            if !matches!(branch_type, BranchType::Release { .. }) {
                return Err("This command is only valid on a release branch.".to_string());
            }
            Ok(Action::SyncWithDevelop)
        }
    }
}

fn check_command_exists(cmd: &str) -> Result<(), String> {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map_err(|_| format!("'{cmd}' is not installed or not in PATH."))?;
    Ok(())
}
