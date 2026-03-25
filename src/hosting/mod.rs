pub mod github;

pub type Result<T> = std::result::Result<T, String>;

pub trait HostingPlatform {
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str) -> Result<String>;
    fn open_url(&self, url: &str) -> Result<()>;
    fn check_auth(&self) -> Result<()>;
}
