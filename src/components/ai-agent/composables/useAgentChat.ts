import { ref, computed, nextTick } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import { invoke } from '@tauri-apps/api/core'
import { getRole, loadAllRoles, type AgentRole } from '@/config/roles'

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system'
  content: string
  modelName?: string
  roleName?: string
  roleIcon?: string
}

export function useAgentChat() {
  const notesStore = useNotesStore()
  useStatusStore()
  const ai = useAiStore()

  // ── Message State ──
  const defaultWelcome: ChatMessage = { role: 'assistant', content: '您好！我是 JC9 AI 助手。我可以帮您润色备忘、提取标签，或开启 Agent 模式自动执行开发任务。' }
  const messages = ref<ChatMessage[]>([{ ...defaultWelcome }])
  const userInput = ref('')
  const sending = ref(false)
  const inputTextarea = ref<HTMLTextAreaElement | null>(null)

  // ── Session message cache: sessionId → messages ──
  const sessionMessages = new Map<string, ChatMessage[]>()

  // ── Mode Toggles ──
  const enableDeepThink = ref(false)
  const enableLocalKb = ref(false)
  const enableAgentMode = ref(false)

  // ── AI Config (from localStorage) ──
  const aiProvider = ref('deepseek')
  const aiEndpoint = ref('https://api.deepseek.com')
  const aiApiKey = ref('')
  const aiModel = ref('deepseek-v4-pro')
  const reasoningEffort = ref<'high' | 'max' | 'off'>('high')

  // ── Role ──
  const activeChatRoleId = ref('auto')
  const chatRolesList = ref<AgentRole[]>([])

  const activeChatRole = computed((): { id: string; name: string; icon: string; description: string; systemPrompt: string } => {
    if (activeChatRoleId.value === 'auto') {
      return { id: 'auto', name: '智能路由', icon: '🤖', description: '', systemPrompt: '' }
    }
    return chatRolesList.value.find(r => r.id === activeChatRoleId.value) || {
      id: 'auto', name: '智能路由', icon: '🤖', description: '', systemPrompt: ''
    }
  })

  // ── Model Options ──
  interface CustomModel {
    id: string; name: string; provider: string; model: string
    endpoint: string; apiKey: string; inputPrice?: number; outputPrice?: number; costLimit?: number
  }
  const customModels = ref<CustomModel[]>([])
  const selectedCombinedModel = ref('')
  const loadingModels = ref(false)

  function loadCustomModels() {
    const saved = localStorage.getItem('notes-ai-models')
    if (saved) {
      try { customModels.value = JSON.parse(saved) } catch { customModels.value = [] }
    }
  }

  const modelOptions = computed(() => {
    const groups: Record<string, Array<{ id: string; name: string; label: string }>> = {}
    for (const cfg of customModels.value) {
      const provName = cfg.provider.charAt(0).toUpperCase() + cfg.provider.slice(1)
      if (!groups[provName]) groups[provName] = []
      const subModels = cfg.model.split(',').map(m => m.trim()).filter(Boolean)
      for (const m of subModels) {
        const configId = cfg.id || `${cfg.provider}-${cfg.model}-${cfg.name}`
        groups[provName].push({ id: `${configId}::${m}`, name: m, label: `${cfg.name} (${m})` })
      }
    }
    return groups
  })

  function loadConfig() {
    loadCustomModels()
    const lastSelected = localStorage.getItem('jc9-last-model')
    if (customModels.value.length > 0) {
      try {
        let target: CustomModel | null = null
        if (lastSelected) {
          if (lastSelected.includes('::')) {
            const [cfgId] = lastSelected.split('::')
            target = customModels.value.find(c => c.id === cfgId) || null
          }
        }
        const cfg = target || customModels.value[0]
        aiProvider.value = cfg.provider
        const subModels = cfg.model.split(',').map(m => m.trim()).filter(Boolean)
        aiModel.value = lastSelected?.includes('::') ? lastSelected.split('::')[1] : (subModels[0] || cfg.model)
        aiEndpoint.value = cfg.endpoint
        aiApiKey.value = cfg.apiKey || ''
        selectedCombinedModel.value = `${cfg.id || ''}::${aiModel.value}`
        return
      } catch { /* ignore */ }
    }
    aiProvider.value = localStorage.getItem('notes-ai-provider') || 'deepseek'
    aiEndpoint.value = localStorage.getItem('notes-ai-endpoint') || 'https://api.deepseek.com'
    aiApiKey.value = localStorage.getItem('notes-ai-apikey') || ''
    aiModel.value = localStorage.getItem('notes-ai-model') || 'deepseek-v4-pro'
  }

  function handleModelChange() {
    const val = selectedCombinedModel.value
    if (!val) return
    let configId = '', modelName = ''
    if (val.includes('::')) {
      configId = val.split('::')[0]
      modelName = val.split('::').slice(1).join('::')
    }
    const cfg = customModels.value.find(c => c.id === configId)
    if (cfg) {
      aiProvider.value = cfg.provider
      aiModel.value = modelName || cfg.model.split(',')[0]?.trim() || cfg.model
      aiEndpoint.value = cfg.endpoint
      aiApiKey.value = cfg.apiKey
      ai.updateCostConfig({
        inputCachedCostPerM: cfg.inputPrice ? cfg.inputPrice * 0.008 : 0.025,
        inputUncachedCostPerM: cfg.inputPrice || 2.0,
        outputCostPerM: cfg.outputPrice || 4.0,
        costLimit: cfg.costLimit || 10.0,
      })
    }
    localStorage.setItem('jc9-last-model', val)
  }

  // ── SSE Stream ──
  async function callAiStream(promptMessages: ChatMessage[], onChunk: (text: string) => void) {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' }
    if (aiProvider.value !== 'ollama' && aiProvider.value !== 'vllm' && aiApiKey.value) {
      headers['Authorization'] = `Bearer ${aiApiKey.value}`
    }

    let body = {}, url = ''
    if (aiProvider.value === 'ollama') {
      url = `${aiEndpoint.value}/api/chat`
      body = { model: aiModel.value, messages: promptMessages.map(m => ({ role: m.role, content: m.content })), stream: true }
    } else {
      url = `${aiEndpoint.value}/chat/completions`
      body = { model: aiModel.value, messages: promptMessages.map(m => ({ role: m.role, content: m.content })), temperature: 0.7, stream: true }
    }

    // 尝试通过 Rust 后端代理（绕过 CSP/CORS），失败则回退前端 fetch
    try {
      const headerPairs = Object.entries(headers).map(([k, v]) => [k, v] as [string, string])
      const responseText = await invoke<string>('proxy_ai_request', {
        url, method: 'POST',
        headers: headerPairs,
        body: JSON.stringify(body),
      })
      // 解析 SSE 响应
      if (aiProvider.value === 'ollama') {
        for (const line of responseText.split('\n')) {
          const trimmed = line.trim()
          if (!trimmed) continue
          try {
            const json = JSON.parse(trimmed)
            const text = json.message?.content || ''
            if (text) onChunk(text)
          } catch { /* ignore */ }
        }
      } else {
        for (const line of responseText.split('\n')) {
          const trimmed = line.trim()
          if (trimmed.startsWith('data:')) {
            const dataVal = trimmed.slice(5).trim()
            if (dataVal === '[DONE]') continue
            try {
              const json = JSON.parse(dataVal)
              const text = json.choices?.[0]?.delta?.content || ''
              if (text) onChunk(text)
            } catch { /* ignore */ }
          }
        }
      }
      return
    } catch (proxyErr) {
      console.warn('代理请求失败，回退前端 fetch:', proxyErr)
    }

    // 回退：直接前端 fetch
    const res = await fetch(url, { method: 'POST', headers, body: JSON.stringify(body) })
    if (!res.ok) throw new Error(`请求失败 (${res.status}): ${await res.text()}`)

    const reader = res.body?.getReader()
    if (!reader) throw new Error('当前环境不支持流式读取')

    const decoder = new TextDecoder('utf-8')
    let buffer = ''
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      buffer = lines.pop() || ''
      for (const line of lines) {
        const trimmed = line.trim()
        if (!trimmed) continue
        if (aiProvider.value === 'ollama') {
          try {
            const json = JSON.parse(trimmed)
            const text = json.message?.content || ''
            if (text) onChunk(text)
          } catch { /* ignore */ }
        } else {
          if (trimmed.startsWith('data:')) {
            const dataVal = trimmed.slice(5).trim()
            if (dataVal === '[DONE]') continue
            try {
              const json = JSON.parse(dataVal)
              const text = json.choices?.[0]?.delta?.content || ''
              if (text) onChunk(text)
            } catch { /* ignore */ }
          }
        }
      }
    }
  }

  // ── Scroll ──
  function scrollToBottom() {
    nextTick(() => {
      const el = document.querySelector('.chat-messages')
      if (el) el.scrollTop = el.scrollHeight
    })
  }

  // ── Helper ──
  function extractTerms(query: string): string[] {
    const terms: string[] = []
    const blocks = query.toLowerCase().match(/[\u4e00-\u9fa5]+|[a-z0-9]+/g) || []
    const stopWords = new Set(['的','了','和','是','就','都','而','及','与','这','那','有','无','一个','一下','关于','内容','笔记','总结','阅读','查找','搜索','这篇','那些','哪些','什么','请问','有没有','怎么','如何','帮我','看看','谁','哪里','记了'])
    for (const block of blocks) {
      if (stopWords.has(block)) continue
      if (/[\u4e00-\u9fa5]/.test(block)) {
        if (block.length > 1 && !stopWords.has(block)) terms.push(block)
        for (let i = 0; i < block.length - 1; i++) {
          const sub = block.slice(i, i + 2)
          if (!stopWords.has(sub)) terms.push(sub)
        }
        if (block.length === 1 && !stopWords.has(block)) terms.push(block)
      } else {
        if (block.length > 0) terms.push(block)
      }
    }
    return Array.from(new Set(terms))
  }

  // ── Agent Mode ──
  async function sendAgentMessage(text: string) {
    try {
      await invoke('ai_configure_llm', {
        provider: aiProvider.value, apiKey: aiApiKey.value,
        baseUrl: aiEndpoint.value, model: aiModel.value,
      })
    } catch { /* ignore */ }

    if (!ai.currentSessionId) {
      const title = text.slice(0, 30) + (text.length > 30 ? '...' : '')
      await ai.createSession(title)
      addSystemBubble('📋 新会话已创建')
    }
    addSystemBubble('🧠 正在分析并拆解任务...')
    const tasks = await ai.planTask(ai.currentSessionId!, text)
    if (tasks.length > 0) {
      addSystemBubble(`✅ 已规划 **${tasks.length}** 个子任务`)
      for (const task of tasks) {
        if (task.status === 'pending') {
          const role = getRole(task.assignedWorker)
          addSystemBubble(`🚀 启动: ${task.title} [${role.icon} ${role.name}]`)
          const sp = `${role.systemPrompt}\n\n任务描述：${task.description}`
          await ai.spawnWorker(ai.currentSessionId!, task, sp)
        }
      }
    } else {
      addSystemBubble('⚠️ 任务规划为空，请尝试更具体的描述。')
    }
  }

  function addSystemBubble(content: string) {
    messages.value.push({ role: 'system', content })
    scrollToBottom()
  }

  // ── Main Send ──
  async function sendMessage() {
    const text = userInput.value.trim()
    if (!text || sending.value) return

    if (enableAgentMode.value) {
      messages.value.push({ role: 'user', content: text })
      userInput.value = ''
      sending.value = true
      scrollToBottom()
      try { await sendAgentMessage(text) }
      catch (e: any) { addSystemBubble(`❌ Agent 错误: ${e}`) }
      finally {
        sending.value = false; scrollToBottom()
        if (ai.currentSessionId) sessionMessages.set(ai.currentSessionId, [...messages.value])
      }
      return
    }

    // Normal chat — 自动创建会话
    if (!ai.currentSessionId) {
      const title = text.length > 30 ? text.slice(0, 30) + '...' : text
      await ai.createSession(title)
    }

    messages.value.push({ role: 'user', content: text })
    userInput.value = ''
    sending.value = true
    scrollToBottom()

    let systemPromptStr = '你是一个通用 AI 助手。请直接、简洁地回答用户的问题。不要说"作为XX助手"之类的话。'

    if (enableLocalKb.value) {
      await notesStore.loadAllNotes()
      const activeNotes = notesStore.notes.filter(n => !n.isDeleted && !n.isArchived)
      const userQuery = text.toLowerCase()
      const terms = extractTerms(userQuery)
      const scored = activeNotes.map(n => {
        let score = 0
        const titleLower = (n.title || '').toLowerCase()
        const contentLower = (n.content || '').toLowerCase()
        terms.forEach(term => {
          if (titleLower.includes(term)) score += term.length * 30
          if (contentLower.includes(term)) score += term.length * 8
        })
        return { note: n, score }
      })
      const topNotes = scored.sort((a, b) => b.score - a.score).filter(x => x.score > 0).map(x => x.note).slice(0, 5)
      if (topNotes.length > 0) {
        const refText = topNotes.map(n => `[${n.title}]\n${(n.content || '').slice(0, 500)}`).join('\n\n---\n\n')
        systemPromptStr += `\n\n以下是用户本地笔记库中相关内容：\n${refText}\n请结合参考回答。`
      }
    }

    messages.value.push({ role: 'assistant', content: '', modelName: aiModel.value })
    const aiMsgIndex = messages.value.length - 1
    try {
      let currentContent = ''
      await callAiStream(
        [{ role: 'system', content: systemPromptStr }, ...messages.value.filter(m => m.role === 'user' || m.role === 'assistant')],
        (chunk) => {
          currentContent += chunk
          messages.value[aiMsgIndex].content = currentContent
          scrollToBottom()
        }
      )
    } catch (e: any) {
      messages.value[aiMsgIndex].content = `❌ 请求失败: ${e.message}`
    } finally {
      sending.value = false
      scrollToBottom()
      // 保存当前会话消息到缓存
      if (ai.currentSessionId) {
        sessionMessages.set(ai.currentSessionId, [...messages.value])
      }
    }
  }

  // ── 切换会话：缓存当前 → 加载目标 ──
  function switchSession(sessionId: string | null) {
    const prevId = ai.currentSessionId
    // 缓存当前会话消息
    if (prevId && messages.value.length > 0) {
      sessionMessages.set(prevId, [...messages.value])
    }
    // 切换到目标会话
    ai.currentSessionId = sessionId
    if (sessionId && sessionMessages.has(sessionId)) {
      messages.value = [...sessionMessages.get(sessionId)!]
    } else {
      messages.value = [{ ...defaultWelcome }]
    }
  }

  // ── 新建对话 ──
  async function clearChat() {
    const prevId = ai.currentSessionId
    if (prevId && messages.value.length > 0) {
      sessionMessages.set(prevId, [...messages.value])
    }
    const title = '新对话'
    const newId = await ai.createSession(title)
    if (newId) {
      messages.value = [{ ...defaultWelcome }]
      sessionMessages.set(newId, [{ ...defaultWelcome }])
    }
  }

  async function init() {
    // 从 JSON 文件同步 AI 配置到 localStorage（跨 dev/build 共享）
    try {
      const json = await invoke<string>('get_ai_config')
      const cfg = JSON.parse(json)
      for (const [k, v] of Object.entries(cfg)) {
        if (typeof v === 'string' && v) localStorage.setItem(k, v)
      }
    } catch { /* ignore */ }

    loadConfig()
    chatRolesList.value = loadAllRoles()
    ai.loadSessions()
    ai.loadWorkspaceRoot()
    ai.initListeners()
    ai.registerAllFrontendTools()
  }

  function destroy() {
    ai.destroyListeners()
  }

  return {
    // State
    messages, userInput, sending, inputTextarea,
    enableDeepThink, enableLocalKb, enableAgentMode,
    aiProvider, aiEndpoint, aiApiKey, aiModel, reasoningEffort,
    activeChatRoleId, chatRolesList, activeChatRole,
    selectedCombinedModel, modelOptions, loadingModels,
    // Actions
    sendMessage, clearChat, switchSession, callAiStream, init, destroy,
    handleModelChange, loadConfig, addSystemBubble,
    scrollToBottom,
  }
}
