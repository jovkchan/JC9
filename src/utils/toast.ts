/**
 * 全局浮动消息提示（类似 Element Plus Message）
 * 使用方式：
 *   toast('保存成功')
 *   toast({ message: '操作失败', type: 'error', duration: 5000 })
 *   toast.success('完成')
 *   toast.warn('注意')
 *   toast.error('出错了')
 */

export type ToastType = 'info' | 'success' | 'warn' | 'error'

export interface ToastOptions {
  message: string
  type?: ToastType
  duration?: number
  icon?: string
}

type ToastArg = string | ToastOptions

// 内部引用，由 App.vue 在 onMounted 时注入
let _addToast: ((t: ToastOptions) => void) | null = null

export function registerToastHandler(handler: (t: ToastOptions) => void) {
  _addToast = handler
}

function doToast(opts: ToastArg) {
  const item: ToastOptions = typeof opts === 'string'
    ? { message: opts, type: 'info', duration: 3000 }
    : { message: opts.message, type: opts.type ?? 'info', duration: opts.duration ?? 3000, icon: opts.icon }
  _addToast?.(item)
}

export const toast = Object.assign(doToast, {
  success(msg: string, duration = 3000) { doToast({ message: msg, type: 'success', duration }) },
  warn   (msg: string, duration = 3000) { doToast({ message: msg, type: 'warn', duration }) },
  error  (msg: string, duration = 4000) { doToast({ message: msg, type: 'error', duration }) },
  info   (msg: string, duration = 3000) { doToast({ message: msg, type: 'info', duration }) },
})
