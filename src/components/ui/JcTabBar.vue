<script setup lang="ts">
import { ref } from 'vue'
import JcContextMenu from './JcContextMenu.vue'
import type { JcContextMenuItem } from './JcContextMenu.vue'

defineOptions({ name: 'JcTabBar' })

// API 对齐「笔记标签页 + 右键菜单」基准
export interface JcTabItem {
  key: string | number
  label: string
  /** 运行状态点（绿色小圆点，终端标签用）；不传则不显示 */
  live?: boolean
  /** 是否显示 ✕ 关闭按钮，默认 true */
  closable?: boolean
}

withDefaults(
  defineProps<{
    tabs: JcTabItem[]
    /** 当前激活标签 key（null 表示无激活） */
    activeKey?: string | number | null
    /** 自定义右键菜单项；不传则用默认「笔记标签页」风格菜单 */
    contextItems?: JcContextMenuItem[]
    /** 禁止右键菜单 */
    noContext?: boolean
  }>(),
  {
    activeKey: null,
    contextItems: undefined,
    noContext: false,
  },
)

const emit = defineEmits<{
  'update:activeKey': [value: string | number | null]
  select: [key: string | number]
  close: [key: string | number]
  /** 右键菜单选中（value 分发由父组件处理） */
  'context-select': [item: JcContextMenuItem, tab: JcTabItem]
}>()

// 默认右键菜单（对齐笔记标签页基准：刷新/关闭/关闭其他/右侧/左侧/全部）
const defaultContextItems: JcContextMenuItem[] = [
  { label: '刷新', value: 'refresh' },
  { label: '关闭', value: 'close' },
  { label: '关闭其他', value: 'closeOthers' },
  { label: '关闭右侧标签页', value: 'closeRight' },
  { label: '关闭左侧标签页', value: 'closeLeft' },
  { label: '全部关闭', value: 'closeAll' },
]

const ctxShow = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxTab = ref<JcTabItem | null>(null)

function onSelect(tab: JcTabItem) {
  emit('update:activeKey', tab.key)
  emit('select', tab.key)
}

function onClose(tab: JcTabItem) {
  emit('close', tab.key)
}

function openCtx(e: MouseEvent, tab: JcTabItem) {
  e.preventDefault()
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxTab.value = tab
  ctxShow.value = true
}

function onCtxSelect(item: JcContextMenuItem) {
  if (ctxTab.value) emit('context-select', item, ctxTab.value)
  ctxShow.value = false
}
</script>

<template>
  <div class="jc-tabbar" role="tablist">
    <div
      v-for="tab in tabs"
      :key="tab.key"
      :class="['jc-tab', { on: activeKey === tab.key }]"
      role="tab"
      :aria-selected="activeKey === tab.key"
      tabindex="0"
      @click="onSelect(tab)"
      @keyup.enter="onSelect(tab)"
      @contextmenu="noContext ? undefined : openCtx($event, tab)"
    >
      <span v-if="tab.live" class="jc-tab__dot" :class="{ live: tab.live }"></span>
      <span class="jc-tab__label">{{ tab.label }}</span>
      <button
        v-if="tab.closable !== false"
        class="jc-tab__close"
        aria-label="关闭标签"
        @click.stop="onClose(tab)"
      >✕</button>
    </div>
    <JcContextMenu
      :show="ctxShow"
      :x="ctxPos.x"
      :y="ctxPos.y"
      :items="contextItems ?? defaultContextItems"
      @select="onCtxSelect"
      @update:show="ctxShow = $event"
    />
  </div>
</template>

<style scoped>
/* Chrome 风格标签页：顶部圆角 + 激活标签亮色突出 */
.jc-tabbar {
  display: flex;
  align-items: flex-end;          /* 标签贴底，激活标签与内容区融为一体 */
  background: var(--jc-bg-app, #1e1e1e); /* 轨道 = 未激活标签颜色 */
  overflow-x: auto;
  flex-shrink: 0;
  padding: 6px 8px 0;
}
.jc-tab {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 14px 8px;
  font-size: 12px;
  cursor: pointer;
  color: var(--jc-text-secondary, #858585);
  border-radius: 10px 10px 0 0;   /* 顶部圆角 */
  white-space: nowrap;
  user-select: none;
  background: var(--jc-bg-app, #1e1e1e); /* 未激活 = 与轨道同色，融入背景 */
  transition: background 0.15s ease, color 0.15s ease;
}
.jc-tab:hover {
  color: var(--jc-text-primary, #ccc);
  background: var(--jc-bg-panel, #252526);
}
.jc-tab.on {
  color: var(--jc-text-highlight, #e0e0e0);
  background: var(--jc-bg-hover, #2a2d2e); /* 激活 = 原未激活颜色，亮色突出 */
}
.jc-tab__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--jc-border-strong, #555);
  flex-shrink: 0;
}
.jc-tab__dot.live {
  background: var(--jc-color-success, #4ec9b0);
}
.jc-tab__label {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.jc-tab__close {
  background: none;
  border: none;
  color: var(--jc-text-secondary, #858585);
  font-size: 14px;
  padding: 0 4px;
  cursor: pointer;
  flex-shrink: 0;
}
.jc-tab__close:hover {
  color: var(--jc-color-error, #f14c4c);
}
</style>
