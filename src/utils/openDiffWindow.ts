/**
 * 打开 JC9 版本对比独立窗口
 */
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

export async function openDiffWindow(noteId: string, versionId: string, versionLabel: string) {
  try {
    // 通过 localStorage 传递参数（同源窗口共享）
    localStorage.setItem('jc9-diff-note-id', noteId)
    localStorage.setItem('jc9-diff-version-id', versionId)
    localStorage.setItem('jc9-diff-version-label', versionLabel)

    // 尝试获取已有窗口
    const existing = await WebviewWindow.getByLabel('version-diff')
    if (existing) {
      try {
        await existing.emit('diff:reload')
        await existing.show()
        await existing.setFocus()
        return
      } catch {
        // 窗口已关闭，忽略
      }
    }

    // 创建新窗口
    const win = new WebviewWindow('version-diff', {
      url: '/',
      title: `版本对比: ${versionLabel}`,
      width: 1100,
      height: 700,
      minWidth: 800,
      minHeight: 400,
      decorations: false,
    })

    win.once('tauri://error', (e) => {
      console.error('Diff window error:', e)
    })
  } catch (e) {
    console.error('Failed to open diff window:', e)
  }
}
