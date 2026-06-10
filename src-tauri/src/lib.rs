mod command_registry;
mod path_status;
mod responses;
mod tray;

use crate::{
    command_registry::{
        create_registered_command, current_timestamp, delete_registered_command_in_dir,
        list_registered_commands_in_dir, spawn_registered_command_run,
    },
    path_status::{get_path_status as get_path_status_response, recommended_bin_dir},
    responses::{CommandOutputChunkResponse, CommandRunResultResponse, PathStatusResponse},
    tray::{refresh_tray_menu, setup_tray, show_main_window},
};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, RunEvent, WindowEvent};

#[tauri::command]
fn get_path_status() -> PathStatusResponse {
    get_path_status_response()
}

#[tauri::command(rename_all = "camelCase")]
async fn register_command(
    app: AppHandle,
    script_path: String,
    command_name: String,
) -> Result<responses::RegisteredCommandResponse, String> {
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
async fn list_registered_commands() -> Result<Vec<responses::RegisteredCommandResponse>, String> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())
                .map_err(Box::<dyn std::error::Error>::from)?;
            setup_tray(app.handle()).map_err(Box::<dyn std::error::Error>::from)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_path_status,
            register_command,
            list_registered_commands,
            delete_registered_command,
            run_registered_command
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let RunEvent::Reopen { .. } = event {
                show_main_window(app);
            }
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn close_request_hides_main_window_instead_of_exiting() {
        let source = include_str!("lib.rs");
        let run_start = source
            .find("#[cfg_attr(mobile, tauri::mobile_entry_point)]")
            .expect("run function exists");
        let tests_start = source[run_start..]
            .find("#[cfg(test)]")
            .map(|index| run_start + index)
            .expect("test module exists");
        let run_source = &source[run_start..tests_start];

        assert!(
            run_source.contains(".on_window_event("),
            "应用必须监听窗口关闭事件，避免默认关闭后退出进程。"
        );
        assert!(
            run_source.contains("WindowEvent::CloseRequested"),
            "应用必须处理 CloseRequested 事件。"
        );
        assert!(
            run_source.contains("api.prevent_close()"),
            "点击窗口关闭时必须阻止真实关闭。"
        );
        assert!(
            run_source.contains(".hide()"),
            "点击窗口关闭时必须隐藏窗口，让应用后台常驻。"
        );
    }

    #[test]
    fn dock_reopen_event_shows_main_window() {
        let source = include_str!("lib.rs");
        let run_start = source
            .find("#[cfg_attr(mobile, tauri::mobile_entry_point)]")
            .expect("run function exists");
        let tests_start = source[run_start..]
            .find("#[cfg(test)]")
            .map(|index| run_start + index)
            .expect("test module exists");
        let run_source = &source[run_start..tests_start];

        assert!(
            run_source.contains(".build(tauri::generate_context!())"),
            "应用必须显式 build 后运行，才能处理 Dock 重新打开事件。"
        );
        assert!(
            run_source.contains("RunEvent::Reopen"),
            "点击 Dock 图标时必须处理 Reopen 事件。"
        );
        assert!(
            run_source.contains("show_main_window(app)"),
            "点击 Dock 图标时必须重新显示主窗口。"
        );
    }
}
