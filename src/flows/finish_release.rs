use crate::git::Git;
use crate::version::SemVer;

pub fn bump_version(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let branch = format!("release/{major}.{minor}");
    let tags = git.tags_on_branch(&branch)?;

    let latest = tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor)
        .max()
        .ok_or_else(|| format!("No tags found on branch {branch}"))?;

    let next = latest.bump_patch();
    let tag = next.to_string();

    println!("Bumping version: {latest} → {next}");
    git.create_tag(&tag, &format!("chore: bump version to {tag}"))?;
    git.push_tag(&tag)?;
    println!("Tagged and pushed: {tag}");

    Ok(())
}

pub fn sync_with_develop(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release_branch = format!("release/{major}.{minor}");
    let current = git.current_branch()?;

    println!("Merging {release_branch} into develop...");
    git.checkout("develop")?;
    git.pull("origin/develop")?;
    git.merge(&release_branch, &format!("chore: sync release {major}.{minor} with develop"))?;
    git.push("develop")?;

    git.checkout(&current)?;
    println!("Develop synced with {release_branch}.");

    Ok(())
}

pub fn finish_release(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release_branch = format!("release/{major}.{minor}");

    let tags = git.tags_on_branch(&release_branch)?;
    let latest_tag = tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor)
        .max()
        .ok_or_else(|| "No version tag found on this release branch. Run 'bump version' first.".to_string())?;

    // Auto-bump if there are commits since the latest tag
    let commits_since_tag = git.rev_list_count(&latest_tag.to_string(), &release_branch)?;
    let latest_tag = if commits_since_tag > 0 {
        let next = latest_tag.bump_patch();
        println!("Commits found since {latest_tag}, bumping to {next}...");
        git.create_tag(&next.to_string(), &format!("chore: bump version to {next}"))?;
        git.push_tag(&next.to_string())?;
        next
    } else {
        latest_tag
    };

    println!("Finishing release {release_branch} (tag: {latest_tag})...");

    println!("Merging into main...");
    git.checkout("main")?;
    git.pull("origin/main")?;
    git.merge(&release_branch, &format!("chore: merge release {major}.{minor} into main"))?;
    git.push("main")?;

    println!("Merging into develop...");
    git.checkout("develop")?;
    git.pull("origin/develop")?;
    git.merge(&release_branch, &format!("chore: merge release {major}.{minor} into develop"))?;
    git.push("develop")?;

    println!("Cleaning up release branch...");
    git.delete_branch_local(&release_branch)?;
    git.delete_branch_remote(&release_branch)?;

    println!("Release {latest_tag} complete.");
    Ok(())
}
