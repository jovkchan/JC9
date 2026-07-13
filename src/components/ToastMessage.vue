<script setup lang="ts">
import { ref } from 'vue'
import type { ToastOptions } from '@/utils/toast'

interface ToastItem extends ToastOptions {
  id: string
}

const toasts = ref<ToastItem[]>([])

// 外部通过 expose 调用
function addToast(t: ToastOptions) {
  const id = `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
  const item: ToastItem = { ...t, id }
  toasts.value.push(item)
  if ((t.duration ?? 3000) > 0) {
    setTimeout(() => {
      const idx = toasts.value.findIndex(x => x.id === id)
      if (idx !== -1) toasts.value.splice(idx, 1)
    }, t.duration ?? 3000)
  }
}
function removeToast(id: string) {
  const idx = toasts.value.findIndex(x => x.id === id)
  if (idx !== -1) toasts.value.splice(idx, 1)
}

defineExpose({ addToast })

// ── 类型 -> 颜色/图标映射 ──
const typeMap: Record<string, { bg: string; border: string; icon: string }> = {
  info:    { bg: 'rgba(22,119,255,.1)', border: 'rgba(22,119,255,.35)', icon: 'ℹ️' },
  success: { bg: 'rgba(103,194,58,.1)',  border: 'rgba(103,194,58,.35)',  icon: '✅' },
  warn:    { bg: 'rgba(230,162,60,.1)',  border: 'rgba(230,162,60,.35)',  icon: '⚠️' },
  error:   { bg: 'rgba(245,108,108,.1)', border: 'rgba(245,108,108,.35)', icon: '❌' },
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="item in toasts"
          :key="item.id"
          class="toast-item"
          :style="{
            background: typeMap[item.type ?? 'info']?.bg || typeMap.info.bg,
            borderColor: typeMap[item.type ?? 'info']?.border || typeMap.info.border,
          }"
          @click="removeToast(item.id)"
        >
          <span class="toast-icon">{{ item.icon || typeMap[item.type ?? 'info']?.icon || 'ℹ️' }}</span>
          <span class="toast-msg">{{ item.message }}</span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 999999;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  pointer-events: none;
}
.toast-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: 8px;
  border: 1px solid;
  font-size: 13px;
  color: var(--jc-text-primary, #e6e6e6);
  backdrop-filter: blur(12px);
  box-shadow: 0 4px 16px rgba(0,0,0,.25);
  cursor: pointer;
  pointer-events: auto;
  max-width: 480px;
  user-select: none;
}
.toast-icon { font-size: 15px; flex-shrink: 0; }
.toast-msg { line-height: 1.5; }

/* ── 进出动画 ── */
.toast-enter-active { animation: toast-in 0.28s ease; }
.toast-leave-active { animation: toast-out 0.2s ease; }
@keyframes toast-in {
  from { opacity: 0; transform: translateY(-16px) scale(0.92); }
  to   { opacity: 1; transform: translateY(0) scale(1); }
}
@keyframes toast-out {
  from { opacity: 1; transform: translateY(0) scale(1); }
  to   { opacity: 0; transform: translateY(-12px) scale(0.92); }
}
</style>
