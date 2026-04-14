use clap::{Args, Subcommand};
use crate::git::branch::BranchType;
use crate::menu::{self, Action};

#[derive(Subcommand)]
pub enum Commands {
    /// Start a new branch
    Start {
        #[command(subcommand)]
        kind: StartKind,
    },
    /// Finish the current branch (infers action from branch type)
    Finish {
        /// Mark PR as containing breaking changes (adds ! to commit type).
        /// Omit to prompt interactively. Use --breaking (true) or --breaking=false
        /// to skip the prompt non-interactively.
        #[arg(long, num_args = 0..=1, default_missing_value = "true")]
        breaking: Option<bool>,
    },
    /// Bump the patch version on the current release branch
    Bump,
    /// Sync the current release branch into develop
    Sync,
}

#[derive(Args, Debug, Clone)]
pub struct StartOptions {
    /// Create and push the branch without checking it out
    #[arg(long)]
    pub no_checkout: bool,
}

#[derive(Subcommand)]
pub enum StartKind {
    /// Start a new feature branch
    Feature {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new fix branch
    Fix {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new chore branch
    Chore {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new docs branch
    Docs {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new refactor branch
    Refactor {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new release branch (or resume existing)
    Release,
    /// Start a release fix branch (must be on a release branch)
    ReleaseFix {
        #[arg(long)]
        name: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a hotfix fix branch (must be on main or hotfix branch)
    HotfixFix {
        #[arg(long)]
        name: String,
        #[command(flatten)]
        opts: StartOptions,
    },
}

fn start_work_branch(prefix: &str, name: String, base: String, no_checkout: bool) -> Result<Action, String> {
    menu::validate_branch_name(&name)?;
    Ok(Action::StartWorkBranch { prefix: prefix.to_string(), name, from: base, no_checkout })
}

fn require_release_branch(branch_type: &BranchType) -> Result<(), String> {
    if !matches!(branch_type, BranchType::Release { .. }) {
        return Err("This command is only valid on a release branch.".to_string());
    }
    Ok(())
}

pub fn resolve_action(command: Commands, branch_type: &BranchType) -> Result<Action, String> {
    match command {
        Commands::Start { kind } => match kind {
            StartKind::Feature { name, base, opts } => start_work_branch("feature", name, base, opts.no_checkout),
            StartKind::Fix { name, base, opts } => start_work_branch("fix", name, base, opts.no_checkout),
            StartKind::Chore { name, base, opts } => start_work_branch("chore", name, base, opts.no_checkout),
            StartKind::Docs { name, base, opts } => start_work_branch("docs", name, base, opts.no_checkout),
            StartKind::Refactor { name, base, opts } => start_work_branch("refactor", name, base, opts.no_checkout),
            StartKind::Release => Ok(Action::StartRelease),
            StartKind::ReleaseFix { name, opts } => {
                menu::validate_branch_name(&name)?;
                if !opts.no_checkout {
                    require_release_branch(branch_type)?;
                }
                Ok(Action::StartReleaseFix { name, no_checkout: opts.no_checkout })
            }
            StartKind::HotfixFix { name, opts } => {
                menu::validate_branch_name(&name)?;
                if !opts.no_checkout && !matches!(branch_type, BranchType::Main | BranchType::Hotfix { .. }) {
                    return Err("This command is only valid on a main or hotfix branch.".to_string());
                }
                Ok(Action::StartHotfixFix { name, no_checkout: opts.no_checkout })
            }
        },
        Commands::Finish { breaking } => match branch_type {
            BranchType::Main | BranchType::Develop => {
                Err("Nothing to finish on this branch.".to_string())
            }
            BranchType::Feature { .. } | BranchType::Fix { .. } | BranchType::Chore { .. }
            | BranchType::Docs { .. } | BranchType::Refactor { .. } => {
                Ok(Action::FinishWorkBranch { breaking })
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
            require_release_branch(branch_type)?;
            Ok(Action::BumpVersion)
        }
        Commands::Sync => {
            require_release_branch(branch_type)?;
            Ok(Action::SyncWithDevelop)
        }
    }
}
