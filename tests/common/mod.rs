use std::cell::RefCell;
use bflow::git::Git;
use bflow::hosting::HostingPlatform;

pub struct MockGit {
    pub calls: RefCell<Vec<String>>,
    pub current_branch: String,
    pub tags: Vec<String>,
    pub tags_on_branch: Vec<String>,
    pub branches_matching: Vec<String>,
    pub remote_branches: Vec<String>,
    pub merge_base_result: String,
    pub rev_list_count_result: u32,
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
            rev_list_count_result: 0,
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
        Ok(self.merge_base_result.clone())
    }

    fn rev_list_count(&self, from: &str, to: &str) -> Result<u32, String> {
        self.calls.borrow_mut().push(format!("rev_list_count:{from}:{to}"));
        Ok(self.rev_list_count_result)
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
    fn create_or_get_pr(&self, head: &str, base: &str, title: &str) -> Result<String, String> {
        self.calls.borrow_mut().push(format!("create_or_get_pr:{head}:{base}:{title}"));
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
