use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use bflow::editor::Editor;
use bflow::git::Git;
use bflow::hosting::HostingPlatform;
use bflow::prompt::Prompter;

pub struct MockGit {
    pub calls: RefCell<Vec<String>>,
    pub current_branch: String,
    pub tags: Vec<String>,
    pub tags_on_branch: Vec<String>,
    pub branches_matching: Vec<String>,
    pub remote_branches: Vec<String>,
    pub merge_base_result: String,
    /// Per-(a, b) merge bases; falls back to `merge_base_result` when absent.
    pub merge_bases: HashMap<(String, String), String>,
    pub rev_list_count_result: u32,
    /// Per-(from, to) counts; falls back to `rev_list_count_result` when absent.
    pub rev_list_counts: HashMap<(String, String), u32>,
    pub commit_messages: Vec<String>,
    /// Refs (the `to` arg) that should fail with an error. Used to simulate missing branches.
    pub fail_commit_messages_for: Vec<String>,
    /// 1-indexed merge call to fail (simulates a merge conflict). None = never fail.
    pub fail_nth_merge: Option<u32>,
    merge_call_count: RefCell<u32>,

    // Idempotency state — branches/tags considered "already done".
    /// Pairs of (ancestor, descendant) where ancestor is treated as merged into descendant.
    pub ancestors: HashSet<(String, String)>,
    /// Existing local tags (idempotent skipping for tag creation).
    pub existing_tags: HashSet<String>,
    /// Existing remote tags.
    pub existing_remote_tags: HashSet<String>,
    /// Existing local branches.
    pub existing_local_branches: HashSet<String>,
    /// Existing remote branches.
    pub existing_remote_branches: HashSet<String>,
    /// Branches whose local SHA matches origin's (treated as pushed).
    pub pushed_branches: HashSet<String>,
    pub mid_merge: bool,
    pub unmerged_paths: bool,
    pub git_dir: PathBuf,
    /// Stash messages currently in the stash list (most recent first).
    pub stashes: RefCell<Vec<String>>,
    /// git config values returned by `get_config` (key -> value).
    pub config: HashMap<String, String>,
    /// URL returned by `remote_url`.
    pub remote_url: String,
    /// Value returned by `repo_root`.
    pub repo_root: PathBuf,
}

impl MockGit {
    pub fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            current_branch: "develop".to_string(),
            tags: Vec::new(),
            tags_on_branch: Vec::new(),
            branches_matching: Vec::new(),
            remote_branches: Vec::new(),
            merge_base_result: "abc123".to_string(),
            merge_bases: HashMap::new(),
            rev_list_count_result: 0,
            rev_list_counts: HashMap::new(),
            commit_messages: Vec::new(),
            fail_commit_messages_for: Vec::new(),
            fail_nth_merge: None,
            merge_call_count: RefCell::new(0),
            ancestors: HashSet::new(),
            existing_tags: HashSet::new(),
            existing_remote_tags: HashSet::new(),
            existing_local_branches: HashSet::new(),
            existing_remote_branches: HashSet::new(),
            pushed_branches: HashSet::new(),
            mid_merge: false,
            unmerged_paths: false,
            git_dir: PathBuf::from(".git"),
            stashes: RefCell::new(Vec::new()),
            config: HashMap::new(),
            remote_url: "https://github.com/acme/repo.git".to_string(),
            repo_root: PathBuf::from("/repos/beans-gitflow"),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl Git for MockGit {
    fn current_branch(&self) -> Result<String, String> {
        self.calls.borrow_mut().push("current_branch".to_string());
        Ok(self.current_branch.clone())
    }

    fn fetch(&self) -> Result<(), String> {
        self.calls.borrow_mut().push("fetch".to_string());
        Ok(())
    }

    fn checkout(&self, branch: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("checkout:{branch}"));
        Ok(())
    }

