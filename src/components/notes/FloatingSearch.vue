<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { highlightKeywords, makeSnippet, autoTitle } from '@/utils/searchHighlight'
import JcSearchIcon from '@/components/ui/JcSearchIcon.vue'
import type { Note } from '@/types/notes'

const store = useNotesStore()
const inputRef = ref<HTMLInputElement | null>(null)

// 即时下拉结果（全局搜索 Top10）
const results = computed(() => store.searchResults.slice(0, 10))
const selectedIdx = ref(0)

// 监听全局 showSearchPanel 状态以聚焦输入框
watch(
  () => store.showSearchPanel,
  (show) => {
    if (show) {
      selectedIdx.value = 0
      nextTick(() => {
        inputRef.value?.focus()
        inputRef.value?.select()
      })
    }
  }
)

// 查询变化时重置选中项
watch(
  () => store.searchQuery,
  () => { selectedIdx.value = 0 }
)

function closeSearch() {
  store.showSearchPanel = false
}

function clearSearch() {
  store.searchQuery = ''
  inputRef.value?.focus()
}

function selectResult(note: Note) {
  store.openNoteTab(note.id)
  closeSearch()
}

/** 进入独立搜索 Tab 页 */
function openSearchPage() {
  store.openSearchTab()
  closeSearch()
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    closeSearch()
  }
  if (e.key === 'ArrowDown' && results.value.length > 0) {
    e.preventDefault()
    selectedIdx.value = (selectedIdx.value + 1) % results.value.length
  }
  if (e.key === 'ArrowUp' && results.value.length > 0) {
    e.preventDefault()
    selectedIdx.value = (selectedIdx.value - 1 + results.value.length) % results.value.length
  }
  if (e.key === 'Enter') {
    e.preventDefault()
    if (results.value.length > 0) {
      selectResult(results.value[selectedIdx.value])
    } else {
      openSearchPage() // 无结果也进入搜索页，便于调整关键词
    }
  }
}

function noteTitle(n: Note): string {
  return n.title || autoTitle(n.content) || '无标题'
}

// 点击外部关闭搜索框
const searchPanelRef = ref<HTMLElement | null>(null)
function handleOutsideClick(e: MouseEvent) {
  if (!store.showSearchPanel) return
  if (searchPanelRef.value && !searchPanelRef.value.contains(e.target as Node)) {
    closeSearch()
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handleOutsideClick)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleOutsideClick)
})
</script>

<template>
  <Transition name="fade-slide">
    <div v-if="store.showSearchPanel" class="floating-search-wrapper" role="dialog" aria-modal="true">
      <div ref="searchPanelRef" class="floating-search-panel">
        <!-- 输入行 -->
        <div class="floating-search-row">
          <span class="search-icon"><JcSearchIcon :size="13" /></span>
          <input
            ref="inputRef"
            v-model="store.searchQuery"
            type="text"
            placeholder="搜索笔记 (空格分隔多词，支持 is:pinned, tag:标签)..."
            class="search-input-box"
            @keydown="handleKeydown"
          />
          <button v-if="store.searchQuery" class="clear-btn" @click="clearSearch" title="清空">✕</button>
          <button class="close-btn" @click="closeSearch" title="关闭 (Esc)">
            <span class="close-btn-key">Esc</span>
          </button>
        </div>

        <!-- 即时下拉结果（仅结果项，可滚动） -->
        <div v-if="store.searchQuery.trim()" class="floating-search-results">
          <div v-if="results.length === 0" class="fsr-empty">无匹配结果，回车进入搜索页</div>
          <div
            v-for="(note, i) in results"
            :key="note.id"
            class="fsr-item"
            :class="{ sel: i === selectedIdx }"
            @click="selectResult(note)"
            @mouseenter="selectedIdx = i"
          >
            <div class="fsr-title" v-html="highlightKeywords(noteTitle(note), store.searchQuery)"></div>
            <div class="fsr-snippet" v-html="highlightKeywords(makeSnippet(note.content, store.searchQuery, 90), store.searchQuery)"></div>
          </div>
        </div>

        <!-- 底部固定操作栏（不随结果滚动） -->
        <div v-if="store.searchQuery.trim() && results.length > 0" class="fsr-footer" @click="openSearchPage">
          <span>共 {{ store.searchResults.length }} 条结果</span>
          <span class="fsr-footer-go">在搜索页查看全部 →</span>
        </div>

        <span class="search-hint">↑↓ 选择 · Enter 打开 / 进搜索页 · Esc 关闭</span>
      </div>
    </div>
  </Transition>
