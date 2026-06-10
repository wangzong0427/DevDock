# PATH 状态读取 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 DevDock 能从 Tauri 后端读取真实 `PATH` 环境变量，并在命令页面显示推荐命令目录是否已加入 `PATH`。

**Architecture:** Rust 后端负责读取和判断运行时 `PATH`，通过 Tauri command 返回序列化状态。Vue 前端在页面加载和刷新时调用该 command，并复用现有 `PathStatusPanel` 显示结果。

**Tech Stack:** Tauri 2、Rust、serde、Vue 3 `<script setup lang="ts">`、TypeScript、Element Plus。

---

### Task 1: Rust PATH 状态计算

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write the failing tests**

在 `src-tauri/src/lib.rs` 底部增加测试模块，覆盖推荐目录存在、不存在和空 `PATH`：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        let paths = vec![PathBuf::from("/usr/bin"), PathBuf::from("/opt/homebrew/bin")];

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
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `build_path_status` and `PathStatusResponse` are not defined.

- [x] **Step 3: Write minimal implementation**

Replace the example `greet` command with `PathStatusResponse`, `get_path_status`, `build_path_status`, and helper functions. Register `get_path_status` in `tauri::generate_handler!`.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS for the three PATH status tests.

### Task 2: Vue 前端调用后端命令

**Files:**
- Modify: `src/types.ts`
- Modify: `src/App.vue`

- [x] **Step 1: Update TypeScript type**

Add `paths?: string[]` to `PathStatus`.

- [x] **Step 2: Wire invoke on mount and refresh**

In `src/App.vue`, import `invoke` and `onMounted`, set initial `pathStatus` to `checking`, add `refreshPathStatus()`, call it in `onMounted`, and call it from `refreshCommands()`.

- [x] **Step 3: Verify frontend build**

Run: `npm run build`

Expected: PASS with `vue-tsc --noEmit` and Vite build completed.

### Task 3: Full verification

**Files:**
- Verify: `src-tauri/src/lib.rs`
- Verify: `src/App.vue`
- Verify: `src/types.ts`

- [x] **Step 1: Run Rust checks**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [x] **Step 2: Run final frontend build**

Run: `npm run build`

Expected: PASS.

- [x] **Step 3: Inspect changed files**

Run: `git diff -- src-tauri/src/lib.rs src/App.vue src/types.ts docs/superpowers/specs/2026-06-09-path-status-design.md docs/superpowers/plans/2026-06-09-path-status-implementation.md`

Expected: Diff only contains PATH 状态读取功能、设计文档和实现计划。
