use crate::git::Git;
use crate::git::branch::BranchType;
use crate::hosting::HostingPlatform;

fn push_and_create_pr(git: &dyn Git, hosting: &dyn HostingPlatform, base: &str, title: &str) -> Result<(), String> {
    let current = git.current_branch()?;

    println!("Pushing branch: {current}");
    git.push(&current)?;

    println!("Creating PR: {title} → {base}");
    let url = hosting.create_or_get_pr(&current, base, title)?;
    println!("PR: {url}");
    hosting.open_url(&url)?;

    Ok(())
}

pub fn finish_work_branch(git: &dyn Git, hosting: &dyn HostingPlatform, branch_type: &BranchType) -> Result<(), String> {
    let commit_type = branch_type.commit_type().ok_or("Cannot finish: not on a work branch")?;
    let name = branch_type.name().ok_or("Cannot finish: branch has no name")?;
    push_and_create_pr(git, hosting, "develop", &format!("{commit_type}: {name}"))
}

pub fn finish_release_fix(git: &dyn Git, hosting: &dyn HostingPlatform, major: u32, minor: u32, name: &str) -> Result<(), String> {
    push_and_create_pr(git, hosting, &format!("release/{major}.{minor}"), &format!("fix: {name}"))
}

pub fn finish_hotfix_fix(git: &dyn Git, hosting: &dyn HostingPlatform, major: u32, minor: u32, patch: u32, name: &str) -> Result<(), String> {
    push_and_create_pr(git, hosting, &format!("hotfix/{major}.{minor}.{patch}"), &format!("fix: {name}"))
}
