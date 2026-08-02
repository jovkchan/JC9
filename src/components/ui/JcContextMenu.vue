<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import JcMenuList from './JcMenuList.vue'
import type { JcMenuItem } from './JcMenuList.vue'

defineOptions({ name: 'JcContextMenu' })

// 支持 children 嵌套子菜单（复用 JcMenuList 递归渲染）
export interface JcContextMenuItem extends JcMenuItem {}

const props = withDefaults(
  defineProps<{
    show?: boolean
    x?: number
    y?: number
    items?: JcContextMenuItem[]
    zIndex?: number
  }>(),
  {
    show: false,
    x: 0,
    y: 0,
    items: () => [],
    zIndex: 9999,
  },
)

const emit = defineEmits<{
  'update:show': [value: boolean]
  select: [item: JcContextMenuItem, e: MouseEvent]
  close: []
}>()

const menuRef = ref<HTMLDivElement | null>(null)
const pos = ref({ x: props.x, y: props.y })

// 溢出视口时自动翻转定位
watch(
  () => props.show,
  async (v) => {
    if (!v) return
    await nextTick()
    const el = menuRef.value
    if (!el) return
    const rect = el.getBoundingClientRect()
    let x = props.x
    let y = props.y
    if (x + rect.width > window.innerWidth) x = Math.max(0, window.innerWidth - rect.width - 4)
    if (y + rect.height > window.innerHeight) y = Math.max(0, window.innerHeight - rect.height - 4)
    pos.value = { x, y }
  },
)
watch(
  () => [props.x, props.y],
  () => {
    pos.value = { x: props.x, y: props.y }
  },
)

function close() {
  emit('update:show', false)
  emit('close')
}
function onGlobalDown(e: MouseEvent) {
  const el = menuRef.value
  if (!el) return
  if (!el.contains(e.target as Node)) close()
}
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}
function onScroll() {
  close()
}

watch(
  () => props.show,
  (v) => {
    if (v) {
      document.addEventListener('mousedown', onGlobalDown)
      document.addEventListener('keydown', onKeydown)
      window.addEventListener('scroll', onScroll, true)
    } else {
      document.removeEventListener('mousedown', onGlobalDown)
      document.removeEventListener('keydown', onKeydown)
      window.removeEventListener('scroll', onScroll, true)
    }
  },
)
onMounted(() => {
  if (props.show) {
    document.addEventListener('mousedown', onGlobalDown)
    document.addEventListener('keydown', onKeydown)
    window.addEventListener('scroll', onScroll, true)
  }
})
onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onGlobalDown)
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('scroll', onScroll, true)
})

function onItemClick(item: JcContextMenuItem, e: MouseEvent) {
  if (item.disabled || item.divider) return
  emit('select', item, e)
  close()
}
</script>

<template>
  <Teleport to="body">
    <Transition name="jc-menu">
      <div
        v-if="show"
        ref="menuRef"
        class="jc-context-menu"
        :style="{ left: pos.x + 'px', top: pos.y + 'px', zIndex }"
        @contextmenu.prevent
      >
        <JcMenuList :items="items" @select="onItemClick" />
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.jc-context-menu {
  position: fixed;
  min-width: 140px;
  padding: 4px 0;
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-strong, #555);
  border-radius: var(--jc-radius, 6px);
  box-shadow: var(--jc-shadow-menu, 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 6px 16px 0 rgba(0, 0, 0, 0.08));
}

.jc-menu-enter-active, .jc-menu-leave-active { transition: opacity 0.12s ease, transform 0.12s ease; }
.jc-menu-enter-from, .jc-menu-leave-to { opacity: 0; transform: scale(0.96); }
</style>
