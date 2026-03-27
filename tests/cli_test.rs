use bflow::cli::{Commands, StartKind, StartOptions, resolve_action};
use bflow::git::branch::BranchType;
use bflow::menu::Action;

// --- Start work branch tests ---

#[test]
fn start_feature_returns_start_work_branch_action() {
    let cmd = Commands::Start { kind: StartKind::Feature { name: "login".to_string(), base: "develop".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { prefix, name, from, .. } if prefix == "feature" && name == "login" && from == "develop"));
}

#[test]
fn start_feature_with_custom_base() {
    let cmd = Commands::Start { kind: StartKind::Feature { name: "login".to_string(), base: "feature/auth".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Feature { name: "auth".to_string() };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { from, .. } if from == "feature/auth"));
}

#[test]
fn start_feature_rejects_invalid_name() {
    let cmd = Commands::Start { kind: StartKind::Feature { name: "bad..name".to_string(), base: "develop".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let result = resolve_action(cmd, &branch_type);
    assert!(result.is_err());
}

// --- Start release-fix tests ---

#[test]
fn start_release_fix_on_release_branch() {
    let cmd = Commands::Start { kind: StartKind::ReleaseFix { name: "broken-login".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Release { major: 1, minor: 2 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartReleaseFix { name, .. } if name == "broken-login"));
}

#[test]
fn start_release_fix_on_wrong_branch() {
    let cmd = Commands::Start { kind: StartKind::ReleaseFix { name: "fix".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "This command is only valid on a release branch.");
}

// --- Start hotfix-fix tests ---

#[test]
fn start_hotfix_fix_on_main_branch() {
    let cmd = Commands::Start { kind: StartKind::HotfixFix { name: "urgent".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Main;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartHotfixFix { name, .. } if name == "urgent"));
}

#[test]
fn start_hotfix_fix_on_hotfix_branch() {
    let cmd = Commands::Start { kind: StartKind::HotfixFix { name: "urgent".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Hotfix { major: 1, minor: 0, patch: 1 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartHotfixFix { name, .. } if name == "urgent"));
}

#[test]
fn start_hotfix_fix_on_wrong_branch() {
    let cmd = Commands::Start { kind: StartKind::HotfixFix { name: "fix".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "This command is only valid on a main or hotfix branch.");
}

// --- Finish tests ---

#[test]
fn finish_on_feature_branch() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::Feature { name: "login".to_string() };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::FinishWorkBranch));
}

#[test]
fn finish_on_release_branch() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::Release { major: 1, minor: 2 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::FinishRelease));
}

#[test]
fn finish_on_hotfix_branch() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::Hotfix { major: 1, minor: 0, patch: 1 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::FinishHotfix));
}

#[test]
fn finish_on_main_errors() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::Main;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "Nothing to finish on this branch.");
}

#[test]
fn finish_on_develop_errors() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::Develop;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "Nothing to finish on this branch.");
}

#[test]
fn finish_on_other_branch_errors() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::Other;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "Not on a recognized gitflow branch.");
}

// --- Additional start tests ---

#[test]
fn start_release_returns_start_release_action() {
    let cmd = Commands::Start { kind: StartKind::Release };
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartRelease));
}

#[test]
fn finish_on_release_fix_branch() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::ReleaseFix { major: 1, minor: 2, name: "broken-login".to_string() };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::FinishReleaseFix));
}

#[test]
fn finish_on_hotfix_fix_branch() {
    let cmd = Commands::Finish;
    let branch_type = BranchType::HotfixFix { major: 1, minor: 0, patch: 1, name: "urgent".to_string() };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::FinishHotfixFix));
}

#[test]
fn start_fix_returns_start_work_branch_action() {
    let cmd = Commands::Start { kind: StartKind::Fix { name: "bug".to_string(), base: "develop".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { prefix, name, .. } if prefix == "fix" && name == "bug"));
}

#[test]
fn start_chore_returns_start_work_branch_action() {
    let cmd = Commands::Start { kind: StartKind::Chore { name: "deps".to_string(), base: "develop".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { prefix, name, .. } if prefix == "chore" && name == "deps"));
}

#[test]
fn start_docs_returns_start_work_branch_action() {
    let cmd = Commands::Start { kind: StartKind::Docs { name: "readme".to_string(), base: "develop".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { prefix, name, .. } if prefix == "docs" && name == "readme"));
}

#[test]
fn start_refactor_returns_start_work_branch_action() {
    let cmd = Commands::Start { kind: StartKind::Refactor { name: "cleanup".to_string(), base: "develop".to_string(), opts: StartOptions { no_checkout: false } } };
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { prefix, name, .. } if prefix == "refactor" && name == "cleanup"));
}

// --- Bump and Sync tests ---

#[test]
fn bump_on_release_branch() {
    let cmd = Commands::Bump;
    let branch_type = BranchType::Release { major: 1, minor: 2 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::BumpVersion));
}

#[test]
fn bump_on_wrong_branch() {
    let cmd = Commands::Bump;
    let branch_type = BranchType::Develop;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "This command is only valid on a release branch.");
}

#[test]
fn sync_on_release_branch() {
    let cmd = Commands::Sync;
    let branch_type = BranchType::Release { major: 1, minor: 2 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::SyncWithDevelop));
}

#[test]
fn sync_on_wrong_branch() {
    let cmd = Commands::Sync;
    let branch_type = BranchType::Develop;
    let result = resolve_action(cmd, &branch_type);
    assert_eq!(result.unwrap_err(), "This command is only valid on a release branch.");
}

#[test]
fn start_feature_with_no_checkout_flag() {
    let cmd = Commands::Start { kind: StartKind::Feature {
        name: "login".to_string(),
        base: "develop".to_string(),
        opts: StartOptions { no_checkout: true },
    }};
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { no_checkout: true, .. }));
}

#[test]
fn start_release_fix_with_no_checkout_flag() {
    let cmd = Commands::Start { kind: StartKind::ReleaseFix {
        name: "broken-login".to_string(),
        opts: StartOptions { no_checkout: true },
    }};
    let branch_type = BranchType::Release { major: 1, minor: 2 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartReleaseFix { no_checkout: true, .. }));
}

#[test]
fn start_hotfix_fix_with_no_checkout_flag() {
    let cmd = Commands::Start { kind: StartKind::HotfixFix {
        name: "urgent".to_string(),
        opts: StartOptions { no_checkout: true },
    }};
    let branch_type = BranchType::Main;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartHotfixFix { no_checkout: true, .. }));
}
