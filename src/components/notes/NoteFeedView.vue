<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { invoke } from '@tauri-apps/api/core'
import type { Note } from '@/types/notes'
import { marked } from 'marked'

const store = useNotesStore()

// 右键菜单状态
const ctxMenu = ref({ show: false, x: 0, y: 0, noteId: '' })

// 编辑器输入框状态
const newContent = ref('')
const newTitle = ref('')
const showTitleInput = ref(false)
const textareaRef = ref<HTMLTextAreaElement | null>(null)

// 正在编辑的卡片 ID
const inlineEditingId = ref<string | null>(null)
const inlineEditContent = ref('')
const inlineEditTitle = ref('')

// ── Markdown 渲染引擎配置 ──
const customRenderer = new marked.Renderer()
let checkboxIndex = 0

// 自定义列表项渲染以注入 Task Checkbox 的索引
customRenderer.listitem = function (item: any) {
  let text = item.text
  if (item.task) {
    const checked = item.checked ? 'checked' : ''
    // 允许点击但拦截默认行为，注入 data-idx 属性
    text = text.replace(/^\[[ xX]\]\s*/, `<input type="checkbox" class="feed-task-checkbox" data-idx="${checkboxIndex++}" ${checked} /> `)
    return `<li class="feed-task-list-item">${text}</li>`
  }
  return `<li>${text}</li>`
}

