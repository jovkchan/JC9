<script setup lang="ts">
import { nextTick, ref } from 'vue'

defineOptions({ name: 'JcMenuList' })

// 递归菜单项渲染：支持 children 嵌套子菜单（悬停展开）、divider、danger、disabled
export interface JcMenuItem {
  /** 文案（divider 分割线项可省略） */
  label?: string
  icon?: string
  danger?: boolean
  disabled?: boolean
  divider?: boolean
  value?: string | number
  /** 子菜单（支持多级嵌套） */
  children?: JcMenuItem[]
}

const props = defineProps<{ items: JcMenuItem[] }>()
const emit = defineEmits<{
  select: [item: JcMenuItem, e: MouseEvent]
}>()

// 当前悬停展开的子菜单索引（带 120ms 延迟避免抖动/误触）
const openIndex = ref(-1)
let hoverTimer: ReturnType<typeof setTimeout> | null = null

function enterSub(i: number) {
  if (hoverTimer) clearTimeout(hoverTimer)
  hoverTimer = setTimeout(() => {
    openIndex.value = i
    void measureSubDir(i)
  }, 120)
}
function leaveSub() {
  if (hoverTimer) clearTimeout(hoverTimer)
  openIndex.value = -1
}
function onItemClick(item: JcMenuItem, e: MouseEvent) {
  if (item.disabled || item.divider || (item.children && item.children.length)) return
  emit('select', item, e)
}

// ── 子菜单展开方向：默认向右，右侧空间不足自动向左 ──
const wrapRefs = ref<(HTMLDivElement | null)[]>([])
const subDir = ref<Record<number, 'left' | 'right'>>({})

function setWrapRef(i: number) {
  return (el: unknown) => {
    wrapRefs.value[i] = (el as HTMLDivElement | null) ?? null
  }
}

async function measureSubDir(i: number) {
  await nextTick()
  const wrap = wrapRefs.value[i]
  if (!wrap) return
  const rect = wrap.getBoundingClientRect()
  const sub = wrap.querySelector('.jc-menu__sub') as HTMLElement | null
  const subW = sub ? sub.getBoundingClientRect().width : 160
  // 右侧空间不足且左侧足够宽 → 向左展开；否则保持向右
  if (rect.right + subW > window.innerWidth && rect.left - subW >= 4) {
    subDir.value[i] = 'left'
  } else {
    subDir.value[i] = 'right'
  }
}
</script>

<template>
  <template v-for="(item, i) in props.items" :key="i">
    <div v-if="item.divider" class="jc-menu__divider" />
    <div
      v-else
      :ref="(el) => setWrapRef(i)(el)"
      class="jc-menu__item-wrap"
      :class="{ 'has-children': !!(item.children && item.children.length) }"
      @mouseenter="item.children && item.children.length && enterSub(i)"
      @mouseleave="item.children && item.children.length && leaveSub()"
    >
      <button
        type="button"
        class="jc-menu__item"
        :class="{ 'is-danger': item.danger, 'is-disabled': item.disabled }"
        :disabled="item.disabled"
        @click="onItemClick(item, $event)"
      >
        <span v-if="item.icon" class="jc-menu__icon">{{ item.icon }}</span>
        <span class="jc-menu__label">{{ item.label }}</span>
        <span v-if="item.children && item.children.length" class="jc-menu__arrow" aria-hidden="true">▸</span>
      </button>
      <Transition name="jc-menu">
        <div
          v-if="item.children && item.children.length && openIndex === i"
          :class="['jc-menu__sub', { 'is-left': subDir[i] === 'left' }]"
          @mouseenter="enterSub(i)"
          @mouseleave="leaveSub()"
        >
          <JcMenuList :items="item.children" @select="(it, e) => emit('select', it, e)" />
        </div>
      </Transition>
    </div>
  </template>
</template>

<style scoped>
.jc-menu__item-wrap {
  position: relative;
}
.jc-menu__item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 14px;
  border: none;
  background: transparent;
  color: var(--jc-text-primary, #ccc);
  font-family: inherit;
  font-size: var(--jc-font-size, 13px);
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}
.jc-menu__item:hover:not(:disabled) {
  background: var(--jc-bg-hover, #2a2d2e);
}
.jc-menu__item.is-danger {
  color: var(--jc-color-error, #f44747);
}
.jc-menu__item.is-disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.jc-menu__icon {
  width: 16px;
  text-align: center;
  flex-shrink: 0;
}
.jc-menu__label {
  flex: 1;
  min-width: 0;
}
.jc-menu__arrow {
  color: var(--jc-text-secondary, #858585);
  font-size: 10px;
  flex-shrink: 0;
}
.jc-menu__divider {
  height: 1px;
  margin: 4px 0;
  background: var(--jc-border-default, #3e3e42);
}

.jc-menu__sub {
  position: absolute;
  left: 100%;
  top: -4px;
  min-width: 140px;
  padding: 4px 0;
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: var(--jc-radius, 6px);
  box-shadow: var(--jc-shadow-menu, 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 6px 16px 0 rgba(0, 0, 0, 0.08));
  z-index: 1;
}
.jc-menu__sub.is-left {
  left: auto;
  right: 100%;
}

.jc-menu-enter-active, .jc-menu-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}
.jc-menu-enter-from, .jc-menu-leave-to {
  opacity: 0;
  transform: scale(0.96);
}
</style>
