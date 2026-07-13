<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { invoke } from '@tauri-apps/api/core'
import type { Note } from '@/types/notes'

const store = useNotesStore()

// 右键菜单状态
const ctxMenu = ref({ show: false, x: 0, y: 0, noteId: '' })
const moveSubShow = ref(false)
const moveHoverGroupId = ref<string | null>(null)
const moveHoverChildId = ref<string | null>(null)
let moveCloseTimer: ReturnType<typeof setTimeout> | null = null

// 级联分组
const moveRootGroups = computed(() => store.groups.filter(g => !g.parentId).sort((a, b) => a.sortOrder - b.sortOrder))
function getMoveChildren(parentId: string) { return store.groups.filter(g => g.parentId === parentId).sort((a, b) => a.sortOrder - b.sortOrder) }
const moveChildren = computed(() => moveHoverGroupId.value ? getMoveChildren(moveHoverGroupId.value) : [])
const moveGrandchildren = computed(() => moveHoverChildId.value ? getMoveChildren(moveHoverChildId.value) : [])

function scheduleCloseMove() {
  moveCloseTimer = setTimeout(() => {
    moveSubShow.value = false; moveHoverGroupId.value = null; moveHoverChildId.value = null
  }, 200)
}
function cancelCloseMove() { if (moveCloseTimer) { clearTimeout(moveCloseTimer); moveCloseTimer = null } }

// 级联菜单定位
function calcMovePos(parentLeft: number, parentTop: number, parentWidth: number, hoverIdx: number, itemCount: number) {
  const vw = window.innerWidth; const vh = window.innerHeight; const itemH = 25; const padTop = 4; const gap = 2
  const menuW = 140; const menuH = Math.min(itemCount * itemH + padTop * 2, 280)
  const rightSpace = vw - parentLeft - parentWidth
  const left = rightSpace >= menuW ? parentLeft + parentWidth + gap : parentLeft - menuW - gap
  const itemTop = parentTop + padTop + hoverIdx * itemH
  const below = vh - itemTop - menuH
  const top = below < 0 ? Math.max(4, itemTop - menuH + itemH) : itemTop
  return { left: `${Math.max(4, left)}px`, top: `${top}px` }
}

const moveSubStyle = computed(() => {
  const x = ctxMenu.value.x + 140; const y = ctxMenu.value.y
  const vw = window.innerWidth; const vh = window.innerHeight; const menuW = 140
  const count = moveRootGroups.value.length; const menuH = Math.min(count * 25 + 8, 280)
  const rightSpace = vw - x
  const left = rightSpace < menuW ? (ctxMenu.value.x - menuW - 2) : x
  const below = vh - y
  const top = below < menuH && y > menuH ? Math.max(4, y - menuH + 4) : Math.min(y, vh - menuH - 4)
  return { left: `${Math.max(4, left)}px`, top: `${top}px` }
})

const moveSub2Style = computed(() => {
  const idx = moveRootGroups.value.findIndex(g => g.id === moveHoverGroupId.value)
  return calcMovePos(parseInt(moveSubStyle.value.left) || 0, parseInt(moveSubStyle.value.top) || 0, 130, idx >= 0 ? idx : 0, moveChildren.value.length)
})

const moveSub3Style = computed(() => {
  const idx = moveChildren.value.findIndex(g => g.id === moveHoverChildId.value)
  return calcMovePos(parseInt(moveSub2Style.value.left) || 0, parseInt(moveSub2Style.value.top) || 0, 130, idx >= 0 ? idx : 0, moveGrandchildren.value.length)
})

// 快速发布
const newContent = ref('')
const newTitle = ref('')
const showTitleInput = ref(false)
const textareaRef = ref<HTMLTextAreaElement | null>(null)