function renderMarkdown(content: string) {
  checkboxIndex = 0
  const opt = { renderer: customRenderer, gfm: true, breaks: true }
  let html = marked.parse(content, opt) as string
  // 匹配 #标签 渲染为可点击超链接，避免在 <code> 标签中匹配
  html = html.replace(/(^|\s)#([^\s#<>]+)/g, '$1<span class="feed-tag-link" data-tag="$2">#$2</span>')
  return html
}

// ── 时间格式化 ──
function formatTime(isoString: string) {
  const date = new Date(isoString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  if (diffMins < 1) return '刚刚'
  if (diffMins < 60) return `${diffMins} 分钟前`
  const diffHours = Math.floor(diffMins / 60)
  if (diffHours < 24) return `${diffHours} 小时前`
  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

// ── 提交新的备忘 ──
async function submitMemo() {
  const contentVal = newContent.value.trim()
  if (!contentVal) return
  
  await store.createNote({
    title: showTitleInput.value ? newTitle.value.trim() : '',
    content: contentVal,
    format: 'markdown',
    tags: [],
    groupId: store.selectedGroupId,
    visibility: 'PRIVATE'
  })

  newContent.value = ''
  newTitle.value = ''
  showTitleInput.value = false
}

// ── 快捷 Markdown 动作 ──
function insertMarkup(prefix: string, suffix: string = '') {
  const el = textareaRef.value
  if (!el) return
  const start = el.selectionStart
  const end = el.selectionEnd
  const text = newContent.value
  const selected = text.substring(start, end)
  
  newContent.value = text.substring(0, start) + prefix + selected + suffix + text.substring(end)
  nextTick(() => {
    el.focus()
    const newPos = start + prefix.length + selected.length + suffix.length
    el.setSelectionRange(newPos, newPos)
  })
}

function handleKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === 'Enter') {
    e.preventDefault()
    submitMemo()
  }
}

// ── 原地编辑 ──
function startInlineEdit(note: Note) {
  inlineEditingId.value = note.id
  inlineEditContent.value = note.content
  inlineEditTitle.value = note.title
}

// ── 右键菜单 ──
function showCtxMenu(e: MouseEvent, noteId: string) {
  ctxMenu.value = { show: true, x: e.clientX, y: e.clientY, noteId }
}

function hideCtxMenu() {
  ctxMenu.value.show = false
}

async function moveNoteToGroup(groupId: string | null) {
  try {
    await invoke('move_note', { noteId: ctxMenu.value.noteId, groupId })
    await store.loadAllNotes()
    hideCtxMenu()
  } catch (e) { console.error(e) }
}

function cancelInlineEdit() {
  inlineEditingId.value = null
}

async function saveInlineEdit(note: Note) {
  const updatedNote = {
    ...note,
    title: inlineEditTitle.value,
    content: inlineEditContent.value,
    updatedAt: new Date().toISOString()
  }
  await store.saveNote(updatedNote)
  inlineEditingId.value = null
}

// ── 任务列表勾选与标签点击事件委托 ──
async function handleCardInteraction(e: MouseEvent, note: Note) {
  const target = e.target as HTMLElement
  
  // 1. 处理待办任务点击
  if (target && target.classList.contains('feed-task-checkbox')) {
    e.preventDefault()
    e.stopPropagation()
    const idx = parseInt(target.getAttribute('data-idx') || '0', 10)
    
    let count = 0
    const updatedContent = note.content.replace(/([-*]\s+\[)([ xX])(\])/g, (match, prefix, status, suffix) => {
      if (count === idx) {
        const newStatus = status === ' ' ? 'x' : ' '
        count++
        return `${prefix}${newStatus}${suffix}`
      }
      count++
      return match
    })

    const updatedNote = {
      ...note,
      content: updatedContent,
      updatedAt: new Date().toISOString()
    }
    await store.saveNote(updatedNote)
  }
  
  // 2. 处理标签点击过滤
  if (target && target.classList.contains('feed-tag-link')) {
    e.preventDefault()
    e.stopPropagation()
    const tag = target.getAttribute('data-tag')
    if (tag) {
      store.listTab = 'tags'
      store.selectedTag = tag
    }
  }
}

// 清除当前所有的过滤状态
function clearFilters() {
  store.selectedTag = null
  store.filterDate = null
  store.searchQuery = ''
  store.selectedGroupId = null
}

const activeFilterSummary = computed(() => {
  const parts = []
  if (store.selectedGroupId) {
    const g = store.groups.find(x => x.id === store.selectedGroupId)
    if (g) parts.push(`分组: ${g.name}`)
  }
  if (store.selectedTag) parts.push(`标签: #${store.selectedTag}`)
  if (store.filterDate) parts.push(`日期: ${store.filterDate}`)
  if (store.searchQuery) parts.push(`搜索: "${store.searchQuery}"`)
  return parts.join(' + ')
})
</script>

<template>
  <div class="feed-view">
    <!-- Filter Indicator Banner -->
    <div v-if="activeFilterSummary" class="filter-banner">
      <div class="fb-text">正在筛选：<span>{{ activeFilterSummary }}</span></div>
      <button class="fb-clear" @click="clearFilters">清除过滤 ✕</button>
    </div>

    <!-- Memos 快速发布框 -->
    <div class="memo-creator">
      <div class="creator-title" v-if="showTitleInput">
        <input v-model="newTitle" placeholder="输入备忘标题（可选）..." class="title-input-field" />
      </div>
      <textarea
        ref="textareaRef"
        v-model="newContent"
        placeholder="写点什么... 支持 Markdown 语法与 #标签 (Ctrl+Enter 发布)"
        class="memo-textarea"
        @keydown="handleKeydown"
      ></textarea>
      <div class="creator-actions">
        <div class="toolbar">
          <button class="tb-btn" :class="{active: showTitleInput}" @click="showTitleInput = !showTitleInput" title="标题">T</button>
          <button class="tb-btn" @click="insertMarkup('**', '**')" title="加粗"><b>B</b></button>
          <button class="tb-btn" @click="insertMarkup('*', '*')" title="斜体"><i>I</i></button>
          <button class="tb-btn" @click="insertMarkup('```\n', '\n```')" title="代码块">&lt;&gt;</button>
          <button class="tb-btn" @click="insertMarkup('- [ ] ')" title="任务列表">☑</button>
          <button class="tb-btn" @click="insertMarkup('#')" title="标签">#</button>
        </div>
        <div class="submit-bar">
          <span class="char-count">{{ newContent.length }} 字</span>
          <button class="submit-btn" :disabled="!newContent.trim()" @click="submitMemo">发布</button>
        </div>
      </div>
    </div>

    <!-- Feed 卡片流 -->
    <div class="feed-container">
      <div v-if="store.filteredNotes.length === 0" class="feed-empty">
        <div class="empty-icon">📝</div>
        <div class="empty-title">没有找到相关的 Memos</div>
        <div class="empty-subtitle">随手记录当下的想法、待办或代码片段吧</div>
      </div>

      <div
        v-for="note in store.filteredNotes"
        :key="note.id"
        class="memo-card"
        :class="{pinned: note.isPinned}"
        @click="handleCardInteraction($event, note)"
        @contextmenu.stop.prevent="showCtxMenu($event, note.id)"
      >
        <!-- 原地编辑状态 -->
        <div v-if="inlineEditingId === note.id" class="card-edit-mode" @click.stop>
          <input v-model="inlineEditTitle" placeholder="标题..." class="card-edit-title" />
          <textarea v-model="inlineEditContent" class="card-edit-textarea"></textarea>
          <div class="card-edit-actions">
            <button class="edit-btn-cancel" @click="cancelInlineEdit">取消</button>
            <button class="edit-btn-save" @click="saveInlineEdit(note)">保存</button>
          </div>
        </div>

        <!-- 普通阅读态 -->
        <div v-else class="card-read-mode">
          <!-- 卡片头部信息 -->
          <div class="card-header">
            <div class="card-info">
              <span class="card-title-text">{{ note.title || '备忘' }}</span>
              <span class="card-time">{{ formatTime(note.updatedAt || note.createdAt) }}</span>
            </div>
            <!-- 卡片右侧动作 -->
            <div class="card-actions">
              <button
                class="act-btn pin"
                :class="{on: note.isPinned}"
                @click.stop="store.togglePin(note.id)"
                :title="note.isPinned ? '取消置顶' : '置顶'"
              >
                ★
              </button>
              <button
                class="act-btn"
                @click.stop="store.toggleArchive(note.id)"
                :title="note.isArchived ? '取消归档' : '归档'"
              >
                📦
              </button>
              <button
                class="act-btn"
                @click.stop="startInlineEdit(note)"
                title="编辑"
              >
                ✏️
              </button>
              <button
                class="act-btn"
                @click.stop="store.copyContent(note.id)"
                title="复制正文"
              >
                📋
              </button>
              <button
                class="act-btn delete"
                @click.stop="store.removeNote(note.id)"
                title="删除"
              >
                ✕
              </button>
            </div>
          </div>

          <!-- 卡片内容区 -->
          <div class="card-body markdown-body" v-html="renderMarkdown(note.content)"></div>

          <!-- 卡片底部标签 -->
          <div class="card-footer" v-if="note.tags && note.tags.length > 0">
            <span
              v-for="tag in note.tags"
              :key="tag"
              class="feed-tag-badge"
              @click.stop="store.listTab = 'tags'; store.selectedTag = tag"
            >
              #{{ tag }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- 右键菜单：移动笔记到分组 -->
  <Teleport to="body">
    <div v-if="ctxMenu.show" class="ctx-overlay" @click="hideCtxMenu" @contextmenu.prevent="hideCtxMenu">
      <div
        class="ctx-menu"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
        @click.stop
      >
        <div class="ctx-menu-title">移动到分组</div>
        <button
          v-for="g in store.groups"
          :key="g.id"
          class="ctx-menu-item"
          @click="moveNoteToGroup(g.id)"
        >
          📁 {{ g.name }}
        </button>
      </div>
    </div>
  </Teleport>
</template>

<style lang="scss">
// 注意：Markdown 渲染的 v-html 元素不能用 scoped，所以我们在这里定义不带 scoped 的全局嵌套样式，或者利用 class 包装
.feed-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px;
  background: var(--jc-bg-app);
  gap: 16px;

  &::-webkit-scrollbar {
    width: 6px;
  }
  &::-webkit-scrollbar-thumb {
    background: var(--jc-border-default);
    border-radius: 3px;
  }
}

