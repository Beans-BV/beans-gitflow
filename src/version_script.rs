//! Discovery and execution of the repo's optional version-bump script.
//!
//! bflow looks for `.bflow/set-version.sh` (or `.bflow/set-version.cmd` on
//! Windows) and, when present, runs it with the new version as the only
//! argument. A repo with neither file behaves exactly as if this module did
//! not exist — the caller sees `Ok(None)` and skips the step entirely.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const SCRIPT_UNIX: &str = ".bflow/set-version.sh";
pub const SCRIPT_WINDOWS: &str = ".bflow/set-version.cmd";

/// Port for running the version script. A trait so flows can be tested
/// without spawning a real process.
pub trait VersionScript {
    fn run(&self, version: &str) -> Result<(), String>;
    /// Run the script inside another working tree: `dir`'s own copy of the
    /// script, with `dir` as the working directory. Repo content applies to
    /// the tree it lives in — a checkout-less branch may carry a different
    /// script than the tree bflow was started from.
    fn run_in(&self, dir: &Path, version: &str) -> Result<(), String>;
    fn display_name(&self) -> String;
}

/// Resolve the version script for the current platform, anchored to
/// `repo_root` (not the process CWD, so it works from subdirectories).
pub fn resolve(repo_root: &Path) -> Result<Option<PathBuf>, String> {
    resolve_for(repo_root, cfg!(windows))
}

/// Platform pick factored out of `resolve` so both platforms are unit-testable
/// on any OS.
fn resolve_for(repo_root: &Path, windows: bool) -> Result<Option<PathBuf>, String> {
    let (own, other) = if windows { (SCRIPT_WINDOWS, SCRIPT_UNIX) } else { (SCRIPT_UNIX, SCRIPT_WINDOWS) };
    let own_path = repo_root.join(own);
    let other_path = repo_root.join(other);
    if own_path.exists() {
        Ok(Some(own_path))
    } else if other_path.exists() {
        Err(format!(
            "Found {} but this platform needs {}. Add {} (or remove {}).",
            other_path.display(),
            own_path.display(),
            own_path.display(),
            other_path.display(),
        ))
    } else {
        Ok(None)
    }
}

/// Turn a finished process outcome into the user-facing result. Pure, so it's
/// unit-tested directly rather than through a real spawn.
fn interpret(path: &Path, code: Option<i32>, stderr: &str) -> Result<(), String> {
    match code {
        Some(0) => Ok(()),
        Some(code) => Err(format!(
            "Version script {} failed (exit {code}): {stderr}\nFix the script, then re-run the command.",
            path.display(),
        )),
        None => Err(format!("Version script {} was terminated by a signal.", path.display())),
    }
}

/// Production `VersionScript`: runs the resolved script as a child process.
pub struct ScriptCli {
    path: PathBuf,
    repo_root: PathBuf,
}

impl ScriptCli {
    pub fn new(path: PathBuf, repo_root: PathBuf) -> Self {
        Self { path, repo_root }
    }
}

impl ScriptCli {
    /// Zero-policy shell, like CommandEditor::open — the spawn itself has no
    /// decision to test; interpret() carries the outcome policy.
    fn spawn_at(&self, script: &Path, cwd: &Path, version: &str) -> Result<(), String> {
        let output = Command::new(script)
            .arg(version)
            .current_dir(cwd)
            .output()
            .map_err(|e| format!(
                "Version script {} could not be run: {e}\nMake it executable: chmod +x {} && git update-index --chmod=+x {}, then re-run the command.",
                script.display(), script.display(), script.display(),
            ))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        interpret(script, output.status.code(), stderr.trim())
    }
}

impl VersionScript for ScriptCli {
    fn run(&self, version: &str) -> Result<(), String> {
        self.spawn_at(&self.path, &self.repo_root, version)
    }

    fn run_in(&self, dir: &Path, version: &str) -> Result<(), String> {
        let rel = self.path.strip_prefix(&self.repo_root).map_err(|_| format!(
            "Version script {} is not under the repo root {}, so it cannot be located in another worktree.",
            self.path.display(), self.repo_root.display(),
        ))?;
        self.spawn_at(&dir.join(rel), dir, version)
    }

