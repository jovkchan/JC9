<script setup lang="ts">
import { dismissToast, toastState } from './toast'

defineOptions({ name: 'JcToast' })

// 在 App 根部挂载一次：<JcToast />，配合 toast.success(...) 命令式调用
</script>

<template>
  <Teleport to="body">
    <div class="jc-toast">
      <TransitionGroup name="jc-toast">
        <div
          v-for="t in toastState.items"
          :key="t.id"
          :class="['jc-toast__item', `jc-toast__item--${t.type}`]"
          role="status"
        >
          <span class="jc-toast__msg">{{ t.message }}</span>
          <button type="button" class="jc-toast__close" title="关闭" @click="dismissToast(t.id)">✕</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.jc-toast {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  z-index: 2000;
  pointer-events: none;
}
.jc-toast__item {
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 480px;
  padding: 9px 14px;
  background: var(--jc-bg-elevated, #2d2d30);
  border: 1px solid var(--jc-border-default, #3e3e42);
  border-radius: var(--jc-radius, 6px);
  box-shadow: var(--jc-shadow-2, 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 6px 16px 0 rgba(0, 0, 0, 0.08));
  font-size: var(--jc-font-size, 13px);
  color: var(--jc-text-primary, #ccc);
  pointer-events: auto;
}
.jc-toast__item::before {
  content: '';
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.jc-toast__item--success::before { background: var(--jc-color-success, #4ec9b0); }
.jc-toast__item--error::before { background: var(--jc-color-error, #f44747); }
.jc-toast__item--warning::before { background: var(--jc-color-warning, #d7ba7d); }
.jc-toast__item--info::before { background: var(--jc-color-info, #5aa2ff); }
.jc-toast__msg { word-break: break-word; }
.jc-toast__close {
  border: none;
  background: transparent;
  color: var(--jc-text-secondary, #858585);
  cursor: pointer;
  font-size: 12px;
  line-height: 1;
  padding: 2px;
}
.jc-toast__close:hover { color: var(--jc-text-primary, #ccc); }

.jc-toast-enter-active, .jc-toast-leave-active { transition: all 0.24s cubic-bezier(0.645, 0.045, 0.355, 1); }
.jc-toast-enter-from { opacity: 0; transform: translateY(-8px); }
.jc-toast-leave-to { opacity: 0; transform: translateY(-8px); }
</style>
