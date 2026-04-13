use bflow::git::branch::BranchType;

#[test]
fn parse_main() {
    assert_eq!(BranchType::parse("main"), BranchType::Main);
    assert_eq!(BranchType::parse("master"), BranchType::Main);
}

#[test]
fn parse_develop() {
    assert_eq!(BranchType::parse("develop"), BranchType::Develop);
}

#[test]
fn parse_feature() {
    assert_eq!(BranchType::parse("feature/passkey-login"), BranchType::Feature { name: "passkey-login".to_string() });
}

#[test]
fn parse_fix() {
    assert_eq!(BranchType::parse("fix/profile-loading"), BranchType::Fix { name: "profile-loading".to_string() });
}

#[test]
fn parse_chore() {
    assert_eq!(BranchType::parse("chore/update-ci"), BranchType::Chore { name: "update-ci".to_string() });
}

#[test]
fn parse_docs() {
    assert_eq!(BranchType::parse("docs/api-reference"), BranchType::Docs { name: "api-reference".to_string() });
}

#[test]
fn parse_refactor() {
    assert_eq!(BranchType::parse("refactor/auth-module"), BranchType::Refactor { name: "auth-module".to_string() });
}

#[test]
fn parse_release() {
    assert_eq!(BranchType::parse("release/2.6.0"), BranchType::Release { major: 2, minor: 6, patch: 0 });
}

#[test]
fn parse_release_fix() {
    assert_eq!(BranchType::parse("release-fix/2.6.0/payment-error"), BranchType::ReleaseFix { major: 2, minor: 6, patch: 0, name: "payment-error".to_string() });
}

#[test]
fn parse_hotfix() {
    assert_eq!(BranchType::parse("hotfix/2.6.1"), BranchType::Hotfix { major: 2, minor: 6, patch: 1 });
}

#[test]
fn parse_hotfix_fix() {
    assert_eq!(BranchType::parse("hotfix-fix/2.6.1/incorrect-dto"), BranchType::HotfixFix { major: 2, minor: 6, patch: 1, name: "incorrect-dto".to_string() });
}

#[test]
fn parse_other() {
    assert_eq!(BranchType::parse("some-random-branch"), BranchType::Other);
}
