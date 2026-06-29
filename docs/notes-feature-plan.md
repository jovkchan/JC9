# JC9 笔记功能 — 完整规划方案

> 目标：在 JC9 中实现「分组 + 新建 + 即时保存」的本地笔记功能，后续扩展网络同步。

---

## 一、设计原则

| 原则 | 说明 |
|------|------|
| **零摩擦** | 打开即写，自动保存，无需手动点保存按钮 |
| **分组清晰** | 树形笔记组，类似文件夹，可嵌套 |
| **与 JC9 风格一致** | 沿用现有 VS Code 风格暗色主题、组件模式、存储方案 |
| **渐进增强** | 本地 MVP 先行，网络同步作为第二阶段 |

---

## 二、数据模型

### NoteGroup（笔记组）

```typescript
interface NoteGroup {
  id: string;          // UUID
  name: string;        // 组名
  parentId: string | null;  // 父组 ID，null 为根组
  sortOrder: number;   // 排序序号
  createdAt: string;   // ISO 时间
}
```

### Note（笔记）

```typescript
interface Note {
  id: string;          // UUID
  groupId: string;     // 所属组 ID
  title: string;       // 标题
  content: string;     // 内容（纯文本 / Markdown）
  format: 'plain' | 'markdown';  // 格式
  isPinned: boolean;   // 是否置顶
  sortOrder: number;   // 排序序号
  createdAt: string;   // ISO 时间
  updatedAt: string;   // ISO 时间
}
```

### 存储文件

- 文件路径：`%APPDATA%/jc9/jc9-notes.json`
- 结构：

```json
{
  "groups": [ /* NoteGroup[] */ ],
  "notes": [ /* Note[] */ ]
}
```

> 沿用现有 `storage.rs` 的 JSON 文件方案，与 `jc9-projects.json` 同级，简单可靠，免 SQLite 依赖。

---

## 三、UI 架构

### 布局方案

```
┌─────────────────────────────────────────────────────┐
│  TitleBar                                            │
├────────────┬────────────────────────────────────────┤
│ Sidebar    │  MainPanel                              │
│            │                                         │
│ [项目]     │  ┌─ TabBar ──────────────────────┐     │
│ [快捷]     │  │ [笔记: 我的日记] ✕           │     │
│ [工具]     │  ├───────────────────────────────┤     │
│ [笔记] ←新 │  │                               │     │
│            │  │  ✏️ 标题输入框               │     │
│ ─────────  │  │  ─────────────────────────    │     │
│ 工作笔记 ▸ │  │                               │     │
│   今日待办  │  │  📝 内容编辑区              │     │
│   技术笔记 ▸│  │  (textarea，自动保存)       │     │
│   API备忘   │  │                               │     │
│   个人 ▸    │  │                               │     │
│ ─────────  │  │                               │     │
│ [+] 新建组  │  │                               │     │
│            │  │                               │     │
└────────────┴─────────────────────────────────────────┘
```

### 左侧边栏 — 新增「笔记」Tab

在 `ProjectSidebar.vue` 的现有的 `projects / shortcuts / tools` 三个 tab 旁，新增第 4 个 tab：**笔记**。

左侧展示：
- **笔记组树**：可展开/折叠的树形结构，支持拖拽排序
- **笔记列表**：当前选中组下的笔记列表（按置顶 + 更新时间排序）
- **右键菜单**：新建组、重命名组、删除组、新建笔记

### 右侧主面板 — 笔记编辑器

通过 ToolTab 方式打开（复用现有 tab 体系）：

- **工具栏**：标题输入框 + 格式切换（纯文本 / Markdown）+ 置顶按钮 + 删除按钮
- **编辑区**：`<textarea>` 或 `<div contenteditable>` 实现
- **自动保存**：输入时 500ms 防抖自动写入后端
- **Markdown 预览**（可选）：在 Markdown 模式下显示预览分栏

---

## 四、Rust 后端新增命令

在 `src-tauri/src/lib.rs` 中新增以下 Tauri 命令：

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_note_groups` | 无 | `Vec<NoteGroup>` | 获取所有笔记组 |
| `save_note_groups` | `groups: Vec<NoteGroup>` | `Result<(), String>` | 保存笔记组 |
| `get_notes` | `groupId?: string` | `Vec<Note>` | 获取笔记列表（可选按组过滤） |
| `save_note` | `note: Note` | `Result<(), String>` | 创建/更新笔记 |
| `delete_note` | `id: string` | `Result<(), String>` | 删除笔记 |
| `delete_note_group` | `id: string` | `Result<(), String>` | 删除笔记组（含组内笔记） |

参考 `storage.rs` 的现有模式，新增 `src-tauri/src/notes_storage.rs` 模块。

---

## 五、前端新增文件

```
src/
  stores/
    notes.ts              ← 新增 Pinia Store：笔记状态管理
  components/
    notes/
      NoteSidebar.vue     ← 左侧笔记面板（组树 + 笔记列表）
      NoteEditor.vue      ← 右侧笔记编辑器（标题 + 内容 + 自动保存）
