<script setup lang="ts">
import { computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import JcInput from '@/components/ui/JcInput.vue'
import JcSearchIcon from '@/components/ui/JcSearchIcon.vue'
import { highlightKeywords, makeSnippet, autoTitle } from '@/utils/searchHighlight'
import type { Note } from '@/types/notes'

defineOptions({ name: 'NoteSearchView' })

const store = useNotesStore()

// 搜索框与 store 双向同步，输入即实时过滤
const query = computed({
  get: () => store.searchQuery,
  set: (v) => (store.searchQuery = v),
})

function noteTitle(n: Note): string {
  return n.title || autoTitle(n.content) || '无标题'
}

function titleHtml(n: Note): string {
  return highlightKeywords(noteTitle(n), store.searchQuery)
}

function snippetHtml(n: Note): string {
  return highlightKeywords(makeSnippet(n.content, store.searchQuery), store.searchQuery)
}

function formatTime(iso: string): string {
  const date = new Date(iso)
  const now = Date.now()
  const diff = now - date.getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return '刚刚'
  if (mins < 60) return `${mins} 分钟前`
  const hours = Math.floor(mins / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days} 天前`
  return date.toLocaleDateString()
}

function openNote(n: Note) {
  store.openNoteTab(n.id)
}
</script>

<template>
  <div class="note-search-view">
    <!-- 顶部搜索栏（百度式居中大搜索框） -->
    <div class="nsv-bar">
      <div class="nsv-box">
        <span class="nsv-box-icon"><JcSearchIcon :size="13" /></span>
        <JcInput
          v-model="query"
          beam
          glow
          class="nsv-input"
          placeholder="搜索笔记标题 / 内容 / 标签… (支持空格分隔多词、is:pinned、tag:标签)"
        />
        <button v-if="query" class="nsv-clear" title="清空" @click="query = ''">✕</button>
      </div>
    </div>

    <!-- 结果统计 -->
    <div class="nsv-meta">
      共找到 <b>{{ store.searchResults.length }}</b> 条结果
      <span v-if="query" class="nsv-meta-q">「{{ query }}」</span>
    </div>

    <!-- 结果列表 -->
    <div class="nsv-results">
      <div v-if="store.searchResults.length === 0" class="nsv-empty">
        <div class="nsv-empty-icon"><JcSearchIcon :size="32" /></div>
        <div class="nsv-empty-title">未找到匹配的笔记</div>
        <div class="nsv-empty-sub">换个关键词，或清空搜索查看全部笔记</div>
      </div>

      <div
        v-for="n in store.searchResults"
        :key="n.id"
        class="nsv-item"
        @click="openNote(n)"
      >
        <div class="nsv-title" v-html="titleHtml(n)"></div>
        <div class="nsv-snippet" v-html="snippetHtml(n)"></div>
        <div class="nsv-meta-row">
          <span v-if="n.isPinned" class="nsv-badge nsv-badge--pin">置顶</span>
          <span v-for="t in n.tags.slice(0, 4)" :key="t" class="nsv-badge nsv-badge--tag">#{{ t }}</span>
          <span v-if="n.tags.length > 4" class="nsv-badge nsv-badge--more">+{{ n.tags.length - 4 }}</span>
          <span class="nsv-time">{{ formatTime(n.updatedAt || n.createdAt) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.note-search-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* 顶部搜索栏 */
.nsv-bar {
  flex: none;
  padding: 16px 24px 8px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  display: flex;
  justify-content: center;
}
.nsv-box {
  position: relative;
  width: 100%;
  max-width: 760px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.nsv-box-icon {
  flex: none;
  font-size: 14px;
  opacity: 0.8;
}
.nsv-input {
  flex: 1;
  min-width: 0;
}
.nsv-clear {
  flex: none;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--jc-text-secondary, #858585);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;
  &:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--jc-text-primary, #ccc);
  }
}

/* 统计行 */
.nsv-meta {
  flex: none;
  padding: 10px 24px 4px;
  font-size: 12px;
  color: var(--jc-text-secondary, #858585);
  b { color: var(--jc-color-accent, #8a58ff); }
}
.nsv-meta-q {
  margin-left: 6px;
  opacity: 0.7;
}

/* 结果列表（滚动区） */
.nsv-results {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* 空态 */
.nsv-empty {
  padding: 64px 0;
  text-align: center;
  color: var(--jc-text-secondary, #858585);
}
.nsv-empty-icon { font-size: 32px; opacity: 0.5; }
.nsv-empty-title { margin-top: 12px; font-size: 14px; color: var(--jc-text-primary, #ccc); }
.nsv-empty-sub { margin-top: 6px; font-size: 12px; opacity: 0.7; }

/* 结果项（百度式） */
.nsv-item {
  padding: 12px 16px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.15s var(--jc-motion-ease, ease);
  &:hover {
    background: var(--jc-bg-btn-hover, rgba(255, 255, 255, 0.04));
  }
  &:active {
    background: var(--jc-bg-selected, #37373d);
  }
}
.nsv-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--jc-color-accent, #8a58ff);
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.nsv-snippet {
  margin-top: 4px;
  font-size: 12px;
  line-height: 1.7;
  color: var(--jc-text-secondary, #858585);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}
.nsv-meta-row {
  margin-top: 6px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}
.nsv-badge {
  font-size: 11px;
  line-height: 1;
  padding: 2px 6px;
  border-radius: 3px;
  background: var(--jc-bg-btn-hover, rgba(255, 255, 255, 0.06));
  color: var(--jc-text-secondary, #858585);
}
.nsv-badge--pin {
  background: rgba(138, 88, 255, 0.15);
  color: var(--jc-color-accent, #8a58ff);
}
.nsv-badge--tag {
  background: rgba(90, 162, 255, 0.12);
  color: var(--jc-color-info, #5aa2ff);
}
.nsv-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--jc-text-secondary, #858585);
  opacity: 0.7;
}

/* 高亮样式 */
:deep(mark) {
  background: rgba(138, 88, 255, 0.28);
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}
</style>