    fn create_branch(&self, branch: &str, from: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("create_branch:{branch}:{from}"));
        Ok(())
    }

    fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("create_branch_no_checkout:{branch}:{from}"));
        Ok(())
    }

    fn push(&self, branch: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("push:{branch}"));
        Ok(())
    }

    fn push_tag(&self, tag: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("push_tag:{tag}"));
        Ok(())
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("create_tag:{tag}:{message}"));
        Ok(())
    }

    fn merge(&self, branch: &str, message: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("merge:{branch}:{message}"));
        let mut count = self.merge_call_count.borrow_mut();
        *count += 1;
        if Some(*count) == self.fail_nth_merge {
            return Err(format!("CONFLICT: merge of {branch} failed"));
        }
        Ok(())
    }

    fn ff_merge(&self, branch: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("ff_merge:{branch}"));
        Ok(())
    }

    fn list_tags(&self) -> Result<Vec<String>, String> {
        self.calls.borrow_mut().push("list_tags".to_string());
        Ok(self.tags.clone())
    }

    fn list_branches_matching(&self, pattern: &str) -> Result<Vec<String>, String> {
        self.calls.borrow_mut().push(format!("list_branches_matching:{pattern}"));
        Ok(self.branches_matching.clone())
    }

    fn is_working_tree_clean(&self) -> Result<bool, String> {
        self.calls.borrow_mut().push("is_working_tree_clean".to_string());
        Ok(true)
    }

    fn delete_branch_local(&self, branch: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("delete_branch_local:{branch}"));
        Ok(())
    }

    fn delete_branch_remote(&self, branch: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("delete_branch_remote:{branch}"));
        Ok(())
    }

    fn tags_on_branch(&self, branch: &str) -> Result<Vec<String>, String> {
        self.calls.borrow_mut().push(format!("tags_on_branch:{branch}"));
        Ok(self.tags_on_branch.clone())
    }

    fn list_remote_branches(&self) -> Result<Vec<String>, String> {
        self.calls.borrow_mut().push("list_remote_branches".to_string());
        Ok(self.remote_branches.clone())
    }

    fn merge_base(&self, a: &str, b: &str) -> Result<String, String> {
        self.calls.borrow_mut().push(format!("merge_base:{a}:{b}"));
        Ok(self.merge_bases
            .get(&(a.to_string(), b.to_string()))
            .cloned()
            .unwrap_or_else(|| self.merge_base_result.clone()))
    }

    fn rev_list_count(&self, from: &str, to: &str) -> Result<u32, String> {
        self.calls.borrow_mut().push(format!("rev_list_count:{from}:{to}"));
        Ok(*self.rev_list_counts
            .get(&(from.to_string(), to.to_string()))
            .unwrap_or(&self.rev_list_count_result))
    }

    fn commit_messages(&self, from: &str, to: &str) -> Result<Vec<String>, String> {
        self.calls.borrow_mut().push(format!("commit_messages:{from}:{to}"));
        if self.fail_commit_messages_for.iter().any(|r| r == to) {
            return Err(format!("ref not found: {to}"));
        }
        Ok(self.commit_messages.clone())
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(format!("is_ancestor:{ancestor}:{descendant}"));
        Ok(self.ancestors.contains(&(ancestor.to_string(), descendant.to_string())))
    }

    fn tag_exists(&self, tag: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(format!("tag_exists:{tag}"));
        Ok(self.existing_tags.contains(tag))
    }

    fn local_branch_exists(&self, branch: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(format!("local_branch_exists:{branch}"));
        Ok(self.existing_local_branches.contains(branch))
    }

    fn remote_branch_exists(&self, branch: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(format!("remote_branch_exists:{branch}"));
        Ok(self.existing_remote_branches.contains(branch))
    }

    fn remote_tag_exists(&self, tag: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(format!("remote_tag_exists:{tag}"));
        Ok(self.existing_remote_tags.contains(tag))
    }

    fn is_pushed(&self, branch: &str) -> Result<bool, String> {
        self.calls.borrow_mut().push(format!("is_pushed:{branch}"));
        Ok(self.pushed_branches.contains(branch))
    }

    fn is_mid_merge(&self) -> Result<bool, String> {
        self.calls.borrow_mut().push("is_mid_merge".to_string());
        Ok(self.mid_merge)
    }

    fn has_unmerged_paths(&self) -> Result<bool, String> {
        self.calls.borrow_mut().push("has_unmerged_paths".to_string());
        Ok(self.unmerged_paths)
    }

    fn git_dir(&self) -> Result<PathBuf, String> {
        self.calls.borrow_mut().push("git_dir".to_string());
        Ok(self.git_dir.clone())
    }

    fn remote_url(&self) -> Result<String, String> {
        self.calls.borrow_mut().push("remote_url".to_string());
        Ok(self.remote_url.clone())
    }

    fn get_config(&self, key: &str) -> Result<Option<String>, String> {
        self.calls.borrow_mut().push(format!("get_config:{key}"));
        Ok(self.config.get(key).cloned())
    }

    fn set_config(&self, key: &str, value: &str, global: bool) -> Result<(), String> {
        let scope = if global { "global" } else { "local" };
        self.calls.borrow_mut().push(format!("set_config:{scope}:{key}:{value}"));
        Ok(())
    }

    fn unset_config(&self, key: &str, global: bool) -> Result<(), String> {
        let scope = if global { "global" } else { "local" };
        self.calls.borrow_mut().push(format!("unset_config:{scope}:{key}"));
        Ok(())
    }

    fn repo_root(&self) -> Result<PathBuf, String> {
        self.calls.borrow_mut().push("repo_root".to_string());
        Ok(self.repo_root.clone())
    }

    fn add_worktree(&self, path: &Path, branch: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("add_worktree:{}:{branch}", path.display()));
        Ok(())
    }

    fn stash_push_with_message(&self, msg: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("stash_push_with_message:{msg}"));
        self.stashes.borrow_mut().insert(0, msg.to_string());
        Ok(())
    }

    fn find_stash_by_message(&self, msg: &str) -> Result<Option<String>, String> {
        self.calls.borrow_mut().push(format!("find_stash_by_message:{msg}"));
        let stashes = self.stashes.borrow();
        for (i, m) in stashes.iter().enumerate() {
            if m == msg {
                return Ok(Some(format!("stash@{{{i}}}")));
            }
        }
        Ok(None)
    }

    fn stash_pop_ref(&self, stash_ref: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("stash_pop_ref:{stash_ref}"));
        Ok(())
    }
}

