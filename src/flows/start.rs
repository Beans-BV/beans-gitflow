use crate::git::Git;
use crate::prompt::Prompter;
use crate::version::SemVer;
use crate::worktree::{open_worktree, WorktreeContext};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReleaseType {
    Major,
    Minor,
}

/// An active worktree context leaves the current checkout untouched, exactly
/// like `--no-checkout`. This is the flows' single derivation of that rule
/// (pinned by the worktree tests: passing no_checkout=false with a worktree
/// context must still take the no-checkout path).
fn effective_no_checkout(no_checkout: bool, worktree: &Option<WorktreeContext<'_>>) -> bool {
    no_checkout || worktree.is_some()
}

/// Bring `branch` into existence off `base`: create it (checked out or not),
/// push it, narrate both, and materialize a worktree when one is active.
///
/// The three `start_*` flows differ only in how they derive `branch` and
/// `base` — everything after that is one piece of knowledge, and the
/// create/push/narrate/worktree ordering is what the start tests assert.
fn materialize_branch(
    git: &dyn Git,
    branch: &str,
    base: &str,
    no_checkout: bool,
    worktree: Option<WorktreeContext<'_>>,
) -> Result<(), String> {
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(branch, base)?;
    } else {
        git.create_branch(branch, base)?;
    }
    git.push(branch)?;
    println!("Branch '{branch}' created and pushed.");
    if let Some(ctx) = worktree {
        open_worktree(git, ctx.editor, ctx.config, branch)?;
    }
    Ok(())
}

/// The version a `release/`/`hotfix/` branch carries. Fix-branch names are
/// generated from it via `SemVer` methods rather than string surgery
/// (decisions.md, Release Discipline), so a parent branch with no parseable
/// version is a hard error instead of a malformed child branch.
fn version_of(branch: &str, prefix: &str) -> Result<SemVer, String> {
    branch.strip_prefix(prefix)
        .and_then(SemVer::parse)
        .ok_or_else(|| format!("Branch '{branch}' does not carry a version; cannot derive a fix branch from it."))
}

pub fn start_work_branch(git: &dyn Git, prefix: &str, name: &str, from: &str, no_checkout: bool, worktree: Option<WorktreeContext<'_>>) -> Result<(), String> {
    let branch = format!("{prefix}/{name}");
    let effective_no_checkout = effective_no_checkout(no_checkout, &worktree);
    // Unlike the fix flows, `from` comes straight from the user (`--base`), so
    // git's "not a commit" is rewritten into guidance naming the flag.
    materialize_branch(git, &branch, from, effective_no_checkout, worktree).map_err(|e| {
        if e.contains("not a commit") {
            format!("Branch '{from}' does not exist. Use --base to specify a different base branch.")
        } else {
            e
        }
    })
}

pub fn start_release(git: &dyn Git, prompter: &dyn Prompter, release_type: Option<ReleaseType>) -> Result<(), String> {
    resolve_or_create_release(git, prompter, release_type)?;
    Ok(())
}

pub fn start_release_fix(git: &dyn Git, name: &str, no_checkout: bool, worktree: Option<WorktreeContext<'_>>) -> Result<(), String> {
    let effective_no_checkout = effective_no_checkout(no_checkout, &worktree);
    let release_branch = if effective_no_checkout {
        super::branches_with_prefix(git, "release")?
            .first()
            .ok_or("No release branch found. Create one with 'bflow start release' first.")?
            .clone()
    } else {
        let current = git.current_branch()?;
        if current.strip_prefix("release/").is_none() {
            return Err("Not on a release branch".to_string());
        }
        current
    };

    let branch = version_of(&release_branch, "release/")?.release_fix_branch(name);
    materialize_branch(git, &branch, &release_branch, effective_no_checkout, worktree)
}

pub fn start_hotfix_fix(git: &dyn Git, name: &str, no_checkout: bool, worktree: Option<WorktreeContext<'_>>) -> Result<(), String> {
    let effective_no_checkout = effective_no_checkout(no_checkout, &worktree);
    let hotfix_branch = resolve_or_create_hotfix(git, effective_no_checkout)?;
    let branch = version_of(&hotfix_branch, "hotfix/")?.hotfix_fix_branch(name);
    materialize_branch(git, &branch, &hotfix_branch, effective_no_checkout, worktree)
}

