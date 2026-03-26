mod common;

use common::MockGit;
use bflow::git::Git;

#[test]
fn stash_push_records_call() {
    let git = MockGit::new();
    git.stash_push().unwrap();
    assert_eq!(git.calls(), vec!["stash_push"]);
}

#[test]
fn stash_pop_records_call() {
    let git = MockGit::new();
    git.stash_pop().unwrap();
    assert_eq!(git.calls(), vec!["stash_pop"]);
}