</template>

<style scoped lang="scss">
.floating-search-wrapper {
  position: fixed;
  top: 60px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1500;
  pointer-events: none; // 允许点击穿透到遮罩层以外的区域
}

.floating-search-panel {
  pointer-events: auto; // 自身区域阻断点击穿透
  display: flex;
  flex-direction: column;
  background: rgba(23, 23, 26, 0.75);
  backdrop-filter: blur(12px) saturate(180%);
  -webkit-backdrop-filter: blur(12px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 8px 12px;
  border-radius: 8px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3), 
              0 0 1px rgba(255, 255, 255, 0.2) inset,
              0 0 10px rgba(0, 102, 204, 0.1);
  width: 460px;
  max-width: 90vw;
  gap: 6px;
  position: relative;
  transition: border-color 0.25s, box-shadow 0.25s;

  &:focus-within {
    border-color: var(--jc-color-accent, #0066cc);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.35),
                0 0 12px rgba(0, 102, 204, 0.2);
  }
}

.floating-search-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

/* 即时下拉结果（仅结果项，可滚动） */
.floating-search-results {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 320px;
  overflow-y: auto;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  padding-top: 4px;
}
.fsr-item {
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.12s;
  &.sel {
    background: rgba(138, 88, 255, 0.18);
  }
  &:hover { background: rgba(255, 255, 255, 0.06); }
}
.fsr-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--jc-text-primary, #fff);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fsr-snippet {
  margin-top: 2px;
  font-size: 11px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fsr-empty {
  padding: 10px 8px;
  font-size: 11.5px;
  color: var(--jc-text-secondary);
  opacity: 0.8;
}
.fsr-footer {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 8px 5px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  font-size: 11px;
  color: var(--jc-text-secondary);
  cursor: pointer;
  border-radius: 6px;
  &:hover { background: rgba(255, 255, 255, 0.05); }
}
.fsr-footer-go {
  color: var(--jc-color-accent, #0066cc);
  font-weight: 600;
}
:deep(mark) {
  background: rgba(138, 88, 255, 0.28);
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}

.search-icon {
  font-size: 13px;
  color: var(--jc-text-secondary);
  opacity: 0.8;
  flex-shrink: 0;
}

.search-input-box {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--jc-text-primary, #fff);
  font-size: 12px;
  outline: none;
  font-family: inherit;
  min-width: 0;

  &::placeholder {
    color: var(--jc-text-secondary);
    opacity: 0.6;
  }
}

.clear-btn, .close-btn {
  background: transparent;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 11px;
  cursor: pointer;
  width: 30px;
  height: 30px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.15s;

  &:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--jc-text-primary);
  }
}

/* 关闭按钮：DIV 模拟的 ESC 键帽（替代图标） */
.close-btn {
  width: auto;
  height: auto;
  padding: 0 2px;
}
.close-btn-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 20px;
  min-width: 32px;
  padding: 0 6px;
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.5px;
  color: var(--jc-text-secondary, #858585);
  background: var(--jc-bg-input, #3c3c3c);
  user-select: none;
  transition: border-color 0.15s, color 0.15s, background-color 0.15s;
}
.close-btn:hover .close-btn-key {
  border-color: var(--jc-color-accent-hover, #a070ff);
  color: var(--jc-text-primary, #ccc);
}

.clear-btn {
  margin-right: -4px;
}

.close-btn {
  font-size: 12px;
  &:hover {
    color: var(--jc-color-error, #dc2626);
    background: rgba(220, 38, 38, 0.1);
  }
}

.search-hint {
  position: absolute;
  bottom: -20px;
  right: 6px;
  font-size: 9px;
  color: var(--jc-text-secondary);
  opacity: 0.5;
  user-select: none;
}

// 过渡效果: 顶部滑入与淡出
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translate(-50%, -15px);
}
</style>
