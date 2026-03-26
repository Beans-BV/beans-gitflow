use clap::Subcommand;
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
    Finish,
    /// Bump the patch version on the current release branch
    Bump,
    /// Sync the current release branch into develop
    Sync,
}

#[derive(Subcommand)]
pub enum StartKind {
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

pub fn resolve_action(command: Commands, branch_type: &BranchType) -> Result<Action, String> {
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
