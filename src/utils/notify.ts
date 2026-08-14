/**
 * 系统级通知（OS 通知中心弹出，如 VS Code 提示）——跨平台 Windows / macOS / Linux。
 * 通过官方 tauri-plugin-notification 触发，不是软件内 Toast / 通知中心。
 *
 * 用法：
 *   notify('构建完成')
 *   notify('构建失败', 'exit code 1')
 *   notify.error('部署失败', '详见日志')
 */
import { invoke } from '@tauri-apps/api/core'

export type NotifyLevel = 'info' | 'success' | 'warn' | 'error'

export interface NotifyOptions {
  /** 通知级别（保留语义，可扩展为不同图标/声音） */
  level?: NotifyLevel
}

async function doNotify(title: string, body?: string, _opts: NotifyOptions = {}) {
  // 系统级通知：官方插件（OS 通知中心）
  try {
    await invoke('send_notification', { title, body: body || '' })
  } catch (e) {
    // 非 Tauri 环境（浏览器调试）无系统通知，静默忽略
    console.error('[notify] 系统通知失败:', e)
  }
}

/** 统一通知入口（命令式，可链式分级） */
export const notify = Object.assign(doNotify, {
  success(title: string, body?: string, opts?: Omit<NotifyOptions, 'level'>) {
    return doNotify(title, body, { ...opts, level: 'success' })
  },
  error(title: string, body?: string, opts?: Omit<NotifyOptions, 'level'>) {
    return doNotify(title, body, { ...opts, level: 'error' })
  },
  warn(title: string, body?: string, opts?: Omit<NotifyOptions, 'level'>) {
    return doNotify(title, body, { ...opts, level: 'warn' })
  },
  info(title: string, body?: string, opts?: Omit<NotifyOptions, 'level'>) {
    return doNotify(title, body, { ...opts, level: 'info' })
  },
})
