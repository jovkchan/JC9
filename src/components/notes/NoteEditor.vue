<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useNotesStore } from '@/stores/notes'
import type { Note } from '@/types/notes'
import { MdEditor } from 'md-editor-v3'
import 'md-editor-v3/lib/style.css'

const store = useNotesStore()

const props = defineProps<{
  existingNote?: Note | null
}>()

const emit = defineEmits<{
  saved: [note: Note]
  cancel: []
}>()

const title = ref(props.existingNote?.title ?? '')
const content = ref(props.existingNote?.content ?? '')
const tagInput = ref(props.existingNote?.tags.join(', ') ?? '')
const saving = ref(false)
const lastSaved = ref('')

const editorTheme = ref<'light' | 'dark'>('dark')

// Track whether we're editing an existing note or creating new
const editNoteId = ref<string | null>(props.existingNote?.id ?? null)

let saveTimer: ReturnType<typeof setTimeout> | null = null

const tags = ref<string[]>([])
function syncTags() {
  tags.value = tagInput.value
    .split(/[,，]/)
    .map(t => t.trim())
    .filter(Boolean)
}

syncTags()

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(doSave, 500)
}

async function doSave() {
  if (!title.value.trim() && !content.value.trim()) return
  saving.value = true
  syncTags()

  if (editNoteId.value) {
    const existing = store.notes.find(n => n.id === editNoteId.value)
    if (!existing) { saving.value = false; return }

    const note: Note = {
      ...existing,
      title: title.value,
      content: content.value,
      format: 'markdown',
      tags: tags.value,
      updatedAt: new Date().toISOString(),
    }
    await store.saveNote(note)
    emit('saved', note)
  } else {
    const note = await store.createNote({
      title: title.value,
      content: content.value,
      format: 'markdown',
      tags: tags.value,
      groupId: store.selectedGroupId,
      visibility: 'PRIVATE',
    })
    if (note) {
      editNoteId.value = note.id
      emit('saved', note)
    }
  }

  lastSaved.value = new Date().toLocaleTimeString()
  saving.value = false
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    doSave()
  }
}

// Sync theme from document
function syncTheme() {
  const t = document.documentElement.getAttribute('data-theme')
  editorTheme.value = t === 'light' ? 'light' : 'dark'
}

onMounted(() => syncTheme())

// Watch theme changes
const observer = new MutationObserver(() => syncTheme())
onMounted(() => {
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
})
onUnmounted(() => observer.disconnect())

watch([title, tagInput], () => scheduleSave())

onUnmounted(() => {
  if (saveTimer) {
    clearTimeout(saveTimer)
    doSave()
  }
})

// Custom toolbar: include only relevant buttons
const toolbars = [
  'bold', 'italic', 'underline', 'strikeThrough', '-',
  'title', 'sub', 'sup', '-',
  'quote', 'unorderedList', 'orderedList', 'task', '-',
  'codeRow', 'code', '-',
  'link', 'image', 'table', '-',
  'revoke', 'next', 'save', '=', 'preview', 'catalog'
] as any
</script>

<template>
  <div class="editor" :class="{ editing: !!existingNote }">
    <div class="editor-bar">
      <input
        v-model="title"
        class="title-input"
        placeholder="笔记标题..."
        @keydown="handleKeydown"
      />
      <div class="editor-actions">
        <span v-if="lastSaved" class="saved-hint">已保存 {{ lastSaved }}</span>
        <span v-if="saving" class="saving-hint">保存中...</span>
        <button v-if="existingNote" class="cancel-btn" @click="emit('cancel')">✕</button>
      </div>
    </div>

    <div class="md-editor-wrapper">
      <MdEditor
        v-model="content"
        :theme="editorTheme"
        language="zh-CN"
        :noPrettier="true"
        :toolbars="toolbars"
        :autoDetectCode="true"
        @onChange="scheduleSave"
      />
    </div>

    <div class="editor-footer">
      <input
        v-model="tagInput"
        class="tag-input"
        placeholder="标签, 用逗号分隔"
      />
      <span class="char-count">{{ content.length }} 字</span>
    </div>
  </div>
</template>

<style scoped lang="scss">
.editor {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 10px;
  overflow: hidden;

  &.editing {
    background: var(--jc-bg-app);
  }
}

.editor-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.title-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--jc-text-highlight);
  font-size: 15px;
  font-weight: 600;
  padding: 4px 0;
  outline: none;

  &::placeholder {
    color: var(--jc-text-secondary);
    font-weight: 400;
  }
}

.editor-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.saved-hint {
  font-size: 10px;
  color: var(--jc-color-success);
}

.saving-hint {
  font-size: 10px;
  color: var(--jc-color-warning);
}

.cancel-btn {
  background: none;
  color: var(--jc-text-secondary);
  border: none;
  font-size: 14px;
  cursor: pointer;
  padding: 0 4px;

  &:hover { color: var(--jc-color-error); }
}

.md-editor-wrapper {
  flex: 1;
  min-height: 0;
  margin: 8px 0;

  // Make md-editor-v3 fill the wrapper
  :deep(.md-editor) {
    height: 100% !important;
  }
}

.editor-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.tag-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 11px;
  outline: none;
  padding: 2px 0;

  &::placeholder {
    color: var(--jc-text-secondary);
    opacity: 0.5;
  }
}

.char-count {
  font-size: 10px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
}
</style>
