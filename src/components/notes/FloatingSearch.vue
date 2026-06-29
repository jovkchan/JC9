<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { useNotesStore } from '@/stores/notes'

const store = useNotesStore()
const inputRef = ref<HTMLInputElement | null>(null)

// 监听全局 showSearchPanel 状态以聚焦输入框
watch(
  () => store.showSearchPanel,
  (show) => {
    if (show) {
      nextTick(() => {
        inputRef.value?.focus()
        inputRef.value?.select()
      })
    }
  }
)

function closeSearch() {
  store.showSearchPanel = false
}

function clearSearch() {
  store.searchQuery = ''
  inputRef.value?.focus()
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    closeSearch()
  }
  if (e.key === 'Enter') {
    e.preventDefault()
    closeSearch() // 按回车搜索并关闭浮动面板
  }
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
        <span class="search-icon">🔍</span>
        <input
          ref="inputRef"
          v-model="store.searchQuery"
          type="text"
          placeholder="搜索笔记 (空格分隔多词，支持 is:pinned, tag:标签)..."
          class="search-input-box"
          @keydown="handleKeydown"
        />
        <button v-if="store.searchQuery" class="clear-btn" @click="clearSearch" title="清空">✕</button>
        <button class="close-btn" @click="closeSearch" title="关闭 (Esc)">✕</button>
        <span class="search-hint">Ctrl+F 激活 / Esc 隐藏</span>
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
  align-items: center;
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
  gap: 8px;
  position: relative;
  transition: border-color 0.25s, box-shadow 0.25s;

  &:focus-within {
    border-color: var(--jc-color-accent, #0066cc);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.35),
                0 0 12px rgba(0, 102, 204, 0.2);
  }
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
  width: 18px;
  height: 18px;
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
