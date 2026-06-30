# JC9 前端规范

## 技术选型

| 项目 | 规范 |
|------|------|
| 框架 | Vue 3 + Composition API |
| 组件风格 | `<script setup lang="ts">` |
| 状态管理 | Pinia |
| 样式 | SCSS |
| 构建 | Vite |
| 包管理 | npm |
| 类型检查 | `vue-tsc --noEmit` |

## 组件规范

### 组件结构

```vue
<script setup lang="ts">
import { ref, computed } from 'vue'
import { useYourStore } from '@/stores/your'

// Props 类型定义
interface Props {
  itemId: string
  title?: string
}
const props = withDefaults(defineProps<Props>(), {
  title: '默认标题'
})

// Emits
const emit = defineEmits<{
  update: [id: string]
  delete: [id: string]
}>()

// Store
const store = useYourStore()

// 响应式状态
const loading = ref(false)

// 方法
async function handleAction() {
  loading.value = true
  try {
    await store.doSomething()
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="component-name">
    <h2>{{ title }}</h2>
  </div>
</template>

<style scoped lang="scss">
.component-name {
  // 使用 variables.scss 中的变量
}
</style>
```

### 命名规范

| 元素 | 规范 | 示例 |
|------|------|------|
| 组件文件 | PascalCase | `CommandDialog.vue` |
| 组件名 | PascalCase | `CommandDialog` |
| 目录 | kebab-case | `ai/`, `notes/`, `tools/` |
| Props | camelCase | `itemId`, `groupName` |
| Emits | camelCase | `update`, `deleteNote` |
| CSS 类 | kebab-case | `.note-card`, `.status-bar` |
| Pinia Store | `useXxxStore` | `useNotesStore` |

### 组件职责

每个组件只做一件事。如果一个组件超过 300 行，拆分子组件。

## Pinia Store 规范

```typescript
// stores/notes.ts
import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

export const useNotesStore = defineStore('notes', () => {
  // State
  const notes = ref<Note[]>([])
  const loading = ref(false)

  // Getters
  const pinnedNotes = computed(() => notes.value.filter(n => n.isPinned))

  // Actions
  async function fetchNotes(groupId?: string) {
    loading.value = true
    try {
      notes.value = await invoke<Note[]>('get_notes', { groupId })
    } finally {
      loading.value = false
    }
  }

  return { notes, loading, pinnedNotes, fetchNotes }
})
```

## Tauri invoke 调用规范

```typescript
// 始终使用泛型指定返回类型
const result = await invoke<string[]>('some_command', { param1: 'value' })

// 错误处理
try {
  await invoke('save_note', { note })
} catch (err) {
  console.error('保存失败:', err)
}
```

## 样式规范

### 使用全局变量

```scss
// 从 variables.scss 导入
@use '@/styles/variables' as *;

.component {
  background: $bg-primary;
  color: $text-primary;
  border: 1px solid $border-color;
  border-radius: $border-radius;
}
```

### 组件内样式

- 始终使用 `<style scoped lang="scss">`
- 不写全局样式，除非在 `global.scss` 中
- 使用 flexbox/grid 布局，避免 float
