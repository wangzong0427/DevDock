use serde::Serialize;
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(desktop)]
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

const DEVDOC_MARKER: &str = "DevDock managed command";
const COMMAND_NAME_META: &str = "devdock-command-name:";
const SCRIPT_PATH_META: &str = "devdock-script-path:";
const CREATED_AT_META: &str = "devdock-created-at:";
const TRAY_ID: &str = "devdock-main-tray";
const TRAY_COMMAND_PREFIX: &str = "tray-command-";
const TRAY_MENU_OPEN_WINDOW: &str = "tray-open-window";
const TRAY_MENU_REFRESH_COMMANDS: &str = "tray-refresh-commands";
const TRAY_MENU_QUIT: &str = "tray-quit";

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandRunResultResponse {
    command_name: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandRunStartedResponse {
    command_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandRunFailedResponse {
    command_name: String,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandOutputChunkResponse {
    command_name: String,
    stream: String,
    text: String,
}

#[tauri::command]
fn get_path_status() -> PathStatusResponse {
    let bin_dir = recommended_bin_dir();
    let paths = environment_path_entries();

    build_path_status(bin_dir, paths)
}

#[tauri::command(rename_all = "camelCase")]
async fn register_command(
    app: AppHandle,
    script_path: String,
    command_name: String,
) -> Result<RegisteredCommandResponse, String> {
    let created_at = current_timestamp();
    let script_path = PathBuf::from(script_path);
    let bin_dir = recommended_bin_dir();

    run_blocking_task(move || {
        create_registered_command(&script_path, &command_name, &bin_dir, &created_at)
    })
    .await
    .inspect(|_| {
        let _ = refresh_tray_menu(&app);
    })
}

#[tauri::command]
async fn list_registered_commands() -> Result<Vec<RegisteredCommandResponse>, String> {
    let bin_dir = recommended_bin_dir();
    run_blocking_task(move || list_registered_commands_in_dir(&bin_dir)).await
}

#[tauri::command(rename_all = "camelCase")]
async fn delete_registered_command(app: AppHandle, command_name: String) -> Result<(), String> {
    let bin_dir = recommended_bin_dir();
    run_blocking_task(move || delete_registered_command_in_dir(&command_name, &bin_dir))
        .await
        .inspect(|_| {
            let _ = refresh_tray_menu(&app);
        })
}

#[tauri::command(rename_all = "camelCase")]
async fn run_registered_command(
    app: AppHandle,
    command_name: String,
) -> Result<CommandRunResultResponse, String> {
    let bin_dir = recommended_bin_dir();
    let event_command_name = command_name.clone();
    spawn_registered_command_run(command_name, bin_dir, move |stream, text| {
        let _ = app.emit(
            "command-output",
            CommandOutputChunkResponse {
                command_name: event_command_name.clone(),
                stream: stream.to_string(),
                text: text.to_string(),
            },
        );
    })
    .await
    .map_err(|error| format!("后台任务执行失败：{error}"))?
}

async fn run_blocking_task<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("后台任务执行失败：{error}"))?
}

fn spawn_registered_command_run(
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

fn tray_command_menu_id(command_name: &str) -> String {
    format!("{TRAY_COMMAND_PREFIX}{command_name}")
}

fn command_name_from_tray_menu_id(menu_id: &str) -> Option<String> {
    menu_id
        .strip_prefix(TRAY_COMMAND_PREFIX)
        .filter(|command_name| !command_name.is_empty())
        .map(ToString::to_string)
}

fn should_show_window_for_tray_result(exit_code: Option<i32>, execution_error: bool) -> bool {
    execution_error || exit_code != Some(0)
}

#[cfg(desktop)]
fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let menu = build_tray_menu(app)?;
    let tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("DevDock")
        .icon(tray_template_icon())
        .icon_as_template(true)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            handle_tray_menu_event(app, event.id().as_ref());
        });

    tray_builder
        .build(app)
        .map(|_| ())
        .map_err(|error| format!("状态栏图标创建失败：{error}"))
}