    fn display_name(&self) -> String {
        self.path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_dir() -> PathBuf {
        crate::test_support::tmp_dir("bflow-version-script-test")
    }

    #[test]
    fn resolve_for_none_present_yields_none() {
        let root = tmp_dir();
        assert_eq!(resolve_for(&root, false).unwrap(), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_for_matching_platform_present_yields_its_path() {
        let root = tmp_dir();
        std::fs::create_dir_all(root.join(".bflow")).unwrap();
        std::fs::write(root.join(SCRIPT_UNIX), "#!/bin/sh\n").unwrap();
        assert_eq!(resolve_for(&root, false).unwrap(), Some(root.join(SCRIPT_UNIX)));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_for_only_other_platform_present_errors_naming_both() {
        let root = tmp_dir();
        std::fs::create_dir_all(root.join(".bflow")).unwrap();
        std::fs::write(root.join(SCRIPT_WINDOWS), "@echo off\n").unwrap();
        let err = resolve_for(&root, false).unwrap_err();
        assert!(err.contains(SCRIPT_UNIX), "got: {err}");
        assert!(err.contains(SCRIPT_WINDOWS), "got: {err}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_for_both_present_yields_own_platform() {
        let root = tmp_dir();
        std::fs::create_dir_all(root.join(".bflow")).unwrap();
        std::fs::write(root.join(SCRIPT_UNIX), "#!/bin/sh\n").unwrap();
        std::fs::write(root.join(SCRIPT_WINDOWS), "@echo off\n").unwrap();
        assert_eq!(resolve_for(&root, true).unwrap(), Some(root.join(SCRIPT_WINDOWS)));
        assert_eq!(resolve_for(&root, false).unwrap(), Some(root.join(SCRIPT_UNIX)));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn interpret_exit_zero_is_ok() {
        let path = Path::new(".bflow/set-version.sh");
        assert_eq!(interpret(path, Some(0), ""), Ok(()));
    }

    #[test]
    fn interpret_nonzero_exit_names_path_code_stderr_and_remedy() {
        let path = Path::new(".bflow/set-version.sh");
        let err = interpret(path, Some(3), "boom\n").unwrap_err();
        assert!(err.contains(".bflow/set-version.sh"), "got: {err}");
        assert!(err.contains("exit 3"), "got: {err}");
        assert!(err.contains("boom"), "got: {err}");
        assert!(err.contains("Fix the script, then re-run the command."), "got: {err}");
    }

    #[test]
    fn interpret_signal_termination_names_path() {
        let path = Path::new(".bflow/set-version.sh");
        let err = interpret(path, None, "").unwrap_err();
        assert_eq!(err, "Version script .bflow/set-version.sh was terminated by a signal.");
    }

    #[test]
    fn script_cli_display_name_is_the_file_name() {
        let script = ScriptCli::new(PathBuf::from("/repo/.bflow/set-version.sh"), PathBuf::from("/repo"));
        assert_eq!(script.display_name(), "set-version.sh");
    }

    #[cfg(unix)]
    #[test]
    fn run_in_executes_the_target_trees_own_copy_with_cwd_there() {
        use std::os::unix::fs::PermissionsExt;
        let root = tmp_dir();
        let worktree = tmp_dir();
        for (dir, marker) in [(&root, "root"), (&worktree, "worktree")] {
            std::fs::create_dir_all(dir.join(".bflow")).unwrap();
            let script = dir.join(SCRIPT_UNIX);
            std::fs::write(&script, format!("#!/bin/sh\necho {marker} $1 > marker.txt\n")).unwrap();
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let script = ScriptCli::new(root.join(SCRIPT_UNIX), root.clone());

        script.run_in(&worktree, "1.0.1").unwrap();

        let marker = std::fs::read_to_string(worktree.join("marker.txt")).unwrap();
        assert_eq!(marker.trim(), "worktree 1.0.1");
        assert!(!root.join("marker.txt").exists(), "the root tree must stay untouched");
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&worktree).ok();
    }

    #[test]
    fn run_in_errors_when_the_script_is_not_under_the_repo_root() {
        let script = ScriptCli::new(PathBuf::from("/elsewhere/set-version.sh"), PathBuf::from("/repo"));
        let err = script.run_in(Path::new("/wt"), "1.0.1").unwrap_err();
        assert!(err.contains("/elsewhere/set-version.sh"), "got: {err}");
        assert!(err.contains("/repo"), "got: {err}");
    }

    #[test]
    fn run_names_the_chmod_remedy_when_the_script_cannot_be_spawned() {
        // A path that cannot be spawned (here: does not exist) drives the same
        // Command::output() Err branch a non-executable file would — the OS
        // refuses at exec time, so no process ever actually runs.
        let path = PathBuf::from("/definitely/does/not/exist/set-version.sh");
        let script = ScriptCli::new(path.clone(), std::env::temp_dir());

        let err = script.run("1.0.0").unwrap_err();

        let path_str = path.display().to_string();
        assert!(err.starts_with(&format!("Version script {path_str} could not be run: ")), "got: {err}");
        assert!(
            err.contains(&format!("Make it executable: chmod +x {path_str} && git update-index --chmod=+x {path_str}, then re-run the command.")),
            "got: {err}"
        );
    }
}
