use crate::flows::resume_hint;
use crate::git::Git;
use crate::version::SemVer;

pub fn bump_version(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let branch = release.release_branch();
    let tags = git.tags_on_branch(&branch)?;

    let latest = tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor && v.patch == 0 && v.is_rc())
        .max()
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
    git.pull("origin/develop")?;
    git.merge(&release_branch, &format!("chore: sync release {release} with develop"))?;
    git.push("develop")?;

    git.checkout(&current)?;
    println!("Develop synced with {release_branch}.");

    Ok(())
}

pub fn finish_release(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let release_branch = release.release_branch();

    let tags = git.tags_on_branch(&release_branch)?;
    let latest_rc = tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor && v.patch == 0 && v.is_rc())
        .max()
        .ok_or_else(|| "No RC tag found on this release branch. Run 'bflow bump' first.".to_string())?;

    let release_version = latest_rc.to_release();
    let tag = release_version.tag_name();

    println!("Finishing release {release_branch} (tag: {tag})...");

    // Merge into main (with RC gate)
    if !git.is_ancestor(&release_branch, "main")? {
        let latest_rc_tag = latest_rc.tag_name();
        let commits_past_rc = git.rev_list_count(&latest_rc_tag, &release_branch)?;
        if commits_past_rc > 0 {
            let noun = if commits_past_rc == 1 { "commit" } else { "commits" };
            return Err(format!(
                "HEAD of {release_branch} is {commits_past_rc} {noun} past {latest_rc_tag}.\n\
                 Every commit merged to main must be validated on staging via an RC deploy.\n\
                 Run 'bflow bump' to cut the next RC, wait for staging to pass, then 'bflow finish'."
            ));
        }
        println!("Merging into main...");
        git.checkout("main")?;
        git.pull("origin/main")?;
        git.merge(&release_branch, &format!("chore: merge release {release} into main"))
            .map_err(|e| format!("{e}\n{}", resume_hint(&release_branch)))?;
    } else {
        println!("↷ skipped: merge into main (already merged)");
    }

    // Tag main
    if !git.tag_exists(&tag)? {
        println!("Tagging main: {tag}");
        git.create_tag(&tag, &format!("chore: release {release_version}"))?;
    } else {
        println!("↷ skipped: tag {tag} (already exists)");
    }

    // Push main
    if !git.is_pushed("main")? {
        git.push("main")?;
    } else {
        println!("↷ skipped: push main (already up to date)");
    }

    // Push tag
    if !git.remote_tag_exists(&tag)? {
        git.push_tag(&tag)?;
    } else {
        println!("↷ skipped: push tag {tag} (already pushed)");
    }

    // Merge into develop
    if !git.is_ancestor(&release_branch, "develop")? {
        println!("Merging into develop...");
        git.checkout("develop")?;
        git.pull("origin/develop")?;
        git.merge(&release_branch, &format!("chore: merge release {release} into develop"))
            .map_err(|e| format!("{e}\n{}", resume_hint(&release_branch)))?;
    } else {
        println!("↷ skipped: merge into develop (already merged)");
    }

    // Push develop
    if !git.is_pushed("develop")? {
        git.push("develop")?;
    } else {
        println!("↷ skipped: push develop (already up to date)");
    }

    // Cleanup
    println!("Cleaning up release branch...");
    // On a resume path that skipped the develop merge, HEAD may still be on the
    // release branch — git refuses to delete the currently checked-out branch, so
    // switch off it first. `main` is always safe (the release is now in main).
    if git.current_branch()? == release_branch {
        git.checkout("main")?;
    }
    if git.local_branch_exists(&release_branch)? {
        git.delete_branch_local(&release_branch)?;
    } else {
        println!("↷ skipped: delete local {release_branch} (already gone)");
    }
    if git.remote_branch_exists(&release_branch)? {
        git.delete_branch_remote(&release_branch)?;
    } else {
        println!("↷ skipped: delete remote {release_branch} (already gone)");
    }

    println!("Release {release_version} complete.");
    Ok(())
}