// ── 纯文本摘要（去 Markdown 语法）──
function plainExcerpt(content: string, maxLen = 120): string {
  if (!content) return ''
  // 去掉代码块
  let text = content.replace(/```[\s\S]*?```/g, '')
  // 去掉行内代码
  text = text.replace(/`([^`]+)`/g, '$1')
  // 去掉 Markdown 链接，保留文字
  text = text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
  // 去掉图片
  text = text.replace(/!\[([^\]]*)\]\([^)]+\)/g, '$1')
  // 去掉标题标记
  text = text.replace(/^#{1,6}\s+/gm, '')
  // 去掉加粗/斜体
  text = text.replace(/(\*{1,3}|_{1,3})(.*?)\1/g, '$2')
  // 去掉 ~~删除线~~
  text = text.replace(/~~(.*?)~~/g, '$1')
  // 去掉列表标记
  text = text.replace(/^[\s]*[-*+]\s+/gm, '')
  text = text.replace(/^[\s]*\d+\.\s+/gm, '')
  // 去掉任务标记
  text = text.replace(/\[[ xX]\]\s*/g, '')
  // 去掉分隔线
  text = text.replace(/^---+/gm, '')
  text = text.replace(/^___+/gm, '')
  // 去掉引用
  text = text.replace(/^>\s+/gm, '')
  // 去掉 ++== 等自定义语法
  text = text.replace(/\+\+/g, '').replace(/==/g, '')
  // 合并多余空白
  text = text.replace(/\n+/g, ' ').replace(/\s+/g, ' ').trim()
  if (text.length <= maxLen) return text
  return text.slice(0, maxLen) + '…'
}

// ── 从内容中取首行作为默认标题 ──
function autoTitle(content: string): string {
  if (!content) return ''
  const line = content.split('\n').find(l => l.trim()) || ''
  // 去掉 Markdown 标题标记
  return line.replace(/^#+\s*/, '').trim()
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

// ── 快速发布 ──
async function submitMemo() {
  const contentVal = newContent.value.trim()
  if (!contentVal) return

  await store.createNote({
    title: showTitleInput.value ? newTitle.value.trim() : autoTitle(contentVal),
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

// ── 点击卡片 → 打开 NoteEditor ──
function openNote(note: Note) {
  store.selectedNoteId = note.id
  store.openNoteTab(note.id)
}

// ── 清除所有过滤状态 ──
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
        @click="openNote(note)"
        @contextmenu.stop.prevent="showCtxMenu($event, note.id)"
      >
        <!-- 卡片头部 -->
        <div class="card-header">
          <div class="card-info">
            <span class="card-title-text">{{ note.title || autoTitle(note.content) || '备忘' }}</span>
            <span class="card-time">{{ formatTime(note.updatedAt || note.createdAt) }}</span>
          </div>
          <div class="card-actions">
            <button class="act-btn pin" :class="{on: note.isPinned}" @click.stop="store.togglePin(note.id)" title="置顶">★</button>
            <button class="act-btn" @click.stop="store.copyContent(note.id)" title="复制正文">📋</button>
            <button class="act-btn delete" @click.stop="store.removeNote(note.id)" title="删除">✕</button>
          </div>
        </div>

        <!-- 纯文本摘要（无样式） -->
        <div class="card-excerpt">{{ plainExcerpt(note.content) }}</div>

        <!-- 标签（卡片主体视觉） -->
        <div class="card-tags" v-if="note.tags && note.tags.length > 0">
          <span
            v-for="tag in note.tags"
            :key="tag"
            class="tag-badge"
            @click.stop="store.listTab = 'tags'; store.selectedTag = tag"
          >
            #{{ tag }}
          </span>
        </div>
      </div>
    </div>
  </div>

  <!-- 右键菜单：移动笔记到分组（级联 3 级） -->
  <Teleport to="body">
    <div v-if="ctxMenu.show" class="ctx-overlay" @click="hideCtxMenu" @contextmenu.prevent="hideCtxMenu">
      <div class="ctx-menu" :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }" @click.stop>
        <div class="ctx-menu-item" @click="moveNoteToGroup(null)">📁 根目录</div>
        <div class="ctx-menu-item" style="display:flex;justify-content:space-between" @mouseenter="moveSubShow = true">
          移动到分组 <span style="font-size:10px">▸</span>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- 一级子菜单 -->
  <Teleport to="body">
    <div v-if="ctxMenu.show && moveSubShow" class="ctx-menu" :style="moveSubStyle"
      @mouseleave="scheduleCloseMove" @mouseenter="cancelCloseMove">
      <div v-for="g in moveRootGroups" :key="g.id" class="ctx-menu-item"
        style="display:flex;justify-content:space-between"
        @click="moveNoteToGroup(g.id)"
        @mouseenter="cancelCloseMove(); moveHoverGroupId = getMoveChildren(g.id).length > 0 ? g.id : null; moveHoverChildId = null">
        📁 {{ g.name }}
        <span v-if="getMoveChildren(g.id).length > 0" style="font-size:10px">▸</span>
      </div>
    </div>
  </Teleport>

  <!-- 二级子菜单 -->
  <Teleport to="body">
    <div v-if="ctxMenu.show && moveSubShow && moveHoverGroupId && moveChildren.length > 0"
      class="ctx-menu" :style="moveSub2Style"
      @mouseleave="moveHoverChildId = null; moveHoverGroupId = null"
      @mouseenter="cancelCloseMove">
      <div v-for="child in moveChildren" :key="child.id" class="ctx-menu-item"
        style="display:flex;justify-content:space-between"
        @click="moveNoteToGroup(child.id)"
        @mouseenter="cancelCloseMove(); moveHoverChildId = getMoveChildren(child.id).length > 0 ? child.id : null">
        📁 {{ child.name }}
        <span v-if="getMoveChildren(child.id).length > 0" style="font-size:10px">▸</span>
      </div>
    </div>
  </Teleport>

  <!-- 三级子菜单 -->
  <Teleport to="body">
    <div v-if="ctxMenu.show && moveSubShow && moveHoverChildId && moveGrandchildren.length > 0"
      class="ctx-menu" :style="moveSub3Style" @mouseenter="cancelCloseMove">
      <div v-for="gc in moveGrandchildren" :key="gc.id" class="ctx-menu-item" @click="moveNoteToGroup(gc.id)">
        📁 {{ gc.name }}
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
  margin-bottom: 6px;
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;

  .card-title-text {
    font-size: 13px;
    font-weight: 600;
    color: var(--jc-text-highlight);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-time {
    font-size: 10px;
    color: var(--jc-text-secondary);
  }
}

.card-actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
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
    background: rgba(220, 38, 38, 0.1);
  }
}

// 纯文本摘要
.card-excerpt {
  font-size: 12px;
  color: var(--jc-text-secondary);
  line-height: 1.5;
  margin-bottom: 8px;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  word-break: break-word;
}

// 标签区（卡片主要视觉）
.card-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tag-badge {
  display: inline-block;
  font-size: 11px;
  color: var(--jc-color-accent);
  background: color-mix(in srgb, var(--jc-color-accent) 12%, transparent);
  padding: 2px 8px;
  border-radius: 10px;
  cursor: pointer;
  transition: background 0.15s;

  &:hover {
    background: color-mix(in srgb, var(--jc-color-accent) 25%, transparent);
  }
}

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
