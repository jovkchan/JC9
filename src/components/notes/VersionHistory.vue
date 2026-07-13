<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { invoke } from '@tauri-apps/api/core'
import type { NoteVersion } from '@/types/notes'

const store = useNotesStore()

const versions = ref<NoteVersion[]>([])
const loading = ref(false)
const selectedVersionId = ref<string | null>(null)

async function loadVersions() {
  if (!store.activeNoteTabId) return
  loading.value = true
  try {
    versions.value = await invoke<NoteVersion[]>('get_note_versions', { noteId: store.activeNoteTabId })
  } catch (e) {
    console.error('加载版本失败:', e)
  } finally {
    loading.value = false
  }
}

function selectVersion(id: string) {
  selectedVersionId.value = id
  store.previewNoteVersion(id)
}

function restore() {
  if (!store.activeNoteTabId || !selectedVersionId.value) return
  store.restoreNoteVersion(store.activeNoteTabId, selectedVersionId.value)
}

function compareWithCurrent() {
  if (!store.activeNoteTabId || !store.previewVersionData) return
  import('@/utils/openDiffWindow').then(({ openDiffWindow }) => {
    openDiffWindow(store.activeNoteTabId!, store.previewVersionData!.id, `v${store.previewVersionData!.version}`)
  })
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    const now = new Date()
    const diffMs = now.getTime() - d.getTime()
    const diffMin = Math.floor(diffMs / 60000)
    if (diffMin < 1) return '刚刚'
    if (diffMin < 60) return `${diffMin} 分钟前`
    const diffHour = Math.floor(diffMin / 60)
    if (diffHour < 24) return `${diffHour} 小时前`
    const diffDay = Math.floor(diffHour / 24)
    if (diffDay < 7) return `${diffDay} 天前`
    return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
  } catch {
    return iso
  }
}

onMounted(() => {
  loadVersions()
})
</script>

<template>
  <div class="version-history-panel">
    <div class="vh-header">
      <span class="vh-title">
        <svg t="1783910028647" viewBox="0 0 1024 1024" width="15" height="15" style="vertical-align:middle;margin-right:4px">
          <path d="M256 298.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333z" fill="currentColor"></path>
          <path d="M298.666667 489.173333V725.333333a85.333333 85.333333 0 0 0 85.333333 85.333334h256v85.333333H384a170.666667 170.666667 0 0 1-170.666667-170.666667V298.666667h85.333334v42.666666a85.333333 85.333333 0 0 0 85.333333 85.333334h256v85.333333H384a169.898667 169.898667 0 0 1-85.333333-22.826667z" fill="currentColor" opacity=".3"></path>
          <path d="M768 938.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333zM768 554.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333z" fill="currentColor"></path>
        </svg>
        版本历史
      </span>
      <button class="vh-close" @click="store.closeVersionHistory()" title="关闭">✕</button>
    </div>

    <!-- 预览区 -->
    <div v-if="store.previewVersionData" class="vh-preview">
      <div class="vh-preview-bar">
        <span class="vh-preview-label">
          <svg t="1783910028647" viewBox="0 0 1024 1024" width="13" height="13" style="vertical-align:middle;margin-right:3px">
            <path d="M256 298.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333z" fill="currentColor"></path>
            <path d="M298.666667 489.173333V725.333333a85.333333 85.333333 0 0 0 85.333333 85.333334h256v85.333333H384a170.666667 170.666667 0 0 1-170.666667-170.666667V298.666667h85.333334v42.666666a85.333333 85.333333 0 0 0 85.333333 85.333334h256v85.333333H384a169.898667 169.898667 0 0 1-85.333333-22.826667z" fill="currentColor" opacity=".3"></path>
            <path d="M768 938.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333zM768 554.666667a85.333333 85.333333 0 1 0 0-170.666667 85.333333 85.333333 0 0 0 0 170.666667z m0 85.333333a170.666667 170.666667 0 1 1 0-341.333333 170.666667 170.666667 0 0 1 0 341.333333z" fill="currentColor"></path>
          </svg>
          预览 v{{ store.previewVersionData.version }}
          <span class="vh-preview-time">({{ formatTime(store.previewVersionData.createdAt) }})</span>
        </span>
        <button class="vh-restore-btn" @click="restore">恢复此版本</button>
        <button class="vh-diff-btn" @click="compareWithCurrent" title="在新标签页对比">⇄ 对比</button>
      </div>
      <div class="vh-preview-content">
        <div class="vh-preview-title">{{ store.previewVersionData.title }}</div>
        <pre class="vh-preview-body">{{ store.previewVersionData.content.slice(0, 2000) }}{{ store.previewVersionData.content.length > 2000 ? '...' : '' }}</pre>
      </div>
    </div>

    <!-- 版本列表 -->
    <div class="vh-list" v-if="!loading">
      <div
        v-for="v in versions"
        :key="v.id"
        :class="['vh-item', { active: selectedVersionId === v.id }]"
        @click="selectVersion(v.id)"
      >
        <div class="vh-item-top">
          <span class="vh-version-badge">v{{ v.version }}</span>
          <span class="vh-time">{{ formatTime(v.createdAt) }}</span>
        </div>
        <div class="vh-item-title">{{ v.title || '无标题' }}</div>
        <div class="vh-item-preview">{{ v.content.slice(0, 80) }}{{ v.content.length > 80 ? '...' : '' }}</div>
      </div>
      <div v-if="versions.length === 0" class="vh-empty">暂无历史版本</div>
    </div>
    <div v-else class="vh-loading">加载中...</div>
  </div>