```

### store/notes.ts 职责

- 加载/保存笔记组和笔记数据
- 当前选中的组/笔记
- 自动保存防抖动逻辑
- CRUD 操作

### NoteSidebar.vue 组件

- 组树展示（递归组件）
- 笔记列表
- 右键菜单（新建组/重命名/删除）
- 拖拽排序（可选增强）

### NoteEditor.vue 组件

- 标题输入
- 内容编辑区（textarea）
- 自动保存（500ms 防抖）
- 格式切换（plain/markdown）
- 置顶/删除操作
- 底部状态栏：字数统计 + 最后保存时间

---

## 六、实现步骤

### 第一阶段：本地笔记 MVP（预计 4-6 小时）

| 步骤 | 内容 | 涉及文件 |
|------|------|----------|
| 1 | Rust: 新增 `notes_storage.rs`，定义数据结构 + 读写函数 | `src-tauri/src/notes_storage.rs` |
| 2 | Rust: 注册 6 个 Tauri 命令到 `lib.rs` | `src-tauri/src/lib.rs` |
| 3 | 前端: 新增 `stores/notes.ts` Pinia Store | `src/stores/notes.ts` |
| 4 | 前端: 新增 `NoteSidebar.vue` 左侧面板 | `src/components/notes/NoteSidebar.vue` |
| 5 | 前端: 新增 `NoteEditor.vue` 编辑器（含自动保存） | `src/components/notes/NoteEditor.vue` |
| 6 | 前端: 在 `ProjectSidebar.vue` 新增「笔记」tab | `src/components/ProjectSidebar.vue` |
| 7 | 前端: 在 `MainPanel.vue` 注册笔记 tab 渲染 | `src/components/MainPanel.vue` |
| 8 | 测试: 创建组 → 新建笔记 → 输入内容 → 关闭重开验证数据持久化 | — |

### 第二阶段：体验增强（可选，MVP 后）

| 功能 | 说明 |
|------|------|
| Markdown 预览 | 支持 Markdown 格式编辑 + 实时预览 |
| 搜索笔记 | 在笔记侧栏添加搜索框，按标题/内容全文搜索 |
| 拖拽排序 | 笔记组和笔记支持拖拽调整顺序 |
| 导出 | 导出为 .md / .txt / .json |
| 代码片段 | 笔记内嵌入代码块语法高亮 |

### 第三阶段：网络同步（远期）

| 功能 | 说明 |
|------|------|
| WebDAV 同步 | 支持 WebDAV 协议同步到 NextCloud/OwnCloud 等 |
| 自建服务同步 | JC9 自建同步服务端 |
| 冲突解决 | 简单的时间戳"最后写入胜出"策略，后续支持 3-way merge |
| 跨设备 | 多台电脑间笔记同步 |

---

## 七、自动保存机制

```
用户输入 ──→ 500ms 防抖 ──→ 调用 saveNote() ──→ invoke Rust ──→ 写 JSON 文件
                              ↑
                        状态栏显示"已保存" / "保存中..."
```

- 防抖时间：500ms（兼顾响应速度和写入频率）
- 关闭 tab 时：立即强制保存
- 应用关闭时：Tauri 的 `on_window_event` 中触发保存

---

## 八、与 OneNote / TXT 对比

| 维度 | OneNote | TXT 文件 | JC9 笔记 |
|------|---------|----------|----------|
| 启动速度 | 慢 | 快 | **极快**（常驻） |
| 分组能力 | 笔记本→分区→页 | 文件夹 | **树形组** |
| 即时记录 | 需打开应用 | 需找到文件 | **Alt+Tab 即写** |
| 自动保存 | ✅ | ❌ 需手动 Ctrl+S | ✅ 防抖自动保存 |
| 搜索 | ✅ | ❌ 需用 Everything | ✅ 内置搜索 |
| 网络同步 | ✅ OneDrive | ❌ | ⏳ 后续支持 |
| 功能复杂度 | 过于复杂 | 过于简单 | **恰到好处** |

---

## 九、技术关键点

### 自动保存防抖（TypeScript）

```typescript
// stores/notes.ts
let saveTimer: ReturnType<typeof setTimeout> | null = null

function scheduleSave(note: Note) {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(async () => {
    await invoke('save_note', { note })
    note.updatedAt = new Date().toISOString()
    lastSavedTime.value = Date.now()
  }, 500)
}
```

### Rust 数据文件格式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteGroup {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub group_id: String,
    pub title: String,
    pub content: String,
    pub format: String,  // "plain" | "markdown"
    pub is_pinned: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesData {
    pub groups: Vec<NoteGroup>,
    pub notes: Vec<Note>,
}
```
