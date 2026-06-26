# jc9 (Qidong) - 极简项目与终端管理器

`jc9` 是一个专为开发者设计的现代、精致且高效的跨平台项目与终端管理器。基于 Rust Tauri v2 与 Vue 3 技术栈构建，通过集成底层的 portable-pty 支持，提供极致流畅且多窗口的命令行终端开发体验。

## ✨ 特性

- 📂 **智能项目感知**：自动扫描并检测 Go、Node.js / Vue / Tauri、Rust 等项目，智能匹配与推荐常用的编译及运行指令。
- ⚡ **全局快捷命令**：提供高频、常用与收藏命令的分类检索与一键调用，支持多平台文档在线查询。
- 🖥️ **ConPTY/PTY 底层集成**：与 VS Code 和 WezTerm 同级的底层伪终端实现，无缝适配各种 CLI 交互。
- 📊 **智能日志分析**：流式解码分析终端输出，实时汇总匹配 `ERROR`、`WARN`、`DEBUG` 及 `INFO` 等级别日志数据。
- 🎨 **双色主题系统**：自带高级黑金暗色主题及明亮白色主题，支持窗口拖拽及与系统高度同步的精细化无边框 TitleBar。

## 🛠️ 技术栈

- **前端核心**：Vue 3 + TypeScript + Pinia
- **终端渲染**：@xterm/xterm + @xterm/addon-fit
- **后端框架**：Rust + Tauri v2
- **终端桥接**：portable-pty

## 🚀 开始使用

### 前期准备

确保您的本地机器安装了以下环境：
- Node.js (>= 18.0.0)
- Rust 工具链 (Cargo, rustc)

### 安装依赖

在项目根目录下执行以下命令：

```bash
npm install
```

### 开发环境启动

启动开发模式，Tauri 将在后台编译 Rust 模块并唤起前端热重载窗口：

```bash
npm run dev
```

或者使用 Tauri CLI 直接开发：

```bash
npx tauri dev
```

### 构建生产包

打包生成高度压缩的可执行应用：

```bash
npm run build
```

或者：

```bash
npx tauri build
```
