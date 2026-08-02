<script setup lang="ts">
/**
 * QuickNote 浮动快速笔记组件
 *
 * 在任务栏附近弹出的浮动窗口，支持快速记录笔记并保存到数据库。
 * 可通过 (TODO) 标签将笔记关联到知识系统。
 */
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNotesStore } from '@/stores/notes'

const emit = defineEmits<{ close: [] }>()
const notesStore = useNotesStore()

const content = ref('')
const saving = ref(false)
const saved = ref(false)
const error = ref('')

async function save() {
  const text = content.value.trim()
  if (!text) return

  saving.value = true
  error.value = ''
  try {
    await invoke('save_note', {
      note: {
        id: crypto.randomUUID(),
        groupId: 'fixed_uncategorized',
        title: text.length > 50 ? text.slice(0, 50) + '…' : text,
        content: text,
        format: 'plain',
        isPinned: false,
        tags: text.includes('(TODO)') ? ['todo'] : [],
        visibility: 'PRIVATE',
        sortOrder: 0,
        version: 1,
        isDeleted: false,
        isArchived: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
    })
    saved.value = true
    // 只刷新当前选中的分组列表，不展开全部
    await notesStore.loadNotes(notesStore.selectedGroupId)
    setTimeout(() => {
      content.value = ''
      saved.value = false
      emit('close')
    }, 800)
  } catch (e) {
    error.value = `保存失败: ${e}`
    console.error('保存笔记失败:', e)
  } finally {
    saving.value = false
  }
}

function cancel() {
  content.value = ''
  emit('close')
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') cancel()
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) save()
}

onMounted(async () => {
  await nextTick()
  // 自动聚焦
  const textarea = document.querySelector('.quick-note-textarea') as HTMLTextAreaElement
  textarea?.focus()
})
</script>

<template>
  <div class="quick-note-overlay" @mousedown.self="cancel">
    <div class="quick-note-window">
      <div class="quick-note-header">
        <span class="quick-note-title">📝 快速笔记</span>
        <div class="quick-note-actions">
          <span class="quick-note-hint">Ctrl+Enter 保存</span>
          <button class="quick-note-btn close" @click="cancel" title="关闭 (Esc)">✕</button>
        </div>
      </div>

      <textarea
        class="quick-note-textarea"
        v-model="content"
        placeholder="写点什么...&#10;&#10;💡 输入 (TODO) 标记将自动归类为待办事项"
        @keydown="onKeydown"
        :disabled="saving"
      />

      <div class="quick-note-footer">
        <span v-if="error" class="quick-note-error">{{ error }}</span>
        <button
          class="quick-note-btn primary"
          :disabled="!content.trim() || saving"
          @click="save"
        >
          {{ saving ? '保存中…' : saved ? '✅ 已保存' : '保存笔记' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.quick-note-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.15);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 60px;
  z-index: 9999;
}

.quick-note-window {
  width: 420px;
  max-width: 90vw;
  background: var(--bg-primary, #1e1e2e);
  border: 1px solid var(--border-color, #313244);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  animation: slideIn 0.2s ease-out;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateY(-20px); }
  to { opacity: 1; transform: translateY(0); }
}

.quick-note-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #313244);
}

.quick-note-title {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-primary, #cdd6f4);
}

.quick-note-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.quick-note-hint {
  font-size: 11px;
  color: var(--text-muted, #6c7086);
}

.quick-note-btn {
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  padding: 6px 14px;
  transition: background 0.15s;
}

.quick-note-btn.close {
  background: transparent;
  color: var(--text-muted, #6c7086);
  padding: 4px 8px;
  font-size: 16px;
}

.quick-note-btn.close:hover {
  background: var(--hover-bg, #313244);
  color: var(--text-primary, #cdd6f4);
}

.quick-note-btn.primary {
  background: var(--accent-color, #89b4fa);
  color: #1e1e2e;
  font-weight: 500;
}

.quick-note-btn.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.quick-note-btn.primary:not(:disabled):hover {
  filter: brightness(1.1);
}

.quick-note-textarea {
  width: 100%;
  min-height: 120px;
  max-height: 300px;
  padding: 16px;
  border: none;
  background: transparent;
  color: var(--text-primary, #cdd6f4);
  font-size: 14px;
  line-height: 1.6;
  resize: vertical;
  outline: none;
  font-family: inherit;
}

.quick-note-textarea::placeholder {
  color: var(--text-muted, #6c7086);
}

.quick-note-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 10px 16px;
  border-top: 1px solid var(--border-color, #313244);
  gap: 8px;
}

.quick-note-error {
  color: #ef4444;
  font-size: 12px;
  margin-right: auto;
}
</style>