pub struct MockHosting {
    pub calls: RefCell<Vec<String>>,
    pub pr_url: String,
}

impl MockHosting {
    pub fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            pr_url: "https://github.com/org/repo/pull/1".to_string(),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl HostingPlatform for MockHosting {
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str, template: Option<&str>) -> Result<String, String> {
        let suffix = template.map(|t| format!(":template={t}")).unwrap_or_default();
        self.calls.borrow_mut().push(format!("create_or_get_pr:{head}:{base}:{title}{suffix}"));
        Ok(self.pr_url.clone())
    }

    fn open_url(&self, url: &str) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("open_url:{url}"));
        Ok(())
    }

    fn check_auth(&self) -> Result<(), String> {
        self.calls.borrow_mut().push("check_auth".to_string());
        Ok(())
    }
}

pub struct MockEditor {
    pub calls: RefCell<Vec<String>>,
    /// When true, `open` returns an error (simulates editor not on PATH).
    pub fail: bool,
}

impl MockEditor {
    pub fn new() -> Self {
        Self { calls: RefCell::new(Vec::new()), fail: false }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl Editor for MockEditor {
    fn open(&self, path: &Path) -> Result<(), String> {
        self.calls.borrow_mut().push(format!("open:{}", path.display()));
        if self.fail {
            Err("editor failed".to_string())
        } else {
            Ok(())
        }
    }
}

/// Scripted `Prompter`: records every select as `select:{prompt}:[items]` and
/// answers from a queue. An unscripted select is an error, so a test proves a
/// flow never prompted simply by not scripting anything.
pub struct MockPrompter {
    pub calls: RefCell<Vec<String>>,
    pub selections: RefCell<VecDeque<usize>>,
}

impl MockPrompter {
    pub fn new() -> Self {
        Self { calls: RefCell::new(Vec::new()), selections: RefCell::new(VecDeque::new()) }
    }

    pub fn scripted(selections: &[usize]) -> Self {
        let p = Self::new();
        p.selections.borrow_mut().extend(selections.iter().copied());
        p
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl Prompter for MockPrompter {
    fn select(&self, prompt: &str, items: &[&str]) -> Result<usize, String> {
        self.calls.borrow_mut().push(format!("select:{prompt}:[{}]", items.join(", ")));
        self.selections.borrow_mut().pop_front()
            .ok_or_else(|| format!("MockPrompter: unscripted select('{prompt}')"))
    }
}
