<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import JcMenuList from './JcMenuList.vue'
import type { JcMenuItem } from './JcMenuList.vue'

defineOptions({ name: 'JcDropdown' })

// API 对齐 Ant Design Dropdown：items / placement / trigger / disabled（支持 children 嵌套子菜单）
// 参考: https://ant.design/components/dropdown-cn
export interface JcDropdownItem extends JcMenuItem {
  key?: string | number
}

const props = withDefaults(
  defineProps<{
    items?: JcDropdownItem[]
    placement?: 'bottomLeft' | 'bottom' | 'bottomRight' | 'topLeft' | 'top' | 'topRight'
    trigger?: 'hover' | 'click'
    disabled?: boolean
  }>(),
  {
    items: () => [],
    placement: 'bottomLeft',
    trigger: 'hover',
    disabled: false,
  },
)

const emit = defineEmits<{
  select: [item: JcDropdownItem]
  'update:open': [value: boolean]
}>()

const open = ref(false)
const rootRef = ref<HTMLElement | null>(null)

function onGlobalDown(e: MouseEvent) {
  const el = rootRef.value
  if (el && !el.contains(e.target as Node)) open.value = false
}
function show() {
  if (!props.disabled) open.value = true
}
function hide() {
  open.value = false
}
function toggle() {
  if (props.disabled) return
  open.value = !open.value
}

watch(open, (v) => {
  if (v) document.addEventListener('mousedown', onGlobalDown)
  else document.removeEventListener('mousedown', onGlobalDown)
})
onBeforeUnmount(() => document.removeEventListener('mousedown', onGlobalDown))

function onItem(item: JcDropdownItem) {
  if (item.disabled || item.divider) return
  emit('select', item)
  open.value = false
}
</script>

<template>
  <span
    ref="rootRef"
    :class="['jc-dropdown', `is-${placement}`]"
    @mouseleave="trigger === 'hover' && hide()"
  >
    <span
      class="jc-dropdown__trigger"
      @mouseenter="trigger === 'hover' && show()"
      @click="trigger === 'click' && toggle()"
    >
      <slot />
    </span>
    <Transition name="jc-menu">
      <div v-if="open" :class="['jc-dropdown__menu', `is-${placement}`]">
        <JcMenuList :items="items" @select="onItem" />
      </div>
    </Transition>
  </span>
</template>

<style scoped>
.jc-dropdown {
  position: relative;
  display: inline-flex;
}
.jc-dropdown__trigger {
  display: inline-flex;
  cursor: pointer;
}
.jc-dropdown__menu {
  position: absolute;
  z-index: var(--jc-z-index-popup, 1000);
  min-width: 140px;
  padding: 4px 0;
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: var(--jc-radius, 6px);
  box-shadow: var(--jc-shadow-menu, 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 6px 16px 0 rgba(0, 0, 0, 0.08));
}
.jc-dropdown.is-bottomLeft .jc-dropdown__menu,
.jc-dropdown__menu.is-bottomLeft { top: calc(100% + 4px); left: 0; }
.jc-dropdown.is-bottom .jc-dropdown__menu,
.jc-dropdown__menu.is-bottom { top: calc(100% + 4px); left: 50%; transform: translateX(-50%); }
.jc-dropdown.is-bottomRight .jc-dropdown__menu,
.jc-dropdown__menu.is-bottomRight { top: calc(100% + 4px); right: 0; }
.jc-dropdown.is-topLeft .jc-dropdown__menu,
.jc-dropdown__menu.is-topLeft { bottom: calc(100% + 4px); left: 0; }
.jc-dropdown.is-top .jc-dropdown__menu,
.jc-dropdown__menu.is-top { bottom: calc(100% + 4px); left: 50%; transform: translateX(-50%); }
.jc-dropdown.is-topRight .jc-dropdown__menu,
.jc-dropdown__menu.is-topRight { bottom: calc(100% + 4px); right: 0; }

.jc-menu-enter-active, .jc-menu-leave-active { transition: opacity 0.12s ease, transform 0.12s ease; }
.jc-menu-enter-from, .jc-menu-leave-to { opacity: 0; transform: scale(0.96); }
</style>
