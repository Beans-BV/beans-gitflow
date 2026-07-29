pub mod branch;

use std::path::{Path, PathBuf};
use std::process::Command;

pub type Result<T> = std::result::Result<T, String>;

pub trait Git {
    fn current_branch(&self) -> Result<String>;
    fn fetch(&self) -> Result<()>;
    fn checkout(&self, branch: &str) -> Result<()>;
    fn create_branch(&self, branch: &str, from: &str) -> Result<()>;
    fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<()>;
    fn push(&self, branch: &str) -> Result<()>;
    fn push_tag(&self, tag: &str) -> Result<()>;
    fn create_tag(&self, tag: &str, message: &str) -> Result<()>;
    fn merge(&self, branch: &str, message: &str) -> Result<()>;
    fn ff_merge(&self, branch: &str) -> Result<()>;
    fn list_tags(&self) -> Result<Vec<String>>;
    /// Contract: returns a sorted, deduplicated list — consumers pick
    /// candidates by position (`.first()`), so order must be deterministic.
    fn list_branches_matching(&self, pattern: &str) -> Result<Vec<String>>;
    fn is_working_tree_clean(&self) -> Result<bool>;
    fn delete_branch_local(&self, branch: &str) -> Result<()>;
    fn delete_branch_remote(&self, branch: &str) -> Result<()>;
    fn tags_on_branch(&self, branch: &str) -> Result<Vec<String>>;
    fn list_remote_branches(&self) -> Result<Vec<String>>;
    fn merge_base(&self, a: &str, b: &str) -> Result<String>;
    fn rev_list_count(&self, from: &str, to: &str) -> Result<u32>;
    fn commit_messages(&self, from: &str, to: &str) -> Result<Vec<String>>;

    // Idempotency primitives
    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool>;
    fn tag_exists(&self, tag: &str) -> Result<bool>;
    fn local_branch_exists(&self, branch: &str) -> Result<bool>;
    fn remote_branch_exists(&self, branch: &str) -> Result<bool>;
    fn remote_tag_exists(&self, tag: &str) -> Result<bool>;
    fn is_pushed(&self, branch: &str) -> Result<bool>;
    fn is_mid_merge(&self) -> Result<bool>;
    fn has_unmerged_paths(&self) -> Result<bool>;
    fn git_dir(&self) -> Result<PathBuf>;

    /// URL of the `origin` remote (`git remote get-url origin`). Errors when the
    /// remote does not exist; callers decide how to handle a missing remote.
    fn remote_url(&self) -> Result<String>;

    // Worktree / config primitives
    /// Read a git config value (`git config --get <key>`). Returns `None` when unset.
    fn get_config(&self, key: &str) -> Result<Option<String>>;
    /// Write a git config value. `global` selects `--global` (user) vs local (repo) scope.
    fn set_config(&self, key: &str, value: &str, global: bool) -> Result<()>;
    /// Remove a git config value. A key that is already unset is treated as success.
    fn unset_config(&self, key: &str, global: bool) -> Result<()>;
    /// Absolute path to the MAIN working tree's root. Stable even when run from
    /// inside a linked worktree (`rev-parse --show-toplevel` would return the
    /// worktree's own directory there, compounding worktree folder names).
    fn repo_root(&self) -> Result<PathBuf>;
    /// Add a worktree at `path` checked out to the (already existing) `branch`.
    fn add_worktree(&self, path: &Path, branch: &str) -> Result<()>;

    // Stash by message (safer than blind pop)
    fn stash_push_with_message(&self, msg: &str) -> Result<()>;
    fn find_stash_by_message(&self, msg: &str) -> Result<Option<String>>;
    fn stash_pop_ref(&self, stash_ref: &str) -> Result<()>;
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

