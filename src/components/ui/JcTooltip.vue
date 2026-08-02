<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue'

defineOptions({ name: 'JcTooltip' })

// API 对齐 Ant Design Tooltip：title / placement / trigger / disabled
// 参考: https://ant.design/components/tooltip-cn
withDefaults(
  defineProps<{
    title?: string
    placement?: 'top' | 'bottom' | 'left' | 'right'
    trigger?: 'hover' | 'click' | 'focus'
    disabled?: boolean
    /** 显示延迟 ms */
    delay?: number
  }>(),
  {
    title: '',
    placement: 'top',
    trigger: 'hover',
    disabled: false,
    delay: 100,
  },
)

const visible = ref(false)
let timer: ReturnType<typeof setTimeout> | null = null

function clearTimer() {
  if (timer) {
    clearTimeout(timer)
    timer = null
  }
}
function show() {
  clearTimer()
  timer = setTimeout(() => {
    visible.value = true
  }, 100)
}
function hide() {
  clearTimer()
  visible.value = false
}
function toggle() {
  if (visible.value) hide()
  else show()
}
onBeforeUnmount(clearTimer)
</script>

<template>
  <span :class="['jc-tooltip', `is-${placement}`]">
    <span
      class="jc-tooltip__trigger"
      @mouseenter="trigger === 'hover' && show()"
      @mouseleave="trigger === 'hover' && hide()"
      @click="trigger === 'click' && toggle()"
      @focus="trigger === 'focus' && show()"
      @blur="trigger === 'focus' && hide()"
    >
      <slot />
    </span>
    <Transition name="jc-tooltip">
      <span v-if="visible" class="jc-tooltip__pop" role="tooltip">
        <slot name="title">{{ title }}</slot>
        <span class="jc-tooltip__arrow" aria-hidden="true" />
      </span>
    </Transition>
  </span>
</template>

<style scoped>
.jc-tooltip {
  position: relative;
  display: inline-flex;
}
.jc-tooltip__trigger { display: inline-flex; }
.jc-tooltip__pop {
  position: absolute;
  z-index: var(--jc-z-index-popup, 1000);
  max-width: 240px;
  padding: 6px 10px;
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: var(--jc-radius, 6px);
  box-shadow: var(--jc-shadow-2, 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 6px 16px 0 rgba(0, 0, 0, 0.08));
  font-size: var(--jc-font-size-sm, 12px);
  color: var(--jc-text-primary, #ccc);
  white-space: normal;
  word-break: break-word;
  pointer-events: none;
}
.jc-tooltip__arrow {
  position: absolute;
  width: 8px;
  height: 8px;
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-default, #3e3e42);
  transform: rotate(45deg);
}
/* placement */
.jc-tooltip.is-top .jc-tooltip__pop { bottom: calc(100% + 8px); left: 50%; transform: translateX(-50%); }
.jc-tooltip.is-top .jc-tooltip__arrow { bottom: -5px; left: 50%; margin-left: -4px; border-top: none; border-left: none; }
.jc-tooltip.is-bottom .jc-tooltip__pop { top: calc(100% + 8px); left: 50%; transform: translateX(-50%); }
.jc-tooltip.is-bottom .jc-tooltip__arrow { top: -5px; left: 50%; margin-left: -4px; border-bottom: none; border-right: none; }
.jc-tooltip.is-left .jc-tooltip__pop { right: calc(100% + 8px); top: 50%; transform: translateY(-50%); }
.jc-tooltip.is-left .jc-tooltip__arrow { right: -5px; top: 50%; margin-top: -4px; border-bottom: none; border-left: none; }
.jc-tooltip.is-right .jc-tooltip__pop { left: calc(100% + 8px); top: 50%; transform: translateY(-50%); }
.jc-tooltip.is-right .jc-tooltip__arrow { left: -5px; top: 50%; margin-top: -4px; border-top: none; border-right: none; }

.jc-tooltip-enter-active, .jc-tooltip-leave-active { transition: opacity 0.15s ease; }
.jc-tooltip-enter-from, .jc-tooltip-leave-to { opacity: 0; }
</style>