.filter-banner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: rgba(var(--jc-color-success-rgb, 0, 109, 50), 0.1);
  border: 1px dashed var(--jc-color-success);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 12px;

  .fb-text {
    color: var(--jc-text-primary);
    span {
      font-weight: 600;
      color: var(--jc-color-success);
    }
  }

  .fb-clear {
    background: none;
    border: none;
    color: var(--jc-text-secondary);
    cursor: pointer;
    font-weight: 500;
    &:hover {
      color: var(--jc-color-error);
    }
  }
}

// Memos 快捷输入区
.memo-creator {
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  padding: 12px;
  gap: 8px;
  transition: border-color 0.2s;

  &:focus-within {
    border-color: var(--jc-color-accent);
  }

  .title-input-field {
    width: 100%;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--jc-border-default);
    color: var(--jc-text-highlight);
    font-size: 13px;
    font-weight: 600;
    padding: 4px 0 8px;
    outline: none;
  }

  .memo-textarea {
    width: 100%;
    min-height: 80px;
    background: transparent;
    border: none;
    color: var(--jc-text-primary);
    font-size: 13px;
    line-height: 1.6;
    resize: vertical;
    outline: none;
    font-family: inherit;

    &::placeholder {
      color: var(--jc-text-secondary);
      opacity: 0.7;
    }
  }

  .creator-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--jc-border-default);
    padding-top: 8px;
  }

  .toolbar {
    display: flex;
    gap: 4px;
  }

  .tb-btn {
    background: transparent;
    border: none;
    color: var(--jc-text-secondary);
    width: 26px;
    height: 26px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;

    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
    }
    
    &.active {
      background: var(--jc-bg-selected);
      color: var(--jc-color-accent);
    }
  }

  .submit-bar {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .char-count {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }

  .submit-btn {
    background: var(--jc-color-accent);
    color: #fff;
    border: none;
    border-radius: 4px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.2s;

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    &:not(:disabled):hover {
      opacity: 0.9;
    }
  }
}