    /// Run a check command that uses exit 0/1 as a true/false result
    /// (e.g., `merge-base --is-ancestor`, `show-ref --verify`).
    /// Exit codes other than 0 or 1 are treated as errors.
    fn run_check(&self, args: &[&str]) -> Result<bool> {
        let output = Command::new("git").args(args).output()
            .map_err(|e| format!("Failed to run git: {e}"))?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            Some(code) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(format!("git {} failed (exit {code}): {}", args.join(" "), stderr))
            }
            None => Err(format!("git {} terminated by signal", args.join(" "))),
        }
    }

    /// Resolve a ref name to its SHA. Only needed internally (`is_pushed`
    /// compares local vs remote SHAs), so not part of the `Git` port.
    fn rev_parse(&self, refname: &str) -> Result<String> {
        self.run(&["rev-parse", refname])
    }

    /// Run a `git config --get`-style command that uses exit 1 to mean "key not set".
    /// Returns `Ok(None)` on exit 1, `Ok(Some(value))` on exit 0, and an error otherwise.
    fn run_config(&self, args: &[&str]) -> Result<Option<String>> {
        let output = Command::new("git").args(args).output()
            .map_err(|e| format!("Failed to run git: {e}"))?;
        match output.status.code() {
            Some(0) => Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string())),
            Some(1) => Ok(None),
            Some(code) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(format!("git {} failed (exit {code}): {}", args.join(" "), stderr))
            }
            None => Err(format!("git {} terminated by signal", args.join(" "))),
        }
    }
}

impl Default for GitCli {
    fn default() -> Self { Self::new() }
}

