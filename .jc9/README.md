# JC9 AI 知识库

本目录包含 JC9 项目的完整 AI 知识体系，供开发期 AI 和运行期 Agent 使用。

## 读取顺序

AI 在启动或第一次操作本项目时，按以下顺序阅读：

```
1. README.md          ← 本文件，先了解结构
2. general.md         ← 核心规则和避坑指南（最重要）
3. architecture.md    ← 系统架构总览
4. database.md        ← 数据库规范
5. frontend.md        ← 前端编码规范
6. backend.md         ← 后端编码规范
7. ai-agent.md        ← AI Agent 架构
```

## 文件索引

| 文件 | scope | 用途 |
|------|-------|------|
| `README.md` | dev | 知识库入口和读取指引 |
| `general.md` | dev | 核心规则、避坑指南、搜索规则 |
| `architecture.md` | dev | 系统架构图和模块说明 |
| `database.md` | dev | 数据库表结构、规范、迁移流程 |
| `frontend.md` | dev | Vue 3 / TypeScript 编码规范 |
| `backend.md` | dev | Rust / Tauri 编码规范 |
| `ai-agent.md` | dev | AI Agent 20 模块架构和工具注册 |

## Skills（运行期 Agent 可加载）

| 文件 | scope | 用途 |
|------|-------|------|
| `skills/git-workflow/SKILL.md` | runtime | Git 分支策略和提交规范 |
| `skills/tauri-development/SKILL.md` | both | Tauri 2 开发指南 |
| `skills/sqlite-optimization/SKILL.md` | both | SQLite 优化指南 |
| `skills/vector-search/SKILL.md` | both | 向量搜索工作原理 |

## Workflows（开发期使用）

| 文件 | scope | 用途 |
|------|-------|------|
| `workflows/add-new-tool.md` | dev | 添加 Agent 工具完整流程 |
| `workflows/add-new-command.md` | dev | 添加 Tauri command 完整流程 |
| `workflows/database-migration.md` | dev | 数据库迁移完整流程 |
