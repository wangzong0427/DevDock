use crate::{
    path_status::{display_path, environment_path_entries},
    responses::{CommandRunResultResponse, RegisteredCommandResponse},
};
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEVDOC_MARKER: &str = "DevDock managed command";
const COMMAND_NAME_META: &str = "devdock-command-name:";
const SCRIPT_PATH_META: &str = "devdock-script-path:";
const CREATED_AT_META: &str = "devdock-created-at:";

pub(crate) fn create_registered_command(
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

pub(crate) fn list_registered_commands_in_dir(
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

pub(crate) fn delete_registered_command_in_dir(
    command_name: &str,
    bin_dir: &Path,
) -> Result<(), String> {
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

pub(crate) fn spawn_registered_command_run(
    command_name: String,
    bin_dir: PathBuf,
    mut on_output: impl FnMut(&str, &str) + Send + 'static,
) -> tauri::async_runtime::JoinHandle<Result<CommandRunResultResponse, String>> {
    tauri::async_runtime::spawn_blocking(move || {
        run_registered_command_streaming_in_dir(&command_name, &bin_dir, |stream, text| {
            on_output(stream, text);
        })
    })
}

#[cfg(test)]
fn run_registered_command_in_dir(
    command_name: &str,
    bin_dir: &Path,
) -> Result<CommandRunResultResponse, String> {
    run_registered_command_streaming_in_dir(command_name, bin_dir, |_stream, _text| {})
}

fn run_registered_command_streaming_in_dir(
    command_name: &str,
    bin_dir: &Path,
    mut on_output: impl FnMut(&str, &str),
) -> Result<CommandRunResultResponse, String> {
    run_registered_command_streaming_with_path_entries_in_dir(
        command_name,
        bin_dir,
        environment_path_entries(),
        |stream, text| on_output(stream, text),
    )
}

#[cfg(test)]
fn run_registered_command_with_path_entries_in_dir(
    command_name: &str,
    bin_dir: &Path,
    path_entries: Vec<PathBuf>,
) -> Result<CommandRunResultResponse, String> {
    run_registered_command_streaming_with_path_entries_in_dir(
        command_name,
        bin_dir,
        path_entries,
        |_stream, _text| {},
    )
}

fn run_registered_command_streaming_with_path_entries_in_dir(
    command_name: &str,
    bin_dir: &Path,
    path_entries: Vec<PathBuf>,
    mut on_output: impl FnMut(&str, &str),
) -> Result<CommandRunResultResponse, String> {
    validate_command_name(command_name)?;

    let entry_path = command_entry_path(bin_dir, command_name);
    if !entry_path.exists() {
        return Err("命令不存在。".to_string());
    }

    if read_registered_command(&entry_path)?.is_none() {
        return Err("不会执行非 DevDock 生成的入口。".to_string());
    }

    let mut command = Command::new(&entry_path);
    if !path_entries.is_empty() {
        let path_value =
            env::join_paths(path_entries).map_err(|error| format!("PATH 生成失败：{error}"))?;
        command.env("PATH", path_value);
    }

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("命令执行失败：{error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "命令标准输出读取失败。".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "命令错误输出读取失败。".to_string())?;
    let (sender, receiver) = mpsc::channel::<(String, String)>();
    let stdout_handle = spawn_output_reader(stdout, "stdout", sender.clone());
    let stderr_handle = spawn_output_reader(stderr, "stderr", sender);

    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let mut exit_status = None;

    loop {
        match receiver.recv_timeout(Duration::from_millis(20)) {
            Ok((stream, text)) => {
                if stream == "stdout" {
                    stdout_text.push_str(&text);
                } else {
                    stderr_text.push_str(&text);
                }
                on_output(&stream, &text);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if exit_status.is_none() {
                    exit_status = Some(
                        child
                            .wait()
                            .map_err(|error| format!("命令状态读取失败：{error}"))?,
                    );
                }
                break;
            }
        }

        if exit_status.is_none() {
            exit_status = child
                .try_wait()
                .map_err(|error| format!("命令状态读取失败：{error}"))?;
        }
    }

    stdout_handle
        .join()
        .map_err(|_| "标准输出读取线程异常退出。".to_string())??;
    stderr_handle
        .join()
        .map_err(|_| "错误输出读取线程异常退出。".to_string())??;

    let status = match exit_status {
        Some(status) => status,
        None => child
            .wait()
            .map_err(|error| format!("命令状态读取失败：{error}"))?,
    };

    Ok(CommandRunResultResponse {
        command_name: command_name.to_string(),
        exit_code: status.code(),
        stdout: stdout_text,
        stderr: stderr_text,
    })
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

fn spawn_output_reader<R>(
    reader: R,
    stream: &'static str,
    sender: Sender<(String, String)>,
) -> thread::JoinHandle<Result<(), String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || read_output_chunks(reader, stream, sender))
}

fn read_output_chunks<R>(
    mut reader: R,
    stream: &'static str,
    sender: Sender<(String, String)>,
) -> Result<(), String>
where
    R: Read,
{
    let mut buffer = [0; 4096];
    loop {
        let read_count = reader
            .read(&mut buffer)
            .map_err(|error| format!("命令输出读取失败：{error}"))?;
        if read_count == 0 {
            break;
        }

        let text = String::from_utf8_lossy(&buffer[..read_count]).into_owned();
        sender
            .send((stream.to_string(), text))
            .map_err(|error| format!("命令输出发送失败：{error}"))?;
    }

    Ok(())
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

pub(crate) fn current_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        process::Command,
        time::{Duration, Instant},
    };

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

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn runs_registered_command_and_captures_stdout() {
        let temp_dir = unique_test_dir("run-stdout");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("hello.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho hello\n").unwrap();
        create_registered_command(&script_path, "hello", &bin_dir, "1700000000")
            .expect("register command");

        let result = run_registered_command_in_dir("hello", &bin_dir).expect("run command");

        assert_eq!(result.command_name, "hello");
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "hello\n");
        assert_eq!(result.stderr, "");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn runs_registered_command_with_supplied_path_entries() {
        let temp_dir = unique_test_dir("run-supplied-path");
        let bin_dir = temp_dir.join("bin");
        let tool_dir = temp_dir.join("tools");
        let script_path = temp_dir.join("uses-helper.sh");
        let helper_path = tool_dir.join("helper-tool");
        fs::create_dir_all(&tool_dir).unwrap();
        fs::write(&helper_path, "#!/bin/sh\necho helper-ok\n").unwrap();
        make_entry_executable(&helper_path).unwrap();
        fs::write(&script_path, "#!/bin/sh\nhelper-tool\n").unwrap();
        create_registered_command(&script_path, "uses-helper", &bin_dir, "1700000000")
            .expect("register command");

        let result = run_registered_command_with_path_entries_in_dir(
            "uses-helper",
            &bin_dir,
            vec![PathBuf::from("/usr/bin"), tool_dir],
        )
        .expect("run command");

        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "helper-ok\n");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn starts_registered_command_run_without_waiting_for_process_exit() {
        let temp_dir = unique_test_dir("run-background");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("slow.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\nsleep 0.2\necho done\n").unwrap();
        create_registered_command(&script_path, "slow", &bin_dir, "1700000000")
            .expect("register command");

        let started_at = Instant::now();
        let run_handle =
            spawn_registered_command_run("slow".to_string(), bin_dir.clone(), |_stream, _text| {});

        assert!(
            started_at.elapsed() < Duration::from_millis(100),
            "spawning a command run should not wait for the process to exit"
        );

        let result = tauri::async_runtime::block_on(run_handle)
            .expect("join command task")
            .expect("run command");
        assert_eq!(result.stdout, "done\n");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn streams_stdout_before_registered_command_exits() {
        let temp_dir = unique_test_dir("run-streaming");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("stream.sh");
        let marker_path = temp_dir.join("stdout-seen");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(
            &script_path,
            format!(
                "#!/bin/sh\n\
                 echo first\n\
                 i=0\n\
                 while [ \"$i\" -lt 50 ]; do\n\
                 \tif [ -f {} ]; then\n\
                 \t\techo second\n\
                 \t\texit 0\n\
                 \tfi\n\
                 \ti=$((i + 1))\n\
                 \tsleep 0.1\n\
                 done\n\
                 echo timeout >&2\n\
                 exit 9\n",
                shell_single_quote(&marker_path)
            ),
        )
        .unwrap();
        create_registered_command(&script_path, "stream", &bin_dir, "1700000000")
            .expect("register command");

        let mut first_chunk_at = None;
        let result = run_registered_command_streaming_in_dir("stream", &bin_dir, |stream, text| {
            if stream == "stdout" && text == "first\n" && first_chunk_at.is_none() {
                first_chunk_at = Some(Instant::now());
                fs::write(&marker_path, "seen").unwrap();
            }
        })
        .expect("run command");

        assert_eq!(result.stdout, "first\nsecond\n");
        assert_eq!(result.exit_code, Some(0));
        assert!(first_chunk_at.is_some());
        assert!(marker_path.exists());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn returns_non_zero_exit_output() {
        let temp_dir = unique_test_dir("run-non-zero");
        let bin_dir = temp_dir.join("bin");
        let script_path = temp_dir.join("fail.sh");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(&script_path, "#!/bin/sh\necho failed >&2\nexit 7\n").unwrap();
        create_registered_command(&script_path, "fail-command", &bin_dir, "1700000000")
            .expect("register command");

        let result = run_registered_command_in_dir("fail-command", &bin_dir).expect("run command");

        assert_eq!(result.exit_code, Some(7));
        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "failed\n");

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn refuses_to_run_unmanaged_entry() {
        let temp_dir = unique_test_dir("run-unmanaged");
        let bin_dir = temp_dir.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let entry_path = command_entry_path(&bin_dir, "deploy-preview");
        fs::write(&entry_path, "#!/bin/sh\necho unmanaged\n").unwrap();

        let error = run_registered_command_in_dir("deploy-preview", &bin_dir).unwrap_err();

        assert_eq!(error, "不会执行非 DevDock 生成的入口。");

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
