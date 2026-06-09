use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DEVDOC_MARKER: &str = "DevDock managed command";
const COMMAND_NAME_META: &str = "devdock-command-name:";
const SCRIPT_PATH_META: &str = "devdock-script-path:";
const CREATED_AT_META: &str = "devdock-created-at:";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PathStatusResponse {
    state: String,
    bin_dir: String,
    message: String,
    suggested_command: Option<String>,
    paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisteredCommandResponse {
    name: String,
    script_path: String,
    entry_path: String,
    entry_type: String,
    created_at: String,
}

#[tauri::command]
fn get_path_status() -> PathStatusResponse {
    let bin_dir = recommended_bin_dir();
    let paths = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    build_path_status(bin_dir, paths)
}

#[tauri::command(rename_all = "camelCase")]
fn register_command(
    script_path: String,
    command_name: String,
) -> Result<RegisteredCommandResponse, String> {
    let created_at = current_timestamp();
    create_registered_command(
        &PathBuf::from(script_path),
        &command_name,
        &recommended_bin_dir(),
        &created_at,
    )
}

#[tauri::command]
fn list_registered_commands() -> Result<Vec<RegisteredCommandResponse>, String> {
    list_registered_commands_in_dir(&recommended_bin_dir())
}

#[tauri::command(rename_all = "camelCase")]
fn delete_registered_command(command_name: String) -> Result<(), String> {
    delete_registered_command_in_dir(&command_name, &recommended_bin_dir())
}

fn build_path_status(bin_dir: PathBuf, paths: Vec<PathBuf>) -> PathStatusResponse {
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

#[cfg(target_os = "windows")]
fn recommended_bin_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("%LOCALAPPDATA%"))
        .join("devdock")
        .join("bin")
}

#[cfg(not(target_os = "windows"))]
fn recommended_bin_dir() -> PathBuf {
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

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn create_registered_command(
    script_path: &Path,
    command_name: &str,
    bin_dir: &Path,
    created_at: &str,
) -> Result<RegisteredCommandResponse, String> {
    validate_command_name(command_name)?;
    validate_script_path(script_path)?;
    make_script_executable(script_path)?;

    fs::create_dir_all(bin_dir).map_err(|error| format!("命令目录创建失败：{error}"))?;

    let entry_path = command_entry_path(bin_dir, command_name);
    if entry_path.exists() {
        return Err("命令名已存在。".to_string());
    }

    let content = wrapper_content(script_path, command_name, created_at);
    fs::write(&entry_path, content).map_err(|error| format!("入口文件写入失败：{error}"))?;
    make_entry_executable(&entry_path)?;

    Ok(RegisteredCommandResponse {
        name: command_name.to_string(),
        script_path: display_path(script_path),
        entry_path: display_path(&entry_path),
        entry_type: entry_type_for_path(&entry_path).to_string(),
        created_at: created_at.to_string(),
    })
}

fn list_registered_commands_in_dir(
    bin_dir: &Path,
) -> Result<Vec<RegisteredCommandResponse>, String> {
    if !bin_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(bin_dir).map_err(|error| format!("命令目录读取失败：{error}"))?;
    let mut commands = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("命令入口读取失败：{error}"))?;
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        if let Some(command) = read_registered_command(&entry_path)? {
            commands.push(command);
        }
    }

    commands.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(commands)
}

fn read_registered_command(entry_path: &Path) -> Result<Option<RegisteredCommandResponse>, String> {
    let bytes = fs::read(entry_path).map_err(|error| format!("命令入口读取失败：{error}"))?;
    let content = String::from_utf8_lossy(&bytes);

    if !content.contains(DEVDOC_MARKER) {
        return Ok(None);
    }

    let Some(name) = metadata_value(content.as_ref(), COMMAND_NAME_META) else {
        return Ok(None);
    };
    let Some(script_path) = metadata_value(content.as_ref(), SCRIPT_PATH_META) else {
        return Ok(None);
    };
    let created_at =
        metadata_value(content.as_ref(), CREATED_AT_META).unwrap_or_else(|| "未知时间".to_string());

    Ok(Some(RegisteredCommandResponse {
        name,
        script_path,
        entry_path: display_path(entry_path),
        entry_type: entry_type_for_path(entry_path).to_string(),
        created_at,
    }))
}

