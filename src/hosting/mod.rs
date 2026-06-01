pub mod github;
pub mod template;

pub type Result<T> = std::result::Result<T, String>;

pub trait HostingPlatform {
    /// Create a PR (or return the URL of an existing open one). When `template` is
    /// `Some`, its contents become the PR body; when `None`, the platform falls back to
    /// the repository's own default template.
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String>;
    fn open_url(&self, url: &str) -> Result<()>;
    fn check_auth(&self) -> Result<()>;
}