#[cfg(desktop)]
fn tray_template_icon() -> Image<'static> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0; (SIZE * SIZE * 4) as usize];

    fn fill_rect(rgba: &mut [u8], x1: u32, y1: u32, x2: u32, y2: u32) {
        for y in y1..y2 {
            for x in x1..x2 {
                let index = ((y * 32 + x) * 4) as usize;
                rgba[index] = 255;
                rgba[index + 1] = 255;
                rgba[index + 2] = 255;
                rgba[index + 3] = 255;
            }
        }
    }

    fill_rect(&mut rgba, 8, 7, 13, 22);
    fill_rect(&mut rgba, 11, 7, 21, 12);
    fill_rect(&mut rgba, 11, 17, 21, 22);
    fill_rect(&mut rgba, 19, 9, 24, 20);
    fill_rect(&mut rgba, 8, 25, 24, 28);

    Image::new_owned(rgba, SIZE, SIZE)
}

#[cfg(not(desktop))]
fn setup_tray(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(desktop)]
fn refresh_tray_menu(app: &AppHandle) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let menu = build_tray_menu(app)?;
    tray.set_menu(Some(menu))
        .map_err(|error| format!("状态栏菜单刷新失败：{error}"))
}

#[cfg(not(desktop))]
fn refresh_tray_menu(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(desktop)]
fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
    let menu = Menu::new(app).map_err(|error| format!("状态栏菜单创建失败：{error}"))?;
    let commands = list_registered_commands_in_dir(&recommended_bin_dir())?;

    if commands.is_empty() {
        let empty_item = MenuItem::with_id(
            app,
            "tray-empty-commands",
            "暂无已注册命令",
            false,
            None::<&str>,
        )
        .map_err(|error| format!("状态栏空菜单创建失败：{error}"))?;
        menu.append(&empty_item)
            .map_err(|error| format!("状态栏空菜单追加失败：{error}"))?;
    } else {
        for command in commands {
            let command_item = MenuItem::with_id(
                app,
                tray_command_menu_id(&command.name),
                command.name,
                true,
                None::<&str>,
            )
            .map_err(|error| format!("状态栏命令菜单创建失败：{error}"))?;
            menu.append(&command_item)
                .map_err(|error| format!("状态栏命令菜单追加失败：{error}"))?;
        }
    }

    let separator = PredefinedMenuItem::separator(app)
        .map_err(|error| format!("状态栏分隔线创建失败：{error}"))?;
    let open_item = MenuItem::with_id(
        app,
        TRAY_MENU_OPEN_WINDOW,
        "打开 DevDock",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("状态栏打开菜单创建失败：{error}"))?;
    let refresh_item = MenuItem::with_id(
        app,
        TRAY_MENU_REFRESH_COMMANDS,
        "刷新命令",
        true,
        None::<&str>,
    )
    .map_err(|error| format!("状态栏刷新菜单创建失败：{error}"))?;
    let quit_item = MenuItem::with_id(app, TRAY_MENU_QUIT, "退出 DevDock", true, None::<&str>)
        .map_err(|error| format!("状态栏退出菜单创建失败：{error}"))?;

    menu.append(&separator)
        .map_err(|error| format!("状态栏分隔线追加失败：{error}"))?;
    menu.append(&open_item)
        .map_err(|error| format!("状态栏打开菜单追加失败：{error}"))?;
    menu.append(&refresh_item)
        .map_err(|error| format!("状态栏刷新菜单追加失败：{error}"))?;
    menu.append(&quit_item)
        .map_err(|error| format!("状态栏退出菜单追加失败：{error}"))?;

    Ok(menu)
}

#[cfg(desktop)]
fn handle_tray_menu_event(app: &AppHandle, menu_id: &str) {
    if let Some(command_name) = command_name_from_tray_menu_id(menu_id) {
        run_registered_command_from_tray(app.clone(), command_name);
        return;
    }

    match menu_id {
        TRAY_MENU_OPEN_WINDOW => show_main_window(app),
        TRAY_MENU_REFRESH_COMMANDS => {
            let _ = refresh_tray_menu(app);
        }
        TRAY_MENU_QUIT => app.exit(0),
        _ => {}
    }
}

