<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { diffLines, type Change } from 'diff'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface VersionData {
  id: string
  noteId: string
  title: string
  content: string
  format: string
  tags: string[]
  version: number
  createdAt: string
}

interface NoteData {
  id: string
  title: string
  content: string
}

const currentNote = ref<NoteData | null>(null)
const versionData = ref<VersionData | null>(null)
const loading = ref(true)
const versionLabel = ref('')
const maximized = ref(false)
let unlisten: UnlistenFn | null = null
let unlistenResized: (() => void) | null = null

async function loadData() {
  loading.value = true
  const noteId = localStorage.getItem('jc9-diff-note-id')
  const versionId = localStorage.getItem('jc9-diff-version-id')
  const label = localStorage.getItem('jc9-diff-version-label')
  versionLabel.value = label || ''

  if (!noteId || !versionId) {
    loading.value = false
    return
  }

  try {
    const [note, version] = await Promise.all([
      invoke<NoteData>('get_note_by_id', { id: noteId }),
      invoke<VersionData>('get_note_version_by_id', { versionId }),
    ])
    currentNote.value = note
    versionData.value = version

    // 更新窗口标题
    try {
      const win = getCurrentWindow()
      await win.setTitle(`版本对比: ${version?.title || label || ''}`)
    } catch {}
  } catch (e) {
    console.error('加载对比数据失败:', e)
  } finally {
    loading.value = false
  }
}

const diffResult = computed<Change[]>(() => {
  if (!versionData.value || !currentNote.value) return []
  return diffLines(versionData.value.content, currentNote.value.content)
})

const diffStats = computed(() => {
  let add = 0, remove = 0
  for (const c of diffResult.value) {
    if (c.added) add += c.count ?? 0
    if (c.removed) remove += c.count ?? 0
  }
  return { add, remove }
})

interface DiffRow {
  left: { text: string; type: 'same' | 'remove' | 'empty' }
  right: { text: string; type: 'same' | 'add' | 'empty' }
}

const diffRows = computed<DiffRow[]>(() => {
  const rows: DiffRow[] = []
  for (const c of diffResult.value) {
    if (c.added) {
      for (const line of c.value.split('\n').filter(Boolean)) {
        rows.push({ left: { text: '', type: 'empty' }, right: { text: line, type: 'add' } })
      }
    } else if (c.removed) {
      for (const line of c.value.split('\n').filter(Boolean)) {
        rows.push({ left: { text: line, type: 'remove' }, right: { text: '', type: 'empty' } })
      }
    } else {
      const parts = c.value.split('\n')
      for (let li = 0; li < parts.length; li++) {
        if (li === parts.length - 1 && parts[li] === '') continue
        rows.push({
          left: { text: parts[li], type: 'same' },
          right: { text: parts[li], type: 'same' },
        })
      }
    }
  }
  return rows
})

function doMinimize() { try { getCurrentWindow().minimize() } catch {} }
function doMaximize() { try { getCurrentWindow().toggleMaximize() } catch {} }
function doClose() { try { getCurrentWindow().close() } catch {} }

onMounted(async () => {
  await loadData()

  try {
    maximized.value = await getCurrentWindow().isMaximized()
    unlistenResized = await getCurrentWindow().onResized(async () => {
      maximized.value = await getCurrentWindow().isMaximized()
    })
  } catch {}

  // 监听刷新事件（窗口已存在时重新加载数据）
  unlisten = await listen('diff:reload', () => {
    loadData()
  })
})

onUnmounted(() => {
  unlisten?.()
  if (unlistenResized) unlistenResized()
})
</script>