fn delete_registered_command_in_dir(command_name: &str, bin_dir: &Path) -> Result<(), String> {
    validate_command_name(command_name)?;

    let entry_path = command_entry_path(bin_dir, command_name);
    if !entry_path.exists() {
        return Err("命令不存在。".to_string());
    }

    if read_registered_command(&entry_path)?.is_none() {
        return Err("不会删除非 DevDock 生成的入口。".to_string());
    }

    fs::remove_file(&entry_path).map_err(|error| format!("命令入口删除失败：{error}"))
}

fn metadata_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let normalized = line
            .trim_start_matches('#')
            .trim_start_matches("REM")
            .trim();
        normalized
            .strip_prefix(key)
            .map(|value| value.trim().to_string())
    })
}

fn validate_command_name(command_name: &str) -> Result<(), String> {
    if command_name.is_empty() {
        return Err("命令名不能为空。".to_string());
    }

    let is_valid = command_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'));
    if !is_valid {
        return Err("命令名只能包含字母、数字、点、短横线或下划线。".to_string());
    }

    Ok(())
}

fn validate_script_path(script_path: &Path) -> Result<(), String> {
    if !script_path.exists() {
        return Err("脚本文件不存在。".to_string());
    }

    if !script_path.is_file() {
        return Err("脚本路径不是文件。".to_string());
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn command_entry_path(bin_dir: &Path, command_name: &str) -> PathBuf {
    bin_dir.join(format!("{command_name}.cmd"))
}

#[cfg(not(target_os = "windows"))]
fn command_entry_path(bin_dir: &Path, command_name: &str) -> PathBuf {
    bin_dir.join(command_name)
}

#[cfg(target_os = "windows")]
fn wrapper_content(script_path: &Path, command_name: &str, created_at: &str) -> String {
    format!(
        "@echo off\r\nREM {DEVDOC_MARKER}\r\nREM {COMMAND_NAME_META} {command_name}\r\nREM {SCRIPT_PATH_META} {}\r\nREM {CREATED_AT_META} {created_at}\r\n\"{}\" %*\r\n",
        display_path(script_path),
        display_path(script_path)
    )
}

#[cfg(not(target_os = "windows"))]
fn wrapper_content(script_path: &Path, command_name: &str, created_at: &str) -> String {
    format!(
        "#!/bin/sh\n# {DEVDOC_MARKER}\n# {COMMAND_NAME_META} {command_name}\n# {SCRIPT_PATH_META} {}\n# {CREATED_AT_META} {created_at}\nexec {} \"$@\"\n",
        display_path(script_path),
        shell_single_quote(script_path)
    )
}

#[cfg(not(target_os = "windows"))]
fn shell_single_quote(path: &Path) -> String {
    format!("'{}'", display_path(path).replace('\'', r#"'\''"#))
}

#[cfg(target_os = "windows")]
fn make_entry_executable(_entry_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn make_entry_executable(entry_path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(entry_path)
        .map_err(|error| format!("入口文件权限读取失败：{error}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(entry_path, permissions)
        .map_err(|error| format!("入口文件权限设置失败：{error}"))
}

#[cfg(target_os = "windows")]
fn make_script_executable(_script_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn make_script_executable(script_path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(script_path)
        .map_err(|error| format!("脚本文件权限读取失败：{error}"))?
        .permissions();
    let mode = permissions.mode();
    if mode & 0o111 != 0 {
        return Ok(());
    }

    permissions.set_mode(mode | 0o755);
    fs::set_permissions(script_path, permissions)
        .map_err(|error| format!("脚本文件无法设置为可执行：{error}"))
}

fn entry_type_for_path(entry_path: &Path) -> &'static str {
    if entry_path
        .extension()
        .is_some_and(|extension| extension == "cmd")
    {
        "cmd-shim"
    } else {
        "wrapper"
    }
}

fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_path_status,
            register_command,
            list_registered_commands,
            delete_registered_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

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
    fn reads_current_process_path() {
        let status = get_path_status();

        assert!(!status.bin_dir.is_empty());
        assert!(!status.paths.is_empty());
        assert_ne!(status.state, "error");
    }

    #[test]
    fn creates_wrapper_and_lists_registered_command() {
        let temp_dir = unique_test_dir("register-list");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("deploy.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();

        let command =
            create_registered_command(&script_path, "deploy-preview", &bin_dir, "1700000000")
                .expect("register command");

        assert_eq!(command.name, "deploy-preview");
        assert_eq!(command.script_path, display_path(&script_path));
        assert!(PathBuf::from(&command.entry_path).exists());

        let commands = list_registered_commands_in_dir(&bin_dir).expect("list commands");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "deploy-preview");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn deletes_managed_entry_and_keeps_source_script() {
        let temp_dir = unique_test_dir("delete-managed");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("deploy.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();
        let command =
            create_registered_command(&script_path, "deploy-preview", &bin_dir, "1700000000")
                .expect("register command");

        delete_registered_command_in_dir("deploy-preview", &bin_dir).expect("delete command");

        assert!(!PathBuf::from(command.entry_path).exists());
        assert!(script_path.exists());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn refuses_to_delete_unmanaged_entry() {
        let temp_dir = unique_test_dir("delete-unmanaged");
        let bin_dir = temp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let entry_path = bin_dir.join("deploy-preview");
        fs::write(&entry_path, "#!/bin/sh\necho unmanaged\n").unwrap();

        let error = delete_registered_command_in_dir("deploy-preview", &bin_dir).unwrap_err();

        assert_eq!(error, "不会删除非 DevDock 生成的入口。");
        assert!(entry_path.exists());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn refuses_invalid_command_name() {
        let temp_dir = unique_test_dir("invalid-name");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("deploy.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();

        let error = create_registered_command(&script_path, "bad name", &bin_dir, "1700000000")
            .unwrap_err();

        assert_eq!(error, "命令名只能包含字母、数字、点、短横线或下划线。");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn refuses_existing_entry() {
        let temp_dir = unique_test_dir("existing-entry");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("deploy.sh");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();
        fs::write(command_entry_path(&bin_dir, "deploy-preview"), "existing").unwrap();

        let error =
            create_registered_command(&script_path, "deploy-preview", &bin_dir, "1700000000")
                .unwrap_err();

        assert_eq!(error, "命令名已存在。");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn skips_unmanaged_binary_entries_when_listing() {
        let temp_dir = unique_test_dir("binary-entry");
        let bin_dir = temp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("binary-tool"), [0, 159, 146, 150]).unwrap();

        let commands = list_registered_commands_in_dir(&bin_dir).expect("list commands");

        assert!(commands.is_empty());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn makes_source_script_executable() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = unique_test_dir("script-executable");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("deploy.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&script_path, permissions).unwrap();

        create_registered_command(&script_path, "deploy-preview", &bin_dir, "1700000000")
            .expect("register command");

        let mode = fs::metadata(&script_path).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn generated_wrapper_executes_source_script() {
        let temp_dir = unique_test_dir("execute-wrapper");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("echo-arg.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho \"$1\"\n").unwrap();

        let command = create_registered_command(&script_path, "echo-arg", &bin_dir, "1700000000")
            .expect("register command");

        let output = Command::new(&command.entry_path)
            .arg("ok")
            .output()
            .expect("run generated wrapper");

        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");

        let _ = fs::remove_dir_all(temp_dir);
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir().join(format!("devdock-{name}-{now}"))
    }
}
