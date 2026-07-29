mod common;

use std::path::{Path, PathBuf};
use common::MockGit;
use bflow::worktree::{
    worktree_path, WorktreeConfig, EDITOR_PRESETS,
    set_enabled, set_editor, set_path, use_default_path, show_status,
};

// --- worktree_path (pure) ---

#[test]
fn worktree_path_default_base_is_repo_parent_with_prefixed_folder() {
    let root = Path::new("/Users/jop/Projects/beans/beans-gitflow");
    let p = worktree_path(root, "beans-gitflow", None, "feature/login");
    assert_eq!(p, PathBuf::from("/Users/jop/Projects/beans/beans-gitflow-feature-login"));
}

#[test]
fn worktree_path_slashes_become_dashes() {
    let root = Path::new("/repos/beans-gitflow");
    let p = worktree_path(root, "beans-gitflow", None, "release-fix/1.2.0/null-crash");
    assert_eq!(p, PathBuf::from("/repos/beans-gitflow-release-fix-1.2.0-null-crash"));
}

#[test]
fn worktree_path_custom_base_is_used_verbatim() {
    let root = Path::new("/repos/beans-gitflow");
    let p = worktree_path(root, "beans-gitflow", Some("/Users/jop/worktrees"), "feature/login");
    assert_eq!(p, PathBuf::from("/Users/jop/worktrees/beans-gitflow-feature-login"));
}

#[test]
fn worktree_path_expands_leading_tilde_in_custom_base() {
    // `~` in git config never passes through a shell, so bflow expands it itself.
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap();
    let root = Path::new("/repos/beans-gitflow");
    let p = worktree_path(root, "beans-gitflow", Some("~/worktrees"), "feature/login");
    assert_eq!(p, PathBuf::from(home).join("worktrees/beans-gitflow-feature-login"));
}

// --- WorktreeConfig::load ---

#[test]
fn config_defaults_when_unset() {
    let git = MockGit::new(); // empty config map
    let cfg = WorktreeConfig::load(&git).unwrap();
    assert!(!cfg.enabled);
    assert_eq!(cfg.editor, "code");
    assert_eq!(cfg.base_path, None);
}

#[test]
fn config_reads_all_values() {
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.enabled".to_string(), "true".to_string());
    git.config.insert("bflow.worktree.editor".to_string(), "cursor".to_string());
    git.config.insert("bflow.worktree.path".to_string(), "/wt".to_string());

    let cfg = WorktreeConfig::load(&git).unwrap();
    assert!(cfg.enabled);
    assert_eq!(cfg.editor, "cursor");
    assert_eq!(cfg.base_path.as_deref(), Some("/wt"));
}

#[test]
fn config_trims_whitespace_from_editor_and_path() {
    // Stray whitespace in git config would otherwise break Command::new("code ")
    // or produce oddly named directories.
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.editor".to_string(), "code ".to_string());
    git.config.insert("bflow.worktree.path".to_string(), " /wt ".to_string());

    let cfg = WorktreeConfig::load(&git).unwrap();
    assert_eq!(cfg.editor, "code");
    assert_eq!(cfg.base_path.as_deref(), Some("/wt"));
}

#[test]
fn config_whitespace_only_editor_falls_back_to_code() {
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.editor".to_string(), "   ".to_string());
    let cfg = WorktreeConfig::load(&git).unwrap();
    assert_eq!(cfg.editor, "code");
}

#[test]
fn config_enabled_is_false_when_value_is_not_true() {
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.enabled".to_string(), "false".to_string());
    let cfg = WorktreeConfig::load(&git).unwrap();
    assert!(!cfg.enabled);
}

#[test]
fn config_empty_editor_falls_back_to_code() {
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.editor".to_string(), "".to_string());
    let cfg = WorktreeConfig::load(&git).unwrap();
    assert_eq!(cfg.editor, "code");
}

#[test]
fn config_preserves_none_editor() {
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.editor".to_string(), "none".to_string());
    let cfg = WorktreeConfig::load(&git).unwrap();
    assert_eq!(cfg.editor, "none");
}

// --- `bflow worktree` config setters ---

#[test]
fn set_enabled_writes_global_by_default() {
    let git = MockGit::new();
    set_enabled(&git, true, false).unwrap();
    assert_eq!(git.calls(), vec!["set_config:global:bflow.worktree.enabled:true"]);
}

#[test]
fn set_enabled_false_writes_local_when_requested() {
    let git = MockGit::new();
    set_enabled(&git, false, true).unwrap();
    assert_eq!(git.calls(), vec!["set_config:local:bflow.worktree.enabled:false"]);
}

#[test]
fn set_editor_writes_value_verbatim() {
    let git = MockGit::new();
    set_editor(&git, "cursor", false).unwrap();
    assert_eq!(git.calls(), vec!["set_config:global:bflow.worktree.editor:cursor"]);
}

#[test]
fn set_path_writes_value() {
    let git = MockGit::new();
    set_path(&git, "/Users/jop/worktrees", false).unwrap();
    assert_eq!(git.calls(), vec!["set_config:global:bflow.worktree.path:/Users/jop/worktrees"]);
}

#[test]
fn use_default_path_unsets_the_key() {
    let git = MockGit::new();
    use_default_path(&git, false).unwrap();
    assert_eq!(git.calls(), vec!["unset_config:global:bflow.worktree.path"]);
}

#[test]
fn show_status_reads_the_three_keys() {
    let mut git = MockGit::new();
    git.config.insert("bflow.worktree.enabled".to_string(), "true".to_string());
    show_status(&git).unwrap();
    let calls = git.calls();
    assert!(calls.iter().any(|c| c == "get_config:bflow.worktree.enabled"));
    assert!(calls.iter().any(|c| c == "get_config:bflow.worktree.editor"));
    assert!(calls.iter().any(|c| c == "get_config:bflow.worktree.path"));
}

#[test]
fn editor_presets_map_friendly_names_to_commands() {
    let map = |label: &str| EDITOR_PRESETS.iter().find(|(l, _)| *l == label).map(|(_, c)| *c);
    assert_eq!(map("VS Code"), Some("code"));
    assert_eq!(map("Cursor"), Some("cursor"));
    assert_eq!(map("PyCharm"), Some("pycharm"));
}
