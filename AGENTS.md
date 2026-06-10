# AGENTS.md

## 沟通规则

- 默认使用中文回答用户；除非用户明确要求其他语言。
- 回答要直接、具体，优先给出可执行结论和必要上下文。
- 涉及代码、命令、文件路径时，使用准确的文件名和命令，不要含糊描述。
- 如果无法完成某一步，说明阻塞原因、已尝试的操作，以及建议的下一步。
- 页面里展示出来的文字需要是中文

## 项目概览

这是一个 Tauri 2 + Vue 3 + TypeScript + Vite 项目，Rust 代码位于 `src-tauri/`，前端代码位于 `src/`。

主要目录：

- `src/`：Vue 前端应用入口和组件。
- `src-tauri/`：Tauri/Rust 后端、配置、能力声明和应用图标。
- `public/`：Vite 静态资源。
- `docs/superpowers/`：设计规格和实现计划文档。

关键文件：

- `package.json`：前端脚本和 npm 依赖。
- `vite.config.ts`：Vite 配置。
- `src-tauri/Cargo.toml`：Rust crate 和 Tauri 依赖。
- `src-tauri/tauri.conf.json`：Tauri 应用配置。
- `src-tauri/capabilities/default.json`：Tauri 权限能力配置。

## 常用命令

- 安装前端依赖：`npm install`
- 启动前端开发服务器：`npm run dev`
- 构建前端：`npm run build`
- 预览前端构建结果：`npm run preview`
- 启动 Tauri 开发应用：`npm run tauri dev`
- 构建 Tauri 应用：`npm run tauri build`
- Rust 格式化：`cargo fmt --manifest-path src-tauri/Cargo.toml`
- Rust 检查：`cargo check --manifest-path src-tauri/Cargo.toml`

如果当前环境缺少依赖或命令失败，先报告具体错误，再决定是否需要安装依赖或调整环境。

## 代码约定

- 前端使用 Vue 3 `<script setup lang="ts">` 和 TypeScript。
- 保持组件状态、事件处理和模板结构清晰；避免把无关逻辑堆在同一个组件里。
- Rust 代码遵循 `rustfmt` 默认格式，优先使用明确的类型和可读的错误处理。
- Tauri 命令应保持参数和返回值可序列化，跨前后端调用时同步更新 TypeScript 调用方。
- 修改 Tauri 权限、插件或窗口能力时，同步检查 `src-tauri/capabilities/` 和 `tauri.conf.json`。
- 不要无故引入新依赖；确实需要时说明用途，并优先选择项目生态内成熟、维护良好的库。
- 根据功能进行页面或组件拆分
- 后端代码要根据功能进行拆分

## 变更边界

- 保持改动聚焦用户请求，不做无关重构。
- 不要删除或覆盖用户已有改动，除非用户明确要求。
- 不要修改生成物、图标、锁文件或配置文件，除非这些文件与任务直接相关。
- 对 `src-tauri/gen/` 下的 schema 文件保持谨慎，通常不手动编辑。
- 涉及安全、权限、文件系统访问、命令执行、网络访问时，明确说明影响范围。

## 文档维护

- 文档应使用简洁中文，命令和代码标识保留英文原文。
- 当项目脚本、目录结构或技术栈变化时，同步更新本文件。
- 新增重要工作流时，把可复用规则写入本文件，而不是只写在一次性回复中。

## 提交
- 用户明确要提交的时候才进行提交
- 提交消息主内容必须是中文
- tag 号必须递增，不允许强制推送
