use crate::git::Git;

pub fn finish_hotfix(git: &dyn Git, major: u32, minor: u32, patch: u32) -> Result<(), String> {
    let hotfix_branch = format!("hotfix/{major}.{minor}.{patch}");
    let tag = format!("{major}.{minor}.{patch}");

    println!("Finishing hotfix {hotfix_branch}...");

    println!("Merging into main...");
    git.checkout("main")?;
    git.merge(&hotfix_branch, &format!("chore: finish hotfix {tag}"))?;

    println!("Tagging: {tag}");
    git.create_tag(&tag, &format!("chore: hotfix {tag}"))?;
    git.push("main")?;
    git.push_tag(&tag)?;

    println!("Merging into develop...");
    git.checkout("develop")?;
    git.merge(&hotfix_branch, &format!("chore: merge hotfix {tag} into develop"))?;
    git.push("develop")?;

    println!("Cleaning up hotfix branch...");
    git.delete_branch_local(&hotfix_branch)?;
    git.delete_branch_remote(&hotfix_branch)?;

    println!("Hotfix {tag} complete.");
    Ok(())
}
