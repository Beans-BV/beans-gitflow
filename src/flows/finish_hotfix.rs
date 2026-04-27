use crate::git::Git;
use crate::version::SemVer;

pub fn finish_hotfix(git: &dyn Git, major: u32, minor: u32, patch: u32) -> Result<(), String> {
    let hotfix_branch = format!("hotfix/{major}.{minor}.{patch}");
    let version = SemVer::new(major, minor, patch);
    let tag = version.tag_name();

    println!("Finishing hotfix {hotfix_branch}...");

    println!("Merging into main...");
    git.checkout("main")?;
    git.pull("origin/main")?;
    git.merge(&hotfix_branch, &format!("chore: merge hotfix {version} into main"))?;

    println!("Tagging: {tag}");
    git.create_tag(&tag, &format!("chore: hotfix {version}"))?;
    git.push("main")?;
    git.push_tag(&tag)?;

    println!("Merging into develop...");
    git.checkout("develop")?;
    git.pull("origin/develop")?;
    git.merge(&hotfix_branch, &format!("chore: merge hotfix {version} into develop"))?;
    git.push("develop")?;

    let mut release_branches: Vec<String> = git
        .list_branches_matching("release/*")?
        .into_iter()
        .filter(|b| b.starts_with("release/") && !b.starts_with("release-fix/"))
        .collect();
    release_branches.sort();
    release_branches.dedup();

    for release in &release_branches {
        println!("Merging into {release}...");
        git.checkout(release)?;
        git.pull(&format!("origin/{release}"))?;
        git.merge(
            &hotfix_branch,
            &format!("chore: merge hotfix {version} into {release}"),
        ).map_err(|e| format!(
            "{e}\n\
             Hotfix {version} was merged into main and develop, but propagation into {release} failed.\n\
             Resolve the conflict on {release}, then run 'bflow bump' to cut a new RC.\n\
             The hotfix branch '{hotfix_branch}' has been kept for retry."
        ))?;
        git.push(release)?;
    }

    println!("Cleaning up hotfix branch...");
    git.delete_branch_local(&hotfix_branch)?;
    git.delete_branch_remote(&hotfix_branch)?;

    if release_branches.is_empty() {
        println!("Hotfix {version} complete.");
    } else {
        let list = release_branches.join(", ");
        println!("Hotfix {version} propagated to: main, develop, {list}");
        println!("Run 'bflow bump' on each release branch to cut a new RC.");
    }
    Ok(())
}