</template>

<style scoped>
.version-history-panel {
  width: 300px;
  min-width: 300px;
  border-left: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  font-size: 13px;
  color: var(--jc-text-primary);
  animation: vh-slide-in 0.2s ease-out;
}

@keyframes vh-slide-in {
  from { width: 0; min-width: 0; opacity: 0; }
  to { width: 300px; min-width: 300px; opacity: 1; }
}

.vh-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--jc-border-default);
  background: var(--jc-bg-elevated);
}

.vh-title {
  font-weight: 600;
  font-size: 14px;
  display: flex;
  align-items: center;
}

.vh-close {
  border: none;
  background: none;
  cursor: pointer;
  font-size: 16px;
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--jc-text-secondary);
}
.vh-close:hover { background: var(--jc-bg-hover); }

.vh-preview {
  border-bottom: 1px solid var(--jc-border-default);
  background: var(--jc-bg-elevated);
}

.vh-preview-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  gap: 8px;
}

.vh-preview-label {
  font-size: 12px;
  font-weight: 500;
  display: flex;
  align-items: center;
}

.vh-preview-time {
  font-weight: 400;
  color: var(--jc-text-secondary);
  margin-left: 4px;
}

.vh-restore-btn {
  padding: 4px 10px;
  border: none;
  border-radius: 4px;
  background: var(--jc-color-accent);
  color: #fff;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}
.vh-restore-btn:hover { background: var(--jc-color-accent-hover); }

.vh-diff-btn {
  padding: 4px 8px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-primary);
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
}
.vh-diff-btn:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); }

.vh-preview-content {
  padding: 8px 12px 12px;
  max-height: 200px;
  overflow-y: auto;
}

.vh-preview-title {
  font-weight: 600;
  margin-bottom: 4px;
  font-size: 14px;
  color: var(--jc-text-primary);
}

.vh-preview-body {
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--jc-text-secondary);
  line-height: 1.5;
  margin: 0;
}

.vh-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.vh-item {
  padding: 10px 12px;
  cursor: pointer;
  border-left: 3px solid transparent;
  position: relative;
}
.vh-item:hover { background: var(--jc-bg-hover); }
.vh-item.active {
  background: var(--jc-bg-selected);
  border-left-color: var(--jc-color-accent);
}

.vh-item-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.vh-version-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-secondary);
}

.vh-time {
  font-size: 11px;
  color: var(--jc-border-strong);
}

.vh-item-title {
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--jc-text-primary);
}

.vh-item-preview {
  font-size: 11px;
  color: var(--jc-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vh-empty, .vh-loading {
  padding: 40px 12px;
  text-align: center;
  color: var(--jc-text-secondary);
}
</style>