<template>
  <div class="vd-window">
    <!-- TitleBar -->
    <div class="vd-titlebar">
      <div class="vd-titlebar-drag">
        <span class="vd-titlebar-title">版本对比: {{ versionLabel || '加载中...' }}</span>
      </div>
      <div class="vd-titlebar-actions">
        <button class="vd-tb-btn" @click="doMinimize" title="最小化">
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 13h10"/></svg>
        </button>
        <button class="vd-tb-btn" @click="doMaximize" title="最大化">
          <svg v-if="!maximized" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/></svg>
          <svg v-else viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3.5" y="5.5" width="7" height="7" rx="1"/><path d="M5.5 5.5V3.5h7v7h-2"/></svg>
        </button>
        <button class="vd-tb-btn vd-close" @click="doClose" title="关闭">
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="vd-loading">加载中...</div>

    <!-- No data -->
    <div v-else-if="!versionData || !currentNote" class="vd-loading">
      请在主窗口中选择一个版本后再打开对比
    </div>

    <!-- Diff content -->
    <template v-else>
      <div class="vd-header">
        <div class="vd-header-left">
          <span class="vd-badge old">◀ {{ versionData.title || '旧版本' }} (v{{ versionData.version }})</span>
          <span class="vd-arrow">⇄</span>
          <span class="vd-badge new">{{ currentNote.title || '当前版本' }} ▶</span>
        </div>
        <div class="vd-header-right">
          <span class="vd-stat add">+{{ diffStats.add }}</span>
          <span class="vd-stat remove">-{{ diffStats.remove }}</span>
        </div>
      </div>

      <div class="vd-body">
        <div class="vd-pane vd-pane-left">
          <div class="vd-pane-header">旧版本 (v{{ versionData.version }})</div>
          <div class="vd-pane-content">
            <div v-for="(row, i) in diffRows" :key="i" :class="['vd-line', row.left.type === 'remove' ? 'remove' : '']">
              <span class="vd-ln">{{ i + 1 }}</span>
              <pre class="vd-text">{{ row.left.text }}</pre>
            </div>
          </div>
        </div>
        <div class="vd-divider"></div>
        <div class="vd-pane vd-pane-right">
          <div class="vd-pane-header">当前版本</div>
          <div class="vd-pane-content">
            <div v-for="(row, i) in diffRows" :key="i" :class="['vd-line', row.right.type === 'add' ? 'add' : '']">
              <span class="vd-ln">{{ i + 1 }}</span>
              <pre class="vd-text">{{ row.right.text }}</pre>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.vd-window {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-app, #1e1e1e);
  color: var(--jc-text-primary, #ccc);
  font-size: 13px;
  overflow: hidden;
}

/* ── TitleBar ── */
.vd-titlebar {
  display: flex;
  align-items: center;
  height: 32px;
  background: var(--jc-titlebar-bg, #1e1e1e);
  flex-shrink: 0;
  user-select: none;
}

.vd-titlebar-drag {
  flex: 1;
  -webkit-app-region: drag;
  padding-left: 14px;
  font-size: 12px;
  color: var(--jc-text-secondary, #858585);
}

.vd-titlebar-actions {
  display: flex;
  -webkit-app-region: no-drag;
}

.vd-tb-btn {
  width: 46px;
  height: 32px;
  border: none;
  background: none;
  color: var(--jc-text-secondary, #858585);
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.vd-tb-btn:hover { background: var(--jc-titlebar-btn-hover, #3c3c3c); }
.vd-close:hover { background: #e81123; color: #fff; }

/* ── Loading ── */
.vd-loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--jc-text-secondary, #858585);
  font-size: 14px;
}

/* ── Diff header ── */
.vd-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  background: var(--jc-bg-panel, #252526);
  flex-shrink: 0;
}

.vd-header-left { display: flex; align-items: center; gap: 12px; }

.vd-badge {
  padding: 3px 10px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 600;
}
.vd-badge.old { background: rgba(244, 71, 71, 0.15); color: #f44747; }
.vd-badge.new { background: rgba(78, 201, 176, 0.15); color: #4ec9b0; }

.vd-arrow { color: var(--jc-text-secondary, #858585); font-size: 16px; }

.vd-header-right { display: flex; gap: 10px; }

.vd-stat {
  font-size: 12px;
  font-weight: 700;
  padding: 2px 8px;
  border-radius: 3px;
}
.vd-stat.add { background: rgba(78, 201, 176, 0.12); color: #4ec9b0; }
.vd-stat.remove { background: rgba(244, 71, 71, 0.12); color: #f44747; }

/* ── Diff body ── */
.vd-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.vd-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.vd-pane-header {
  padding: 6px 12px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  background: var(--jc-bg-panel, #252526);
  color: var(--jc-text-secondary, #858585);
  flex-shrink: 0;
}

.vd-pane-content {
  flex: 1;
  overflow-y: auto;
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.6;
}

.vd-divider {
  width: 3px;
  background: var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}

.vd-line {
  display: flex;
  padding: 0 8px;
  min-height: 1.6em;
}
.vd-line.remove { background: rgba(244, 71, 71, 0.1); }
.vd-line.add { background: rgba(78, 201, 176, 0.1); }

.vd-ln {
  width: 40px;
  flex-shrink: 0;
  text-align: right;
  padding-right: 12px;
  color: var(--jc-text-secondary, #858585);
  user-select: none;
  font-size: 11px;
}

.vd-text {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  flex: 1;
  font-family: inherit;
}
</style>
