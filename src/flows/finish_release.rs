use crate::flows::{delete_source_branch, merge_into, push_if_needed, push_tag_if_missing, resume_hint, tag_if_missing};
use crate::git::Git;
use crate::version::SemVer;

/// Highest `v{major}.{minor}.0-rc.N` tag on `branch`, or `None` when the branch
/// has no matching RC tag. Callers turn `None` into their own per-command error.
fn latest_rc(git: &dyn Git, branch: &str, major: u32, minor: u32) -> Result<Option<SemVer>, String> {
    let tags = git.tags_on_branch(branch)?;
    Ok(tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor && v.patch == 0 && v.is_rc())
        .max())
}

pub fn bump_version(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let branch = release.release_branch();

    let latest = latest_rc(git, &branch, major, minor)?
        .ok_or_else(|| format!("No RC tags found on branch {branch}. Run 'bflow start release' first."))?;

    let next = latest.bump_rc();
    let tag = next.tag_name();

    println!("Bumping version: {latest} → {next}");
    git.create_tag(&tag, &format!("chore: bump version to {tag}"))?;
    git.push_tag(&tag)?;
    println!("Tagged and pushed: {tag}");

    Ok(())
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
