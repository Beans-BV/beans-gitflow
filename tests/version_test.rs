use bflow::version::SemVer;

#[test]
fn parse_semver() {
    assert_eq!(SemVer::parse("2.5.3"), Some(SemVer::new(2, 5, 3)));
    assert_eq!(SemVer::parse("v2.5.3"), Some(SemVer::new(2, 5, 3)));
    assert_eq!(SemVer::parse("not-a-version"), None);
    assert_eq!(SemVer::parse("1.2"), None);
}

#[test]
fn parse_semver_with_pre_release() {
    let v = SemVer::parse("1.2.0-rc.1").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 0);
    assert!(v.is_pre_release());
    assert_eq!(v.pre.as_ref().unwrap().label, "rc");
    assert_eq!(v.pre.as_ref().unwrap().number, 1);
}

#[test]
fn parse_semver_with_v_prefix_and_pre_release() {
    let v = SemVer::parse("v1.2.0-rc.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 0);
    assert_eq!(v.pre.as_ref().unwrap().number, 3);
}

#[test]
fn parse_rejects_empty_pre_release_label() {
    assert_eq!(SemVer::parse("1.2.0-.1"), None);
}

#[test]
fn parse_rejects_leading_zeros_in_pre_release() {
    assert_eq!(SemVer::parse("1.2.0-rc.01"), None);
    assert_eq!(SemVer::parse("1.2.0-rc.00"), None);
    // Single zero is valid
    assert!(SemVer::parse("1.2.0-rc.0").is_some());
}

#[test]
fn parse_rejects_pre_release_without_number() {
    assert_eq!(SemVer::parse("1.2.0-rc"), None);
}

#[test]
fn bump_major() {
    let v = SemVer::new(2, 5, 3);
    assert_eq!(v.bump_major(), SemVer::new(3, 0, 0));
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
fn bump_rc() {
    let v = SemVer::new(1, 2, 0).with_rc(1);
    let bumped = v.bump_rc();
    assert_eq!(bumped.to_string(), "1.2.0-rc.2");
}

#[test]
fn bump_rc_on_clean_version() {
    let v = SemVer::new(1, 2, 0);
    let bumped = v.bump_rc();
    assert_eq!(bumped.to_string(), "1.2.0-rc.1");
}

#[test]
fn is_rc() {
    assert!(SemVer::new(1, 2, 0).with_rc(1).is_rc());
    assert!(!SemVer::new(1, 2, 0).is_rc());
    // Non-rc pre-release
    let beta = SemVer::parse("1.2.0-beta.1").unwrap();
    assert!(!beta.is_rc());
}

#[test]
fn with_rc() {
    let v = SemVer::new(1, 2, 0).with_rc(3);
    assert_eq!(v.to_string(), "1.2.0-rc.3");
}

#[test]
fn to_release() {
    let v = SemVer::new(1, 2, 0).with_rc(3);
    let release = v.to_release();
    assert_eq!(release.to_string(), "1.2.0");
    assert!(!release.is_pre_release());
}

#[test]
fn tag_name() {
    assert_eq!(SemVer::new(1, 2, 0).tag_name(), "v1.2.0");
    assert_eq!(SemVer::new(1, 2, 0).with_rc(1).tag_name(), "v1.2.0-rc.1");
}

#[test]
fn semver_display() {
    assert_eq!(SemVer::new(2, 6, 0).to_string(), "2.6.0");
    assert_eq!(SemVer::new(2, 6, 0).with_rc(1).to_string(), "2.6.0-rc.1");
}

#[test]
fn semver_ordering() {
    let versions = vec![
        SemVer::new(1, 0, 0),
        SemVer::new(2, 5, 3),
        SemVer::new(2, 5, 1),
        SemVer::new(2, 6, 0),
    ];
    let mut sorted = versions.clone();
    sorted.sort();
    assert_eq!(sorted, vec![
        SemVer::new(1, 0, 0),
        SemVer::new(2, 5, 1),
        SemVer::new(2, 5, 3),
        SemVer::new(2, 6, 0),
    ]);
}

#[test]
fn semver_ordering_pre_release_before_release() {
    let rc1 = SemVer::new(1, 2, 0).with_rc(1);
    let rc2 = SemVer::new(1, 2, 0).with_rc(2);
    let release = SemVer::new(1, 2, 0);
    let mut versions = vec![release.clone(), rc1.clone(), rc2.clone()];
    versions.sort();
    assert_eq!(versions, vec![rc1, rc2, release]);
}

#[test]
fn release_branch_name() {
    assert_eq!(SemVer::new(2, 6, 0).release_branch(), "release/2.6.0");
}

#[test]
fn hotfix_branch_name() {
    assert_eq!(SemVer::new(2, 5, 4).hotfix_branch(), "hotfix/2.5.4");
    assert_eq!(SemVer::new(2, 5, 4).with_rc(1).hotfix_branch(), "hotfix/2.5.4");
}
