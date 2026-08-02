<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import JcButton from './JcButton.vue'

defineOptions({ name: 'JcModal' })

// API 对齐 Ant Design Modal：open / title / width / footer / maskClosable / closable / mask / confirmLoading / onOk / onCancel
// 参考: https://ant.design/components/modal-cn
const props = withDefaults(
  defineProps<{
    open?: boolean
    title?: string
    width?: number | string
    closable?: boolean
    mask?: boolean
    /** 点击遮罩是否可关闭 */
    maskClosable?: boolean
    /** 是否显示底部按钮区 */
    footer?: boolean
    /** 确定按钮加载态（onOk 异步时用，防重复提交） */
    confirmLoading?: boolean
    zIndex?: number
  }>(),
  {
    open: false,
    title: '',
    width: 520,
    closable: true,
    mask: true,
    maskClosable: true,
    footer: true,
    confirmLoading: false,
    zIndex: 1000,
  },
)

const emit = defineEmits<{
  'update:open': [value: boolean]
  ok: []
  cancel: []
}>()

// 宽度：数字 → px；数字字符串（如 "440"）→ 补 px；其他字符串（如 "90%"）→ 原样
const panelWidth = computed(() => {
  const w = props.width
  if (typeof w === 'number') return `${w}px`
  if (typeof w === 'string') {
    const t = w.trim()
    if (/^\d+(\.\d+)?$/.test(t)) return `${t}px`
    return w
  }
  return undefined
})

function onOk() {
  emit('ok')
}
function onCancel() {
  if (props.confirmLoading) return
  emit('update:open', false)
  emit('cancel')
}
// 统一遮罩关闭策略：仅当 mousedown 目标就是遮罩本身才关闭（避免输入框内拖选越界误关）
function onMaskMousedown() {
  if (props.maskClosable) onCancel()
}
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') onCancel()
}

function toggleListeners(v: boolean) {
  if (v) document.addEventListener('keydown', onKeydown)
  else document.removeEventListener('keydown', onKeydown)
}
watch(() => props.open, (v) => toggleListeners(v))
onMounted(() => toggleListeners(props.open))
onBeforeUnmount(() => toggleListeners(false))
</script>

<template>
  <Teleport to="body">
    <Transition name="jc-modal">
      <div v-if="open" class="jc-modal" :style="{ zIndex }">
        <div v-if="mask" class="jc-modal__mask" @mousedown.self="onMaskMousedown" />
        <div class="jc-modal__panel" :style="{ width: panelWidth }">
          <div class="jc-modal__header">
            <div class="jc-modal__title"><slot name="title">{{ title }}</slot></div>
            <button v-if="closable" type="button" class="jc-modal__close" title="关闭" @click="onCancel">✕</button>
          </div>
          <div class="jc-modal__body"><slot /></div>
          <div v-if="footer" class="jc-modal__footer">
            <slot name="footer">
              <JcButton @click="onCancel">取消</JcButton>
              <JcButton type="primary" :loading="confirmLoading" @click="onOk">确定</JcButton>
            </slot>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.jc-modal {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
.jc-modal__mask {
  position: absolute;
  inset: 0;
  background: var(--jc-bg-overlay, rgba(0, 0, 0, 0.5));
}
.jc-modal__panel {
  position: relative;
  display: flex;
  flex-direction: column;
  max-width: calc(100vw - 48px);
  max-height: calc(100vh - 96px);
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: var(--jc-radius-lg, 8px);
  box-shadow: var(--jc-shadow-modal, 0 6px 16px -8px rgba(0, 0, 0, 0.08), 0 9px 28px 0 rgba(0, 0, 0, 0.05));
}
.jc-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--jc-space, 16px) var(--jc-space-lg, 24px);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}
.jc-modal__title {
  font-size: var(--jc-font-size-lg, 16px);
  font-weight: var(--jc-font-weight-medium, 500);
  color: var(--jc-text-highlight, #e0e0e0);
}
.jc-modal__close {
  border: none;
  background: transparent;
  color: var(--jc-text-secondary, #858585);
  font-size: 14px;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  line-height: 1;
}
.jc-modal__close:hover { background: var(--jc-bg-hover, #2a2d2e); color: var(--jc-text-primary, #ccc); }
.jc-modal__body {
  padding: var(--jc-space-lg, 24px);
  overflow: auto;
  flex: 1;
  min-height: 0;
  color: var(--jc-text-primary, #ccc);
}
.jc-modal__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--jc-space-xs, 8px);
  padding: var(--jc-space-sm, 12px) var(--jc-space-lg, 24px);
  border-top: 1px solid var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}

.jc-modal-enter-active, .jc-modal-leave-active { transition: opacity 0.2s ease; }
.jc-modal-enter-from, .jc-modal-leave-to { opacity: 0; }
.jc-modal-enter-active .jc-modal__panel { animation: jc-modal-in 0.24s cubic-bezier(0.645, 0.045, 0.355, 1); }
@keyframes jc-modal-in {
  from { transform: translateY(-12px) scale(0.98); opacity: 0; }
  to { transform: translateY(0) scale(1); opacity: 1; }
}
</style>
