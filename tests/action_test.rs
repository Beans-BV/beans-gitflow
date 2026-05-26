use bflow::menu::Action;

#[test]
fn start_actions_return_true() {
    let actions = vec![
        Action::StartWorkBranch { prefix: "feature".into(), name: "x".into(), from: "develop".into(), no_checkout: false },
        Action::StartRelease(None),
        Action::StartReleaseFix { name: "x".into(), no_checkout: false },
        Action::StartHotfixFix { name: "x".into(), no_checkout: false },
    ];
    for action in actions {
        assert!(action.is_start(), "Expected is_start() == true for {:?}", action);
    }
}

#[test]
fn finish_actions_return_false() {
    let actions: Vec<Action> = vec![
        Action::FinishWorkBranch { breaking: None },
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
    };
    assert!(!action.no_checkout());
}

#[test]
fn no_checkout_returns_true_for_start_release_fix() {
    let action = Action::StartReleaseFix { name: "x".into(), no_checkout: true };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_true_for_start_hotfix_fix() {
    let action = Action::StartHotfixFix { name: "x".into(), no_checkout: true };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_false_for_non_start_actions() {
    let actions: Vec<Action> = vec![
        Action::StartRelease(None),
        Action::FinishWorkBranch { breaking: None },
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
