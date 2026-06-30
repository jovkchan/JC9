---
name: git-workflow
description: JC9 项目的 Git 分支策略和提交规范
type: skill
scope: runtime
trigger: 用户要求进行 Git 操作时
version: 1
---

# Git 工作流

## 分支策略

| 分支 | 用途 | 来源 |
|------|------|------|
| `main` | 稳定版本 | 保护分支，只能通过 PR 合并 |
| `feature/*` | 新功能开发 | 从 `main` 创建 |
| `fix/*` | Bug 修复 | 从 `main` 创建 |
| `release/*` | 发布准备 | 从 `main` 创建 |

## 提交流程

```bash
# 1. 查看当前状态
git status

# 2. 暂存变更
git add <file>     # 指定文件
git add .          # 全部（谨慎使用）

# 3. 提交
git commit -m "类型: 简短描述"

# 4. 拉取最新
git pull --rebase

# 5. 推送
git push
```

## Commit Message 格式

```
<类型>: <描述>

类型: feat | fix | refactor | docs | chore | style | test
示例: feat: 添加向量搜索技能同步功能
示例: fix: 修复笔记同步 ID 冲突问题
```

## PR 流程

1. 确保分支名符合规范
2. 运行 `cargo check` + `vue-tsc --noEmit`
3. 创建 PR 到 `main`
4. 等待审查通过后合并