impl Git for GitCli {
    fn current_branch(&self) -> Result<String> { self.run(&["rev-parse", "--abbrev-ref", "HEAD"]) }
    fn fetch(&self) -> Result<()> { self.run(&["fetch", "--all", "--prune"]).map(|_| ()) }
    fn checkout(&self, branch: &str) -> Result<()> { self.run(&["checkout", branch]).map(|_| ()) }
    fn create_branch(&self, branch: &str, from: &str) -> Result<()> { self.run(&["checkout", "-b", branch, from]).map(|_| ()) }
    fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<()> { self.run(&["branch", branch, from]).map(|_| ()) }
    fn push(&self, branch: &str) -> Result<()> { self.run(&["push", "-u", "origin", branch]).map(|_| ()) }
    fn push_tag(&self, tag: &str) -> Result<()> { self.run(&["push", "origin", tag]).map(|_| ()) }
    fn create_tag(&self, tag: &str, message: &str) -> Result<()> { self.run(&["tag", "-a", tag, "-m", message]).map(|_| ()) }
    fn merge(&self, branch: &str, message: &str) -> Result<()> { self.run(&["merge", branch, "--no-ff", "-m", message]).map(|_| ()) }
    fn ff_merge(&self, branch: &str) -> Result<()> { self.run(&["merge", branch, "--ff-only"]).map(|_| ()) }
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
        // Sorted + deduped per the trait contract.
        let mut branches: Vec<String> = output
            .lines()
            .map(|s| s.trim_start_matches("origin/").to_string())
            .filter(|s| !s.is_empty())
            .collect();
        branches.sort();
        branches.dedup();
        Ok(branches)
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
    fn commit_messages(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let range = format!("{from}..{to}");
        // Use NUL byte as separator — guaranteed not to appear in commit messages
        let output = self.run(&["log", &range, "--format=%B%x00"])?;
        Ok(output.split('\0')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        self.run_check(&["merge-base", "--is-ancestor", ancestor, descendant])
    }
    fn tag_exists(&self, tag: &str) -> Result<bool> {
        self.run_check(&["show-ref", "--verify", "--quiet", &format!("refs/tags/{tag}")])
    }
    fn local_branch_exists(&self, branch: &str) -> Result<bool> {
        self.run_check(&["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")])
    }
    fn remote_branch_exists(&self, branch: &str) -> Result<bool> {
        self.run_check(&["show-ref", "--verify", "--quiet", &format!("refs/remotes/origin/{branch}")])
    }
    fn remote_tag_exists(&self, tag: &str) -> Result<bool> {
        let output = self.run(&["ls-remote", "--tags", "origin", tag])?;
        Ok(!output.trim().is_empty())
    }
    fn is_pushed(&self, branch: &str) -> Result<bool> {
        let local = self.rev_parse(branch)?;
        let remote_ref = format!("refs/remotes/origin/{branch}");
        let remote = match self.rev_parse(&remote_ref) {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };
        Ok(local == remote)
    }
    fn is_mid_merge(&self) -> Result<bool> {
        let dir = self.git_dir()?;
        Ok(dir.join("MERGE_HEAD").exists()
            || dir.join("CHERRY_PICK_HEAD").exists()
            || dir.join("REVERT_HEAD").exists()
            || dir.join("rebase-merge").exists()
            || dir.join("rebase-apply").exists())
    }
    fn has_unmerged_paths(&self) -> Result<bool> {
        let output = self.run(&["status", "--porcelain"])?;
        for line in output.lines() {
            // Porcelain conflict markers: U? / ?U / AA / DD / AU / UA / DU / UD
            let bytes = line.as_bytes();
            if bytes.len() < 2 { continue; }
            let (x, y) = (bytes[0] as char, bytes[1] as char);
            if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn git_dir(&self) -> Result<PathBuf> {
        let s = self.run(&["rev-parse", "--git-dir"])?;
        Ok(PathBuf::from(s))
    }

    fn remote_url(&self) -> Result<String> {
        self.run(&["remote", "get-url", "origin"])
    }

    fn get_config(&self, key: &str) -> Result<Option<String>> {
        self.run_config(&["config", "--get", key])
    }
    fn set_config(&self, key: &str, value: &str, global: bool) -> Result<()> {
        let mut args = vec!["config"];
        if global { args.push("--global"); }
        args.push(key);
        args.push(value);
        self.run(&args).map(|_| ())
    }
    fn unset_config(&self, key: &str, global: bool) -> Result<()> {
        let mut args = vec!["config"];
        if global { args.push("--global"); }
        args.push("--unset");
        args.push(key);
        let output = Command::new("git").args(&args).output()
            .map_err(|e| format!("Failed to run git: {e}"))?;
        match output.status.code() {
            // 0 = removed; 5 = key was not set (already at default) — both fine.
            Some(0) | Some(5) => Ok(()),
            Some(code) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(format!("git {} failed (exit {code}): {}", args.join(" "), stderr))
            }
            None => Err(format!("git {} terminated by signal", args.join(" "))),
        }
    }
    fn repo_root(&self) -> Result<PathBuf> {
        // `git worktree list` always lists the main working tree first, so this
        // resolves the same root regardless of which worktree we run from.
        let output = self.run(&["worktree", "list", "--porcelain"])?;
        output
            .lines()
            .find_map(|l| l.strip_prefix("worktree "))
            .map(PathBuf::from)
            .ok_or_else(|| "Could not determine the main working tree from 'git worktree list'.".to_string())
    }
    fn add_worktree(&self, path: &Path, branch: &str) -> Result<()> {
        let path_str = path.to_str().ok_or("Worktree path is not valid UTF-8")?;
        self.run(&["worktree", "add", path_str, branch]).map(|_| ())
    }

    fn stash_push_with_message(&self, msg: &str) -> Result<()> {
        self.run(&["stash", "push", "-u", "-m", msg]).map(|_| ())
    }
    fn find_stash_by_message(&self, msg: &str) -> Result<Option<String>> {
        let output = self.run(&["stash", "list", "--format=%gd %s"])?;
        for line in output.lines() {
            if let Some((ref_, rest)) = line.split_once(' ') {
                if rest.contains(msg) {
                    return Ok(Some(ref_.to_string()));
                }
            }
        }
        Ok(None)
    }
    fn stash_pop_ref(&self, stash_ref: &str) -> Result<()> {
        self.run(&["stash", "pop", stash_ref]).map(|_| ())
    }
}
