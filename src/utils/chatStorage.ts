import { invoke } from '@tauri-apps/api/core'
import type { Message } from '@/components/tools/composables/useAiHelper'

/**
 * 获取对话文件存放的基础目录（会在 Rust 端自动拼接到用户数据目录）
 * 实际路径由 Rust 端的 path_base 命令返回
 */
let _baseDir = ''

async function ensureBaseDir(): Promise<string> {
  if (_baseDir) return _baseDir
  try {
    _baseDir = await invoke<string>('get_chat_storage_dir')
  } catch {
    // 降级：使用当前目录
    _baseDir = '.jc9/chats'
  }
  return _baseDir
}

function chatFileName(sessionId: string): string {
  return `${sessionId}.json`
}

/**
 * 保存会话消息到 JSON 文件
 */
export async function saveChatMessages(sessionId: string, messages: Message[]): Promise<void> {
  try {
    const baseDir = await ensureBaseDir()
    const data = JSON.stringify(messages, null, 2)
    await invoke('write_text_file', { path: `${baseDir}/${chatFileName(sessionId)}`, content: data })
  } catch (e) {
    console.warn('保存对话失败:', e)
  }
}

/**
 * 从 JSON 文件加载会话消息
 */
export async function loadChatMessages(sessionId: string): Promise<Message[] | null> {
  try {
    const baseDir = await ensureBaseDir()
    const content = await invoke<string>('read_text_file', { path: `${baseDir}/${chatFileName(sessionId)}` })
    return JSON.parse(content) as Message[]
  } catch {
    return null
  }
}

/**
 * 删除会话消息文件
 */
export async function deleteChatMessages(sessionId: string): Promise<void> {
  try {
    const baseDir = await ensureBaseDir()
    await invoke('delete_file', { path: `${baseDir}/${chatFileName(sessionId)}` })
  } catch { /* ignore */ }
}
