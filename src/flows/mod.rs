pub mod start;
pub mod finish_work;
pub mod finish_release;
pub mod finish_hotfix;

use crate::git::Git;

/// Open `{prefix}/*` branches (e.g. `release/*`), in the order git returns them.
/// The glob itself can never match `{prefix}-fix/*` (`release-` ≠ `release/`);
/// the prefix filter keeps that guarantee for mocks, which return their
/// configured list regardless of the pattern.
pub(crate) fn branches_with_prefix(git: &dyn Git, prefix: &str) -> Result<Vec<String>, String> {
    let matching = git.list_branches_matching(&format!("{prefix}/*"))?;
    let with_slash = format!("{prefix}/");
    Ok(matching.into_iter().filter(|b| b.starts_with(&with_slash)).collect())
}

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

/// Delete the finished source branch locally and remotely, both idempotent.
/// Switches to `main` first when HEAD is still on the branch — a resume that
/// skipped the develop merge leaves it there, git refuses to delete the
/// checked-out branch, and `main` is always safe (the work is merged there).
pub(crate) fn delete_source_branch(git: &dyn Git, branch: &str) -> Result<(), String> {
    if git.current_branch()? == branch {
        git.checkout("main")?;
    }
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
