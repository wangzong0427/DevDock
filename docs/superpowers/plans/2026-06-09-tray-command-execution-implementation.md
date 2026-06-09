# 状态栏命令执行功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让用户能从 macOS 顶部状态栏执行 DevDock 已注册命令，并且只在失败时弹出主窗口。

**Architecture:** Rust 侧创建 Tauri tray icon 和动态菜单，菜单项 id 映射到已注册命令名。菜单点击复用现有命令执行 helper，并通过 Tauri 事件把开始、输出、完成和失败状态发送给前端。

**Tech Stack:** Tauri 2.11、Rust、Vue 3、TypeScript、Element Plus。

---

## File Structure

- Modify: `src-tauri/Cargo.toml`，启用 `tauri` 的 `tray-icon` feature。
- Modify: `src-tauri/src/lib.rs`，新增状态栏菜单构建、菜单事件处理、失败时显示主窗口和测试 helper。
- Modify: `src/types.ts`，新增托盘执行事件类型。
- Modify: `src/App.vue`，监听托盘执行开始、完成和失败事件，更新现有日志面板状态。

### Task 1: 后端托盘行为测试

- [ ] **Step 1: Write failing tests**

在 `src-tauri/src/lib.rs` 的 `tests` 模块中新增测试：

```rust
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
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parses_tray_command_menu_id only_failed_tray_results_should_show_main_window`

Expected: FAIL because the helper functions are not defined.

### Task 2: 后端状态栏菜单

- [ ] **Step 1: Enable tray API**

在 `src-tauri/Cargo.toml` 中把 `tauri = { version = "2", features = [] }` 改成 `tauri = { version = "2", features = ["tray-icon"] }`。

- [ ] **Step 2: Implement tray helpers**

在 `src-tauri/src/lib.rs` 增加菜单 id 常量、`command_name_from_tray_menu_id()`、`should_show_window_for_tray_result()`、`build_tray_menu()`、`setup_tray()`、`handle_tray_menu_event()` 和 `show_main_window()`。

- [ ] **Step 3: Wire command lifecycle**

菜单命令点击后发送 `command-run-started`，执行时继续发送 `command-output`，完成后发送 `command-run-finished`，执行错误时发送 `command-run-failed`。只有 `should_show_window_for_tray_result()` 返回 true 时调用 `show_main_window()`。

- [ ] **Step 4: Refresh menu after changes**

`register_command()` 和 `delete_registered_command()` 成功后刷新状态栏菜单。

### Task 3: 前端事件接入

- [ ] **Step 1: Add event types**

在 `src/types.ts` 增加 `CommandRunStarted`、`CommandRunFinished`、`CommandRunFailed`。

- [ ] **Step 2: Listen to tray lifecycle events**

在 `src/App.vue` 中监听 `command-run-started`、`command-run-finished` 和 `command-run-failed`，让现有 `runningCommandName` 与 `runOutput` 能展示状态栏执行结果。

- [ ] **Step 3: Cleanup listeners**

在 `onBeforeUnmount()` 中解除新增监听。

### Task 4: Verification

- [ ] **Step 1: Format Rust**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 2: Run backend tests and checks**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 3: Run frontend build**

Run: `npm run build`

- [ ] **Step 4: Review diff**

Run: `git diff -- src-tauri/Cargo.toml src-tauri/src/lib.rs src/types.ts src/App.vue docs/superpowers/specs/2026-06-09-tray-command-execution-design.md docs/superpowers/plans/2026-06-09-tray-command-execution-implementation.md`

Expected: diff only contains status bar command execution functionality and its docs.
