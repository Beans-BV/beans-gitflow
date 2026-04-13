use crate::git::Git;
use crate::version::SemVer;

pub fn bump_version(git: &dyn Git, major: u32, minor: u32) -> Result<(), String> {
    let release = SemVer::new(major, minor, 0);
    let branch = release.release_branch();
    let tags = git.tags_on_branch(&branch)?;

    let latest = tags.iter().filter_map(|t| SemVer::parse(t))
        .filter(|v| v.major == major && v.minor == minor && v.patch == 0)
        .max()
        .ok_or_else(|| format!("No tags found on branch {branch}"))?;

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
        .filter(|v| v.major == major && v.minor == minor && v.patch == 0)
        .max()
        .ok_or_else(|| "No version tag found on this release branch. Run 'bump version' first.".to_string())?;

    let release_version = latest_rc.to_release();
    let tag = release_version.tag_name();

    println!("Finishing release {release_branch} (tag: {tag})...");

    println!("Merging into main...");
    git.checkout("main")?;
    git.pull("origin/main")?;
    git.merge(&release_branch, &format!("chore: merge release {release} into main"))?;

    println!("Tagging main: {tag}");
    git.create_tag(&tag, &format!("chore: release {release_version}"))?;
    git.push("main")?;
    git.push_tag(&tag)?;

    println!("Merging into develop...");
    git.checkout("develop")?;
    git.pull("origin/develop")?;
    git.merge(&release_branch, &format!("chore: merge release {release} into develop"))?;
    git.push("develop")?;

    println!("Cleaning up release branch...");
    git.delete_branch_local(&release_branch)?;
    git.delete_branch_remote(&release_branch)?;

    println!("Release {release_version} complete.");
    Ok(())
}
