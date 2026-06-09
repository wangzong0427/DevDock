# DevDock UI 设计

日期：2026-06-09
状态：待用户 review
前端目标：Tauri 内的 Vue 3 + TypeScript

## 产品背景

DevDock 是一个本地开发者工具箱。第一版聚焦于把本地脚本文件注册成终端可调用的快捷命令。后续版本可能增加 ADB 相关工作流，所以界面应该体现“可扩展的开发者工具箱”，而不是只服务于脚本管理的小工具。

第一版需要覆盖：

- 选择一个脚本文件。
- 输入快捷命令名。
- 注册命令。
- 生成可执行入口。
- 检查命令目录是否已配置到 `PATH`。
- 查看已注册命令。
- 删除已注册命令。
- 预留 ADB 模块入口，但暂不实现 ADB 功能。

界面风格应安静、清晰、实用，信息密度适合开发者反复使用。不要做成营销页或介绍页。

## 已选信息架构

采用 Workbench 工具台布局：

- 左侧固定侧栏，用于展示产品身份、模块导航和紧凑的全局状态。
- 右侧为当前模块的工作区。
- 第一版默认进入 `Commands` 模块。
- `ADB` 作为后续模块出现在侧栏中，可以禁用或标记为 `Planned`。

这个结构更符合 DevDock 长期扩展成开发者工具箱的方向，同时能让第一版保持聚焦。

## 窗口结构

推荐桌面窗口尺寸：

- 最小宽度：960px。
- 最小高度：640px。
- 侧栏宽度：224px。
- 内容区内边距：28px 到 32px。

响应式行为：

- 低于 820px 时，侧栏可以压缩成更紧凑的导航栏。
- 第一版尽量保持导航可见，除非窗口宽度过窄。
- 窄屏下，注册表单和命令列表垂直堆叠。

## 侧栏

侧栏包含：

- 产品身份：
  - 应用名：`DevDock`。
  - 辅助说明：`Local developer commands`。
- 模块导航：
  - `Commands`，当前选中。
  - `ADB`，禁用或标记为 `Planned`。
- 底部状态：
  - 平台标签，例如 `macOS`、`Linux` 或 `Windows`。
  - PATH 状态：`PATH OK`、`PATH Missing` 或 `Checking`。

视觉处理：

- 侧栏背景使用安静的中性浅灰，比内容区略深。
- 当前选中的导航项需要有清晰的左侧强调线或填充态。
- 计划中的模块保持可见，但视觉上降低权重。
- 侧栏只放短标签，不放长说明文字。

## Commands 页面

Commands 页面包含四个区域：

1. 页面头部。
2. 注册命令面板。
3. PATH 状态面板。
4. 已注册命令列表。

### 页面头部

内容：

- 标题：`Commands`。
- 副标题：`Register local scripts as commands you can run from your terminal.`。
- 右侧可预留 `Refresh` 操作，等后端列表和 PATH 检查命令接入后启用。

头部应保持紧凑，不做 hero 区。

### 注册命令面板

目的：

- 让第一版核心工作流一打开就可见。
- 用户无需跳转页面即可完成脚本注册。

字段和控件：

- 脚本文件选择：
  - 只读路径展示。
  - `Browse` 按钮。
  - 空状态文案：`Choose a script file`。
- 命令名输入：
  - 占位符：`my-command`。
  - 使用行内校验提示命令名是否合法。
- 生成入口预览：
  - macOS/Linux 示例：`~/.local/bin/my-command`。
  - Windows 示例：`%LOCALAPPDATA%\devdock\bin\my-command.cmd`。
  - 用户输入命令名时，预览路径实时更新。
- 主操作：
  - `Register command`。
  - 只有脚本文件和合法命令名都存在时才可点击。

校验规则：

- 命令名必填。
- 第一版使用保守命名规则：字母、数字、`_`、`-`、`.`。
- 如果后端返回命令名冲突，显示冲突状态。
- 第一版不提供覆盖已有命令，除非后续明确加入。

加载和反馈：

- 注册中禁用主按钮，并在按钮内显示 loading 状态。
- 注册成功后清空表单，将新命令加入列表，并显示短 toast。
- 注册失败时，在面板内显示简短错误信息，并保留用户已输入内容。

### PATH 状态面板

目的：

- 告诉用户已注册命令是否能在终端全局调用。
- 第一版只检测 PATH，不自动修改 shell 配置或系统设置。

状态：

- 检查中：
  - `Checking PATH...`
- 正常：
  - `Command directory is on PATH.`
  - 展示命令目录。
- 缺失：
  - `Command directory is not on PATH.`
  - 展示命令目录。
  - 展示可复制的 shell 片段或路径说明。

平台目录：

- macOS/Linux：`~/.local/bin`。
- Windows：`%LOCALAPPDATA%\devdock\bin`。

PATH 缺失引导：

- 文案保持短、明确、可复制。
- 第一版不要自动修改 PATH。
- 明确说明：注册可以成功，但在配置 PATH 前，命令不会在终端中全局可用。

### 已注册命令列表

目的：

- 让用户查看和移除 DevDock 已注册的命令。

列表字段：

- 命令名。
- 源脚本路径。
- 入口类型，例如 `symlink`、`wrapper` 或 `cmd shim`。
- 注册时间。
- 操作。

行操作：

- `Reveal` 或 `Open location`，如果 Tauri opener 支持。
- `Delete`。

删除行为：

- 删除前弹出确认对话框。
- 确认文案应包含命令名。
- 删除只移除 DevDock 生成的入口，不删除原始脚本文件。

空状态：

