use std::process::Command;
use super::{HostingPlatform, Result};

pub struct GitHub;

impl GitHub {
    pub fn new() -> Self { Self }

    fn run_gh(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("gh").args(args).output()
            .map_err(|e| format!("Failed to run gh: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("gh {} failed: {}", args.join(" "), stderr))
        }
    }
}

impl HostingPlatform for GitHub {
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str) -> Result<String> {
        let existing = self.run_gh(&["pr", "view", head, "--json", "url", "--jq", ".url"]);
        if let Ok(url) = existing {
            if !url.is_empty() { return Ok(url); }
        }
        self.run_gh(&["pr", "create", "--head", head, "--base", base, "--title", title])
    }

    fn open_url(&self, url: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        let result = Command::new("open").arg(url).output();
        #[cfg(target_os = "windows")]
        let result = Command::new("cmd").args(["/C", "start", "", url]).output();
        #[cfg(target_os = "linux")]
        let result = Command::new("xdg-open").arg(url).output();
        result.map_err(|e| format!("Failed to open URL: {e}"))?;
        Ok(())
    }

    fn check_auth(&self) -> Result<()> {
        self.run_gh(&["auth", "status"]).map(|_| ())
    }
}
