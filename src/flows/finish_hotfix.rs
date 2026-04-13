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

    println!("Cleaning up hotfix branch...");
    git.delete_branch_local(&hotfix_branch)?;
    git.delete_branch_remote(&hotfix_branch)?;

    println!("Hotfix {version} complete.");
    Ok(())
}
