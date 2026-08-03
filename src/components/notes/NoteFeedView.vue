<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { invoke } from '@tauri-apps/api/core'
import type { Note, NoteGroup } from '@/types/notes'
import JcContextMenu from '@/components/ui/JcContextMenu.vue'
import type { JcContextMenuItem } from '@/components/ui/JcContextMenu.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const store = useNotesStore()

// 右键菜单状态（JcContextMenu 负责定位 / 全局关闭 / 嵌套子菜单）
const ctxMenu = ref({ show: false, x: 0, y: 0, noteId: '' })

// 分组 → 菜单树（递归构造 children，交给 JcMenuList 多级嵌套渲染）
function getMoveChildren(parentId: string) { return store.groups.filter(g => g.parentId === parentId).sort((a, b) => a.sortOrder - b.sortOrder) }
function buildGroupMenu(groups: NoteGroup[]): JcContextMenuItem[] {
  return groups.map(g => {
    const children = buildGroupMenu(getMoveChildren(g.id))
    return { label: g.name, icon: '📁', value: g.id, children: children.length ? children : undefined }
  })
}
const moveMenuItems = computed<JcContextMenuItem[]>(() => [
  { label: '根目录', icon: '📁', value: '' },
  { divider: true },
  ...buildGroupMenu(store.groups.filter(g => !g.parentId).sort((a, b) => a.sortOrder - b.sortOrder)),
])

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

function onCtxSelect(item: JcContextMenuItem) {
  // 根目录 value 为空字符串 → 移动到 null；分组 value 为分组 id
  void moveNoteToGroup(item.value ? String(item.value) : null)
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
        <JcInput beam v-model="newTitle" placeholder="输入备忘标题（可选）..." />
      </div>
      <JcTextarea
        ref="textareaRef"
        v-model="newContent"
        placeholder="写点什么... 支持 Markdown 语法与 #标签 (Ctrl+Enter 发布)"
        class="memo-textarea"
        @keydown="handleKeydown"
      />
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
            <button class="act-btn pin" :class="{on: note.isPinned}" @click.stop="store.togglePin(note.id)" title="置顶"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M855.872 106.432a42.464 42.464 0 0 1-42.464 42.464H203.44a42.464 42.464 0 0 1 0-84.928h609.968a42.464 42.464 0 0 1 42.464 42.464z m-344.048 157.92a42.464 42.464 0 0 0-42.464 42.464v609.968a42.464 42.464 0 0 0 84.928 0V306.816a42.464 42.464 0 0 0-42.464-42.464z m30.144-31.328c-16.592-16.576-42.528-17.536-57.92-2.128L171.232 543.68c-15.408 15.408-14.448 41.344 2.128 57.92 16.592 16.592 42.512 17.536 57.92 2.128l312.8-312.784c15.392-15.408 14.448-41.344-2.128-57.92z m-60.272 0c-16.576 16.576-17.536 42.512-2.128 57.92l312.8 312.8c15.392 15.392 41.328 14.448 57.92-2.144 16.576-16.576 17.52-42.512 2.112-57.92L539.616 230.896c-15.408-15.408-41.344-14.448-57.92 2.128z"/></svg></button>
            <button class="act-btn" @click.stop="store.copyContent(note.id)" title="复制正文"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M281.6 32h374.464a70.4 70.4 0 0 1 49.792 20.608l201.536 201.536a70.4 70.4 0 0 1 20.608 49.792V806.4a57.6 57.6 0 0 1-57.6 57.6H281.6a57.6 57.6 0 0 1-57.6-57.6V89.6a57.6 57.6 0 0 1 57.6-57.6z m19.2 768h550.4a12.8 12.8 0 0 0 12.8-12.8V303.936a6.4 6.4 0 0 0-0.512-2.496l-1.344-2.048-201.536-201.536a6.4 6.4 0 0 0-4.48-1.856H300.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8z"/><path d="M256 160v64H172.8a12.8 12.8 0 0 0-12.8 12.8v678.4c0 7.04 5.76 12.8 12.8 12.8h550.4a12.8 12.8 0 0 0 12.8-12.8V832h64v102.4a57.6 57.6 0 0 1-57.6 57.6H153.6a57.6 57.6 0 0 1-57.6-57.6V217.6a57.6 57.6 0 0 1 57.6-57.6H256zM672 64v211.2c0 7.04 5.76 12.8 12.8 12.8H896v64h-243.2a44.8 44.8 0 0 1-44.8-44.8V64h64z"/></svg></button>
            <button class="act-btn delete" @click.stop="store.removeNote(note.id)" title="删除"><svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M301.382 184.46h46.545v1.722h-46.545v-1.722z m186.135 0h46.546v1.722h-46.546v-1.722zM208.244 1024h605.091l93.091-837.818H720.105l-46.406 744.727h-46.546l46.406-744.727H534.063v744.727h-46.546V186.182H347.974l46.452 744.727h-46.545l-46.453-744.727H115.153z m465.408-839.54h46.546v1.722h-46.546v-1.722z m280.53-91.37c0-46.545-23.32-46.545-23.32-46.545H627.154S627.153 0 580.608 0H440.972c-46.546 0-46.546 46.545-46.546 46.545H93.137s-23.319 0-23.319 46.546c0 46.545 23.32 46.545 23.32 46.545h837.725s23.319 0 23.319-46.545z"/></svg></button>
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

  <!-- 右键菜单：移动笔记到分组（JcContextMenu + JcMenuList 递归嵌套子菜单） -->
  <JcContextMenu
    v-model:show="ctxMenu.show"
    :x="ctxMenu.x"
    :y="ctxMenu.y"
    :items="moveMenuItems"
    @select="onCtxSelect"
  />
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
</style>
