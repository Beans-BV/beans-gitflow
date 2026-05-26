use crate::git::Git;
use crate::version::SemVer;

pub fn finish_hotfix(git: &dyn Git, major: u32, minor: u32, patch: u32) -> Result<(), String> {
    let hotfix_branch = format!("hotfix/{major}.{minor}.{patch}");
    let version = SemVer::new(major, minor, patch);
    let tag = version.tag_name();

    println!("Finishing hotfix {hotfix_branch}...");

    // Merge into main
    if !git.is_ancestor(&hotfix_branch, "main")? {
        println!("Merging into main...");
        git.checkout("main")?;
        git.pull("origin/main")?;
        git.merge(&hotfix_branch, &format!("chore: merge hotfix {version} into main"))?;
    } else {
        println!("↷ skipped: merge into main (already merged)");
    }

    // Tag
    if !git.tag_exists(&tag)? {
        println!("Tagging: {tag}");
        git.create_tag(&tag, &format!("chore: hotfix {version}"))?;
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
    if !git.is_ancestor(&hotfix_branch, "develop")? {
        println!("Merging into develop...");
        git.checkout("develop")?;
        git.pull("origin/develop")?;
        git.merge(&hotfix_branch, &format!("chore: merge hotfix {version} into develop"))?;
    } else {
        println!("↷ skipped: merge into develop (already merged)");
    }

    // Push develop
    if !git.is_pushed("develop")? {
        git.push("develop")?;
    } else {
        println!("↷ skipped: push develop (already up to date)");
    }

    // Propagate into every open release branch
    let mut release_branches: Vec<String> = git
        .list_branches_matching("release/*")?
        .into_iter()
        .filter(|b| b.starts_with("release/") && !b.starts_with("release-fix/"))
        .collect();
    release_branches.sort();
    release_branches.dedup();

    for release in &release_branches {
        if git.is_ancestor(&hotfix_branch, release)? {
            println!("↷ skipped: merge into {release} (already merged)");
        } else {
            println!("Merging into {release}...");
            git.checkout(release)?;
            git.pull(&format!("origin/{release}"))?;
            git.merge(
                &hotfix_branch,
                &format!("chore: merge hotfix {version} into {release}"),
            ).map_err(|e| format!(
                "{e}\n\
                 Hotfix {version} was merged into main and develop, but propagation into {release} failed.\n\
                 Resolve the conflict on {release}, commit the merge, then re-run 'bflow finish' to continue.\n\
                 (After all releases are updated, run 'bflow bump' on each to cut a fresh RC for staging.)"
            ))?;
        }
        if !git.is_pushed(release)? {
            git.push(release)?;
        } else {
            println!("↷ skipped: push {release} (already up to date)");
        }
    }

    // Cleanup
    println!("Cleaning up hotfix branch...");
    // On a resume path that skipped the develop merge, HEAD may still be on the
    // hotfix branch — git refuses to delete the currently checked-out branch, so
    // switch off it first. `main` is always safe (the hotfix is now in main).
    if git.current_branch()? == hotfix_branch {
        git.checkout("main")?;
    }
    if git.local_branch_exists(&hotfix_branch)? {
        git.delete_branch_local(&hotfix_branch)?;
    } else {
        println!("↷ skipped: delete local {hotfix_branch} (already gone)");
    }
    if git.remote_branch_exists(&hotfix_branch)? {
        git.delete_branch_remote(&hotfix_branch)?;
    } else {
        println!("↷ skipped: delete remote {hotfix_branch} (already gone)");
    }

    if release_branches.is_empty() {
        println!("Hotfix {version} complete.");
    } else {
        let list = release_branches.join(", ");
        println!("Hotfix {version} propagated to: main, develop, {list}");
        println!("Run 'bflow bump' on each release branch to cut a new RC.");
    }
    Ok(())
}
