mod common;

use common::MockGit;
use bflow::git::Git;

// Pins the mock's own recording contract (see decisions.md, Testing Strategy):
// the call string format is part of every exact-sequence expectation.

#[test]
fn create_branch_no_checkout_records_call() {
    let git = MockGit::new();
    git.create_branch_no_checkout("feature/test", "develop").unwrap();
    assert_eq!(git.calls(), vec!["create_branch_no_checkout:feature/test:develop"]);
}
