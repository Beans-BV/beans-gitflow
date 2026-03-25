use crate::git::Git;
use crate::menu;
use crate::version::SemVer;

pub fn start_work_branch(git: &dyn Git, prefix: &str, name: &str) -> Result<(), String> {
    let branch = format!("{prefix}/{name}");
    println!("Creating branch: {branch}");
    git.create_branch(&branch, "develop")?;
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}

pub fn start_release_fix(git: &dyn Git) -> Result<(), String> {
    let release_branch = resolve_or_create_release(git)?;
    let version = release_branch.strip_prefix("release/").unwrap();
    let name = menu::prompt_name("Name for release-fix branch")?;
    let branch = format!("release-fix/{version}/{name}");
    println!("Creating branch: {branch}");
    git.create_branch(&branch, &release_branch)?;
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}

pub fn start_hotfix_fix(git: &dyn Git) -> Result<(), String> {
    let hotfix_branch = resolve_or_create_hotfix(git)?;
    let version = hotfix_branch.strip_prefix("hotfix/").unwrap();
    let name = menu::prompt_name("Name for hotfix-fix branch")?;
    let branch = format!("hotfix-fix/{version}/{name}");
    println!("Creating branch: {branch}");
    git.create_branch(&branch, &hotfix_branch)?;
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}

fn resolve_or_create_release(git: &dyn Git) -> Result<String, String> {
    let branches = git.list_branches_matching("release/*")?;
    let release_branches: Vec<&String> = branches.iter()
        .filter(|b| b.starts_with("release/") && !b.starts_with("release-fix/"))
        .collect();

    if let Some(branch) = release_branches.first() {
        println!("Using existing release branch: {branch}");
        git.checkout(branch)?;
        return Ok(branch.to_string());
    }

    let latest = find_latest_tag(git)?;
    let next = latest.bump_minor();
    let branch = next.release_branch();
    let tag = format!("{}.{}.0", next.major, next.minor);

    println!("Creating release branch: {branch}");
    git.checkout("develop")?;
    git.create_branch(&branch, "develop")?;
    git.push(&branch)?;

    println!("Tagging: {tag}");
    git.create_tag(&tag, &format!("chore: create release branch {}.{}", next.major, next.minor))?;
    git.push_tag(&tag)?;

    Ok(branch)
}

fn resolve_or_create_hotfix(git: &dyn Git) -> Result<String, String> {
    let branches = git.list_branches_matching("hotfix/*")?;
    let hotfix_branches: Vec<&String> = branches.iter()
        .filter(|b| b.starts_with("hotfix/") && !b.starts_with("hotfix-fix/"))
        .collect();

    if let Some(branch) = hotfix_branches.first() {
        println!("Using existing hotfix branch: {branch}");
        git.checkout(branch)?;
        return Ok(branch.to_string());
    }

    let latest = find_latest_tag(git)?;
    let next = latest.bump_patch();
    let branch = next.hotfix_branch();

    println!("Creating hotfix branch: {branch}");
    git.checkout("main")?;
    git.create_branch(&branch, "main")?;
    git.push(&branch)?;

    Ok(branch)
}

fn find_latest_tag(git: &dyn Git) -> Result<SemVer, String> {
    let tags = git.list_tags()?;
    let mut versions: Vec<SemVer> = tags.iter().filter_map(|t| SemVer::parse(t)).collect();
    versions.sort();
    versions.last().cloned()
        .ok_or_else(|| "No semver tags found. Create an initial tag first (e.g. 0.0.0).".to_string())
}
