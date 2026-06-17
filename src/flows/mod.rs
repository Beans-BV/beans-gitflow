pub mod start;
pub mod finish_work;
pub mod finish_release;
pub mod finish_hotfix;

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
