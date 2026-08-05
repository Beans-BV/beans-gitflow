use crate::flows::{
    delete_branch_guarded, delete_source_branch, merge_into, push_if_needed, push_tag_if_missing,
    require_clean_tree, resume_hint, run_version_script, tag_if_missing,
};
use crate::git::Git;
use crate::hosting::HostingPlatform;
use crate::repo_config::{Mode, RepoConfig};
use crate::version::SemVer;
use crate::version_script::VersionScript;

/// Highest `v{major}.{minor}.0-rc.N` tag on `branch`, or `None` when the branch
/// has no matching RC tag. Callers turn `None` into their own per-command error.
fn latest_rc(git: &dyn Git, branch: &str, major: u32, minor: u32) -> Result<Option<SemVer>, String> {
    let tags = git.tags_on_branch(branch)?;
    Ok(tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor && v.patch == 0 && v.is_rc())
        .max())
}

/// The RC to cut next: one past the branch's highest existing RC tag, or `.1`
/// when it has none yet (trap 3: `bflow start release` leaves no tag behind).
/// Reads `branch`'s tags, so callers call it only where a tag is actually about
/// to be cut or checked — protected mode's deferred/reuse paths never do.
fn next_rc(git: &dyn Git, branch: &str, major: u32, minor: u32, release: &SemVer) -> Result<(Option<SemVer>, SemVer, String), String> {
    let latest = latest_rc(git, branch, major, minor)?;
    let next = latest.clone().map(|v| v.bump_rc()).unwrap_or_else(|| release.with_rc(1));
    let tag = next.tag_name();
    Ok((latest, next, tag))
}

/// Announce the tag about to be cut: the ordinary "bumping from X" line, or —
/// when the branch has no RC yet — the first-RC line instead.
fn announce_next_rc(latest: Option<&SemVer>, next: &SemVer, tag: &str) {
    match latest {
        Some(l) => println!("Bumping version: {l} → {next}"),
        None => println!("Tagging first RC: {tag}"),
    }
}

/// Cut `tag` at the branch tip (the common case: nothing to wait on).
fn cut_tag_at_tip(git: &dyn Git, latest: Option<&SemVer>, next: &SemVer, tag: &str) -> Result<(), String> {
    announce_next_rc(latest, next, tag);
    git.create_tag(tag, &format!("chore: bump version to {tag}"))?;
    git.push_tag(tag)?;
    println!("Tagged and pushed: {tag}");
    Ok(())
}

pub fn bump_version(
    git: &dyn Git,
    hosting: &dyn HostingPlatform,
    script: Option<&dyn VersionScript>,
    cfg: &RepoConfig,
    major: u32,
    minor: u32,
) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let branch = release.release_branch();

    match cfg.mode {
        Mode::Free => bump_free(git, script, &release, &branch, major, minor),
        Mode::Protected => bump_protected(git, hosting, script, &release, &branch, major, minor),
    }
}

/// Free mode: no landing PR to wait for, so a version-script commit (if there
/// is a script) lands straight on the release branch and the tag is cut at the
/// tip in the same run.
fn bump_free(git: &dyn Git, script: Option<&dyn VersionScript>, release: &SemVer, branch: &str, major: u32, minor: u32) -> Result<(), String> {
    let (latest, next, tag) = next_rc(git, branch, major, minor, release)?;

    if let Some(script) = script {
        require_clean_tree(git)?;
        if run_version_script(git, script, release)? {
            git.push(branch)?;
        }
    }
    cut_tag_at_tip(git, latest.as_ref(), &next, &tag)
}

