use crate::responses::PathStatusResponse;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn get_path_status() -> PathStatusResponse {
    let bin_dir = recommended_bin_dir();
    let paths = environment_path_entries();

    build_path_status(bin_dir, paths)
}

pub(crate) fn build_path_status(bin_dir: PathBuf, paths: Vec<PathBuf>) -> PathStatusResponse {
    let bin_dir_display = display_path(&bin_dir);
    let path_values = paths
        .iter()
        .map(|path| display_path(path))
        .collect::<Vec<_>>();
    let suggested_command = Some(suggested_path_command(&bin_dir));

    if paths.is_empty() {
        return PathStatusResponse {
            state: "error".to_string(),
            bin_dir: bin_dir_display,
            message: "无法读取系统 PATH，请检查当前应用启动环境。".to_string(),
            suggested_command,
            paths: path_values,
        };
    }

    let contains_bin_dir = paths.iter().any(|path| path == &bin_dir);
    if contains_bin_dir {
        return PathStatusResponse {
            state: "ok".to_string(),
            bin_dir: bin_dir_display,
            message: "推荐命令目录已加入 PATH，可以直接在终端调用已注册命令。".to_string(),
            suggested_command: None,
            paths: path_values,
        };
    }

    PathStatusResponse {
        state: "missing".to_string(),
        bin_dir: bin_dir_display,
        message: "推荐命令目录未加入 PATH，注册后的命令可能无法直接调用。".to_string(),
        suggested_command,
        paths: path_values,
    }
}

pub(crate) fn environment_path_entries() -> Vec<PathBuf> {
    let process_paths = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    merge_path_entry_sources(vec![
        process_paths,
        login_shell_path_entries(),
        interactive_login_shell_path_entries(),
    ])
}

pub(crate) fn merge_path_entries(
    mut primary: Vec<PathBuf>,
    secondary: Vec<PathBuf>,
) -> Vec<PathBuf> {
    for path in secondary {
        if !primary.contains(&path) {
            primary.push(path);
        }
    }

    primary
}

pub(crate) fn merge_path_entry_sources(sources: Vec<Vec<PathBuf>>) -> Vec<PathBuf> {
    sources.into_iter().fold(Vec::new(), |entries, source| {
        merge_path_entries(entries, source)
    })
}

#[cfg(target_os = "macos")]
fn login_shell_path_entries() -> Vec<PathBuf> {
    shell_path_entries(&["-l", "-c", "printf %s \"$PATH\""])
}

#[cfg(target_os = "macos")]
fn interactive_login_shell_path_entries() -> Vec<PathBuf> {
    shell_path_entries(&["-l", "-i", "-c", "printf %s \"$PATH\""])
}

#[cfg(target_os = "macos")]
fn shell_path_entries(args: &[&str]) -> Vec<PathBuf> {
    let shell = login_shell_path();
    let output = Command::new(shell).args(args).output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let path = String::from_utf8_lossy(&output.stdout);
    env::split_paths(std::ffi::OsStr::new(path.as_ref())).collect()
}

#[cfg(not(target_os = "macos"))]
fn login_shell_path_entries() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(target_os = "macos"))]
fn interactive_login_shell_path_entries() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn login_shell_path() -> PathBuf {
    env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
}

#[cfg(target_os = "windows")]
pub(crate) fn recommended_bin_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("%LOCALAPPDATA%"))
        .join("devdock")
        .join("bin")
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn recommended_bin_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("$HOME"))
        .join(".local")
        .join("bin")
}

#[cfg(target_os = "windows")]
fn suggested_path_command(_bin_dir: &Path) -> String {
    r#"setx PATH "%LOCALAPPDATA%\devdock\bin;%PATH%""#.to_string()
}

#[cfg(not(target_os = "windows"))]
fn suggested_path_command(_bin_dir: &Path) -> String {
    r#"export PATH="$HOME/.local/bin:$PATH""#.to_string()
}

pub(crate) fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_ok_when_bin_dir_is_in_path() {
        let bin_dir = PathBuf::from("/Users/test/.local/bin");
        let paths = vec![PathBuf::from("/usr/bin"), bin_dir.clone()];

        let status = build_path_status(bin_dir, paths);

        assert_eq!(status.state, "ok");
        assert!(status.suggested_command.is_none());
    }

    #[test]
    fn returns_missing_when_bin_dir_is_not_in_path() {
        let bin_dir = PathBuf::from("/Users/test/.local/bin");
        let paths = vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/opt/homebrew/bin"),
        ];

        let status = build_path_status(bin_dir, paths);

        assert_eq!(status.state, "missing");
        assert!(status.suggested_command.is_some());
    }

    #[test]
    fn returns_error_when_path_is_empty() {
        let bin_dir = PathBuf::from("/Users/test/.local/bin");

        let status = build_path_status(bin_dir, Vec::new());

        assert_eq!(status.state, "error");
        assert!(status.suggested_command.is_some());
    }

    #[test]
    fn merges_login_shell_path_when_process_path_is_missing_target_dir() {
        let bin_dir = PathBuf::from("/Users/test/.local/bin");
        let paths = merge_path_entries(
            vec![PathBuf::from("/usr/bin")],
            vec![PathBuf::from("/opt/homebrew/bin"), bin_dir.clone()],
        );

        let status = build_path_status(bin_dir, paths);

        assert_eq!(status.state, "ok");
        assert_eq!(
            status.paths,
            vec![
                "/usr/bin".to_string(),
                "/opt/homebrew/bin".to_string(),
                "/Users/test/.local/bin".to_string(),
            ]
        );
    }

    #[test]
    fn merges_interactive_login_shell_path_when_login_shell_is_missing_target_dir() {
        let bin_dir = PathBuf::from("/Users/test/.local/bin");
        let paths = merge_path_entry_sources(vec![
            vec![PathBuf::from("/usr/bin")],
            vec![PathBuf::from("/opt/homebrew/bin")],
            vec![bin_dir.clone(), PathBuf::from("/Users/test/.cargo/bin")],
        ]);

        let status = build_path_status(bin_dir, paths);

        assert_eq!(status.state, "ok");
        assert_eq!(
            status.paths,
            vec![
                "/usr/bin".to_string(),
                "/opt/homebrew/bin".to_string(),
                "/Users/test/.local/bin".to_string(),
                "/Users/test/.cargo/bin".to_string(),
            ]
        );
    }

    #[test]
    fn reads_current_process_path() {
        let status = get_path_status();

        assert!(!status.bin_dir.is_empty());
        assert!(!status.paths.is_empty());
        assert_ne!(status.state, "error");
    }
}
