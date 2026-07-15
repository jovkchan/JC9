<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'

const props = withDefaults(defineProps<{
  /** 左侧面板默认宽度 (px) */
  defaultSize?: number
  /** 左侧面板最小宽度 (px) */
  minSize?: number
  /** 左侧面板最大宽度 (px) */
  maxSize?: number
  /** 是否可折叠 */
  collapsible?: boolean
  /** localStorage 存储键（用于跨会话记住宽度） */
  storageKey?: string
  /** 是否禁用拖拽 */
  disabled?: boolean
}>(), {
  defaultSize: 280,
  minSize: 180,
  maxSize: 500,
  collapsible: true,
  storageKey: '',
  disabled: false,
})

const emit = defineEmits<{
  (e: 'resize', size: number): void
  (e: 'collapse'): void
  (e: 'expand'): void
}>()

// ── 面板宽度状态 ──
const collapsed = ref(false)
const savedExpandedSize = ref(props.defaultSize)

// 从 localStorage 恢复宽度
function loadStoredSize(): number {
  if (!props.storageKey) return props.defaultSize
  try {
    const stored = localStorage.getItem(props.storageKey)
    if (stored !== null) {
      const v = parseFloat(stored)
      if (!isNaN(v) && v >= props.minSize && v <= props.maxSize) return v
    }
  } catch { /* ignore */ }
  return props.defaultSize
}

const currentSize = ref(loadStoredSize())

// 持久化宽度
function persistSize(size: number) {
  if (!props.storageKey) return
  localStorage.setItem(props.storageKey, String(size))
}

// ── 拖拽状态 ──
const dragging = ref(false)
const startX = ref(0)
const startSize = ref(0)

function onBarMouseDown(e: MouseEvent) {
  if (props.disabled) return
  e.preventDefault()
  dragging.value = true
  startX.value = e.clientX
  startSize.value = currentSize.value
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function onMouseMove(e: MouseEvent) {
  if (!dragging.value) return
  const delta = e.clientX - startX.value
  const newSize = Math.min(props.maxSize, Math.max(props.minSize, startSize.value + delta))
  currentSize.value = newSize
  persistSize(newSize)
  emit('resize', newSize)
}

function onMouseUp() {
  if (!dragging.value) return
  dragging.value = false
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

function toggleCollapse() {
  if (!props.collapsible) return
  if (collapsed.value) {
    collapsed.value = false
    currentSize.value = savedExpandedSize.value
    emit('expand')
  } else {
    savedExpandedSize.value = currentSize.value
    collapsed.value = true
    emit('collapse')
  }
}

// 双击分隔条折叠/展开
function onBarDblClick() {
  toggleCollapse()
}

// ── 键盘快捷键：Ctrl+B 折叠/展开侧栏 ──
function onGlobalKeydown(e: KeyboardEvent) {
  if (props.disabled || !props.collapsible) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
    e.preventDefault()
    toggleCollapse()
  }
}

// ── 计算左侧面板实际宽度 ──
const leftPanelWidth = computed(() => {
  if (collapsed.value) return 0
  return currentSize.value
})

// ── 生命周期 ──
onMounted(() => {
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
  window.addEventListener('keydown', onGlobalKeydown)
})

onUnmounted(() => {
  document.removeEventListener('mousemove', onMouseMove)
  document.removeEventListener('mouseup', onMouseUp)
  window.removeEventListener('keydown', onGlobalKeydown)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
})

// 暴露方法给父组件调用
defineExpose({ toggleCollapse, collapsed, currentSize })
</script>

<template>
  <div
    class="split-panel"
    :class="{ dragging, collapsed: collapsed }"
  >
    <!-- 左侧面板 -->
    <div
      class="split-panel__left"
      :style="{
        width: leftPanelWidth + 'px',
        minWidth: collapsed ? '0' : undefined,
      }"
    >
      <slot name="left" />
    </div>

    <!-- 分隔条 -->
    <div
      v-if="!disabled"
      class="split-panel__bar"
      :class="{ 'is-collapsed': collapsed }"
      @mousedown="onBarMouseDown"
      @dblclick="onBarDblClick"
    >
      <button
        class="split-panel__toggle"
        :title="collapsed ? '展开侧栏 (Ctrl+B)' : '收起侧栏 (Ctrl+B)'"
        @click.stop="toggleCollapse"
        @mousedown.stop
      >
        <svg
          viewBox="0 0 16 16"
          width="12"
          height="12"
          fill="currentColor"
          :style="{ transform: collapsed ? 'rotate(180deg)' : 'rotate(0deg)' }"
        >
          <path d="M5.5 3.5L2 8l3.5 4.5M6 8h8" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>

    <!-- 右侧面板（主内容区） -->
    <div class="split-panel__right">
      <slot />
    </div>
  </div>
</template>

<style scoped lang="scss">
@use '@/styles/mixins.scss' as *;

.split-panel {
  display: flex;
  flex: 1;
  overflow: hidden;
  min-width: 0;

  &__left {
    flex-shrink: 0;
    overflow: hidden;
    min-height: 0;
    transition: width 0.15s ease;
  }

  &__right {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
    min-height: 0;
  }

  &__bar {
    position: relative;
    flex-shrink: 0;
    width: 1px;
    cursor: col-resize;
    background: var(--jc-border-default);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s ease;
    z-index: 10;

    &:hover,
    .split-panel.dragging & {
      background: var(--jc-color-accent);
    }

    // 扩大拖拽热区（视觉上 2px，实际可拖拽区域更宽）
    &::before {
      content: '';
      position: absolute;
      inset: 0;
      left: -6px;
      right: -6px;
    }

    &.is-collapsed {
      cursor: default;
      background: var(--jc-border-default);

      .split-panel__toggle {
        opacity: 0.6;
      }
    }
  }

  &__toggle {
    position: absolute;
    width: 15px;
    height: 38px;
    border: 1px solid var(--jc-border-default);
    border-radius: 4px;
    background: var(--jc-bg-elevated);
    color: var(--jc-text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    opacity: 0;
    transition: opacity 0.15s ease, color 0.15s ease;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);

    &:hover {
      color: var(--jc-text-highlight);
      background: var(--jc-bg-hover);
    }
  }

  &__bar:hover &__toggle,
  &__bar.is-collapsed &__toggle {
    opacity: 1;
  }

  // 折叠状态
  &.collapsed &__left {
    width: 0 !important;
    min-width: 0 !important;
  }
}

// 拖拽时的全局遮罩，防止 iframe 等元素捕获事件
.split-panel.dragging {
  * {
    pointer-events: none;
  }
}
</style>