- 标题：`No commands registered`。
- 辅助文案：`Choose a script and create your first terminal command.`。
- 可以通过视觉引导指向注册面板，不需要再放第二个主按钮。

列表加载：

- 加载时使用骨架行或紧凑 loading 行。
- 加载失败时显示行内错误和重试操作。

## ADB 预留入口

ADB 模块在第一版中可见，但不实现具体功能。

推荐处理：

- 侧栏项：`ADB`。
- 小状态标签：`Planned`。
- 如果允许点击，进入一个简单的预留页面：
  - 标题：`ADB`。
  - 文案：`Device workflows will live here in a later version.`。
  - 不展示假的设备操作控件。

这样可以让产品方向可见，但不会误导用户以为功能已经完成。

## 视觉系统

基调：

- 开发者工具。
- 安静、克制、以工作效率为中心。
- 避免营销页、大字号 hero、装饰性背景和单一色系堆叠。

颜色：

- 应用背景：浅中性灰。
- 面板：白色或近白色。
- 主文字：深中性色。
- 次级文字：中性灰。
- 边框：浅中性灰。
- 主强调色：克制的青绿或蓝绿色。
- 警告 / PATH 缺失：琥珀色或低饱和橙色。
- 删除 / 危险操作：红色，少量使用。

形状和间距：

- 圆角：6px 到 8px。
- 需要帮助扫描的位置使用明确面板边界。
- 避免卡片套卡片。
- 表单行对齐紧凑，但保留足够点击区域。

字体：

- 使用系统 UI 字体栈。
- 不使用随视口宽度缩放的字体。
- 页面标题约 24px。
- 区块标题约 14px 到 16px。
- 正文和表单文字约 13px 到 14px。
- 不使用负 letter-spacing。

图标：

- 后续如加入图标库，优先使用 Lucide 风格图标。
- 可用图标包括：terminal、file、plus/register、trash、folder/reveal、refresh、check、alert。
- 第一版图标用于辅助扫描，不替代关键文字标签。

## 交互模型

初始加载：

1. 加载平台信息。
2. 检查命令目录和 PATH 状态。
3. 加载已注册命令。
4. 展示 Commands 页面。

注册流程：

1. 用户选择脚本文件。
2. 用户输入命令名。
3. UI 校验命令名，并预览生成入口路径。
4. 用户点击 `Register command`。
5. Rust 后端创建 symlink、wrapper 或 shim。
6. UI 刷新已注册命令列表并展示结果。

删除流程：

1. 用户点击 `Delete`。
2. 显示确认对话框。
3. 用户确认删除。
4. Rust 后端移除生成入口。
5. UI 刷新已注册命令列表。

PATH 流程：

1. UI 展示当前 PATH 状态。
2. 如果 PATH 缺失，用户可以复制建议路径或 shell 片段。
3. 第一版不自动修改 PATH。

## 错误处理

错误应该可操作，并尽量出现在失败操作附近。

常见错误：

- 脚本文件不存在。
- 脚本文件无法设置为可执行。
- 命令名已存在。
- 命令目录无法创建。
- 生成 shim 失败。
- PATH 检查失败。
- 已注册命令加载失败。
- 删除失败，例如入口文件不存在或权限不足。

文案风格：

- 先给出简短解释。
- 必要时包含路径或命令名。
- 不在 UI 中直接展示 Rust 原始错误堆栈。
- 详细日志可以留给开发环境，不放在主界面。

## Vue 组件规划

第一版可以先简单实现，但 UI 边界应保持清晰：

- `App.vue`：整体 shell 布局、当前模块状态、高层 mock 或后端数据。
- `SidebarNav.vue`：产品身份、导航项、平台和 PATH 状态标签。
- `CommandsView.vue`：Commands 页面组合。
- `RegisterCommandPanel.vue`：脚本选择、命令名输入、路径预览、注册操作。
- `PathStatusPanel.vue`：PATH 状态和复制引导。
- `CommandList.vue`：表格 / 列表、空状态、行操作。
- `ConfirmDialog.vue`：共享确认弹窗。
- `ToastRegion.vue`：短暂成功 / 错误提示。

如果第一轮实现追求速度，可以先写在 `App.vue` 中；当行为变多后再拆成组件。

## 后端接口假设

UI 后续可以调用以下 Tauri command：

- `get_platform_info() -> PlatformInfo`
- `check_path_status() -> PathStatus`
- `list_commands() -> Vec<RegisteredCommand>`
- `register_command(script_path, command_name) -> RegisteredCommand`
- `delete_command(command_name) -> DeleteResult`
- `reveal_command(command_name) -> RevealResult`

前端预期数据类型：

```ts
type PathState = "checking" | "ok" | "missing" | "error";

type PathStatus = {
  state: PathState;
  binDir: string;
  message?: string;
  suggestedCommand?: string;
};

type RegisteredCommand = {
  name: string;
  scriptPath: string;
  entryPath: string;
  entryType: "symlink" | "wrapper" | "cmd-shim" | "ps1-shim";
  createdAt: string;
};
```

## 第一版非目标

- 不实现 ADB 工作流。
- 不自动编辑 shell profile 或系统 PATH。
- 不实现命令覆盖。
- 不从 GUI 中执行已注册命令。
- 不做营销落地页。
- 核心注册功能完成前，不增加复杂设置页。

## 验收标准

- 第一屏就是实际可用的 Commands 工作区。
- 用户能清楚知道在哪里选择脚本、输入命令名并注册。
- PATH 状态无需进入设置即可看到。
- 已注册命令可见且可删除。
- ADB 只作为计划中的导航入口存在。
- 界面观感符合实用的桌面开发者工具。
- 布局能用 Vue 3 清晰实现，并在后续接入 Tauri command。
