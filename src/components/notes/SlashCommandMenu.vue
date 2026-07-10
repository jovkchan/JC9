<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Note } from '@/types/notes'

const props = defineProps<{
  visible: boolean
  editorRect: { top?: number; bottom?: number; left: number; flipUp?: boolean } | null
}>()

const emit = defineEmits<{
  close: []
  insertLink: [noteId: string, noteTitle: string]
  command: [cmdId: string]
}>()

// ── State ──
type Stage = 'commands' | 'search'
const stage = ref<Stage>('commands')
const searchQuery = ref('')
const searchResults = ref<Note[]>([])
const searching = ref(false)
const selectedIdx = ref(0)
const menuRef = ref<HTMLElement | null>(null)

// ── Commands ──
const commands = [
  { id: 'link', label: '链接笔记', desc: '插入一个笔记链接，点击可跳转' },
  { id: 'table', label: '插入表格', desc: '插入 3x3 表格' },
  { id: 'code', label: '代码块', desc: '插入代码块' },
  { id: 'hr', label: '分割线', desc: '插入水平分割线' },
  { id: 'task', label: '任务列表', desc: '插入任务列表' },
]

// Reset on close
watch(() => props.visible, (v) => {
  if (!v) {
    stage.value = 'commands'
    searchQuery.value = ''
    searchResults.value = []
    selectedIdx.value = 0
  }
})

// ── Search ──
let searchTimer: ReturnType<typeof setTimeout> | null = null
function doSearch(q: string) {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(async () => {
    const trimmed = q.trim()
    if (!trimmed) {
      searchResults.value = []
      return
    }
    searching.value = true
    try {
      const results = await invoke<Note[]>('search_notes', { query: trimmed })
      searchResults.value = results.slice(0, 10)
      selectedIdx.value = 0
    } catch {
      searchResults.value = []
    } finally {
      searching.value = false
    }
  }, 150)
}

function onSearchInput(e: Event) {
  const value = (e.target as HTMLInputElement).value
  searchQuery.value = value
  doSearch(value)
}

function selectCommand(cmd: typeof commands[0]) {
  switch (cmd.id) {
    case 'link':
      stage.value = 'search'
      nextTick(() => {
        const input = menuRef.value?.querySelector('.slash-search-input') as HTMLInputElement
        input?.focus()
      })
      break
    case 'table':
    case 'code':
    case 'hr':
    case 'task':
      emit('command', cmd.id)
      emit('close')
      break
  }
}

function selectNote(note: Note) {
  emit('insertLink', note.id, note.title || '无标题')
  emit('close')
}

// ── Keyboard ──
function handleMenuKeydown(e: KeyboardEvent) {
  const items = stage.value === 'commands' ? commands : searchResults.value
  if (e.key === 'ArrowDown') { e.preventDefault(); selectedIdx.value = Math.min(selectedIdx.value + 1, items.length - 1) }
  if (e.key === 'ArrowUp') { e.preventDefault(); selectedIdx.value = Math.max(selectedIdx.value - 1, 0) }
  if (e.key === 'Enter') {
    e.preventDefault()
    if (stage.value === 'commands') {
      if (commands[selectedIdx.value]) selectCommand(commands[selectedIdx.value])
    } else {
      if (searchResults.value[selectedIdx.value]) selectNote(searchResults.value[selectedIdx.value])
    }
  }
  if (e.key === 'Escape') {
    if (stage.value === 'search') { stage.value = 'commands'; searchQuery.value = ''; searchResults.value = [] }
    else emit('close')
  }
  if (e.key === 'Backspace' && stage.value === 'search' && !searchQuery.value) {
    stage.value = 'commands'
    searchResults.value = []
  }
}

// Click outside
function handleClickOutside(e: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    emit('close')
  }
}

onMounted(() => document.addEventListener('mousedown', handleClickOutside))
onBeforeUnmount(() => document.removeEventListener('mousedown', handleClickOutside))
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible && editorRect"
      ref="menuRef"
      class="slash-menu"
      :style="{
        position: 'fixed',
        top: editorRect.flipUp ? 'auto' : editorRect.top + 'px',
        bottom: editorRect.flipUp ? editorRect.bottom + 'px' : 'auto',
        left: editorRect.left + 'px',
        zIndex: 9999,
      }"
      @keydown="handleMenuKeydown"
    >
      <!-- Stage 1: Commands -->
      <template v-if="stage === 'commands'">
        <div class="slash-menu-header">命令</div>
        <div
          v-for="(cmd, i) in commands"
          :key="cmd.id"
          :class="['slash-menu-item', { selected: i === selectedIdx }]"
          @click="selectCommand(cmd)"
          @mouseenter="selectedIdx = i"
        >
          <span class="slash-item-label">{{ cmd.label }}</span>
          <span class="slash-item-desc">{{ cmd.desc }}</span>
        </div>
      </template>

      <!-- Stage 2: Note Search -->
      <template v-if="stage === 'search'">
        <div class="slash-search-bar">
          <input
            v-model="searchQuery"
            class="slash-search-input"
            placeholder="搜索笔记..."
            @input="onSearchInput"
          />
          <span v-if="searching" class="slash-searching">搜索中...</span>
        </div>
        <div v-if="searchResults.length === 0 && searchQuery.trim() && !searching" class="slash-empty">
          未找到匹配的笔记
        </div>
        <div
          v-for="(note, i) in searchResults"
          :key="note.id"
          :class="['slash-menu-item', { selected: i === selectedIdx }]"
          @click="selectNote(note)"
          @mouseenter="selectedIdx = i"
        >
          <span class="slash-item-label">📄 {{ note.title || '无标题' }}</span>
          <span class="slash-item-desc">{{ note.id.slice(0, 8) }}...</span>
        </div>
      </template>
    </div>
  </Teleport>
</template>

<style scoped>
.slash-menu {
  background: var(--jc-bg-elevated, #252526);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: 6px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.4);
  min-width: 280px;
  max-width: 360px;
  max-height: 320px;
  overflow-y: auto;
  font-size: 12px;
}
.slash-menu-header {
  padding: 4px 10px;
  color: var(--jc-text-secondary, #888);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.slash-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  cursor: pointer;
  border-left: 2px solid transparent;
}
.slash-menu-item.selected {
  background: rgba(88, 166, 255, 0.1);
  border-left-color: #58a6ff;
}
.slash-item-label {
  color: var(--jc-text-primary, #ccc);
  white-space: nowrap;
}
.slash-item-desc {
  color: var(--jc-text-secondary, #666);
  font-size: 10px;
  margin-left: auto;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.slash-search-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
}
.slash-search-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--jc-text-primary, #ccc);
  font-size: 12px;
  outline: none;
  padding: 4px 0;
}
.slash-search-input::placeholder {
  color: var(--jc-text-secondary, #666);
}
.slash-searching {
  color: var(--jc-text-secondary, #666);
  font-size: 10px;
  white-space: nowrap;
}
.slash-empty {
  padding: 12px 10px;
  color: var(--jc-text-secondary, #666);
  text-align: center;
}
</style>
