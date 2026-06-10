# 命令注册功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 DevDock 的真实命令注册、列表刷新和删除功能，替换当前前端模拟数据。

**Architecture:** Rust 后端在推荐 bin 目录创建带 DevDock 元数据的包装脚本，并通过 Tauri command 暴露注册、列表和删除能力。Vue 前端调用这些命令，注册成功后刷新列表，删除确认后移除真实入口。

**Tech Stack:** Tauri 2、Rust 标准库文件系统 API、serde、Vue 3 `<script setup lang="ts">`、TypeScript、Element Plus。

---

### Task 1: 后端注册核心

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write failing tests**

在 `src-tauri/src/lib.rs` 的测试模块中新增测试：

```rust
#[test]
fn creates_wrapper_and_lists_registered_command() {
    let temp_dir = unique_test_dir("register-list");
    let bin_dir = temp_dir.join("bin");
    let script_path = temp_dir.join("deploy.sh");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();

    let command = create_registered_command(&script_path, "deploy-preview", &bin_dir, "1700000000")
        .expect("register command");

    assert_eq!(command.name, "deploy-preview");
    assert_eq!(command.script_path, display_path(&script_path));
    assert!(PathBuf::from(&command.entry_path).exists());

    let commands = list_registered_commands_in_dir(&bin_dir).expect("list commands");
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "deploy-preview");
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `create_registered_command` and `list_registered_commands_in_dir` are not defined.

- [x] **Step 3: Implement minimal backend registration**

Add `RegisteredCommandResponse`, command name validation, wrapper file creation, metadata parsing, and `register_command` / `list_registered_commands` Tauri commands.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS for existing PATH tests and the new registration/list test.

### Task 2: 后端删除保护

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests that deletion removes only DevDock managed entries and preserves the source script:

```rust
#[test]
fn deletes_managed_entry_and_keeps_source_script() {
    let temp_dir = unique_test_dir("delete-managed");
    let bin_dir = temp_dir.join("bin");
    let script_path = temp_dir.join("deploy.sh");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(&script_path, "#!/bin/sh\necho ok\n").unwrap();
    let command = create_registered_command(&script_path, "deploy-preview", &bin_dir, "1700000000")
        .expect("register command");

    delete_registered_command_in_dir("deploy-preview", &bin_dir).expect("delete command");

    assert!(!PathBuf::from(command.entry_path).exists());
    assert!(script_path.exists());
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
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `delete_registered_command_in_dir` is not defined.

- [x] **Step 3: Implement protected deletion**

Add `delete_registered_command` Tauri command and helper that validates command name, checks DevDock metadata, then removes the entry file.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS for all backend tests.

### Task 3: 前端真实接入

**Files:**
- Modify: `src/features/commands/RegisterCommandPanel.vue`
- Modify: `src/features/commands/CommandsView.vue`
- Modify: `src/App.vue`

- [x] **Step 1: Make script path editable**

Remove the read-only mock browse behavior from the script path input so users can paste an absolute script path.

- [x] **Step 2: Load real commands**

Add `refreshRegisteredCommands()` in `src/App.vue`, initialize `commands` as empty, and call `invoke<RegisteredCommand[]>("list_registered_commands")`.

- [x] **Step 3: Register through backend**

Update `registerCommand()` to call `invoke<RegisteredCommand>("register_command", { scriptPath, commandName })`, then refresh the list and clear the form.

- [x] **Step 4: Delete through backend**

Update `deleteCommand()` to call `invoke("delete_registered_command", { commandName })`, then refresh the list and close the dialog.

- [x] **Step 5: Verify frontend build**

Run: `npm run build`

Expected: PASS.

### Task 4: Full verification

**Files:**
- Verify: `src-tauri/src/lib.rs`
- Verify: `src/App.vue`
- Verify: `src/features/commands/RegisterCommandPanel.vue`
- Verify: `src/features/commands/CommandsView.vue`

- [x] **Step 1: Run Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [x] **Step 2: Run Rust check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [x] **Step 3: Run frontend build**

Run: `npm run build`

Expected: PASS.

- [x] **Step 4: Inspect diff**

Run: `git diff -- src-tauri/src/lib.rs src/App.vue src/features/commands/RegisterCommandPanel.vue src/features/commands/CommandsView.vue src/types.ts docs/superpowers/specs/2026-06-09-command-registration-design.md docs/superpowers/plans/2026-06-09-command-registration-implementation.md`

Expected: Diff only contains command registration functionality and its docs.
