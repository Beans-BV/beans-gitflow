use bflow::menu::Action;

#[test]
fn start_actions_return_true() {
    let actions = vec![
        Action::StartWorkBranch { prefix: "feature".into(), name: "x".into(), from: "develop".into(), no_checkout: false, no_worktree: false },
        Action::StartRelease(None),
        Action::StartReleaseFix { name: "x".into(), no_checkout: false, no_worktree: false },
        Action::StartHotfixFix { name: "x".into(), no_checkout: false, no_worktree: false },
    ];
    for action in actions {
        assert!(action.is_start(), "Expected is_start() == true for {:?}", action);
    }
}

#[test]
fn finish_actions_return_false() {
    let actions: Vec<Action> = vec![
        Action::FinishWorkBranch { breaking: None, base: None },
        Action::FinishReleaseFix,
        Action::FinishRelease,
        Action::FinishHotfix,
        Action::FinishHotfixFix,
        Action::BumpVersion,
        Action::SyncWithDevelop,
    ];
    for action in actions {
        assert!(!action.is_start(), "Expected is_start() == false for {:?}", action);
    }
}

#[test]
fn no_checkout_returns_true_for_start_work_branch() {
    let action = Action::StartWorkBranch {
        prefix: "feature".into(),
        name: "x".into(),
        from: "develop".into(),
        no_checkout: true,
        no_worktree: false,
    };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_false_for_start_work_branch_default() {
    let action = Action::StartWorkBranch {
        prefix: "feature".into(),
        name: "x".into(),
        from: "develop".into(),
        no_checkout: false,
        no_worktree: false,
    };
    assert!(!action.no_checkout());
}

#[test]
fn no_checkout_returns_true_for_start_release_fix() {
    let action = Action::StartReleaseFix { name: "x".into(), no_checkout: true, no_worktree: false };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_true_for_start_hotfix_fix() {
    let action = Action::StartHotfixFix { name: "x".into(), no_checkout: true, no_worktree: false };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_false_for_non_start_actions() {
    let actions: Vec<Action> = vec![
        Action::StartRelease(None),
        Action::FinishWorkBranch { breaking: None, base: None },
        Action::FinishReleaseFix,
        Action::FinishRelease,
        Action::FinishHotfix,
        Action::FinishHotfixFix,
        Action::BumpVersion,
        Action::SyncWithDevelop,
    ];
    for action in actions {
        assert!(!action.no_checkout(), "Expected no_checkout() == false for {:?}", action);
    }
}

#[test]
fn worktree_eligible_true_for_named_work_branch_starts() {
    let actions = vec![
        Action::StartWorkBranch { prefix: "feature".into(), name: "x".into(), from: "develop".into(), no_checkout: false, no_worktree: false },
        Action::StartReleaseFix { name: "x".into(), no_checkout: false, no_worktree: false },
        Action::StartHotfixFix { name: "x".into(), no_checkout: false, no_worktree: false },
    ];
    for action in actions {
        assert!(action.worktree_eligible(), "Expected worktree_eligible() == true for {:?}", action);
    }
}

#[test]
fn worktree_eligible_false_for_start_release_and_finishes() {
    let actions: Vec<Action> = vec![
        Action::StartRelease(None),
        Action::FinishWorkBranch { breaking: None, base: None },
        Action::FinishRelease,
        Action::FinishHotfix,
        Action::BumpVersion,
    ];
    for action in actions {
        assert!(!action.worktree_eligible(), "Expected worktree_eligible() == false for {:?}", action);
    }
}

#[test]
fn no_worktree_reflects_the_flag() {
    let opted_out = Action::StartWorkBranch { prefix: "feature".into(), name: "x".into(), from: "develop".into(), no_checkout: false, no_worktree: true };
    assert!(opted_out.no_worktree());

    let default = Action::StartReleaseFix { name: "x".into(), no_checkout: false, no_worktree: false };
    assert!(!default.no_worktree());

    // Non-start actions never opt out (there's nothing to opt out of).
    assert!(!Action::FinishRelease.no_worktree());
}
