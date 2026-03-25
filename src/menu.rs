use dialoguer::{Select, Input, theme::ColorfulTheme};
use crate::git::branch::BranchType;

#[derive(Debug, Clone, Copy)]
pub enum DevelopOption {
    StartFeature, StartFix, StartChore, StartDocs, StartRefactor, StartReleaseFix,
}

impl DevelopOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StartFeature => "start feature",
            Self::StartFix => "start fix",
            Self::StartChore => "start chore",
            Self::StartDocs => "start docs",
            Self::StartRefactor => "start refactor",
            Self::StartReleaseFix => "start release fix",
        }
    }

    pub fn branch_prefix(&self) -> &'static str {
        match self {
            Self::StartFeature => "feature",
            Self::StartFix => "fix",
            Self::StartChore => "chore",
            Self::StartDocs => "docs",
            Self::StartRefactor => "refactor",
            Self::StartReleaseFix => unreachable!(),
        }
    }

    const ALL: [Self; 6] = [Self::StartFeature, Self::StartFix, Self::StartChore, Self::StartDocs, Self::StartRefactor, Self::StartReleaseFix];
}

#[derive(Debug, Clone, Copy)]
pub enum WorkBranchOption {
    Finish, StartFeature, StartFix, StartChore, StartDocs, StartRefactor,
}

impl WorkBranchOption {
    pub fn label(&self, branch_type: &str) -> String {
        match self {
            Self::Finish => format!("finish {branch_type}"),
            Self::StartFeature => "start feature".to_string(),
            Self::StartFix => "start fix".to_string(),
            Self::StartChore => "start chore".to_string(),
            Self::StartDocs => "start docs".to_string(),
            Self::StartRefactor => "start refactor".to_string(),
        }
    }

    pub fn branch_prefix(&self) -> &'static str {
        match self {
            Self::StartFeature => "feature",
            Self::StartFix => "fix",
            Self::StartChore => "chore",
            Self::StartDocs => "docs",
            Self::StartRefactor => "refactor",
            Self::Finish => unreachable!(),
        }
    }

    const ALL: [Self; 6] = [Self::Finish, Self::StartFeature, Self::StartFix, Self::StartChore, Self::StartDocs, Self::StartRefactor];
}

#[derive(Debug, Clone, Copy)]
pub enum ReleaseOption {
    BumpVersion, SyncWithDevelop, FinishRelease,
}

impl ReleaseOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::BumpVersion => "bump version",
            Self::SyncWithDevelop => "sync with develop",
            Self::FinishRelease => "finish release",
        }
    }

    const ALL: [Self; 3] = [Self::BumpVersion, Self::SyncWithDevelop, Self::FinishRelease];
}

pub fn show_select(prompt: &str, items: &[&str]) -> Result<usize, String> {
    Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()
        .map_err(|e| format!("Menu error: {e}"))
}

pub fn prompt_name(prompt: &str) -> Result<String, String> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .validate_with(|input: &String| -> std::result::Result<(), String> {
            if input.is_empty() {
                return Err("Name cannot be empty".to_string());
            }
            if input.contains(' ') || input.contains("..") || input.contains('~') || input.contains('^') || input.contains(':') || input.contains('\\') {
                return Err("Invalid branch name. Avoid spaces and special characters (.. ~ ^ : \\)".to_string());
            }
            Ok(())
        })
        .interact_text()
        .map_err(|e| format!("Input error: {e}"))?;
    Ok(name)
}

pub fn show_menu(branch_type: &BranchType, current_branch: &str) -> Result<Action, String> {
    match branch_type {
        BranchType::Main => {
            let labels = &["start hotfix fix"];
            show_select("What would you like to do?", labels)?;
            Ok(Action::StartHotfixFix)
        }
        BranchType::Develop => {
            let labels: Vec<&str> = DevelopOption::ALL.iter().map(|o| o.label()).collect();
            let idx = show_select("What would you like to do?", &labels)?;
            let option = DevelopOption::ALL[idx];
            match option {
                DevelopOption::StartReleaseFix => Ok(Action::StartReleaseFix),
                other => {
                    let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
                    Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from: "develop".to_string() })
                }
            }
        }
        BranchType::Feature { .. } | BranchType::Fix { .. } | BranchType::Chore { .. }
        | BranchType::Docs { .. } | BranchType::Refactor { .. } => {
            let branch_type_label = match branch_type {
                BranchType::Feature { .. } => "feature",
                BranchType::Fix { .. } => "fix",
                BranchType::Chore { .. } => "chore",
                BranchType::Docs { .. } => "docs",
                BranchType::Refactor { .. } => "refactor",
                _ => unreachable!(),
            };
            let labels: Vec<String> = WorkBranchOption::ALL.iter().map(|o| o.label(branch_type_label)).collect();
            let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let idx = show_select("What would you like to do?", &label_refs)?;
            let option = WorkBranchOption::ALL[idx];
            match option {
                WorkBranchOption::Finish => Ok(Action::FinishWorkBranch),
                other => {
                    let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
                    Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from: current_branch.to_string() })
                }
            }
        }
        BranchType::ReleaseFix { .. } => Ok(Action::FinishReleaseFix),
        BranchType::Release { .. } => {
            let labels: Vec<&str> = ReleaseOption::ALL.iter().map(|o| o.label()).collect();
            let idx = show_select("What would you like to do?", &labels)?;
            match ReleaseOption::ALL[idx] {
                ReleaseOption::BumpVersion => Ok(Action::BumpVersion),
                ReleaseOption::SyncWithDevelop => Ok(Action::SyncWithDevelop),
                ReleaseOption::FinishRelease => Ok(Action::FinishRelease),
            }
        }
        BranchType::HotfixFix { .. } => Ok(Action::FinishHotfixFix),
        BranchType::Hotfix { .. } => Ok(Action::FinishHotfix),
        BranchType::Other => Err("Not on a recognized gitflow branch. Switch to main or develop first.".to_string()),
    }
}

#[derive(Debug)]
pub enum Action {
    StartWorkBranch { prefix: String, name: String, from: String },
    StartReleaseFix,
    StartHotfixFix,
    FinishWorkBranch,
    FinishReleaseFix,
    FinishRelease,
    FinishHotfix,
    FinishHotfixFix,
    BumpVersion,
    SyncWithDevelop,
}
