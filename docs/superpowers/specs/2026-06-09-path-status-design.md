# 获取 PATH 环境变量状态设计

## 目标

在 DevDock 的“命令”页面显示真实的系统 `PATH` 状态，判断推荐命令目录是否已经加入 `PATH`，替换当前前端模拟数据。

## 范围

- 新增 Tauri 后端命令读取运行时 `PATH` 环境变量。
- 按平台计算推荐命令目录：
  - macOS/Linux：`$HOME/.local/bin`
  - Windows：`%LOCALAPPDATA%\\devdock\\bin`
- 返回现有前端 `PathStatus` 所需字段，并可附带拆分后的路径列表。
- 前端页面加载和点击刷新时调用后端命令。
- 修复命令继续由现有“复制修复命令”按钮复制。

不在本次实现中注册真实命令、创建目录、修改 shell 配置文件，或自动写入用户环境变量。

## 后端设计

在 `src-tauri/src/lib.rs` 新增可序列化响应结构和命令：

- `PathStatusResponse`
  - `state`: `"checking" | "ok" | "missing" | "error"`
  - `bin_dir`: 推荐命令目录，序列化给前端时映射为 `binDir`
  - `message`: 中文状态提示
  - `suggested_command`: 可选修复命令，映射为 `suggestedCommand`
  - `paths`: 当前 `PATH` 拆分后的路径列表

`get_path_status()` 使用 `std::env::var_os("PATH")` 读取真实环境变量，并用 `std::env::split_paths` 按平台拆分。推荐目录存在于拆分结果中时返回 `ok`，不存在时返回 `missing`，无法读取或为空时返回 `error`。

## 前端设计

在 `src/App.vue` 中使用 `@tauri-apps/api/core` 的 `invoke` 调用 `get_path_status`。

- 初始状态为 `checking`。
- `onMounted` 时刷新 PATH 状态。
- 刷新按钮同时触发现有命令列表刷新反馈和 PATH 状态刷新。
- 后端调用失败时显示 `PATH 异常`，并给出中文错误提示。

`src/types.ts` 为 `PathStatus` 增加可选 `paths?: string[]` 字段。`PathStatusPanel.vue` 暂不增加展开列表，保持界面聚焦。

## 错误处理

- `PATH` 不存在或为空：返回 `error`，提示无法读取系统 `PATH`。
- `HOME` 或 `LOCALAPPDATA` 不存在：使用平台默认字符串作为展示值，并返回 `missing` 或 `error` 的中文提示。
- 前端 `invoke` 失败：显示 `PATH 异常`，提示后端读取失败。

## 测试

新增 Rust 单元测试覆盖核心判断函数：

- 推荐目录存在于 `PATH` 时返回 `ok`。
- 推荐目录不存在于 `PATH` 时返回 `missing`。
- `PATH` 为空时返回 `error`。

实现后运行：

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run build`

## 安全影响

本功能只读取当前进程可见的环境变量 `PATH`，不写入文件、不修改 shell 配置、不执行外部命令。前端展示内容限制为路径状态和推荐目录，避免主动暴露完整环境变量内容。
