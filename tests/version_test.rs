use bflow::version::SemVer;

#[test]
fn parse_semver() {
    assert_eq!(SemVer::parse("2.5.3"), Some(SemVer::new(2, 5, 3)));
    assert_eq!(SemVer::parse("v2.5.3"), Some(SemVer::new(2, 5, 3)));
    assert_eq!(SemVer::parse("not-a-version"), None);
    assert_eq!(SemVer::parse("1.2"), None);
}

#[test]
fn bump_minor() {
    let v = SemVer::new(2, 5, 3);
    assert_eq!(v.bump_minor(), SemVer::new(2, 6, 0));
}

#[test]
fn bump_patch() {
    let v = SemVer::new(2, 5, 3);
    assert_eq!(v.bump_patch(), SemVer::new(2, 5, 4));
}

#[test]
fn semver_display() {
    let v = SemVer::new(2, 6, 0);
    assert_eq!(v.to_string(), "2.6.0");
}

#[test]
fn semver_ordering() {
    let versions = vec![SemVer::new(1, 0, 0), SemVer::new(2, 5, 3), SemVer::new(2, 5, 1), SemVer::new(2, 6, 0)];
    let mut sorted = versions.clone();
    sorted.sort();
    assert_eq!(sorted, vec![SemVer::new(1, 0, 0), SemVer::new(2, 5, 1), SemVer::new(2, 5, 3), SemVer::new(2, 6, 0)]);
}

#[test]
fn release_branch_name() {
    let v = SemVer::new(2, 6, 0);
    assert_eq!(v.release_branch(), "release/2.6");
}

#[test]
fn hotfix_branch_name() {
    let v = SemVer::new(2, 5, 4);
    assert_eq!(v.hotfix_branch(), "hotfix/2.5.4");
}