/// Protected mode: a version-script commit needs a human-merged PR before its
/// commit is trustworthy, so the RC tag is deferred to that PR's merge commit
/// rather than cut on a local commit. Order (task brief): consume a landed PR
/// first, then pure-tagging, then PR reuse, then a fresh script run.
fn bump_protected(git: &dyn Git, hosting: &dyn HostingPlatform, script: Option<&dyn VersionScript>, release: &SemVer, branch: &str, major: u32, minor: u32) -> Result<(), String> {
    let chore_branch = release.release_chore_branch("set-version");

    if let Some(pr) = hosting.merged_pr_to(&chore_branch, branch)? {
        let (latest, next, tag) = next_rc(git, branch, major, minor, release)?;
        let consumed = match &latest {
            Some(l) => git.tag_commit_sha(&l.tag_name())? == pr.merge_commit_sha,
            None => false,
        };
        if !consumed {
            announce_next_rc(latest.as_ref(), &next, &tag);
            git.create_tag_at(&tag, &format!("chore: bump version to {tag}"), &pr.merge_commit_sha)?;
            git.push_tag(&tag)?;
            delete_branch_guarded(git, &chore_branch)?;
            println!("Tagged and pushed: {tag}");
            return Ok(());
        }
        // Already cut on a previous run — this run just tidies up remnants
        // (a chore branch the merge didn't delete) and re-evaluates as fresh.
        delete_branch_guarded(git, &chore_branch)?;
    }

    let Some(script) = script else {
        let (latest, next, tag) = next_rc(git, branch, major, minor, release)?;
        return cut_tag_at_tip(git, latest.as_ref(), &next, &tag);
    };

    let title = format!("chore: set version {release}");

    if git.remote_branch_exists(&chore_branch)? {
        let url = hosting.create_or_get_pr(&chore_branch, branch, &title, None)?;
        announce_deferred(&url);
        return Ok(());
    }

    require_clean_tree(git)?;
    git.create_branch(&chore_branch, branch)?;
    if run_version_script(git, script, release)? {
        git.push(&chore_branch)?;
        let url = hosting.create_or_get_pr(&chore_branch, branch, &title, None)?;
        announce_deferred(&url);
        git.checkout(branch)?;
        Ok(())
    } else {
        git.checkout(branch)?;
        git.delete_branch_local(&chore_branch)?;
        let (latest, next, tag) = next_rc(git, branch, major, minor, release)?;
        cut_tag_at_tip(git, latest.as_ref(), &next, &tag)
    }
}

fn announce_deferred(pr_url: &str) {
    println!("Version PR: {pr_url}");
    println!("The RC tag is deferred until this PR merges. After it merges, re-run 'bflow bump' to cut the tag.");
}

pub fn sync_with_develop(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let release_branch = release.release_branch();
    let current = git.current_branch()?;

    println!("Merging {release_branch} into develop...");
    git.checkout("develop")?;
    git.ff_merge("origin/develop")?;
    git.merge(&release_branch, &format!("chore: sync release {release} with develop"))?;
    git.push("develop")?;

    git.checkout(&current)?;
    println!("Develop synced with {release_branch}.");

    Ok(())
}

pub fn finish_release(git: &dyn Git, major: u32, minor: u32, main_branch: &str) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let release_branch = release.release_branch();

    let latest_rc = latest_rc(git, &release_branch, major, minor)?
        .ok_or_else(|| "No RC tag found on this release branch. Run 'bflow bump' first.".to_string())?;

    let release_version = latest_rc.to_release();
    let tag = release_version.tag_name();

    println!("Finishing release {release_branch} (tag: {tag})...");

    // Merge into the mainline — inline rather than merge_into(): the RC gate
    // must run inside the not-yet-merged branch, so a resume past that merge
    // never re-evaluates it (negative-tested in finish_release_test.rs).
    if !git.is_ancestor(&release_branch, main_branch)? {
        let latest_rc_tag = latest_rc.tag_name();
        let commits_past_rc = git.rev_list_count(&latest_rc_tag, &release_branch)?;
        if commits_past_rc > 0 {
            let noun = if commits_past_rc == 1 { "commit" } else { "commits" };
            return Err(format!(
                "HEAD of {release_branch} is {commits_past_rc} {noun} past {latest_rc_tag}.\n\
                 Every commit merged to {main_branch} must be validated on staging via an RC deploy.\n\
                 Run 'bflow bump' to cut the next RC, wait for staging to pass, then 'bflow finish'."
            ));
        }
        println!("Merging into {main_branch}...");
        git.checkout(main_branch)?;
        git.ff_merge(&format!("origin/{main_branch}"))?;
        git.merge(&release_branch, &format!("chore: merge release {release} into {main_branch}"))
            .map_err(|e| format!("{e}\n{}", resume_hint(&release_branch)))?;
    } else {
        println!("↷ skipped: merge into {main_branch} (already merged)");
    }

    tag_if_missing(git, &tag, &format!("chore: release {release_version}"))?;
    push_if_needed(git, main_branch)?;
    push_tag_if_missing(git, &tag)?;

    merge_into(git, &release_branch, "develop",
        &format!("chore: merge release {release} into develop"),
        &resume_hint(&release_branch))?;
    push_if_needed(git, "develop")?;

    println!("Cleaning up release branch...");
    delete_source_branch(git, &release_branch, main_branch)?;

    println!("Release {release_version} complete.");
    Ok(())
}
