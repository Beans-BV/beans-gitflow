use crate::git::Git;
use crate::git::branch::BranchType;
use crate::hosting::HostingPlatform;
use crate::menu;

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

fn detect_parent_branch(git: &dyn Git, current: &str) -> Result<String, String> {
    let work_prefixes = ["feature/", "fix/", "chore/", "docs/", "refactor/"];

    let remote_branches = git.list_remote_branches()?;
    let mut candidates: Vec<(String, u32)> = Vec::new();

    for branch in &remote_branches {
        if branch == current {
            continue;
        }
        let is_work = work_prefixes.iter().any(|p| branch.starts_with(p));
        let is_develop = branch == "develop";
        if !is_work && !is_develop {
            continue;
        }
        let base = match git.merge_base(current, branch) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let current_count = match git.rev_list_count(&base, current) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let candidate_count = match git.rev_list_count(&base, branch) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Skip child branches: if we have fewer commits since divergence
        // than the candidate, it likely branched from us
        if current_count < candidate_count {
            continue;
        }
        candidates.push((branch.clone(), current_count));
    }

    if candidates.is_empty() {
        return Ok("develop".to_string());
    }

    // Sort by distance ascending; on ties prefer develop, then alphabetical
    candidates.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| {
                let a_is_develop = a.0 == "develop";
                let b_is_develop = b.0 == "develop";
                b_is_develop.cmp(&a_is_develop)
            })
            .then_with(|| a.0.cmp(&b.0))
    });

    let labels: Vec<&str> = candidates.iter().map(|(name, _)| name.as_str()).collect();
    let idx = menu::show_select("PR target branch", &labels)?;
    Ok(candidates[idx].0.clone())
}

pub fn finish_work_branch(git: &dyn Git, hosting: &dyn HostingPlatform, branch_type: &BranchType, breaking: Option<bool>) -> Result<(), String> {
    let commit_type = branch_type.commit_type().ok_or("Cannot finish: not on a work branch")?;
    let name = branch_type.name().ok_or("Cannot finish: branch has no name")?;
    let current = git.current_branch()?;
    let base = detect_parent_branch(git, &current)?;

    let bang = match breaking {
        // Explicit flag always honored, for any work type
        Some(b) => b,
        // Prompt only for types that are commonly breaking
        None if commonly_breaking(commit_type) => prompt_breaking_change()?,
        None => false,
    };

    let title = if bang {
        format!("{commit_type}!: {name}")
    } else {
        format!("{commit_type}: {name}")
    };

    push_and_create_pr(git, hosting, &base, &title)
}

fn commonly_breaking(commit_type: &str) -> bool {
    matches!(commit_type, "feat" | "fix" | "refactor")
}

fn prompt_breaking_change() -> Result<bool, String> {
    let idx = menu::show_select("Contains breaking changes?", &["no", "yes"])?;
    Ok(idx == 1)
}

pub fn finish_release_fix(git: &dyn Git, hosting: &dyn HostingPlatform, major: u32, minor: u32, patch: u32, name: &str) -> Result<(), String> {
    push_and_create_pr(git, hosting, &format!("release/{major}.{minor}.{patch}"), &format!("fix: {name}"))
}

pub fn finish_hotfix_fix(git: &dyn Git, hosting: &dyn HostingPlatform, major: u32, minor: u32, patch: u32, name: &str) -> Result<(), String> {
    push_and_create_pr(git, hosting, &format!("hotfix/{major}.{minor}.{patch}"), &format!("fix: {name}"))
}
