pub mod detect;
pub mod devops;
pub mod github;
pub mod template;

pub type Result<T> = std::result::Result<T, String>;

/// A pull request that has been merged: everything a finish needs to decide the
/// work is done and clean up. `head_sha` is the source-branch tip that was
/// merged — the only reliable "is my local branch the merged one" signal, since
/// squash/rebase merges break ancestor checks.
#[derive(Debug, Clone, PartialEq)]
pub struct MergedPr {
    pub url: String,
    pub head_sha: String,
    /// Short name of the branch the PR merged into (e.g. `develop`).
    pub base: String,
}

/// A PR that landed `head` into a specific `base`: everything a protected-mode
/// landing needs to confirm the merge and record its commit. Distinct from
/// `MergedPr` (which answers "is my work branch done, whatever the base") —
/// this answers "did head land into base", and carries the merge commit SHA
/// rather than the base name, since the base is already known by the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct LandedPr {
    pub url: String,
    pub head_sha: String,
    pub merge_commit_sha: String,
}

pub trait HostingPlatform {
    /// Create a PR (or return the URL of an existing open one). When `template` is
    /// `Some`, its contents become the PR body; when `None`, the platform falls back to
    /// the repository's own default template.
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String>;
    /// The merged PR for `head`, if the branch's most recent PR is merged.
    /// An open or abandoned newer PR yields `None` — the branch is still in play.
    fn merged_pr(&self, head: &str) -> Result<Option<MergedPr>>;
    /// The newest `head`→`base` PR, if it is merged. An open or abandoned newer
    /// PR yields `None`.
    fn merged_pr_to(&self, head: &str, base: &str) -> Result<Option<LandedPr>>;
    fn open_url(&self, url: &str) -> Result<()> {
        open_in_browser(url)
    }
    fn check_auth(&self) -> Result<()>;
}

/// Port for invoking a hosting CLI (`gh`, `az`, ...). The providers own the
/// policy — which flags to pass, which failures are normal — and depend on this
/// trait so that policy is testable without an installed CLI. `SystemCli` is the
/// only production implementation, keeping "no subprocess calls outside adapter
/// impls" (SKILL.md principle 1) true at a single point.
pub trait CliRunner {
    /// Run `program` with `args`, returning trimmed stdout on success or a
    /// `"<cli> <args> failed: <stderr>"` error.
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;
}

/// The real runner: spawns the CLI as a child process.
pub struct SystemCli;

impl CliRunner for SystemCli {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        use std::process::Command;
        let output = Command::new(program).args(args).output()
            .map_err(|e| format!("Failed to run {program}: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("{program} {} failed: {stderr}", args.join(" ")))
        }
    }
}

/// PR-body precedence shared by all providers: a bflow-resolved template wins,
/// else the first existing native default-template path, else `None` (empty
/// body). The native path *lists* stay per-provider knowledge on purpose.
fn resolve_body_file(template: Option<&str>, native_paths: &[&str]) -> Option<String> {
    template
        .map(|p| p.to_string())
        .or_else(|| {
            native_paths.iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(|p| p.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::resolve_body_file;

    #[test]
    fn bflow_template_wins_over_native_paths() {
        // The native path need not even exist for the template to win.
        let result = resolve_body_file(Some(".github/pr-templates/bflow-fix.md"), &["nope.md"]);
        assert_eq!(result, Some(".github/pr-templates/bflow-fix.md".to_string()));
    }

    #[test]
    fn no_template_and_no_existing_native_path_is_empty_body() {
        assert_eq!(resolve_body_file(None, &["definitely/not/a/real/path.md"]), None);
    }
}

/// Open a URL in the OS default browser (platform dispatch, provider-agnostic).
pub fn open_in_browser(url: &str) -> Result<()> {
    use std::process::Command;
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).output();
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", url]).output();
    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(url).output();
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let result: std::io::Result<std::process::Output> =
        Err(std::io::Error::other("no browser-opener known for this platform"));

    let output = result.map_err(|e| format!("Failed to open URL: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Failed to open URL: {stderr}"))
    }
}
