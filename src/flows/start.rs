use crate::git::Git;
use crate::menu;
use crate::version::SemVer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReleaseType {
    Major,
    Minor,
}

pub fn start_work_branch(git: &dyn Git, prefix: &str, name: &str, from: &str, no_checkout: bool) -> Result<(), String> {
    let branch = format!("{prefix}/{name}");
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, from)
    } else {
        git.create_branch(&branch, from)
    }.map_err(|e| {
        if e.contains("not a commit") {
            format!("Branch '{from}' does not exist. Use --base to specify a different base branch.")
        } else {
            e
        }
    })?;
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}

pub fn start_release(git: &dyn Git, release_type: Option<ReleaseType>) -> Result<(), String> {
    resolve_or_create_release(git, release_type)?;
    Ok(())
}

pub fn start_release_fix(git: &dyn Git, name: &str, no_checkout: bool) -> Result<(), String> {
    let release_branch = if no_checkout {
        let branches = git.list_branches_matching("release/*")?;
        let release_branches: Vec<&String> = branches.iter()
            .filter(|b| b.starts_with("release/") && !b.starts_with("release-fix/"))
            .collect();
        release_branches.first()
            .ok_or("No release branch found. Create one with 'bflow start release' first.")?
            .to_string()
    } else {
        let current = git.current_branch()?;
        if current.strip_prefix("release/").is_none() {
            return Err("Not on a release branch".to_string());
        }
        current
    };

    let version = release_branch.strip_prefix("release/").unwrap();
    let branch = format!("release-fix/{version}/{name}");
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, &release_branch)?;
    } else {
        git.create_branch(&branch, &release_branch)?;
    }
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}

pub fn start_hotfix_fix(git: &dyn Git, name: &str, no_checkout: bool) -> Result<(), String> {
    let hotfix_branch = resolve_or_create_hotfix(git, no_checkout)?;
    let version = hotfix_branch.strip_prefix("hotfix/").unwrap();
    let branch = format!("hotfix-fix/{version}/{name}");
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, &hotfix_branch)?;
    } else {
        git.create_branch(&branch, &hotfix_branch)?;
    }
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}

fn resolve_or_create_release(git: &dyn Git, release_type: Option<ReleaseType>) -> Result<String, String> {
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
    let next = match release_type {
        Some(ReleaseType::Major) => latest.bump_major(),
        Some(ReleaseType::Minor) => latest.bump_minor(),
        None => {
            let has_breaking = detect_breaking_changes(git, &latest);
            prompt_release_type(&latest, has_breaking)?
        }
    };

    let branch = next.release_branch();
    let rc = next.with_rc(1);
    let tag = rc.tag_name();

    println!("Creating release branch: {branch}");
    git.checkout("develop")?;
    git.create_branch(&branch, "develop")?;
    git.push(&branch)?;

    println!("Tagging: {tag}");
    git.create_tag(&tag, &format!("chore: create release branch {next}"))?;
    git.push_tag(&tag)?;

    Ok(branch)
}

pub fn detect_breaking_changes(git: &dyn Git, latest: &SemVer) -> bool {
    let tag = latest.tag_name();
    let messages = match git.commit_messages(&tag, "HEAD") {
        Ok(msgs) => msgs,
        Err(_) => return false,
    };

    for msg in &messages {
        let first_line = msg.lines().next().unwrap_or("");

        // Conventional commits: "feat!:", "fix!:", "refactor(scope)!:" etc.
        if let Some(colon_pos) = first_line.find(':') {
            let before_colon = &first_line[..colon_pos];
            if before_colon.ends_with('!') {
                return true;
            }
        }

        // "BREAKING CHANGE" or "breaking change" anywhere in the message body
        if msg.to_lowercase().contains("breaking change") {
            return true;
        }
    }

    false
}

fn prompt_release_type(latest: &SemVer, has_breaking: bool) -> Result<SemVer, String> {
    let major_label = format!("major (v{} → v{})", latest, latest.bump_major());
    let minor_label = format!("minor (v{} → v{})", latest, latest.bump_minor());

    let items: Vec<&str> = if has_breaking {
        println!("Breaking changes detected since last release.");
        vec![&major_label, &minor_label]
    } else {
        vec![&minor_label, &major_label]
    };

    let idx = menu::show_select("Release type", &items)?;
    let selected = items[idx];

    if selected.starts_with("major") {
        Ok(latest.bump_major())
    } else {
        Ok(latest.bump_minor())
    }
}

fn resolve_or_create_hotfix(git: &dyn Git, no_checkout: bool) -> Result<String, String> {
    let branches = git.list_branches_matching("hotfix/*")?;
    let hotfix_branches: Vec<&String> = branches.iter()
        .filter(|b| b.starts_with("hotfix/") && !b.starts_with("hotfix-fix/"))
        .collect();

    if let Some(branch) = hotfix_branches.first() {
        println!("Using existing hotfix branch: {branch}");
        if !no_checkout {
            git.checkout(branch)?;
        }
        return Ok(branch.to_string());
    }

    let latest = find_latest_tag(git)?;
    let next = latest.bump_patch();
    let branch = next.hotfix_branch();

    println!("Creating hotfix branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, "main")?;
    } else {
        git.checkout("main")?;
        git.create_branch(&branch, "main")?;
    }
    git.push(&branch)?;

    Ok(branch)
}

fn find_latest_tag(git: &dyn Git) -> Result<SemVer, String> {
    let tags = git.list_tags()?;
    let all: Vec<SemVer> = tags.iter().filter_map(|t| SemVer::parse(t)).collect();

    // Prefer clean release tags
    if let Some(v) = all.iter().filter(|v| !v.is_pre_release()).max() {
        return Ok(v.clone());
    }

    // Fall back to highest RC tag (stripped to release) if no clean tags exist
    if let Some(v) = all.iter().filter(|v| v.is_rc()).max() {
        return Ok(v.to_release());
    }

    Ok(SemVer::new(0, 0, 0))
}
