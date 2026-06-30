---
name: tauri-development
description: Tauri 2 开发指南，包括项目结构、配置修改、插件添加
type: skill
scope: both
trigger: 需要修改 Tauri 配置或添加 Tauri 功能时
version: 1
---

# Tauri 开发指南

## 项目结构

```
src-tauri/
├── Cargo.toml          # Rust 依赖
├── tauri.conf.json     # Tauri 配置（窗口、权限、bundle）
├── capabilities/       # 权限声明
│   ├── default.json    # 默认权限
│   └── splash.json     # 闪屏权限
├── icons/              # 应用图标
├── src/
│   ├── main.rs         # 入口
│   ├── lib.rs          # 所有 Tauri commands 和 AppState
│   ├── database.rs     # SQLite 数据库
│   ├── process.rs      # 终端进程
│   └── ai/             # AI Agent 子系统
└── build.rs            # 构建脚本
```

## 修改 Tauri 配置

`tauri.conf.json` 中的关键配置：

| 字段 | 作用 |
|------|------|
| `app.windows` | 窗口大小、标题、装饰 |
| `app.security` | CSP 安全策略 |
| `bundle` | 打包配置（identifier、icon、categories） |

## 添加 Tauri Plugin

```toml
# 1. Cargo.toml 添加依赖
[dependencies]
tauri-plugin-foo = "2"
```

```rust
// 2. lib.rs 注册插件
.plugin(tauri_plugin_foo::init())
```

```json
// 3. capabilities/default.json 添加权限
{
  "permissions": ["foo:default"]
}
```

## 调试技巧

- 前端日志：`console.log()` → DevTools Console 查看
- 后端日志：`println!()` → 终端输出
- Tauri 日志：`tauri::api::dialog::message()` → 系统弹窗
- 运行：`cargo tauri dev`（热更新前端）
- 构建：`cargo tauri build`（生产版本）