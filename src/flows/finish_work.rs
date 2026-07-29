use std::path::Path;

use crate::git::Git;
use crate::git::branch::BranchType;
use crate::hosting::HostingPlatform;
use crate::prompt::Prompter;
use crate::version::SemVer;

/// `template` is the pre-resolved PR template path (resolved at the composition
/// root, anchored to the repo root) — flows never probe the filesystem.
fn push_and_create_pr(git: &dyn Git, hosting: &dyn HostingPlatform, base: &str, title: &str, template: Option<&Path>) -> Result<(), String> {
    let current = git.current_branch()?;

    println!("Pushing branch: {current}");
    git.push(&current)?;

    if let Some(path) = template {
        println!("Using PR template: {}", path.display());
    }

    println!("Creating PR: {title} → {base}");
    let url = hosting.create_or_get_pr(&current, base, title, template.and_then(|p| p.to_str()))?;
    println!("PR: {url}");
    hosting.open_url(&url)?;

    Ok(())
}

fn detect_parent_branch(git: &dyn Git, prompter: &dyn Prompter, current: &str) -> Result<String, String> {
    let remote_branches = git.list_remote_branches()?;
    let mut candidates: Vec<(String, u32)> = Vec::new();

    for branch in &remote_branches {
        if branch == current {
            continue;
        }
        // Candidate parents are develop and work branches — decided by the
        // taxonomy in BranchType, not a local prefix list.
        let parsed = BranchType::parse(branch);
        if parsed != BranchType::Develop && !parsed.is_work_branch() {
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

    if candidates.len() == 1 {
        let (branch, _) = candidates.pop().expect("len checked above");
        println!("PR target branch: {branch} (auto-detected)");
        return Ok(branch);
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
    let idx = prompter.select("PR target branch", &labels)?;
    Ok(candidates[idx].0.clone())
}

pub fn finish_work_branch(git: &dyn Git, hosting: &dyn HostingPlatform, prompter: &dyn Prompter, branch_type: &BranchType, breaking: Option<bool>, base: Option<String>, template: Option<&Path>) -> Result<(), String> {
    let commit_type = branch_type.commit_type().ok_or("Cannot finish: not on a work branch")?;
    let name = branch_type.name().ok_or("Cannot finish: branch has no name")?;
    let current = git.current_branch()?;
    let base = match base {
        Some(base) => {
            if base == current {
                return Err(format!("Base branch '{base}' is the branch being finished; a PR cannot target its own branch."));
            }
            // PRs are created via the hosting platform, so the base must exist on
            // the remote — a local-only branch would fail later at PR creation.
            if !git.remote_branch_exists(&base)? {
                return Err(format!("Base branch '{base}' not found on origin. Push it first (or fetch if it exists remotely)."));
            }
            base
        }
        None => detect_parent_branch(git, prompter, &current)?,
    };

    let bang = match breaking {
        // Explicit flag always honored, for any work type
        Some(b) => b,
        // Prompt only for types that are commonly breaking
        None if commonly_breaking(commit_type) => prompt_breaking_change(prompter)?,
        None => false,
    };

    let title = if bang {
        format!("{commit_type}!: {name}")
    } else {
        format!("{commit_type}: {name}")
    };

    push_and_create_pr(git, hosting, &base, &title, template)
}

fn commonly_breaking(commit_type: &str) -> bool {
    matches!(commit_type, "feat" | "fix" | "refactor")
}

fn prompt_breaking_change(prompter: &dyn Prompter) -> Result<bool, String> {
    let idx = prompter.select("Contains breaking changes?", &["no", "yes"])?;
    Ok(idx == 1)
}

pub fn finish_release_fix(git: &dyn Git, hosting: &dyn HostingPlatform, branch_type: &BranchType, template: Option<&Path>) -> Result<(), String> {
    let BranchType::ReleaseFix { major, minor, patch, name } = branch_type else {
        return Err("Cannot finish: not on a release-fix branch".to_string());
    };
    let title = format!("fix: {}", name.replace('-', " "));
    push_and_create_pr(git, hosting, &SemVer::new(*major, *minor, *patch).release_branch(), &title, template)
}

pub fn finish_hotfix_fix(git: &dyn Git, hosting: &dyn HostingPlatform, branch_type: &BranchType, template: Option<&Path>) -> Result<(), String> {
    let BranchType::HotfixFix { major, minor, patch, name } = branch_type else {
        return Err("Cannot finish: not on a hotfix-fix branch".to_string());
    };
    let title = format!("fix: {}", name.replace('-', " "));
    push_and_create_pr(git, hosting, &SemVer::new(*major, *minor, *patch).hotfix_branch(), &title, template)
}
