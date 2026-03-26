pub mod branch;

use std::process::Command;

pub type Result<T> = std::result::Result<T, String>;

pub trait Git {
    fn current_branch(&self) -> Result<String>;
    fn fetch(&self) -> Result<()>;
    fn checkout(&self, branch: &str) -> Result<()>;
    fn create_branch(&self, branch: &str, from: &str) -> Result<()>;
    fn push(&self, branch: &str) -> Result<()>;
    fn push_tag(&self, tag: &str) -> Result<()>;
    fn create_tag(&self, tag: &str, message: &str) -> Result<()>;
    fn merge(&self, branch: &str, message: &str) -> Result<()>;
    fn list_tags(&self) -> Result<Vec<String>>;
    fn list_branches_matching(&self, pattern: &str) -> Result<Vec<String>>;
    fn is_working_tree_clean(&self) -> Result<bool>;
    fn delete_branch_local(&self, branch: &str) -> Result<()>;
    fn delete_branch_remote(&self, branch: &str) -> Result<()>;
    fn tags_on_branch(&self, branch: &str) -> Result<Vec<String>>;
    fn list_remote_branches(&self) -> Result<Vec<String>>;
    fn merge_base(&self, a: &str, b: &str) -> Result<String>;
    fn rev_list_count(&self, from: &str, to: &str) -> Result<u32>;
    fn stash_push(&self) -> Result<()>;
    fn stash_pop(&self) -> Result<()>;
}

pub struct GitCli;

impl GitCli {
    pub fn new() -> Self { Self }

    fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git").args(args).output()
            .map_err(|e| format!("Failed to run git: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("git {} failed: {}", args.join(" "), stderr))
        }
    }
}

impl Git for GitCli {
    fn current_branch(&self) -> Result<String> { self.run(&["rev-parse", "--abbrev-ref", "HEAD"]) }
    fn fetch(&self) -> Result<()> { self.run(&["fetch", "--all", "--prune"]).map(|_| ()) }
    fn checkout(&self, branch: &str) -> Result<()> { self.run(&["checkout", branch]).map(|_| ()) }
    fn create_branch(&self, branch: &str, from: &str) -> Result<()> { self.run(&["checkout", "-b", branch, from]).map(|_| ()) }
    fn push(&self, branch: &str) -> Result<()> { self.run(&["push", "-u", "origin", branch]).map(|_| ()) }
    fn push_tag(&self, tag: &str) -> Result<()> { self.run(&["push", "origin", tag]).map(|_| ()) }
    fn create_tag(&self, tag: &str, message: &str) -> Result<()> { self.run(&["tag", "-a", tag, "-m", message]).map(|_| ()) }
    fn merge(&self, branch: &str, message: &str) -> Result<()> { self.run(&["merge", branch, "--no-ff", "-m", message]).map(|_| ()) }
    fn list_tags(&self) -> Result<Vec<String>> {
        let output = self.run(&["tag", "--list"])?;
        Ok(output.lines().map(|s| s.to_string()).filter(|s| !s.is_empty()).collect())
    }
    fn list_branches_matching(&self, pattern: &str) -> Result<Vec<String>> {
        let ref_pattern = format!("refs/remotes/origin/{pattern}");
        let local_pattern = format!("refs/heads/{pattern}");
        let output = self.run(&[
            "for-each-ref", "--format=%(refname:short)",
            &ref_pattern, &local_pattern,
        ])?;
        Ok(output
            .lines()
            .map(|s| s.trim_start_matches("origin/").to_string())
            .filter(|s| !s.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect())
    }
    fn is_working_tree_clean(&self) -> Result<bool> {
        let output = self.run(&["status", "--porcelain"])?;
        Ok(output.is_empty())
    }
    fn delete_branch_local(&self, branch: &str) -> Result<()> { self.run(&["branch", "-D", branch]).map(|_| ()) }
    fn delete_branch_remote(&self, branch: &str) -> Result<()> { self.run(&["push", "origin", "--delete", branch]).map(|_| ()) }
    fn tags_on_branch(&self, branch: &str) -> Result<Vec<String>> {
        let output = self.run(&["tag", "--merged", branch])?;
        Ok(output.lines().map(|s| s.to_string()).filter(|s| !s.is_empty()).collect())
    }
    fn list_remote_branches(&self) -> Result<Vec<String>> {
        let output = self.run(&["for-each-ref", "--format=%(refname:short)", "refs/remotes/origin/"])?;
        Ok(output
            .lines()
            .map(|s| s.trim_start_matches("origin/").to_string())
            .filter(|s| !s.is_empty() && s != "HEAD")
            .collect())
    }
    fn merge_base(&self, a: &str, b: &str) -> Result<String> {
        self.run(&["merge-base", a, b])
    }
    fn rev_list_count(&self, from: &str, to: &str) -> Result<u32> {
        let range = format!("{from}..{to}");
        let output = self.run(&["rev-list", "--count", &range])?;
        output.parse::<u32>().map_err(|e| format!("Failed to parse rev-list count: {e}"))
    }
    fn stash_push(&self) -> Result<()> { self.run(&["stash", "push", "-u"]).map(|_| ()) }
    fn stash_pop(&self) -> Result<()> { self.run(&["stash", "pop"]).map(|_| ()) }
}
