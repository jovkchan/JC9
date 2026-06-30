---
name: vector-search
description: 语义搜索（向量搜索）的工作原理和使用方式
type: skill
scope: both
trigger: 涉及语义搜索、向量检索或知识库搜索时
version: 1
---

# 向量搜索指南

## 工作原理

JC9 的语义搜索有两层：

```
用户搜索 "文件上传功能"
    │
    ▼
┌─ VectorStore ──────────────────────────┐
│  1. generate_embedding("文件上传功能")  │
│     ├─ 有 OPENAI_API_KEY → 调用 API    │
│     └─ 无 API Key → SHA-256 哈希降级   │
│                                        │
│  2. search(embedding, limit, threshold) │
│     ├─ sqlite-vec 已加载 → KNN 虚拟表  │
│     └─ 未加载 → 纯 Rust 余弦相似度遍历 │
└─────────────────────────────────────────┘
    │
    ▼
返回匹配结果 (id, score, content)
```

## 哪些内容可被搜索

| 来源 | 入口 | ID 格式 |
|------|------|--------|
| 用户笔记 | `save_note()` | `note_{id}` |
| AI Takeaways | `Summarizer` | UUID |
| 手动添加 | `ai_add_knowledge` | 自定义 |
| 技能文件 | `skill_loader.rs` | `skill_{name}` |

## 切换 Embedding 模型

默认使用 OpenAI `text-embedding-3-small`（1536 维）。

设置环境变量切换：

```bash
# 使用 OpenAI
export OPENAI_API_KEY=sk-xxx

# 使用兼容 API（如阿里云、DeepSeek）
export OPENAI_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
export OPENAI_API_KEY=sk-xxx
```

无 API Key 时自动使用 SHA-256 哈希降级向量，搜索效果会差很多。

## 故障排除

| 症状 | 原因 | 解决 |
|------|------|------|
| 搜索结果为空 | 知识库为空 | 等待笔记同步或手动添加 |
| 搜索效果差 | 使用哈希降级 | 设置 OPENAI_API_KEY |
| sqlite-vec 未加载 | vec0.dll 不存在 | 运行 install-sqlite-vec.ps1 |
| 搜索慢 | 数据量大且无 sqlite-vec | 安装 vec0.dll 加速 |

## 检查状态

```bash
# 通过 Tauri command 检查
invoke('ai_vec_status')  # 返回 true/false
```