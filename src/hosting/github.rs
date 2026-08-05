use super::{resolve_body_file, CliRunner, HostingPlatform, MergedPr, Result};

const AUTH_REMEDY: &str = "If authentication expired, run 'gh auth login', then re-run 'bflow finish'.";

pub struct GitHub<'a> {
    runner: &'a dyn CliRunner,
}

impl<'a> GitHub<'a> {
    pub fn new(runner: &'a dyn CliRunner) -> Self {
        Self { runner }
    }

    fn run_gh(&self, args: &[&str]) -> Result<String> {
        self.runner.run("gh", args)
    }
}

impl HostingPlatform for GitHub<'_> {
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String> {
        match self.run_gh(&["pr", "view", head, "--json", "url,state", "--jq", "select(.state == \"OPEN\") | .url"]) {
            Ok(url) if !url.is_empty() => return Ok(url),
            // A closed/merged PR filters to empty output — create a new one.
            Ok(_) => {}
            // gh exits non-zero both when no PR exists (normal here) and on real
            // failures (auth expiry, network). Only the former may be swallowed.
            Err(e) if e.contains("no pull requests found") => {}
            Err(e) => return Err(format!("Could not check for an existing PR: {e}\n{AUTH_REMEDY}")),
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

    fn merged_pr(&self, head: &str) -> Result<Option<MergedPr>> {
        // The branch's most recent PR decides: `gh pr list` returns newest first,
        // and the jq filter drops anything not MERGED (open PR → the branch is
        // still in play; closed-unmerged → a fresh PR is the right move).
        let line = self
            .run_gh(&[
                "pr", "list", "--head", head, "--state", "all", "--limit", "1",
                "--json", "url,state,headRefOid,baseRefName",
                "--jq", r#".[0] | select(.state == "MERGED") | [.url, .headRefOid, .baseRefName] | @tsv"#,
            ])
            .map_err(|e| format!("Could not check for a merged PR: {e}\n{AUTH_REMEDY}"))?;
        parse_merged_pr(&line)
    }

    fn check_auth(&self) -> Result<()> {
        self.run_gh(&["auth", "status"]).map(|_| ())
    }
}

/// Parse the `url<TAB>headRefOid<TAB>baseRefName` line the merged-PR jq filter
/// emits. Empty output is the normal "no merged PR" case; anything else that
/// isn't three non-empty fields is a hard error (never guess about cleanup).
fn parse_merged_pr(line: &str) -> Result<Option<MergedPr>> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }
    match line.split('\t').collect::<Vec<_>>().as_slice() {
        [url, sha, base] if !url.is_empty() && !sha.is_empty() && !base.is_empty() => {
            Ok(Some(MergedPr {
                url: url.to_string(),
                head_sha: sha.to_string(),
                base: base.to_string(),
            }))
        }
        _ => Err(format!("Unexpected merged-PR data from gh: '{line}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_output_means_no_merged_pr() {
        assert_eq!(parse_merged_pr(""), Ok(None));
        assert_eq!(parse_merged_pr("  \n"), Ok(None));
    }

    #[test]
    fn three_fields_parse_into_merged_pr() {
        let pr = parse_merged_pr("https://github.com/o/r/pull/49\tabc123\tdevelop").unwrap().unwrap();
        assert_eq!(pr.url, "https://github.com/o/r/pull/49");
        assert_eq!(pr.head_sha, "abc123");
        assert_eq!(pr.base, "develop");
    }

    #[test]
    fn malformed_output_is_a_hard_error() {
        assert!(parse_merged_pr("only-a-url").is_err());
        assert!(parse_merged_pr("url\t\tdevelop").is_err());
    }
}
