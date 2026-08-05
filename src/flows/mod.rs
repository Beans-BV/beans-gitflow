pub mod start;
pub mod finish_work;
pub mod finish_release;
pub mod finish_hotfix;

use std::path::Path;

use crate::git::Git;
use crate::hosting::{HostingPlatform, LandedPr};
use crate::version::SemVer;
use crate::version_script::VersionScript;

// --- Idempotent finish steps -----------------------------------------------
// Shared scaffolding of the release/hotfix finish flows: each step is guarded
// by a real-state predicate and prints a visible "↷ skipped:" line on resume
// (see decisions.md, Resume & Idempotency). Commit messages and conflict help
// stay caller-supplied — wording is per-flow UX, not shared knowledge. The two
// flows themselves are deliberately NOT unified: the RC gate, release
// propagation, and messaging genuinely differ.

/// Merge `source` into `target` (checkout, ff-sync to origin, no-ff merge),
/// skipped entirely when `source` is already an ancestor of `target`.
pub(crate) fn merge_into(git: &dyn Git, source: &str, target: &str, message: &str, conflict_help: &str) -> Result<(), String> {
    if git.is_ancestor(source, target)? {
        println!("↷ skipped: merge into {target} (already merged)");
        return Ok(());
    }
    println!("Merging into {target}...");
    git.checkout(target)?;
    git.ff_merge(&format!("origin/{target}"))?;
    git.merge(source, message).map_err(|e| format!("{e}\n{conflict_help}"))
}

/// Push `branch` unless origin already has its HEAD.
pub(crate) fn push_if_needed(git: &dyn Git, branch: &str) -> Result<(), String> {
    if git.is_pushed(branch)? {
        println!("↷ skipped: push {branch} (already up to date)");
        return Ok(());
    }
    git.push(branch)
}

/// Create annotated tag `tag` unless it already exists locally.
pub(crate) fn tag_if_missing(git: &dyn Git, tag: &str, message: &str) -> Result<(), String> {
    if git.tag_exists(tag)? {
        println!("↷ skipped: tag {tag} (already exists)");
        return Ok(());
    }
    println!("Tagging: {tag}");
    git.create_tag(tag, message)
}

/// Push `tag` unless origin already has it.
pub(crate) fn push_tag_if_missing(git: &dyn Git, tag: &str) -> Result<(), String> {
    if git.remote_tag_exists(tag)? {
        println!("↷ skipped: push tag {tag} (already pushed)");
        return Ok(());
    }
    git.push_tag(tag)
}

// --- Protected-mode landing helpers ----------------------------------------
// bflow never merges a PR (SKILL.md principle: protected mode never pushes
// main/develop) — a landing step opens a PR and stops; a human merges it, and
// the next run picks up from there. Completion is derived from the hosting
// platform the same way `finish_work.rs::try_cleanup_merged` derives it, but
// keyed by a specific (source, target) pair rather than "any merge".

/// Whether `source` has already landed into `target` via a merged PR. `None`
/// also covers "the PR merged, but source moved on since": new commits after
/// the merge are new work, so the caller re-enters its own gate rather than
/// trusting a stale merge (compares `branch_sha`, not `head_sha`, so this
/// reads the named branch regardless of what HEAD currently is).
pub(crate) fn landed_pr(git: &dyn Git, hosting: &dyn HostingPlatform, source: &str, target: &str) -> Result<Option<LandedPr>, String> {
    let Some(pr) = hosting.merged_pr_to(source, target)? else {
        return Ok(None);
    };
    if git.branch_sha(source)? != pr.head_sha {
        println!("PR {} was merged, but {source} has new commits since — opening a new PR.", pr.url);
        return Ok(None);
    }
    Ok(Some(pr))
}

/// Reconcile `source` with its remote, then open (or reuse) its landing PR
/// into `target`. Protected-mode source branches are typically already
/// pushed from an earlier run, so this generalizes `finish_work.rs`'s
/// unconditional push to a branch that may be missing on the remote, ahead,
/// behind, or diverged.
pub(crate) fn open_landing_pr(git: &dyn Git, hosting: &dyn HostingPlatform, source: &str, target: &str, title: &str, template: Option<&Path>) -> Result<String, String> {
    let origin = format!("origin/{source}");
    if !git.remote_branch_exists(source)? {
        git.push(source)?;
    } else if !git.is_pushed(source)? {
        if git.is_ancestor(&origin, source)? {
            git.push(source)?;
        } else if git.is_ancestor(source, &origin)? {
            return Err(format!("Local {source} is behind origin/{source}. Run 'git pull --ff-only', then re-run."));
        } else {
            return Err(format!("Local {source} and origin/{source} have diverged. Reconcile them (e.g. 'git pull --rebase'), then re-run."));
        }
    }
    hosting.create_or_get_pr(source, target, title, template.and_then(|p| p.to_str()))
}

