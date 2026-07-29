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

/// Open a URL in the OS default browser (platform dispatch, provider-agnostic).
pub fn open_in_browser(url: &str) -> Result<()> {
    use std::process::Command;
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).output();
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", url]).output();
    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(url).output();
    result.map_err(|e| format!("Failed to open URL: {e}"))?;
    Ok(())
}
