use super::{resolve_body_file, run_cli, HostingPlatform, Result};

pub struct GitHub;

impl Default for GitHub {
    fn default() -> Self { Self::new() }
}

impl GitHub {
    pub fn new() -> Self { Self }

    fn run_gh(&self, args: &[&str]) -> Result<String> {
        run_cli("gh", args)
    }
}

impl HostingPlatform for GitHub {
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String> {
        let existing = self.run_gh(&["pr", "view", head, "--json", "url,state", "--jq", "select(.state == \"OPEN\") | .url"]);
        if let Ok(url) = existing {
            if !url.is_empty() { return Ok(url); }
        }

        let git_default_paths = [
            ".github/PULL_REQUEST_TEMPLATE.md",
            ".github/pull_request_template.md",
            "PULL_REQUEST_TEMPLATE.md",
            "pull_request_template.md",
            "docs/pull_request_template.md",
        ];
        let body_file = resolve_body_file(template, &git_default_paths);

        if let Some(path) = body_file {
            self.run_gh(&["pr", "create", "--head", head, "--base", base, "--title", title, "--body-file", &path])
        } else {
            self.run_gh(&["pr", "create", "--head", head, "--base", base, "--title", title, "--body", ""])
        }
    }

    fn check_auth(&self) -> Result<()> {
        self.run_gh(&["auth", "status"]).map(|_| ())
    }
}