fn resolve_or_create_release(git: &dyn Git, prompter: &dyn Prompter, release_type: Option<ReleaseType>) -> Result<String, String> {
    let release_branches = super::branches_with_prefix(git, "release")?;

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
            prompt_release_type(prompter, &latest, has_breaking)?
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
    // Scan develop explicitly — start release may run from any branch
    let messages = match git.commit_messages(&tag, "develop") {
        Ok(msgs) => msgs,
        Err(_) => match git.commit_messages(&tag, "origin/develop") {
            Ok(msgs) => msgs,
            Err(_) => return false,
        },
    };

    for msg in &messages {
        if message_is_breaking(msg) {
            return true;
        }
    }

    false
}

pub(crate) fn message_is_breaking(msg: &str) -> bool {
    let mut lines = msg.lines();
    let first_line = lines.next().unwrap_or("");

    // Conventional commits: type or type(scope) followed by "!:" e.g. "feat!:", "refactor(auth)!:"
    if let Some(colon_pos) = first_line.find(':') {
        let before_colon = &first_line[..colon_pos];
        if before_colon.ends_with('!') {
            return true;
        }
    }

    // Conventional commits footer: a line starting with "BREAKING CHANGE:" or "BREAKING-CHANGE:"
    // (case-insensitive, followed by a colon and space per the spec)
    for line in lines {
        let trimmed = line.trim_start();
        let upper = trimmed.to_uppercase();
        if upper.starts_with("BREAKING CHANGE:") || upper.starts_with("BREAKING-CHANGE:") {
            return true;
        }
    }

    false
}

fn prompt_release_type(prompter: &dyn Prompter, latest: &SemVer, has_breaking: bool) -> Result<SemVer, String> {
    let major_label = format!("major (v{} → v{})", latest, latest.bump_major());
    let minor_label = format!("minor (v{} → v{})", latest, latest.bump_minor());

    let items: Vec<&str> = if has_breaking {
        println!("Breaking changes detected since last release.");
        vec![&major_label, &minor_label]
    } else {
        vec![&minor_label, &major_label]
    };

    let idx = prompter.select("Release type", &items)?;
    let selected = items[idx];

    if selected.starts_with("major") {
        Ok(latest.bump_major())
    } else {
        Ok(latest.bump_minor())
    }
}

fn resolve_or_create_hotfix(git: &dyn Git, no_checkout: bool) -> Result<String, String> {
    let hotfix_branches = super::branches_with_prefix(git, "hotfix")?;

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

#[cfg(test)]
mod tests {
    use super::message_is_breaking;

    #[test]
    fn bang_in_title() {
        assert!(message_is_breaking("feat!: remove legacy API"));
    }

    #[test]
    fn bang_with_scope() {
        assert!(message_is_breaking("refactor(auth)!: rewrite token handling"));
    }

    #[test]
    fn breaking_change_footer() {
        let msg = "feat: new auth flow\n\nBREAKING CHANGE: old tokens are invalidated";
        assert!(message_is_breaking(msg));
    }

    #[test]
    fn breaking_change_footer_hyphenated() {
        let msg = "feat: new auth\n\nBREAKING-CHANGE: old tokens invalidated";
        assert!(message_is_breaking(msg));
    }

    #[test]
    fn breaking_change_footer_case_insensitive() {
        let msg = "feat: new auth\n\nbreaking change: old tokens invalidated";
        assert!(message_is_breaking(msg));
    }

    #[test]
    fn non_breaking_change_in_body_is_not_flagged() {
        // This is the bug the reviewer caught — "non-breaking change" should NOT match
        let msg = "feat: new feature\n\nThis is a non-breaking change to the API.";
        assert!(!message_is_breaking(msg));
    }

    #[test]
    fn breaking_change_mention_without_colon_is_not_flagged() {
        // Only the footer format (with colon) should count
        let msg = "feat: new feature\n\nWe discussed breaking change options earlier.";
        assert!(!message_is_breaking(msg));
    }

    #[test]
    fn plain_conventional_commit_is_not_breaking() {
        assert!(!message_is_breaking("feat: add login page"));
        assert!(!message_is_breaking("fix: correct typo"));
        assert!(!message_is_breaking("chore: update deps"));
    }

    #[test]
    fn bang_in_body_does_not_count() {
        // The ! must be in the title before the colon, not in the body
        let msg = "feat: add feature\n\nThis is great!\nReally awesome.";
        assert!(!message_is_breaking(msg));
    }
}