/// Cut `tag` at `sha` (a PR's merge commit) unless it already exists — and
/// when it does, verify it points at that same commit rather than trusting a
/// stale or hand-created tag (a mismatch is fatal, not silently skipped).
pub(crate) fn tag_at_if_missing(git: &dyn Git, tag: &str, message: &str, sha: &str) -> Result<(), String> {
    if !git.tag_exists(tag)? {
        println!("Tagging: {tag}");
        return git.create_tag_at(tag, message, sha);
    }
    let actual = git.tag_commit_sha(tag)?;
    if actual == sha {
        println!("↷ skipped: tag {tag} (already exists)");
        Ok(())
    } else {
        Err(format!(
            "Tag {tag} exists but points at {actual}, not the PR merge commit {sha}. \
             Move or delete the tag, then re-run 'bflow finish'."
        ))
    }
}

/// Delete `branch` locally and remotely, each guarded by an existence check so
/// re-running after a partial cleanup is a no-op.
pub(crate) fn delete_branch_guarded(git: &dyn Git, branch: &str) -> Result<(), String> {
    if git.local_branch_exists(branch)? {
        git.delete_branch_local(branch)?;
    } else {
        println!("↷ skipped: delete local {branch} (already gone)");
    }
    if git.remote_branch_exists(branch)? {
        git.delete_branch_remote(branch)?;
    } else {
        println!("↷ skipped: delete remote {branch} (already gone)");
    }
    Ok(())
}

/// Delete the finished source branch locally and remotely, both idempotent.
/// Switches to the mainline first when HEAD is still on the branch — a resume
/// that skipped the develop merge leaves it there, git refuses to delete the
/// checked-out branch, and the mainline is always safe (the work is merged there).
pub(crate) fn delete_source_branch(git: &dyn Git, branch: &str, main_branch: &str) -> Result<(), String> {
    if git.current_branch()? == branch {
        git.checkout(main_branch)?;
    }
    delete_branch_guarded(git, branch)
}

/// Fail with the catalog error unless the tree is clean. Callers run this
/// before a version script ever runs (trap 2) — the script must never see
/// pre-existing local changes it did not make.
pub(crate) fn require_clean_tree(git: &dyn Git) -> Result<(), String> {
    if git.is_working_tree_clean()? {
        Ok(())
    } else {
        Err("Working tree is not clean. Commit or stash your changes, then re-run.".to_string())
    }
}

/// Run the version script and commit whatever it changed. Returns `Ok(true)`
/// when a commit was made, `Ok(false)` when the script left the tree clean
/// (nothing to commit). Callers must call `require_clean_tree` first — kept
/// as two functions because protected bump creates a branch between the
/// check and the run.
pub(crate) fn run_version_script(git: &dyn Git, script: &dyn VersionScript, version: &SemVer) -> Result<bool, String> {
    let release = version.to_release();
    script.run(&release.to_string())?;
    if git.is_working_tree_clean()? {
        return Ok(false);
    }
    git.stage_all()?;
    git.commit(&format!("chore: set version {release}"))?;
    Ok(true)
}

/// List `{prefix}/*` branches that are still open, excluding any whose clean
/// release already shipped. A release/hotfix branch is shipped once its clean
/// tag (e.g. `v1.1.0`, never the `-rc.N` tag) exists — reusing it would make
/// `bflow start` loop onto a dead branch forever and hotfix fan-out merge into
/// history that already landed. Branches whose version does not parse stay in,
/// unchanged from today's behavior.
pub(crate) fn open_versioned_branches(git: &dyn Git, prefix: &str) -> Result<Vec<String>, String> {
    let branches = git.list_branches_matching(&format!("{prefix}/*"))?;
    let mut open = Vec::with_capacity(branches.len());
    for branch in branches {
        let shipped = match branch.strip_prefix(&format!("{prefix}/")).and_then(SemVer::parse) {
            Some(version) => git.tag_exists(&version.to_release().tag_name())?,
            None => false,
        };
        if !shipped {
            open.push(branch);
        }
    }
    Ok(open)
}

/// Guidance appended to a merge conflict during a release/hotfix finish.
///
/// Resume is branch-scoped: bflow only continues an interrupted finish when you
/// are standing on its source branch. A merge conflict usually leaves HEAD on the
/// target branch (e.g. develop), so the user must switch back before re-running.
pub(crate) fn resume_hint(source_branch: &str) -> String {
    format!(
        "Resolve the conflict and commit the merge, then switch back to the source \
         branch and re-run 'bflow finish' to continue:\n    \
         git switch {source_branch}\n    bflow finish"
    )
}
