# JC9 UI 组件库（可移植）

一套**零业务依赖、可整体复制到任意 Vue 3 项目**的轻量 UI 原语层，设计规范对齐 [Ant Design](https://ant.design/docs/spec/introduce-cn)（色彩 / 字体 / 布局 / 暗黑模式 / 阴影 / 全局规则）。

> 解决 JC9 历史问题：UI 不成体系、复用率低（如 32 个工具页重复手写 `tool-container/tool-header/tool-btn/editor-pane`）。

## ✨ 特性

- 🧱 **原子组件层**：`JcButton / JcInput / JcTextarea / JcSelect / JcSegmented / JcBadge / JcEmpty`
- 🪟 **浮层组件**：`JcModal / JcContextMenu / JcToast`
- 🧩 **分子组件**：`ToolShell`（工具页统一外壳，一次收编 32 个工具页）
- 🎨 **三层设计 Token**（对齐 antd Seed/Map/Alias）：色彩、字阶、8px 间距、圆角、控件高度、三级阴影
- 🌗 **暗黑模式**：`data-theme` 一键切换 + `useJcTheme()` + 系统偏好跟随
- 🔌 **零业务依赖**：仅 Vue 3 + TypeScript，不引 stores / Tauri / Pinia，可直接搬到其他项目

## 📁 目录

```text
src/components/ui/
  tokens.scss        # 设计 Token（Seed/Map/Alias，暗色默认 + 亮色覆盖 + 系统偏好）
  theme.ts           # 主题切换 composable
  toast.ts           # Toast 状态 + 命令式 API
  index.ts           # 桶导出
  JcButton.vue       # 对齐 antd Button：type/danger/ghost/block/loading/htmlType/size/shape
  JcInput.vue        # size: large|middle|small，clearable
  JcTextarea.vue     # mono 等宽 / resize
  JcSelect.vue       # options + placeholder
  JcSegmented.vue    # 对齐 antd Segmented（收编 toggle-group/tab-btn）
  JcBadge.vue        # 对齐 antd Badge：count/max/dot/status/text
  JcEmpty.vue        # 空状态
  JcModal.vue        # 对齐 antd Modal：open/title/footer/maskClosable/confirmLoading/onOk/onCancel
  JcContextMenu.vue  # 右键菜单（溢出视口自动翻转）
  JcToast.vue        # 顶部消息条
  ToolShell.vue      # 工具页外壳（header + actions + 左右分栏）
```

## 🚀 快速开始

### 在 JC9 中使用

```ts
import { JcButton, JcModal, toast, useJcTheme } from '@/components/ui'
```

JC9 已有自己的 `--jc-*` 变量（`styles/variables.scss`），组件直接消费这些变量；新增的字阶/间距/圆角/阴影 Token 通过组件内 `var(--jc-*, fallback)` 兜底，无需额外配置。

### 在任意其他 Vue 3 项目中使用

```ts
// 1. 复制整个 ui/ 目录到你的项目
// 2. 引入默认主题（暗色），亮色用 <html data-theme="light">
import '@/components/ui/tokens.scss'
// 3. 按需引入
import { JcButton, JcModal } from '@/components/ui'
```

## 🎨 设计 Token（对齐 Ant Design）

参照 antd Design Token 三层模型：

| 层级 | 说明 | 例子 |
| --- | --- | --- |
| **Seed** | 品牌/功能色、字阶、间距、圆角、控件高度、阴影 | `--jc-color-accent` `--jc-font-size` `--jc-space-xs` `--jc-radius` `--jc-control-height` `--jc-shadow-1/2/3` |
| **Map** | 背景/文本/边框层级 | `--jc-bg-app/panel/elevated` `--jc-text-primary/secondary` `--jc-border-*` |
| **Alias** | 组件消费的语义变量 | `--jc-shadow-menu` `--jc-shadow-modal` |

- 字阶：`--jc-font-size-xs/sm/base/lg/xl/2xl/3xl/4xl`，主字体 13（antd 主字 14）
- 间距：8px 网格模度 `--jc-space-xxs/xs/sm/base/lg/xl/xxl`
- 阴影：三层表达（antd shadow-1/2/3），菜单=2 层、弹窗=3 层
- 数字：`font-variant-numeric: tabular-nums`（等宽数字）

## 🌗 主题切换

组件全部通过 CSS 变量消费主题，`data-theme` 是唯一数据源：

```ts
// 手动切换
import { applyJcTheme, useJcTheme } from '@/components/ui'

applyJcTheme('light')               // 或 'dark'
const { isDark, toggle } = useJcTheme()  // 组件内响应式使用
```

- 暗色为默认（`:root`），亮色由 `[data-theme='light']` 覆盖
- 未显式指定时自动跟随 `prefers-color-scheme`
- 与 JC9 现有 `TitleBar` 的 `data-theme` 机制天然兼容（都操作同一个 DOM 属性）

## 📋 组件 API 摘要

### JcButton（对齐 antd Button）

| 属性 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- |
| type | `primary\|default\|dashed\|text\|link` | `default` | 按钮类型 |
| danger | `boolean` | `false` | 危险按钮 |
| ghost | `boolean` | `false` | 幽灵按钮（透明背景） |
| block | `boolean` | `false` | 撑满父宽 |
| loading | `boolean` | `false` | 加载态 |
| disabled | `boolean` | `false` | 禁用 |
| size | `large\|middle\|small` | `middle` | 尺寸 |
| shape | `default\|round\|circle` | `default` | 形状 |
| htmlType | `button\|submit\|reset` | `button` | 原生 type |

### JcModal（对齐 antd Modal）

`open` / `title` / `width` / `footer` / `closable` / `mask` / `maskClosable` / `confirmLoading` / `zIndex`，事件 `update:open` `ok` `cancel`。遮罩关闭策略：`@mousedown.self`（输入框内拖选越界不会误关）。

### JcToast

```ts
import { toast } from '@/components/ui'
// App 根部挂载一次
<JcToast />
toast.success('保存成功')  // success/error/warning/info
```

### ToolShell

```vue
<ToolShell title="Base64 转换器" split>
  <template #actions><JcButton type="primary">转换</JcButton></template>
  <template #left-label>原始内容</template>
  <template #left><JcTextarea v-model="input" class="jc-fill" mono /></template>
  <template #right-label>结果</template>
  <template #right><JcTextarea v-model="output" class="jc-fill" mono readonly /></template>
</ToolShell>
```

## 🧰 移植到其他项目（Checklist）

- [ ] 复制 `src/components/ui/` 整个目录
- [ ] `import '@/components/ui/tokens.scss'`（需要默认主题时）
- [ ] 可选：在 `package.json` 增加 `vue@^3.5`、`typescript`
- [ ] 覆盖 `--jc-color-accent` 等 Seed Token 即完成换肤

## 🗺️ 后续规划

- [ ] `JcDropdown`、`JcTooltip`、`JcDrawer`、`JcTree`
- [ ] 用 `ToolShell` 收编现有 32 个工具页
- [ ] 独立发布为 npm 包 / 提取到 monorepo `packages/ui`
