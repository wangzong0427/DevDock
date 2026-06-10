use crate::{
    command_registry::{list_registered_commands_in_dir, spawn_registered_command_run},
    path_status::recommended_bin_dir,
    responses::{CommandOutputChunkResponse, CommandRunFailedResponse, CommandRunStartedResponse},
};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(desktop)]
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

const TRAY_ID: &str = "devdock-main-tray";
const TRAY_COMMAND_PREFIX: &str = "tray-command-";
const TRAY_MENU_OPEN_WINDOW: &str = "tray-open-window";
const TRAY_MENU_REFRESH_COMMANDS: &str = "tray-refresh-commands";
const TRAY_MENU_QUIT: &str = "tray-quit";

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
pub(crate) fn setup_tray(app: &AppHandle) -> Result<(), String> {
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
pub(crate) fn setup_tray(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(desktop)]
pub(crate) fn refresh_tray_menu(app: &AppHandle) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let menu = build_tray_menu(app)?;
    tray.set_menu(Some(menu))
        .map_err(|error| format!("状态栏菜单刷新失败：{error}"))
}

#[cfg(not(desktop))]
pub(crate) fn refresh_tray_menu(_app: &AppHandle) -> Result<(), String> {
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
pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(not(desktop))]
pub(crate) fn show_main_window(_app: &AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

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
        let source = include_str!("tray.rs");
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
}