// Feed 卡片流容器
.feed-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.feed-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 0;
  text-align: center;

  .empty-icon {
    font-size: 32px;
    margin-bottom: 8px;
  }
  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--jc-text-highlight);
    margin-bottom: 4px;
  }
  .empty-subtitle {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}

// Memo 卡片
.memo-card {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  padding: 14px;
  position: relative;
  transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;

  &:hover {
    border-color: var(--jc-border-strong);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  }

  &.pinned {
    border-left: 3px solid var(--jc-color-favorite, #f59e0b);
  }
}

// 卡片头部
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 8px;
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: 2px;

  .card-title-text {
    font-size: 13px;
    font-weight: 600;
    color: var(--jc-text-highlight);
  }

  .card-time {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}

.card-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.2s;

  .memo-card:hover & {
    opacity: 1;
  }
}

.act-btn {
  background: transparent;
  border: none;
  color: var(--jc-text-secondary);
  width: 22px;
  height: 22px;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;

  &:hover {
    background: var(--jc-bg-hover);
    color: var(--jc-text-primary);
  }

  &.pin.on {
    color: var(--jc-color-favorite, #f59e0b);
  }

  &.delete:hover {
    color: var(--jc-color-error);
    background: rgba(var(--jc-color-error-rgb, 220, 38, 38), 0.1);
  }
}

// 卡片编辑模式
.card-edit-mode {
  display: flex;
  flex-direction: column;
  gap: 8px;

  .card-edit-title {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 12px;
    font-weight: 600;
    padding: 4px 8px;
    border-radius: 4px;
    outline: none;
    &:focus {
      border-color: var(--jc-color-accent);
    }
  }

  .card-edit-textarea {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 13px;
    line-height: 1.6;
    padding: 8px;
    min-height: 80px;
    border-radius: 4px;
    resize: vertical;
    outline: none;
    font-family: inherit;
    &:focus {
      border-color: var(--jc-color-accent);
    }
  }

  .card-edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .edit-btn-cancel {
    background: var(--jc-bg-btn);
    color: var(--jc-text-secondary);
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    &:hover {
      color: var(--jc-text-primary);
    }
  }

  .edit-btn-save {
    background: var(--jc-color-accent);
    color: #fff;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    &:hover {
      opacity: 0.9;
    }
  }
}

// 卡片内容样式渲染与 Markdown 排版
.card-body.markdown-body {
  font-size: 13px;
  color: var(--jc-text-primary);
  line-height: 1.6;
  word-break: break-word;

  p {
    margin: 4px 0;
  }

  ul, ol {
    padding-left: 20px;
    margin: 4px 0;
  }

  code {
    background: var(--jc-bg-btn);
    color: var(--jc-color-accent);
    font-family: 'Cascadia Code', Consolas, monospace;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 3px;
  }

  pre {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    padding: 10px;
    border-radius: 6px;
    overflow-x: auto;
    margin: 8px 0;

    code {
      background: transparent;
      color: inherit;
      padding: 0;
    }
  }

  // 任务列表样式
  .feed-task-list-item {
    list-style-type: none;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: -16px;
  }

  .feed-task-checkbox {
    width: 14px;
    height: 14px;
    cursor: pointer;
    accent-color: var(--jc-color-accent);
  }

  // 行内标签可点击样式
  .feed-tag-link {
    color: var(--jc-color-accent);
    font-weight: 500;
    cursor: pointer;
    text-decoration: none;
    padding: 0 2px;
    border-radius: 3px;
    &:hover {
      background: rgba(var(--jc-color-accent-rgb, 0, 102, 204), 0.1);
      text-decoration: underline;
    }
  }
}

// 底部标签 Badge
.card-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 10px;
  border-top: 1px dashed var(--jc-border-default);
  padding-top: 8px;
}

.feed-tag-badge {
  font-size: 10px;
  background: var(--jc-bg-btn);
  color: var(--jc-text-secondary);
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: color 0.15s, background 0.15s;

  &:hover {
    color: var(--jc-color-accent);
    background: var(--jc-bg-selected);
  }
}

// 右键菜单
.ctx-overlay {
  position: fixed; inset: 0; z-index: 10000;
}
.ctx-menu {
  position: fixed;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  padding: 4px 0;
  min-width: 160px;
  box-shadow: 0 4px 16px rgba(0,0,0,.3);
}
.ctx-menu-title {
  padding: 6px 12px;
  font-size: 11px;
  color: var(--jc-text-secondary);
  border-bottom: 1px solid var(--jc-border-default);
  margin-bottom: 2px;
}
.ctx-menu-item {
  display: block; width: 100%;
  padding: 6px 12px;
  border: none; background: none;
  font-size: 13px; text-align: left;
  cursor: pointer;
  color: var(--jc-text-primary);
  &:hover { background: var(--jc-bg-selected); }
}
</style>
