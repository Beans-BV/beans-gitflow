use crate::flows::{delete_source_branch, merge_into, push_if_needed, push_tag_if_missing, resume_hint, tag_if_missing};
use crate::git::Git;
use crate::version::SemVer;

pub fn finish_hotfix(git: &dyn Git, major: u32, minor: u32, patch: u32, main_branch: &str) -> Result<(), String> {
    let version = SemVer::new(major, minor, patch);
    let hotfix_branch = version.hotfix_branch();
    let tag = version.tag_name();

    println!("Finishing hotfix {hotfix_branch}...");

    merge_into(git, &hotfix_branch, main_branch,
        &format!("chore: merge hotfix {version} into {main_branch}"),
        &resume_hint(&hotfix_branch))?;
    tag_if_missing(git, &tag, &format!("chore: hotfix {version}"))?;
    push_if_needed(git, main_branch)?;
    push_tag_if_missing(git, &tag)?;

    merge_into(git, &hotfix_branch, "develop",
        &format!("chore: merge hotfix {version} into develop"),
        &resume_hint(&hotfix_branch))?;
    push_if_needed(git, "develop")?;

    // Propagate into every open release branch. The `release/*` ref pattern
    // already excludes `release-fix/*` (`release-` ≠ `release/`), so no extra
    // filtering is needed. Re-sorted here even though the trait contract
    // already promises sorted output: deterministic replay on resume is a
    // crash-safety invariant, so the flow enforces it rather than trusting the
    // adapter (test-pinned with a deliberately unsorted mock).
    let mut release_branches = git.list_branches_matching("release/*")?;
    release_branches.sort();

    for release in &release_branches {
        merge_into(git, &hotfix_branch, release,
            &format!("chore: merge hotfix {version} into {release}"),
            &format!(
                "Hotfix {version} was merged into {main_branch} and develop, but propagation into {release} failed.\n\
                 {}\n\
                 (After all releases are updated, run 'bflow bump' on each to cut a fresh RC for staging.)",
                resume_hint(&hotfix_branch)
            ))?;
        // The push sits outside the merge guard so merged-but-unpushed crashes
        // still push on resume (see decisions.md).
        push_if_needed(git, release)?;
    }

    println!("Cleaning up hotfix branch...");
    delete_source_branch(git, &hotfix_branch, main_branch)?;

    if release_branches.is_empty() {
        println!("Hotfix {version} complete.");
    } else {
        let list = release_branches.join(", ");
        println!("Hotfix {version} propagated to: {main_branch}, develop, {list}");
        println!("Run 'bflow bump' on each release branch to cut a new RC.");
    }
    Ok(())
}
