# 命令注册功能设计

## 目标

让 DevDock 的“注册命令”从前端模拟变成真实可用功能：用户输入脚本路径和命令名后，Tauri 后端在推荐命令目录生成终端入口，刷新时能读取已注册命令，删除时只移除 DevDock 生成的入口。

## 范围

- 注册命令：
  - 校验命令名只能包含字母、数字、点、短横线和下划线。
  - 校验脚本路径存在且是文件。
  - 自动创建推荐命令目录。
  - 在 Unix 平台确保源脚本带有执行权限。
  - 在推荐目录生成入口文件。
- 查询命令：
  - 扫描推荐命令目录。
  - 只读取带 DevDock 标记的入口文件。
  - 返回命令名、源脚本路径、入口路径、入口类型、注册时间。
- 删除命令：
  - 只删除 DevDock 标记的入口文件。
  - 不删除原始脚本文件。
- 前端：
  - 页面加载和刷新时读取真实命令列表。
  - 注册成功后刷新列表并清空表单。
  - 删除确认后调用后端删除。
  - 脚本路径支持手动输入或粘贴，不引入文件选择器依赖。

不在本次实现中支持覆盖已有命令、执行命令、修改用户 shell 配置、持久化数据库或系统级安装。

## 后端设计

在 `src-tauri/src/lib.rs` 中新增 Tauri commands：

- `register_command(scriptPath, commandName)`
- `list_registered_commands()`
- `delete_registered_command(commandName)`

注册时在推荐命令目录生成包装脚本：

- macOS/Linux：入口路径为 `$HOME/.local/bin/<commandName>`，入口类型为 `wrapper`。
- Windows：入口路径为 `%LOCALAPPDATA%\\devdock\\bin\\<commandName>.cmd`，入口类型为 `cmd-shim`。

入口文件包含 DevDock 元数据：

- DevDock 管理标记。
- 命令名。
- 源脚本路径。
- 注册时间戳。

刷新列表时扫描推荐命令目录并解析这些元数据。删除时先确认入口文件带 DevDock 标记，再删除文件，避免误删用户已有命令。

## 前端设计

`src/App.vue` 负责调用后端：

- 初始 `commands` 改为空数组。
- `onMounted` 同时读取 PATH 状态和已注册命令列表。
- `refreshCommands()` 同时刷新 PATH 状态和命令列表。
- `registerCommand()` 调用后端真实注册接口。
- `deleteCommand()` 调用后端删除接口。

`RegisterCommandPanel.vue` 将脚本路径输入框改为可编辑，方便用户粘贴本地脚本绝对路径。暂不接入系统文件选择器，避免增加新插件和权限。

## 错误处理

- 脚本不存在：返回“脚本文件不存在。”
- 脚本路径不是文件：返回“脚本路径不是文件。”
- 脚本文件无法设置执行权限：返回“脚本文件无法设置为可执行。”
- 命令名非法：返回“命令名只能包含字母、数字、点、短横线或下划线。”
- 入口已存在：返回“命令名已存在。”
- 入口不是 DevDock 生成：删除时返回“不会删除非 DevDock 生成的入口。”
- 文件系统创建、读取、写入或删除失败：返回中文错误信息并保留用户输入。

## 测试

新增 Rust 单元测试覆盖：

- 注册命令会创建包装脚本并返回正确字段。
- 列表只返回 DevDock 生成的入口。
- 删除命令会移除入口但保留原始脚本。
- 非法命令名会被拒绝。
- 已存在入口会被拒绝。

实现后运行：

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run build`

## 安全影响

本功能会在当前用户的推荐命令目录写入和删除 DevDock 管理的入口文件。Unix 平台注册时会给源脚本补充执行权限，确保终端入口可以直接调用。删除操作只针对带 DevDock 标记的入口，不删除原始脚本，也不修改 `PATH` 或 shell 配置文件。
