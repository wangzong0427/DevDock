# 已注册命令一键执行功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在已注册命令列表中增加一键执行操作，并展示命令执行结果。

**Architecture:** Rust 后端只执行推荐命令目录中带 DevDock 标记的入口文件，并返回捕获到的 stdout、stderr 和退出码。Vue 前端在 `App.vue` 维护运行状态和最近一次结果，`CommandList.vue` 负责按钮与结果展示。

**Tech Stack:** Tauri 2、Rust `std::process::Command`、Vue 3 `<script setup lang="ts">`、Element Plus。

---

## File Structure

- Modify: `src-tauri/src/lib.rs`，新增 `CommandRunResultResponse`、执行 helper、Tauri command 和后端测试。
- Modify: `src/types.ts`，新增 `CommandRunResult` 类型。
- Modify: `src/App.vue`，维护执行状态，调用 `run_registered_command`。
- Modify: `src/features/commands/CommandsView.vue`，透传执行状态、结果和事件。
- Modify: `src/features/commands/CommandList.vue`，增加“执行”按钮和最近一次执行结果面板。
- Modify: `src/styles.css`，补充结果面板样式。

### Task 1: 后端执行能力

- [ ] **Step 1: Write failing tests**

在 `src-tauri/src/lib.rs` 的 `tests` 模块中新增测试：

```rust
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
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `run_registered_command_in_dir` and `CommandRunResultResponse` are not defined.

- [ ] **Step 3: Implement minimal backend**

Add `CommandRunResultResponse`, `run_registered_command`, `run_registered_command_in_dir`, and register the Tauri command in `generate_handler!`.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: all tests pass.

### Task 2: 前端接入

- [ ] **Step 1: Add types**

在 `src/types.ts` 增加：

```ts
export type CommandRunResult = {
  commandName: string;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
};
```

- [ ] **Step 2: Add App state and invoke**

在 `src/App.vue` 增加 `runningCommandName`、`lastRunResult`、`runCommand()`，调用 `invoke<CommandRunResult>("run_registered_command", { commandName })`。

- [ ] **Step 3: Wire components**

在 `CommandsView.vue` 和 `CommandList.vue` 透传 `runningCommandName`、`lastRunResult` 和 `runCommand` 事件。

- [ ] **Step 4: Style result panel**

在 `src/styles.css` 增加执行结果面板、输出块和移动端适配。

- [ ] **Step 5: Verify frontend**

Run: `npm run build`

Expected: TypeScript and Vite build pass.

### Task 3: Final verification

- [ ] **Step 1: Format Rust**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 2: Run Rust checks**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 3: Run frontend build**

Run: `npm run build`

- [ ] **Step 4: Review diff**

Run: `git diff -- src-tauri/src/lib.rs src/types.ts src/App.vue src/features/commands/CommandsView.vue src/features/commands/CommandList.vue src/styles.css docs/superpowers/specs/2026-06-09-command-execution-design.md docs/superpowers/plans/2026-06-09-command-execution-implementation.md`

Expected: diff only contains one-click registered command execution functionality and its docs.
