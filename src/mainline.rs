//! Which branch is this repo's mainline: `main` or `master`.
//!
//! Resolved ONCE at the composition root and threaded into the flows as data —
//! the same shape as `hosting/detect.rs` (detect at composition, pass the
//! result), and for the same reason: flows stay pure functions of their inputs
//! and every test can name the mainline it wants without a global.
//!
//! Only `main` and `master` are supported. Arbitrary names are a present-tense
//! non-requirement, and keeping the set closed is what lets `BranchType::parse`
//! keep its two-arm acceptance.

use crate::git::Git;

/// Git config key holding the repo's mainline branch name.
///
/// Written in **local** scope — the mainline is a property of the repository,
/// not of the developer. (Opposite default from `bflow.worktree.*`, which is a
/// personal preference and therefore global.)
pub const MAIN_BRANCH_KEY: &str = "bflow.branch.main";

const SUPPORTED: [&str; 2] = ["main", "master"];

/// The repo's mainline branch: the configured value if set, otherwise detected
/// once and persisted so later runs are a single config read.
pub fn resolve_main_branch(git: &dyn Git) -> Result<String, String> {
    if let Some(configured) = git.get_config(MAIN_BRANCH_KEY)? {
        let configured = configured.trim();
        if !configured.is_empty() {
            if !SUPPORTED.contains(&configured) {
                return Err(format!(
                    "Unsupported mainline branch '{configured}' in {MAIN_BRANCH_KEY}. \
                     bflow supports 'main' or 'master'. \
                     Fix it with 'git config {MAIN_BRANCH_KEY} main'."
                ));
            }
            return Ok(configured.to_string());
        }
    }

    let detected = detect(git)?;
    git.set_config(MAIN_BRANCH_KEY, &detected, false)?;
    println!("Detected mainline branch '{detected}' (saved to {MAIN_BRANCH_KEY}).");
    Ok(detected)
}

/// `main` if it exists locally or on the remote, else `master` on the same
/// terms, else `main` — a repo with neither branch is about to create one, and
/// `main` is the modern default.
fn detect(git: &dyn Git) -> Result<String, String> {
    for candidate in SUPPORTED {
        if git.local_branch_exists(candidate)? || git.remote_branch_exists(candidate)? {
            return Ok(candidate.to_string());
        }
    }
    Ok("main".to_string())
}
