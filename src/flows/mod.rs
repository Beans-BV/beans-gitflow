pub mod start;
pub mod finish_work;
pub mod finish_release;
pub mod finish_hotfix;

use std::path::{Path, PathBuf};

use crate::git::Git;
use crate::hosting::{HostingPlatform, LandedPr};
use crate::repo_config::{BumpStrategy, Mode, RepoConfig};
use crate::version::{finish_branch_name, SemVer};
use crate::version_script::VersionScript;

// --- Idempotent finish steps -----------------------------------------------
// Shared scaffolding of the release/hotfix finish flows: each step is guarded
// by a real-state predicate and prints a visible "↷ skipped:" line on resume
// (see decisions.md, Resume & Idempotency). Commit messages and conflict help
// stay caller-supplied — wording is per-flow UX, not shared knowledge. The two
// flows themselves are deliberately NOT unified: the RC gate, release
// propagation, and messaging genuinely differ.

/// Merge `source` into `target` (ff-sync to origin, then no-ff merge), skipped
/// entirely when `source` is already an ancestor of `target`.
pub(crate) fn merge_into(git: &dyn Git, source: &str, target: &str, message: &str, conflict_help: &str) -> Result<(), String> {
    if git.is_ancestor(source, target)? {
        println!("↷ skipped: merge into {target} (already merged)");
        return Ok(());
    }
    println!("Merging into {target}...");
    merge_where_checked_out(git, source, target, message).map_err(|e| format!("{e}\n{conflict_help}"))
}

