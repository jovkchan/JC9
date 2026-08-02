// 轻量 Toast 状态 + 命令式 API（对齐 antd message：success/error/warning/info）
// 参考: https://ant.design/components/message-cn
// 仅依赖 Vue 的 reactive，跨项目可移植。
// 用法：
//   import { toast } from '@/components/ui'
//   <JcToast />            // 在 App 根部挂载一次
//   toast.success('保存成功')
import { reactive } from 'vue'

export type JcToastType = 'success' | 'error' | 'warning' | 'info'

export interface JcToastItem {
  id: number
  type: JcToastType
  message: string
  duration: number
}

export const toastState = reactive<{ items: JcToastItem[] }>({ items: [] })

let seed = 0

function show(message: string, type: JcToastType = 'info', duration = 3000): number {
  const id = ++seed
  toastState.items.push({ id, type, message, duration })
  if (duration > 0) {
    setTimeout(() => dismissToast(id), duration)
  }
  return id
}

export function dismissToast(id: number) {
  const i = toastState.items.findIndex((t) => t.id === id)
  if (i >= 0) toastState.items.splice(i, 1)
}

export function clearToasts() {
  toastState.items.splice(0)
}

/** 命令式 API：toast.success('...') */
export const toast = {
  show,
  success: (message: string, duration?: number) => show(message, 'success', duration),
  error: (message: string, duration?: number) => show(message, 'error', duration),
  warning: (message: string, duration?: number) => show(message, 'warning', duration),
  info: (message: string, duration?: number) => show(message, 'info', duration),
  dismiss: dismissToast,
  clear: clearToasts,
}
