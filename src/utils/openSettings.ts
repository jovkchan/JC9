/**
 * 打开 JC9 系统设置独立窗口
 * 可在任何组件中调用
 */
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

export async function openSettingsWindow() {
  try {
    // 先尝试获取已有窗口
    const existing = await WebviewWindow.getByLabel('settings')
    if (existing) {
      try {
        await existing.show()
        await existing.setFocus()
        return
      } catch {
        // 窗口已关闭，getByLabel 返回残引用，忽略直接新建
      }
    }
    // 创建新窗口
    const win = new WebviewWindow('settings', {
      url: '/',
      title: 'JC9 设置',
      width: 870,
      height: 620,
      minWidth: 600,
      minHeight: 400,
      decorations: false,
    })
    win.once('tauri://created', () => {
      console.log('Settings window created')
    })
    win.once('tauri://error', (e) => {
      console.error('Settings window error:', e)
    })
  } catch (e) {
    console.error('Failed to open Settings window:', e)
  }
}