#[cfg(desktop)]
fn run_registered_command_from_tray(app: AppHandle, command_name: String) {
    let _ = app.emit(
        "command-run-started",
        CommandRunStartedResponse {
            command_name: command_name.clone(),
        },
    );

    let bin_dir = recommended_bin_dir();
    let event_app = app.clone();
    let event_command_name = command_name.clone();
    let run_handle =
        spawn_registered_command_run(command_name.clone(), bin_dir, move |stream, text| {
            let _ = event_app.emit(
                "command-output",
                CommandOutputChunkResponse {
                    command_name: event_command_name.clone(),
                    stream: stream.to_string(),
                    text: text.to_string(),
                },
            );
        });

    tauri::async_runtime::spawn(async move {
        let run_result = run_handle
            .await
            .map_err(|error| format!("后台任务执行失败：{error}"));

        match run_result {
            Ok(Ok(result)) => {
                let should_show_window =
                    should_show_window_for_tray_result(result.exit_code, false);
                let _ = app.emit("command-run-finished", &result);
                if should_show_window {
                    show_main_window(&app);
                }
            }
            Ok(Err(message)) | Err(message) => {
                let _ = app.emit(
                    "command-run-failed",
                    CommandRunFailedResponse {
                        command_name,
                        message,
                    },
                );
                if should_show_window_for_tray_result(None, true) {
                    show_main_window(&app);
                }
            }
        }
    });
}

#[cfg(desktop)]
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
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

fn environment_path_entries() -> Vec<PathBuf> {
    let process_paths = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();

    merge_path_entries(process_paths, login_shell_path_entries())
}

fn merge_path_entries(mut primary: Vec<PathBuf>, secondary: Vec<PathBuf>) -> Vec<PathBuf> {
    for path in secondary {
        if !primary.contains(&path) {
            primary.push(path);
        }
    }

    primary
}

#[cfg(target_os = "macos")]
fn login_shell_path_entries() -> Vec<PathBuf> {
    let shell = login_shell_path();
    let output = Command::new(shell)
        .arg("-l")
        .arg("-c")
        .arg("printf %s \"$PATH\"")
        .output();

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

#[cfg(target_os = "macos")]
fn login_shell_path() -> PathBuf {
    env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
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
    validate_command_name(command_name)?;

    let entry_path = command_entry_path(bin_dir, command_name);
    if !entry_path.exists() {
        return Err("命令不存在。".to_string());
    }

    if read_registered_command(&entry_path)?.is_none() {
        return Err("不会执行非 DevDock 生成的入口。".to_string());
    }

    let mut child = Command::new(&entry_path)
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
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            setup_tray(app.handle()).map_err(|error| Box::<dyn std::error::Error>::from(error))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_path_status,
            register_command,
            list_registered_commands,
            delete_registered_command,
            run_registered_command
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
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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

    #[test]
    fn parses_tray_command_menu_id() {
        assert_eq!(
            command_name_from_tray_menu_id("tray-command-deploy-preview"),
            Some("deploy-preview".to_string())
        );
        assert_eq!(command_name_from_tray_menu_id("open-window"), None);
    }

    #[test]
    fn only_failed_tray_results_should_show_main_window() {
        assert!(!should_show_window_for_tray_result(Some(0), false));
        assert!(should_show_window_for_tray_result(Some(7), false));
        assert!(should_show_window_for_tray_result(None, false));
        assert!(should_show_window_for_tray_result(Some(0), true));
    }

    #[test]
    fn tray_icon_does_not_reuse_app_window_icon() {
        let source = include_str!("lib.rs");
        let setup_tray_start = source.find("fn setup_tray").expect("setup_tray exists");
        let setup_tray_end = source[setup_tray_start..]
            .find("#[cfg(not(desktop))]")
            .map(|index| setup_tray_start + index)
            .expect("non-desktop setup_tray exists");
        let setup_tray_source = &source[setup_tray_start..setup_tray_end];

        assert!(
            !setup_tray_source.contains("default_window_icon"),
            "状态栏图标必须使用透明的 template 专用图标，不能复用彩色应用图标。"
        );
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
