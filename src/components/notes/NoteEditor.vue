<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { useNotesStore } from '@/stores/notes'
import type { Note } from '@/types/notes'

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
const format = ref<'plain' | 'markdown'>(props.existingNote?.format as 'plain' | 'markdown' ?? 'plain')
const tagInput = ref(props.existingNote?.tags.join(', ') ?? '')
const saving = ref(false)
const lastSaved = ref('')

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
    // Editing existing note — use saveNote
    const existing = store.notes.find(n => n.id === editNoteId.value)
    if (!existing) { saving.value = false; return }

    const note: Note = {
      ...existing,
      title: title.value,
      content: content.value,
      format: format.value,
      tags: tags.value,
      updatedAt: new Date().toISOString(),
    }
    await store.saveNote(note)
    emit('saved', note)
  } else {
    // Creating new note — create once, then switch to edit mode
    const note = await store.createNote({
      title: title.value,
      content: content.value,
      format: format.value,
      tags: tags.value,
      groupId: store.selectedGroupId,
      visibility: 'PRIVATE',
    })
    if (note) {
      editNoteId.value = note.id  // ← Switch to edit mode after first save
      emit('saved', note)
    }
  }

  lastSaved.value = new Date().toLocaleTimeString()
  saving.value = false
}

function handleKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === 'Enter') {
    e.preventDefault()
    doSave()
  }
}

watch([title, content, tagInput], () => scheduleSave())

onUnmounted(() => {
  if (saveTimer) {
    clearTimeout(saveTimer)
    doSave()
  }
})

function toggleFormat() {
  format.value = format.value === 'markdown' ? 'plain' : 'markdown'
}
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
        <button class="fmt-btn" @click="toggleFormat" :title="format === 'markdown' ? '切换到纯文本' : '切换到 Markdown'">
          {{ format === 'markdown' ? 'MD' : 'TXT' }}
        </button>
        <button v-if="existingNote" class="cancel-btn" @click="emit('cancel')">✕</button>
      </div>
    </div>

    <textarea
      v-model="content"
      class="content-area"
      :placeholder="format === 'markdown' ? '支持 Markdown 语法...' : '开始写点什么... Ctrl+Enter 立即保存'"
      @keydown="handleKeydown"
    ></textarea>

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

.fmt-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-secondary);
  border: none;
  padding: 2px 8px;
  font-size: 10px;
  font-family: 'Cascadia Code', Consolas, monospace;
  cursor: pointer;
  border-radius: 3px;

  &:hover {
    color: var(--jc-color-accent);
  }
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

.content-area {
  flex: 1;
  min-height: 0;
  margin: 8px 0;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-primary);
  font-size: 13px;
  font-family: 'Cascadia Code', Consolas, monospace;
  line-height: 1.6;
  padding: 10px;
  resize: none;
  outline: none;
  border-radius: 3px;

  &:focus {
    border-color: var(--jc-color-accent);
  }

  &::placeholder {
    color: var(--jc-text-secondary);
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
