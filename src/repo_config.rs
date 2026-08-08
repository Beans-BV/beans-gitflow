use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Free,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BumpStrategy {
    Rc,
    Patch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepoConfig {
    pub mode: Mode,
    pub keep_release_branches: bool,
    pub bump_strategy: BumpStrategy,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            mode: Mode::Free,
            keep_release_branches: false,
            bump_strategy: BumpStrategy::Rc,
        }
    }
}

pub fn parse(contents: &str) -> Result<RepoConfig, String> {
    let mut config = RepoConfig::default();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("Malformed line in .bflow/config: {line}"))?;
        let value = value.trim();
        match key.trim() {
            "mode" => match value {
                "free" => config.mode = Mode::Free,
                "protected" => config.mode = Mode::Protected,
                _ => {
                    return Err(format!(
                        "Invalid mode '{value}' in .bflow/config. Use 'mode=free' or 'mode=protected'."
                    ));
                }
            },
            "bump-strategy" => match value {
                "rc" => config.bump_strategy = BumpStrategy::Rc,
                "patch" => config.bump_strategy = BumpStrategy::Patch,
                _ => {
                    return Err(format!(
                        "Invalid bump-strategy '{value}' in .bflow/config. Use 'bump-strategy=rc' or 'bump-strategy=patch'."
                    ));
                }
            },
            "keep-release-branches" => match value {
                "true" => config.keep_release_branches = true,
                "false" => config.keep_release_branches = false,
                _ => {
                    return Err(format!(
                        "Invalid keep-release-branches '{value}' in .bflow/config. Use 'true' or 'false'."
                    ));
                }
            },
            _ => {}
        }
    }

    Ok(config)
}

pub fn load(repo_root: &Path) -> Result<RepoConfig, String> {
    let path = repo_root.join(".bflow").join("config");
    if !path.exists() {
        return Ok(RepoConfig::default());
    }
    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    parse(&contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_dir() -> PathBuf {
        crate::test_support::tmp_dir("bflow-repo-config-test")
    }

    #[test]
    fn empty_contents_yields_default() {
        let config = parse("").unwrap();
        assert_eq!(config, RepoConfig::default());
        assert_eq!(config.mode, Mode::Free);
        assert!(!config.keep_release_branches);
    }

    #[test]
    fn mode_protected_parses() {
        let config = parse("mode=protected\n").unwrap();
        assert_eq!(config.mode, Mode::Protected);
    }

    #[test]
    fn keep_release_branches_true_parses() {
        let config = parse("keep-release-branches=true\n").unwrap();
        assert!(config.keep_release_branches);
    }

    #[test]
    fn comments_blank_lines_and_unknown_keys_are_ignored() {
        let contents = "\
# this is a comment

mode=protected
some-future-key=whatever
keep-release-branches=true
";
        let config = parse(contents).unwrap();
        assert_eq!(config.mode, Mode::Protected);
        assert!(config.keep_release_branches);
    }

    #[test]
    fn values_are_trimmed() {
        let config = parse("mode = protected \n").unwrap();
        assert_eq!(config.mode, Mode::Protected);
    }

    #[test]
    fn invalid_mode_is_a_hard_error_naming_the_remedy() {
        let err = parse("mode=banana\n").unwrap_err();
        assert!(
            err.contains("Use 'mode=free' or 'mode=protected'"),
            "got: {err}"
        );
    }

    #[test]
    fn invalid_keep_release_branches_is_a_hard_error_naming_the_remedy() {
        let err = parse("keep-release-branches=yes\n").unwrap_err();
        assert!(err.contains("Use 'true' or 'false'"), "got: {err}");
    }

    #[test]
    fn a_line_without_equals_is_malformed() {
        let err = parse("mode\n").unwrap_err();
        assert!(err.contains("Malformed line in .bflow/config"), "got: {err}");
    }

    #[test]
    fn bump_strategy_defaults_to_rc() {
        let config = parse("").unwrap();
        assert_eq!(config.bump_strategy, BumpStrategy::Rc);
    }

    #[test]
    fn bump_strategy_patch_parses() {
        let config = parse("bump-strategy=patch\n").unwrap();
        assert_eq!(config.bump_strategy, BumpStrategy::Patch);
    }

    #[test]
    fn bump_strategy_rc_parses_explicitly() {
        let config = parse("bump-strategy=rc\n").unwrap();
        assert_eq!(config.bump_strategy, BumpStrategy::Rc);
    }

    #[test]
    fn invalid_bump_strategy_is_a_hard_error_naming_the_remedy() {
        let err = parse("bump-strategy=banana\n").unwrap_err();
        assert!(
            err.contains("Use 'bump-strategy=rc' or 'bump-strategy=patch'"),
            "got: {err}"
        );
    }

    #[test]
    fn load_round_trips_through_the_filesystem() {
        let dir = tmp_dir();
        let bflow_dir = dir.join(".bflow");
        fs::create_dir_all(&bflow_dir).unwrap();
        fs::write(
            bflow_dir.join("config"),
            "mode=protected\nkeep-release-branches=true\n",
        )
        .unwrap();

        let config = load(&dir).unwrap();

        assert_eq!(
            config,
            RepoConfig {
                mode: Mode::Protected,
                keep_release_branches: true,
                bump_strategy: BumpStrategy::Rc,
            }
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_on_a_root_with_no_bflow_dir_yields_default() {
        let dir = tmp_dir();
        let config = load(&dir).unwrap();
        assert_eq!(config, RepoConfig::default());
        fs::remove_dir_all(&dir).ok();
    }
}
