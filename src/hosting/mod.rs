pub mod detect;
pub mod devops;
pub mod github;
pub mod template;

pub type Result<T> = std::result::Result<T, String>;

pub trait HostingPlatform {
    /// Create a PR (or return the URL of an existing open one). When `template` is
    /// `Some`, its contents become the PR body; when `None`, the platform falls back to
    /// the repository's own default template.
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String>;
    fn open_url(&self, url: &str) -> Result<()> {
        open_in_browser(url)
    }
    fn check_auth(&self) -> Result<()>;
}

/// Run a hosting CLI (`gh`, `az`, ...), returning trimmed stdout on success or a
/// `"<cli> <args> failed: <stderr>"` error. Shared by all provider implementations.
fn run_cli(program: &str, args: &[impl AsRef<str>]) -> Result<String> {
    use std::process::Command;
    let output = Command::new(program).args(args.iter().map(|a| a.as_ref())).output()
        .map_err(|e| format!("Failed to run {program}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let joined = args.iter().map(|a| a.as_ref()).collect::<Vec<_>>().join(" ");
        Err(format!("{program} {joined} failed: {stderr}"))
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