/// The merge itself, run in whichever working tree already has `target`
/// checked out — git refuses to check out a branch held by another worktree —
/// and here, after a checkout, when no tree holds it. Refuses to touch another
/// tree that has uncommitted changes.
pub(crate) fn merge_where_checked_out(git: &dyn Git, source: &str, target: &str, message: &str) -> Result<(), String> {
    let upstream = format!("origin/{target}");
    match git.worktree_of(target)? {
        Some(path) => {
            if !git.is_working_tree_clean_at(&path)? {
                return Err(format!(
                    "'{target}' is checked out in {} and that working tree is not clean.\n\
                     Commit or stash the changes there, then re-run.",
                    path.display()
                ));
            }
            git.ff_merge_at(&path, &upstream)?;
            git.merge_at(&path, source, message)
        }
        None => {
            git.checkout(target)?;
            git.ff_merge(&upstream)?;
            git.merge(source, message)
        }
    }
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

/// Bring `branch` and its origin counterpart in line: push a missing or ahead
/// branch, refuse behind/diverged states with the fixing command.
pub(crate) fn reconcile_with_origin(git: &dyn Git, branch: &str) -> Result<(), String> {
    let origin = format!("origin/{branch}");
    if !git.remote_branch_exists(branch)? {
        git.push(branch)?;
    } else if !git.is_pushed(branch)? {
        if git.is_ancestor(&origin, branch)? {
            git.push(branch)?;
        } else if git.is_ancestor(branch, &origin)? {
            return Err(format!("Local {branch} is behind origin/{branch}. Run 'git pull --ff-only', then re-run."));
        } else {
            return Err(format!("Local {branch} and origin/{branch} have diverged. Reconcile them (e.g. 'git pull --rebase'), then re-run."));
        }
    }
    Ok(())
}

/// The migration guard for finish-branch landings: an open PR whose head is
/// the source branch itself comes from an older bflow and must be dealt with
/// by a human — bflow never merges or closes PRs.
pub(crate) fn refuse_open_legacy_pr(hosting: &dyn HostingPlatform, source: &str, target: &str) -> Result<(), String> {
    match hosting.open_pr_to(source, target)? {
        Some(url) => Err(format!(
            "A landing PR from {source} into {target} is still open from an older bflow: {url}\n\
             Either merge it and re-run 'bflow finish', or close/abandon it and re-run — bflow will then reopen it from a finish/* branch."
        )),
        None => Ok(()),
    }
}

/// Lenient leg lookup (the mainline leg, whose tag is already published once
/// landed): landed once stays landed. Checks the finish-branch head first,
/// then the legacy head — drop the fallback once every pre-finish-branch
/// landing has migrated.
pub(crate) fn finish_leg_landed(git: &dyn Git, hosting: &dyn HostingPlatform, source: &str, target: &str) -> Result<Option<LandedPr>, String> {
    let finish = finish_branch_name(source, target);
    if let Some(pr) = leg_landed(git, hosting, &finish, target)? {
        return Ok(Some(pr));
    }
    leg_landed(git, hosting, source, target)
}

/// Strict leg lookup (develop and release legs): landed only while the merged
/// landing still contains the current source tip. A landing that predates new
/// source commits (a mid-release sync, an earlier finish attempt) re-opens
/// with a refreshed finish branch instead of silently dropping the commits.
pub(crate) fn finish_leg_landed_strict(git: &dyn Git, hosting: &dyn HostingPlatform, source: &str, target: &str) -> Result<Option<LandedPr>, String> {
    let Some(pr) = finish_leg_landed(git, hosting, source, target)? else {
        return Ok(None);
    };
    if landing_contains_tip(git, source, target, &pr)? {
        Ok(Some(pr))
    } else {
        println!("The merged {target} landing predates the latest {source} commits — opening a fresh PR.");
        Ok(None)
    }
}

/// Whether the landed PR's content includes the current source tip: head
/// equality (no conflicts were resolved), else ancestry via the finish refs —
/// origin first, since a remote-side resolution never moves the local ref.
fn landing_contains_tip(git: &dyn Git, source: &str, target: &str, pr: &LandedPr) -> Result<bool, String> {
    if git.branch_sha(source)? == pr.head_sha {
        return Ok(true);
    }
    let finish = finish_branch_name(source, target);
    if git.remote_branch_exists(&finish)? && git.is_ancestor(source, &format!("origin/{finish}"))? {
        return Ok(true);
    }
    Ok(git.local_branch_exists(&finish)? && git.is_ancestor(source, &finish)?)
}

/// Make the leg's finish branch exist and contain the source tip, pushed.
/// `origin/{finish}` is the truth: it may carry conflict resolutions the local
/// ref never sees. A finish branch is only ever appended to — a moved source
/// is merged in, never force-pushed over someone's resolution.
pub(crate) fn ensure_finish_branch(git: &dyn Git, source: &str, target: &str) -> Result<String, String> {
    reconcile_with_origin(git, source)?;
    let finish = finish_branch_name(source, target);
    let origin_finish = format!("origin/{finish}");
    let remote = git.remote_branch_exists(&finish)?;
    if remote && git.is_ancestor(source, &origin_finish)? {
        return Ok(finish);
    }
    if !git.local_branch_exists(&finish)? {
        if remote {
            git.create_branch_no_checkout(&finish, &origin_finish)?;
        } else {
            git.create_branch_no_checkout(&finish, source)?;
            git.push(&finish)?;
            return Ok(finish);
        }
    }
    if !git.is_ancestor(source, &finish)? || (remote && !git.is_ancestor(&origin_finish, &finish)?) {
        println!("Refreshing {finish} with {source}...");
        let prior = git.current_branch()?;
        git.checkout(&finish)?;
        if remote {
            git.ff_merge(&origin_finish)?;
        }
        git.merge(source, &format!("chore: refresh {finish} with {source}"))
            .map_err(|e| format!("{e}\n{}", finish_conflict_hint(&finish, target)))?;
        git.checkout(&prior)?;
    }
    git.push(&finish)?;
    Ok(finish)
}

/// How to resolve a conflicted landing without ever touching the source branch.
pub(crate) fn finish_conflict_hint(finish: &str, target: &str) -> String {
    format!(
        "Conflicts? Merge `{target}` into `{finish}` (never rebase it):\n  \
         `git switch {finish} && git merge origin/{target}`\n  \
         then resolve, commit, push, merge the PR, and re-run."
    )
}

/// Outcome of driving one strict protected landing leg.
pub(crate) enum LegState {
    Landed(LandedPr),
    ContentPresent,
    Pending { url: String, finish: String },
}

/// Drive one strict landing leg: refuse legacy PRs, recognize a landing that
/// still contains the tip, skip a target that already has the content, else
/// open (or reuse) the PR from the leg's finish branch.
pub(crate) fn land_leg_strict(git: &dyn Git, hosting: &dyn HostingPlatform, source: &str, target: &str, title: &str, template: Option<&Path>) -> Result<LegState, String> {
    refuse_open_legacy_pr(hosting, source, target)?;
    if let Some(pr) = finish_leg_landed_strict(git, hosting, source, target)? {
        return Ok(LegState::Landed(pr));
    }
    if git.is_ancestor(source, &format!("origin/{target}"))? {
        println!("↷ skipped: landing into {target} (already contains {source})");
        return Ok(LegState::ContentPresent);
    }
    let finish = ensure_finish_branch(git, source, target)?;
    let url = hosting.create_or_get_pr(&finish, target, title, template.and_then(|p| p.to_str()))?;
    Ok(LegState::Pending { url, finish })
}

/// Delete every finish branch of `source` — machinery, never kept, found by
/// pattern so a leg whose target vanished mid-finish leaves no orphan.
pub(crate) fn cleanup_finish_branches(git: &dyn Git, source: &str) -> Result<(), String> {
    let pattern = format!("finish/{}-into-*", source.replace('/', "-"));
    for branch in git.list_branches_matching(&pattern)? {
        delete_branch_guarded(git, &branch)?;
    }
    Ok(())
}

/// Whether `source` has landed into `target` at least once: its most recent
/// merged PR's merge commit is contained in `target`. Unlike `landed_pr`, this
/// stays true when `source` gains commits afterwards — which is what lets a
/// finish resume past a leg it already completed. Compares against
/// `origin/{target}`: protected mode never checks out main or develop, so the
/// local ref may be stale or absent.
pub(crate) fn leg_landed(git: &dyn Git, hosting: &dyn HostingPlatform, source: &str, target: &str) -> Result<Option<LandedPr>, String> {
    let Some(pr) = hosting.merged_pr_to(source, target)? else {
        return Ok(None);
    };
    if git.is_ancestor(&pr.merge_commit_sha, &format!("origin/{target}"))? {
        Ok(Some(pr))
    } else {
        Ok(None)
    }
}

/// Whether `source`'s tip went somewhere that landed. Squash merges leave no
/// ancestry between source and target, so head equality is the primary test;
/// a conflict-resolved landing has extra commits on its finish branch, whose
/// refs (origin first — a remote-side resolution never moves the local ref)
/// prove containment by ancestry. Nothing provable → false: cleanup keeps the
/// branch rather than delete commits that may never have landed.
pub(crate) fn tip_landed_somewhere(git: &dyn Git, source: &str, landed: &[LandedPr], finish_branches: &[String]) -> Result<bool, String> {
    let tip = git.branch_sha(source)?;
    if landed.iter().any(|pr| pr.head_sha == tip) {
        return Ok(true);
    }
    for finish in finish_branches {
        if git.remote_branch_exists(finish)? && git.is_ancestor(source, &format!("origin/{finish}"))? {
            return Ok(true);
        }
        if git.local_branch_exists(finish)? && git.is_ancestor(source, finish)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Report commits pushed to `source` after its `target` landing merged. They
/// are in neither `tag` nor `target`: the tag was cut at that PR's merge commit
/// and the leg is never re-opened, so they reach only the legs still to come.
///
/// Told, not refused — the tag is already published, so there is nothing to
/// redo, only something the operator must know. Free mode catches the same case
/// through its RC gate, which is why protected mode would otherwise be the
/// quieter of the two about a commit that misses the release.
pub(crate) fn report_commits_past_landing(git: &dyn Git, source: &str, pr: &LandedPr, target: &str, tag: &str) -> Result<(), String> {
    if git.branch_sha(source)? == pr.head_sha {
        return Ok(());
    }
    let count = git.rev_list_count(&pr.head_sha, source)?;
    if count == 0 {
        return Ok(());
    }
    let noun = if count == 1 { "commit" } else { "commits" };
    eprintln!("⚠ {source} has {count} {noun} after the {target} landing: not in {tag}, and not reaching {target}.");
    eprintln!("  Release them as a hotfix if they must ship to production.");
    Ok(())
}

/// The full "stopping for a human" block a pending landing prints: PR title,
/// URL, what happens next, and the conflict recipe — blank-line separated.
/// `bold` styles only the title; resolved at the print site so this stays pure.
pub(crate) fn pending_landing_message(title: &str, url: &str, rerun: &str, hint: &str, bold: bool) -> String {
    let title = if bold { format!("\x1b[1m{title}\x1b[0m") } else { title.to_string() };
    format!(
        "\n{title}\n{url}\n\n\
         Waiting for a human to merge this PR.\n\
         Re-run '{rerun}' to continue after the merge.\n\n\
         {hint}"
    )
}

/// Announce a landing step that opened (or reused) a PR and is stopping for a
/// human to merge it.
pub(crate) fn announce_pending_landing(title: &str, url: &str, rerun: &str, hint: &str) {
    use std::io::IsTerminal;
    println!("{}", pending_landing_message(title, url, rerun, hint, std::io::stdout().is_terminal()));
}

/// Cut `tag` at `sha` (a PR's merge commit) unless it already exists — and
/// when it does, verify it points at that same commit rather than trusting a
/// stale or hand-created tag (a mismatch is fatal, not silently skipped).
///
/// Callers establish that the tag is not already on the mainline before
/// reaching here, so the equal-commit arm is unreachable through them: a tag
/// matching `sha` would be a tag on the merge commit, which is on the mainline
/// by definition. It stays as a guard for any future caller without that
/// precondition.
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
/// Moves HEAD off the branch first when it is still there — a resume that
/// skipped the develop merge leaves it there, and git refuses to delete the
/// checked-out branch. Inside the branch's own linked worktree that means
/// detaching and, once the branch is gone, removing the worktree — the last
/// git call of the flow, since the process cwd disappears with it. Elsewhere
/// HEAD moves onto the mainline (always safe, the work is merged there) when
/// no worktree holds it, or detaches, since git also refuses to check out a
/// branch held by another worktree. Returns the removed worktree path, if any.
pub(crate) fn delete_source_branch(git: &dyn Git, branch: &str, main_branch: &str) -> Result<Option<PathBuf>, String> {
    if git.current_branch()? != branch {
        delete_branch_guarded(git, branch)?;
        return Ok(None);
    }
    if git.is_linked_worktree()? {
        git.detach_head()?;
        delete_branch_guarded(git, branch)?;
        return git.remove_current_worktree().map(Some);
    }
    match git.worktree_of(main_branch)? {
        None => git.checkout(main_branch)?,
        Some(_) => git.detach_head()?,
    }
    delete_branch_guarded(git, branch)?;
    Ok(None)
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

/// List `{prefix}/*` branches that are still open, excluding any that already
/// shipped — reusing a shipped branch would make `bflow start` loop onto a
/// dead branch forever and hotfix fan-out merge into history that already
/// landed. What "shipped" means depends on when the clean tag appears:
///
/// - `hotfix/*` (both strategies) and `release/*` under rc: the clean tag
///   (e.g. `v1.1.0`, never the `-rc.N` tag) exists — it is only ever cut at
///   finish, so its existence is the shipped record. Branches whose version
///   does not parse stay in, unchanged from today's behavior.
/// - `release/*` under patch: the clean tag is cut at branch *creation*, so it
///   proves nothing. Shipped is the branch being an ancestor of
///   `origin/{main}`, or — under protected mode, where a squash landing leaves
///   no ancestry — a merged landing PR into the mainline (`leg_landed`).
///   Derived, never stored. The ancestry check reads `origin/{branch}` when
///   the remote branch exists (the local name may be a remote-only branch that
///   resolves to nothing on a fresh clone), falling back to the local name.
pub(crate) fn open_versioned_branches(git: &dyn Git, hosting: &dyn HostingPlatform, cfg: &RepoConfig, main_branch: &str, prefix: &str) -> Result<Vec<String>, String> {
    let branches = git.list_branches_matching(&format!("{prefix}/*"))?;
    let mut open = Vec::with_capacity(branches.len());
    for branch in branches {
        let tag_is_shipped_record = cfg.bump_strategy == BumpStrategy::Rc || prefix == "hotfix";
        let shipped = if tag_is_shipped_record {
            match branch.strip_prefix(&format!("{prefix}/")).and_then(SemVer::parse) {
                Some(version) => git.tag_exists(&version.to_release().tag_name())?,
                None => false,
            }
        } else {
            let branch_ref = if git.remote_branch_exists(&branch)? {
                format!("origin/{branch}")
            } else {
                branch.clone()
            };
            git.is_ancestor(&branch_ref, &format!("origin/{main_branch}"))?
                || (cfg.mode == Mode::Protected
                    && leg_landed(git, hosting, &branch, main_branch)?.is_some())
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
         git add . && git commit --no-edit\n    \
         git switch {source_branch}\n    bflow finish"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_conflict_hint_is_a_compact_recipe() {
        let hint = finish_conflict_hint("finish/hotfix-2.11.6-into-main", "main");
        assert_eq!(
            hint,
            "Conflicts? Merge `main` into `finish/hotfix-2.11.6-into-main` (never rebase it):\n  \
             `git switch finish/hotfix-2.11.6-into-main && git merge origin/main`\n  \
             then resolve, commit, push, merge the PR, and re-run."
        );
    }

    #[test]
    fn pending_landing_message_separates_title_url_wait_and_hint() {
        let msg = pending_landing_message(
            "chore: merge hotfix 2.11.6 into main",
            "https://example.com/pull/473",
            "bflow finish",
            "Conflicts? ...",
            false,
        );
        assert_eq!(
            msg,
            "\nchore: merge hotfix 2.11.6 into main\n\
             https://example.com/pull/473\n\
             \n\
             Waiting for a human to merge this PR.\n\
             Re-run 'bflow finish' to continue after the merge.\n\
             \n\
             Conflicts? ..."
        );
    }

    #[test]
    fn pending_landing_message_bolds_only_the_title_on_a_terminal() {
        let msg = pending_landing_message("title", "url", "bflow sync", "hint", true);
        assert!(msg.starts_with("\n\x1b[1mtitle\x1b[0m\nurl\n"), "got: {msg:?}");
        assert_eq!(msg.matches('\x1b').count(), 2, "only the title is styled; got: {msg:?}");
    }

    #[test]
    fn resume_hint_commits_before_switching_back() {
        // The copy-pasteable block must not start with `git switch` — mid-merge,
        // that fails with "fatal: cannot switch branch while merging".
        let hint = resume_hint("release/1.1.0");
        assert_eq!(
            hint,
            "Resolve the conflict and commit the merge, then switch back to the source \
             branch and re-run 'bflow finish' to continue:\n    \
             git add . && git commit --no-edit\n    \
             git switch release/1.1.0\n    bflow finish"
        );
    }
}
