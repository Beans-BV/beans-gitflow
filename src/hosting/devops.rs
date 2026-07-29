use std::process::Command;
use super::{HostingPlatform, Result};

pub struct AzureDevOps {
    org: String,
    project: String,
    repo: String,
}

impl AzureDevOps {
    pub fn new(org: String, project: String, repo: String) -> Self {
        Self { org, project, repo }
    }

    fn org_url(&self) -> String {
        // Valid for legacy {org}.visualstudio.com organizations too.
        format!("https://dev.azure.com/{}", self.org)
    }

    /// Canonical PR web URL, built from the coordinates parsed out of the remote.
    /// az's `repository.webUrl` is unreliable (absent in `pr list` responses,
    /// legacy-format for visualstudio.com orgs), so it is never used.
    fn pr_url(&self, id: &str) -> String {
        format!(
            "{}/{}/_git/{}/pullrequest/{id}",
            self.org_url(),
            encode_segment(&self.project),
            encode_segment(&self.repo),
        )
    }

    fn run_az(&self, args: &[String]) -> Result<String> {
        let output = Command::new("az").args(args).output()
            .map_err(|e| format!("Failed to run az: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("az {} failed: {}", args.join(" "), stderr))
        }
    }

    fn repo_args(&self) -> Vec<String> {
        vec![
            "--organization".into(), self.org_url(),
            "--project".into(), self.project.clone(),
            "--repository".into(), self.repo.clone(),
        ]
    }
}

/// URL-encode a path segment. Spaces (decoded during remote-URL detection) are
/// the only realistic case in ADO project/repo names.
fn encode_segment(s: &str) -> String {
    s.replace(' ', "%20")
}

/// Validate a `--query pullRequestId -o tsv` result: a single line of digits.
/// Anything else (empty, az's "None" for null) is a hard error.
fn validate_pr_id(id: &str) -> Result<&str> {
    let id = id.trim();
    if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) {
        Ok(id)
    } else {
        Err(format!("Unexpected az pull request id: '{id}'"))
    }
}

/// az's `--description` is a list argument where each element becomes one line.
fn description_args(body: &str) -> Vec<String> {
    let mut args = vec!["--description".to_string()];
    args.extend(body.split('\n').map(|l| l.to_string()));
    args
}

impl HostingPlatform for AzureDevOps {
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String> {
        // Return the existing active PR for this head/base if there is one.
        let mut list_args: Vec<String> = vec!["repos".into(), "pr".into(), "list".into()];
        list_args.extend(self.repo_args());
        list_args.extend([
            "--source-branch".into(), head.into(),
            "--target-branch".into(), base.into(),
            "--status".into(), "active".into(),
            "--query".into(), "[0].pullRequestId".into(),
            "-o".into(), "tsv".into(),
        ]);
        let existing = self.run_az(&list_args)?;
        if !existing.is_empty() {
            return Ok(self.pr_url(validate_pr_id(&existing)?));
        }

        // A bflow-resolved template (branch-specific/group/default) wins; otherwise fall
        // back to the repository's own default PR template, then to an empty body.
        let default_paths = [
            ".azuredevops/pull_request_template.md",
            ".azuredevops/PULL_REQUEST_TEMPLATE.md",
            ".vsts/pull_request_template.md",
            "pull_request_template.md",
            "PULL_REQUEST_TEMPLATE.md",
            "docs/pull_request_template.md",
        ];
        let body_file = template
            .map(|p| p.to_string())
            .or_else(|| default_paths.iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string()));
        let body = match body_file {
            Some(path) => std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read PR template '{path}': {e}"))?,
            None => String::new(),
        };

        let mut create_args: Vec<String> = vec!["repos".into(), "pr".into(), "create".into()];
        create_args.extend(self.repo_args());
        create_args.extend([
            "--source-branch".into(), head.into(),
            "--target-branch".into(), base.into(),
            "--title".into(), title.into(),
        ]);
        create_args.extend(description_args(&body));
        create_args.extend([
            "--query".into(), "pullRequestId".into(),
            "-o".into(), "tsv".into(),
        ]);
        let created = self.run_az(&create_args)?;
        Ok(self.pr_url(validate_pr_id(&created)?))
    }

    fn check_auth(&self) -> Result<()> {
        // Explicit extension check first: it also prevents az's interactive
        // dynamic-install prompt from firing inside a non-tty command later.
        self.run_az(&["extension".into(), "show".into(), "--name".into(), "azure-devops".into()])
            .map_err(|_| "Azure DevOps CLI extension is missing. Run 'az extension add --name azure-devops'.".to_string())?;
        self.run_az(&["account".into(), "show".into()]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_url_is_canonical_dev_azure_form() {
        // Legacy visualstudio.com orgs get the canonical URL too — it redirects.
        let ado = AzureDevOps::new("wilko".into(), "Shuttel".into(), "Shuttel".into());
        assert_eq!(ado.pr_url("2662"), "https://dev.azure.com/wilko/Shuttel/_git/Shuttel/pullrequest/2662");
    }

    #[test]
    fn pr_url_encodes_spaces_in_project_and_repo() {
        let ado = AzureDevOps::new("beans".into(), "My Shop".into(), "the repo".into());
        assert_eq!(ado.pr_url("7"), "https://dev.azure.com/beans/My%20Shop/_git/the%20repo/pullrequest/7");
    }

    #[test]
    fn validate_pr_id_accepts_digits_only() {
        assert_eq!(validate_pr_id("2662"), Ok("2662"));
        assert_eq!(validate_pr_id(" 42\n"), Ok("42"));
        assert!(validate_pr_id("").is_err());
        assert!(validate_pr_id("None").is_err());
        assert!(validate_pr_id("https://x\t42").is_err());
    }

    #[test]
    fn description_args_one_arg_per_line() {
        assert_eq!(
            description_args("line one\nline two\n\nline four"),
            vec!["--description", "line one", "line two", "", "line four"],
        );
    }

    #[test]
    fn description_args_empty_body_is_single_empty_line() {
        assert_eq!(description_args(""), vec!["--description", ""]);
    }
}
