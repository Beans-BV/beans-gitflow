use std::path::Path;

use crate::flows::{
    announce_pending_landing, delete_source_branch, landed_pr, merge_into, open_landing_pr,
    open_versioned_branches, push_if_needed, push_tag_if_missing, resume_hint, tag_at_if_missing,
    tag_if_missing,
};
use crate::git::Git;
use crate::hosting::HostingPlatform;
use crate::repo_config::{Mode, RepoConfig};
use crate::version::SemVer;

#[allow(clippy::too_many_arguments)]
pub fn finish_hotfix(
    git: &dyn Git,
    hosting: &dyn HostingPlatform,
    cfg: &RepoConfig,
    major: u32,
    minor: u32,
    patch: u32,
    main_branch: &str,
    template: Option<&Path>,
) -> Result<(), String> {
    if cfg.mode == Mode::Protected {
        return finish_hotfix_protected(git, hosting, cfg, major, minor, patch, main_branch, template);
    }

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

    // `release/*` cannot match `release-fix/*`, so no extra filtering is needed
    // for that; `open_versioned_branches` also drops any release already shipped.
    // Sorted despite the trait contract already promising it: deterministic
    // replay on resume is a crash-safety invariant, not something to delegate.
    let mut release_branches = open_versioned_branches(git, "release")?;
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

    finish_hotfix_cleanup(git, cfg, &hotfix_branch, main_branch, &version, &release_branches)
}

fn finish_hotfix_cleanup(git: &dyn Git, cfg: &RepoConfig, hotfix_branch: &str, main_branch: &str, version: &SemVer, release_branches: &[String]) -> Result<(), String> {
    println!("Cleaning up hotfix branch...");
    if cfg.keep_release_branches {
        println!("Keeping {hotfix_branch} (keep-release-branches=true).");
    } else {
        delete_source_branch(git, hotfix_branch, main_branch)?;
    }

    if release_branches.is_empty() {
        println!("Hotfix {version} complete.");
    } else {
        let list = release_branches.join(", ");
        println!("Hotfix {version} propagated to: {main_branch}, develop, {list}");
        println!("Run 'bflow bump' on each release branch to cut a new RC.");
    }
    Ok(())
}

/// Protected mode: hotfixes carry no RC gate (unlike releases, `finish_release.rs`),
/// so a landing step opens straight into the same sequential PR-per-run shape —
/// main, then develop, then each open release branch in sorted order, one PR per
/// run, stopping at the first still-pending target.
#[allow(clippy::too_many_arguments)]
fn finish_hotfix_protected(git: &dyn Git, hosting: &dyn HostingPlatform, cfg: &RepoConfig, major: u32, minor: u32, patch: u32, main_branch: &str, template: Option<&Path>) -> Result<(), String> {
    let version = SemVer::new(major, minor, patch);
    let hotfix_branch = version.hotfix_branch();
    let tag = version.tag_name();

    match landed_pr(git, hosting, &hotfix_branch, main_branch)? {
        None => {
            let title = format!("chore: merge hotfix {version} into {main_branch}");
            let url = open_landing_pr(git, hosting, &hotfix_branch, main_branch, &title, template)?;
            announce_pending_landing(&url);
            return Ok(());
        }
        Some(pr) => {
            tag_at_if_missing(git, &tag, &format!("chore: hotfix {version}"), &pr.merge_commit_sha)?;
            push_tag_if_missing(git, &tag)?;
        }
    }

    if landed_pr(git, hosting, &hotfix_branch, "develop")?.is_none() {
        let title = format!("chore: merge hotfix {version} into develop");
        let url = open_landing_pr(git, hosting, &hotfix_branch, "develop", &title, template)?;
        announce_pending_landing(&url);
        return Ok(());
    }

    let mut release_branches = open_versioned_branches(git, "release")?;
    release_branches.sort();

    for release in &release_branches {
        if landed_pr(git, hosting, &hotfix_branch, release)?.is_none() {
            let title = format!("chore: merge hotfix {version} into {release}");
            let url = open_landing_pr(git, hosting, &hotfix_branch, release, &title, template)?;
            announce_pending_landing(&url);
            return Ok(());
        }
    }

    finish_hotfix_cleanup(git, cfg, &hotfix_branch, main_branch, &version, &release_branches)
}
