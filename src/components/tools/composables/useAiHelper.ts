import { ref, computed, nextTick } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getRole, loadAllRoles, type AgentRole } from '@/config/roles'
import type { WorkerState, ApprovalRequest, KbEntry } from '@/types/ai'
import { saveChatMessages, loadChatMessages } from '@/utils/chatStorage'

// ── Types ──
export interface Message {
  role: 'user' | 'assistant' | 'system'
  content: string
  modelName?: string
  roleName?: string
  roleIcon?: string
}

export type ChatMode = '创作' | '问答' | '规划'

interface CustomModel {
  id: string
  name: string
  provider: string
  model: string
  endpoint: string
  apiKey: string
  inputPrice?: number
  outputPrice?: number
  costLimit?: number
}

// ── 模块级持久状态（跨组件生命周期） ──
const _sessionMessages = new Map<string, Message[]>()
const _defaultWelcome: Message = { role: 'assistant', content: '您好！我是您的 JC9 笔记 AI 助手。我可以帮您润色备忘、推荐标签，或者基于您的本地笔记库回答问题。' }
const _messages = ref<Message[]>([{ ..._defaultWelcome }])
const _userInput = ref('')
const _sending = ref(false)

// ── Composable ──
export function useAiHelper() {
  const notesStore = useNotesStore()
  const status = useStatusStore()
  const ai = useAiStore()

  // ── 持久化辅助 ──
  let _persistTimer: number | null = null

  async function persistCurrentMessages() {
    const sid = ai.currentSessionId
    if (!sid || messages.value.length === 0) return
    await saveChatMessages(sid, messages.value)
  }

  function schedulePersist() {
    if (_persistTimer !== null) clearTimeout(_persistTimer)
    _persistTimer = window.setTimeout(() => { persistCurrentMessages() }, 500)
  }

  // ═══════════════════════════════════════════
  //  1. STATE
  // ═══════════════════════════════════════════

  const sessionMessages = _sessionMessages
  const defaultWelcome = _defaultWelcome
  const messages = _messages
  const userInput = _userInput
  const sending = _sending

  // ── Mode Selector ──
  const chatMode = ref<ChatMode>('问答')
  const chatModes = ref<ChatMode[]>(['创作', '问答', '规划'])

  // ── Toggles ──
  const enableDeepThink = ref(false)
  const enableLocalKb = ref(false)
  const reasoningEffort = ref<'high' | 'max' | 'off'>('high')

  // ── AI Config ──
  const aiProvider = ref('ollama')
  const aiEndpoint = ref('http://127.0.0.1:11434')
  const aiApiKey = ref('')
  const aiModel = ref('llama3')
  const selectedCombinedModel = ref('')

  // ── Custom Models ──
  const customModels = ref<CustomModel[]>([])
  const ollamaModels = ref<string[]>([])
  const vllmModels = ref<string[]>([])
  const loadingModels = ref(false)

  // ── Role ──
  const activeChatRoleId = ref('auto')
  const chatRolesList = ref<AgentRole[]>([])

  const activeChatRole = computed(() => {
    if (activeChatRoleId.value === 'auto') {
      return { id: 'auto', name: '智能路由', icon: '🤖', description: '根据提问内容自动选择最适合的角色', systemPrompt: '' }
    }
    return chatRolesList.value.find(r => r.id === activeChatRoleId.value) || { id: 'auto', name: '智能路由', icon: '🤖', description: '', systemPrompt: '' }
  })

  // ── UI State ──
  const isFocused = ref(false)
  const inputTextarea = ref<HTMLTextAreaElement | null>(null)
  const showModelSettingsModal = ref(false)
  const showSessionPopup = ref(false)
  const newSessionTitle = ref('')
  const showBrowserDialog = ref(false)
  const browserUrlInput = ref('https://google.com')

  // ── Console ──
  const isConsoleExpanded = ref(true)
  const expandedWorkers = ref<Record<string, boolean>>({})

  // ── KB Search ──
  const kbSearchQuery = ref('')
  const kbSearchResults = ref<KbEntry[]>([])

  // ── Polling ──
  const pollTimer = ref<number | null>(null)
  const workerUnlisten = ref<(() => void) | null>(null)

  // ── Local config ──
  const localConfig = ref({
    inputCachedCostPerM: 0.025,
    inputUncachedCostPerM: 3.0,
    outputCostPerM: 6.0,
    costLimit: 5.0
  })

  // ═══════════════════════════════════════════
  //  2. COMPUTED
  // ═══════════════════════════════════════════

  const placeholderText = computed(() => {
    if (chatMode.value === '创作') return '描述开发任务，Agent 自动规划并执行... (Enter 发送)'
    if (chatMode.value === '规划') return '描述任务需求，AI 将拆解为多级子任务... (Enter 发送)'
    return `给 ${aiModel.value || 'AI'} 发送消息... (Enter 发送, Shift+Enter 换行)`
  })

  const workspaceShortName = computed(() => {
    const p = ai.workspaceRoot
    if (!p) return '未设置'
    const parts = p.replace(/\\/g, '/').split('/')
    return (parts[parts.length - 1] || p)
  })

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

    if (ollamaModels.value.length > 0) {
      if (!groups['Ollama']) groups['Ollama'] = []
      ollamaModels.value.forEach(m => groups['Ollama'].push({ id: `ollama/${m}`, name: m, label: m }))
    }
    if (vllmModels.value.length > 0) {
      if (!groups['Vllm']) groups['Vllm'] = []
      vllmModels.value.forEach(m => groups['Vllm'].push({ id: `vllm/${m}`, name: m, label: m }))
    }

    return groups
  })

  // ═══════════════════════════════════════════
  //  3. METHODS — Model Config
  // ═══════════════════════════════════════════

  function loadCustomModels() {
    const saved = localStorage.getItem('notes-ai-models')
    if (saved) {
      try { customModels.value = JSON.parse(saved) } catch { customModels.value = [] }
    }
  }

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
        ai.updateCostConfig({
          inputCachedCostPerM: cfg.inputPrice ? cfg.inputPrice * 0.008 : 0.025,
          inputUncachedCostPerM: cfg.inputPrice || 2.0,
          outputCostPerM: cfg.outputPrice || 4.0,
          costLimit: cfg.costLimit || 10.0,
        })
        return
      } catch { /* ignore */ }
    }

    aiProvider.value = localStorage.getItem('notes-ai-provider') || 'ollama'
    aiEndpoint.value = localStorage.getItem('notes-ai-endpoint') || 'http://127.0.0.1:11434'
    aiApiKey.value = localStorage.getItem('notes-ai-apikey') || ''
    aiModel.value = localStorage.getItem('notes-ai-model') || 'llama3'
  }

  function handleModelChange() {
    const val = selectedCombinedModel.value
    if (!val) return

    if (val.startsWith('ollama/') || val.startsWith('vllm/')) {
      const parts = val.split('/')
      const prov = parts[0]
      const model = parts.slice(1).join('/')
      aiProvider.value = prov
      aiModel.value = model
      const savedEndpoint = localStorage.getItem(`notes-ai-endpoint-${prov}`)
      aiEndpoint.value = savedEndpoint || (prov === 'ollama' ? 'http://127.0.0.1:11434' : 'http://localhost:8000/v1')
      aiApiKey.value = ''
      saveQuickConfig()
      localStorage.setItem('jc9-last-model', val)
      return
    }

    let configId = '', selectedModelName = ''
    if (val.includes('::')) {
      configId = val.split('::')[0]
      selectedModelName = val.split('::').slice(1).join('::')
    } else {
      configId = val
    }

    const cfg = customModels.value.find(c => c.id === configId || `${c.provider}-${c.model}-${c.name}` === configId)
    if (cfg) {
      aiProvider.value = cfg.provider
      aiModel.value = selectedModelName || cfg.model.split(',')[0]?.trim() || cfg.model
      aiEndpoint.value = cfg.endpoint
      aiApiKey.value = cfg.apiKey
      ai.updateCostConfig({
        inputCachedCostPerM: cfg.inputPrice ? cfg.inputPrice * 0.008 : 0.025,
        inputUncachedCostPerM: cfg.inputPrice || 2.0,
        outputCostPerM: cfg.outputPrice || 4.0,
        costLimit: cfg.costLimit || 10.0,
      })
      saveQuickConfig()
      localStorage.setItem('jc9-last-model', val)
    }
  }

  function saveQuickConfig() {
    localStorage.setItem('notes-ai-provider', aiProvider.value)
    localStorage.setItem('notes-ai-endpoint', aiEndpoint.value)
    localStorage.setItem('notes-ai-apikey', aiApiKey.value)
    localStorage.setItem('notes-ai-model', aiModel.value)
    localStorage.setItem(`notes-ai-endpoint-${aiProvider.value}`, aiEndpoint.value)
    localStorage.setItem(`notes-ai-apikey-${aiProvider.value}`, aiApiKey.value)
    localStorage.setItem(`notes-ai-model-${aiProvider.value}`, aiModel.value)
    status.pushMessage(`已切换为模型: ${aiProvider.value} / ${aiModel.value}`, 'success')
  }

  async function fetchOllamaModels() {
    const endpoint = localStorage.getItem('notes-ai-endpoint-ollama') || 'http://127.0.0.1:11434'
    try {
      const res = await fetch(`${endpoint}/api/tags`)
      if (res.ok) {
        const json = await res.json()
        if (json.models && Array.isArray(json.models)) {
          ollamaModels.value = json.models.map((m: any) => m.name)
        }
      }
    } catch { /* ignore */ }
  }

  async function fetchVllmModels() {
    const endpoint = localStorage.getItem('notes-ai-endpoint-vllm') || 'http://192.168.5.100:8000/v1'
    loadingModels.value = true
    try {
      const res = await fetch(`${endpoint}/models`)
      if (res.ok) {
        const json = await res.json()
        if (json.data && Array.isArray(json.data)) {
          vllmModels.value = json.data.map((m: any) => m.id)
        }
      }
    } catch { /* ignore */ }
    finally { loadingModels.value = false }
  }

  async function refreshLocalModels() {
    loadingModels.value = true
    status.pushMessage('正在刷新本地模型列表...', 'info')
    await Promise.all([fetchOllamaModels(), fetchVllmModels()])
    loadingModels.value = false
    status.pushMessage('本地模型列表刷新完毕', 'success')
  }

  // ═══════════════════════════════════════════
  //  4. METHODS — Streaming & RAG
  // ═══════════════════════════════════════════

  async function callAiStream(promptMessages: Message[], onChunk: (text: string) => void) {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' }
    if (aiProvider.value !== 'ollama' && aiProvider.value !== 'vllm' && aiApiKey.value) {
      headers['Authorization'] = `Bearer ${aiApiKey.value}`
    } else if (aiProvider.value === 'vllm' && aiApiKey.value) {
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

    const res = await fetch(url, { method: 'POST', headers, body: JSON.stringify(body) })
    if (!res.ok) throw new Error(`请求失败 (${res.status}): ${await res.text()}`)

    const reader = res.body?.getReader()
    if (!reader) throw new Error('当前环境或接口未支持流式读取')

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

  function extractTerms(query: string): string[] {
    const terms: string[] = []
    const blocks = query.toLowerCase().match(/[\u4e00-\u9fa5]+|[a-z0-9]+/g) || []
    const stopWords = new Set(['的','了','和','是','就','都','而','及','与','这','那','有','无','一个','一下','关于','内容','笔记','总结','阅读','查找','搜索','一篇','这篇','那些','哪些','什么','请问','有没有','怎么','如何','帮我','看看','谁','哪里','记了'])
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

  function scrollToBottom() {
    nextTick(() => {
      const el = document.querySelector('.chat-messages')
      if (el) el.scrollTop = el.scrollHeight
    })
  }

  function autoResizeTextarea() {
    const el = inputTextarea.value
    if (!el) return
    el.style.height = 'auto'
    el.style.height = Math.min(el.scrollHeight, 480) + 'px'
  }

  // ═══════════════════════════════════════════
  //  5. METHODS — Notes & Polish
  // ═══════════════════════════════════════════

  function attachActiveNote() {
    const activeNoteId = notesStore.activeNoteTabId
    if (!activeNoteId) return
    const note = notesStore.notes.find(n => n.id === activeNoteId)
    if (note) {
      userInput.value += `\n[参考笔记: ${note.title || '无标题'}]\n${note.content}\n`
      status.pushMessage('已成功附加当前活动笔记', 'success')
    }
  }

  async function handleCreateNoteFromAi(text: string) {
    const urlRegex = /(https?:\/\/[^\s]+)/gi
    const urlMatch = text.match(urlRegex)
    let contentText = ''

    sending.value = true
    messages.value.push({ role: 'user', content: text })
    userInput.value = ''
    scrollToBottom()

    const currentModel = aiModel.value || 'Default Model'
    messages.value.push({ role: 'assistant', content: '🤖 正在分析您的笔记生成请求，请稍候...', modelName: currentModel })
    const aiMsgIndex = messages.value.length - 1

    if (urlMatch && urlMatch[0]) {
      const url = urlMatch[0]
      messages.value[aiMsgIndex].content = `🔍 检测到网页链接: \`${url}\`，正在尝试抓取网页内容...`
      try {
        const html = await invoke<string>('fetch_url_html', { url })
        const doc = new DOMParser().parseFromString(html, 'text/html')
        doc.querySelectorAll('script, style, meta, link, header, footer, nav, iframe').forEach(el => el.remove())
        contentText = doc.body.innerText.replace(/\s+/g, ' ').slice(0, 12000)
        if (!contentText.trim()) throw new Error('抓取到的网页正文内容为空。')
        messages.value[aiMsgIndex].content = `📥 网页内容抓取成功（约 ${contentText.length} 字符），正在调用 AI 智能生成总结笔记...`
      } catch (e: any) {
        messages.value[aiMsgIndex].content = `❌ 网页抓取失败: ${e.message || e}。我们将跳过网页抓取，直接根据您提供的信息生成笔记。`
      }
    }

    let prompt = ''
    if (contentText) {
      prompt = `用户指令："${text}"\n\n抓取到的网页网页内容如下：\n${contentText}\n\n请根据以上网页内容，提取核心要点并整理出一篇结构清晰、排版优美并具有清晰层级的 Markdown 备忘笔记。
请必须仅输出一个符合以下 JSON 格式的字符串，不要包含任何 markdown 标志（如 \`\`\`json）：
{
  "title": "网页核心主题标题",
  "content": "# 标题\\n\\n## 概述\\n...内容详情...\\n使用 Markdown 格式。",
  "tags": ["标签1", "标签2"]
}`
    } else {
      const historyText = messages.value
        .slice(0, -2)
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .map(m => `${m.role === 'user' ? '用户' : 'AI助理'}: ${m.content}`)
        .join('\n\n')
      prompt = `用户指令："${text}"\n\n当前对话的历史上下文如下：\n${historyText || '无上下文历史。'}\n\n请根据以上历史对话内容（如有），结合您的知识库，提取核心要点并整理出一篇结构清晰、排版优美并具有清晰层级的 Markdown 备忘笔记。
请必须仅输出一个符合以下 JSON 格式的字符串，不要包含任何 markdown 标志（如 \`\`\`json）：
{
  "title": "本篇对话核心主题标题",
  "content": "# 标题\\n\\n## 对话要点总结\\n...内容详情...\\n使用 Markdown 格式。",
  "tags": ["标签1", "标签2"]
}`
    }

    let responseText = ''
    try {
      const promptMsgs: Message[] = [
        { role: 'system', content: '你是一个专业的 Markdown 笔记整理专家。请必须按指定的 JSON 格式输出生成的笔记标题、正文和标签列表。不要输出任何 JSON 之外的多余内容或 markdown 围栏。' },
        { role: 'user', content: prompt }
      ]
      await callAiStream(promptMsgs, (chunk) => {
        responseText += chunk
        messages.value[aiMsgIndex].content = `✍️ 正在流式接收并生成笔记中...\n\n${responseText.slice(0, 1000)}${responseText.length > 1000 ? '...' : ''}`
      })

      const cleanedJson = responseText.trim().replace(/^```json\s*/i, '').replace(/```$/, '').trim()
      let parsed: any
      try { parsed = JSON.parse(cleanedJson) }
      catch {
        const match = cleanedJson.match(/\{[\s\S]*\}/)
        if (match) parsed = JSON.parse(match[0])
        else throw new Error('无法解析 JSON')
      }

      if (parsed.title && parsed.content) {
        const newNote = await notesStore.createNote({
          title: parsed.title, content: parsed.content,
          format: 'markdown', visibility: 'PRIVATE', groupId: null,
          tags: parsed.tags || []
        })
        if (newNote) {
          messages.value[aiMsgIndex].content = `🎉 **本地备忘笔记已自动生成并保存成功！**\n\n- **标题**：${parsed.title}\n- **分类标签**：${(parsed.tags || []).map((t: string) => '#' + t).join(' ')}\n\n*系统已为您自动在左侧开启该笔记编辑 Tab，您可以直接查看或做进一步润色。*`
          notesStore.openNoteTab(newNote.id)
        } else {
          messages.value[aiMsgIndex].content = `❌ 笔记整理完毕，但保存至本地 SQLite 数据库失败。`
        }
      } else {
        messages.value[aiMsgIndex].content = `❌ 笔记生成失败。大模型未按预期返回 title 或 content 字段。\n\n原始回复：\n${responseText}`
      }
    } catch (e: any) {
      messages.value[aiMsgIndex].content = `❌ 整理笔记失败: ${e.message || e}`
    } finally {
      sending.value = false
      scrollToBottom()
    }
  }

  async function polishMemo() {
    const text = userInput.value.trim()
    if (!text) { status.pushMessage('请先在输入框中输入需要润色的文字', 'warn'); return }
    sending.value = true
    messages.value.push({ role: 'user', content: `帮我润色以下内容，整理为漂亮的 Markdown 格式并保持原意：\n\n${text}` })
    userInput.value = ''
    scrollToBottom()
    const currentModel = aiModel.value || 'Default Model'
    messages.value.push({ role: 'assistant', content: '', modelName: currentModel })
    const aiMsgIndex = messages.value.length - 1
    try {
      await callAiStream(
        [{ role: 'system', content: '你是一个文字润色专家，专门负责把粗糙的书写、碎碎念或无格式文本整理成结构清晰、排版优美并具有清晰层级的 Markdown 备忘录。' },
         { role: 'user', content: text }],
        (chunk) => { messages.value[aiMsgIndex].content += chunk; scrollToBottom() }
      )
    } catch (e: any) { messages.value[aiMsgIndex].content = `润色失败: ${e.message}` }
    finally { sending.value = false; scrollToBottom() }
  }

  async function recommendTags() {
    const text = userInput.value.trim()
    if (!text) { status.pushMessage('请先在输入框中输入需要提取标签的文字', 'warn'); return }
    sending.value = true
    messages.value.push({ role: 'user', content: `为以下文字推荐并提取合适的主题标签（用 #标签 格式输出）：\n\n${text}` })
    userInput.value = ''
    scrollToBottom()
    const currentModel = aiModel.value || 'Default Model'
    messages.value.push({ role: 'assistant', content: '', modelName: currentModel })
    const aiMsgIndex = messages.value.length - 1
    try {
      await callAiStream(
        [{ role: 'system', content: '你是一个标签提取专家。用户会提供一段文字，你需要输出 2-5 个代表这段文字核心主题的标签。必须采用 #标签 格式。不需要输出其他解释内容。' },
         { role: 'user', content: text }],
        (chunk) => { messages.value[aiMsgIndex].content += chunk; scrollToBottom() }
      )
    } catch (e: any) { messages.value[aiMsgIndex].content = `提取标签失败: ${e.message}` }
    finally { sending.value = false; scrollToBottom() }
  }

  // ═══════════════════════════════════════════
  //  6. METHODS — Send Messages
  // ═══════════════════════════════════════════

  function handleEnterKey(e: KeyboardEvent) {
    if (e.shiftKey) return
    e.preventDefault()
    sendMessage()
  }

  async function sendMessage() {
    const text = userInput.value.trim()
    if (!text || sending.value) return

    // Mode-based routing
    if (chatMode.value === '创作') {
      await agentSendMessage(text)
      return
    }
    if (chatMode.value === '规划') {
      await planSendMessage(text)
      return
    }
    // 问答 mode
    await doSendMessage(text)
  }

  async function doSendMessage(text: string) {
    // 确保有当前会话
    if (!ai.currentSessionId) {
      const title = text.length > 30 ? text.slice(0, 30) + '...' : text
      await ai.createSession(title)
      // 缓存新会话的欢迎消息
      sessionMessages.set(ai.currentSessionId!, [{ ...defaultWelcome }])
    }

    messages.value.push({ role: 'user', content: text })
    userInput.value = ''
    sending.value = true
    scrollToBottom()

    let systemPromptStr = ''
    let promptInstruction = ''
    const userQuery = text.toLowerCase()
    const hasNoteKeywords = /笔记|备忘|文档|知识|记录|草稿|我的/.test(userQuery)
    const shouldRetrieve = enableLocalKb.value || hasNoteKeywords

    if (shouldRetrieve) {
      await notesStore.loadAllNotes()
      const activeNotes = notesStore.notes.filter(n => !n.isDeleted && !n.isArchived)
      const isListNotesRequest = /查看笔记|列出笔记|有什么笔记|所有笔记|我的笔记|有哪些笔记|有哪些文档|有什么备忘|找下笔记/.test(userQuery)

      if (isListNotesRequest) {
        const sortedRecent = [...activeNotes].sort((a, b) => {
          const tA = new Date(a.updatedAt || a.createdAt).getTime()
          const tB = new Date(b.updatedAt || b.createdAt).getTime()
          return tB - tA
        }).slice(0, 15)
        const listText = sortedRecent.map((n, idx) =>
          `${idx + 1}. 【${n.title || '无标题'}】(更新于: ${n.updatedAt || n.createdAt}) - ${n.content ? n.content.slice(0, 80) + (n.content.length > 80 ? '...' : '') : '无内容'}`
        ).join('\n')
        promptInstruction = `\n[系统感知] 用户提出了查看或列出其笔记的请求。以下是用户最近更新的 15 篇笔记列表摘要：\n${listText}\n请直接向用户展现此列表，并温柔、主动地询问用户想要详细阅读或处理哪一篇。\n`
      } else {
        const terms = extractTerms(userQuery)
        const titleMatched = activeNotes.filter(n => n.title && (userQuery.includes(n.title.toLowerCase()) || n.title.toLowerCase().includes(userQuery)))
        const scored = activeNotes.map(n => {
          let score = 0
          const titleLower = (n.title || '').toLowerCase()
          const contentLower = (n.content || '').toLowerCase()
          if (titleLower && (userQuery.includes(titleLower) || titleLower.includes(userQuery))) score += 150
          n.tags.forEach(t => { if (userQuery.includes(t.toLowerCase())) score += 50 })
          terms.forEach(term => {
            if (titleLower && titleLower.includes(term)) score += term.length * 30
            if (contentLower && contentLower.includes(term)) score += term.length * 8
            n.tags.forEach(t => { if (t.toLowerCase().includes(term)) score += term.length * 15 })
          })
          return { note: n, score }
        })
        const isSummaryRequest = /汇总|所有|全部|概括|总结我|整理我/.test(userQuery)
        let contextNotes = scored.filter(x => x.score > 0).map(x => x.note)
        if (contextNotes.length === 0 || isSummaryRequest) contextNotes = activeNotes.slice(0, 10)
        else if (titleMatched.length > 0) contextNotes = Array.from(new Set([...titleMatched, ...contextNotes])).slice(0, 5)
        else contextNotes = scored.sort((a, b) => b.score - a.score).map(x => x.note).slice(0, 5)
        const referenceText = contextNotes.map(n => `[笔记标题: ${n.title || '无标题'}, 更新时间: ${n.updatedAt || n.createdAt}]\n内容:\n${n.content}`).join('\n\n---\n\n')
        promptInstruction = referenceText
          ? `\n以下是用户本地笔记库中相关的笔记供参考：\n${referenceText}\n请结合这些参考笔记直接且简洁地回答用户的问题，如果有相关内容可进行引用或说明。\n`
          : '\n未找到明确相关的本地笔记内容。请正常回答用户，或告诉用户您的本地笔记库中目前可能还没有相关内容。\n'
      }
      systemPromptStr = `你是通用 AI 助手，也是用户的本地备忘笔记助理。${promptInstruction}请直接、友好地回答用户。`
    } else {
      systemPromptStr = '你是一个通用 AI 助手。请直接、简洁地回答用户的问题。不要说"作为XX助手"之类的话，直接回答问题即可。'
    }

    if (activeChatRoleId.value === 'auto') {
      systemPromptStr = `你是 JC9 本地桌面应用中的 AI 助手，运行在用户的电脑上。
你可以直接读取、写入用户电脑上的本地文件（通过 Agent 模式的工具）。
如果用户请求涉及操作本地文件、代码、终端命令，请告知用户开启"执行模式"（顶部的模式选择器）来执行。
如果用户只是提问或聊天，请直接、简洁地回答问题，不要说"作为XX助手"之类的话。`
      if (enableLocalKb.value && promptInstruction) systemPromptStr += `\n\n以下是用户本地笔记库中相关的参考内容：\n${promptInstruction}`
    } else {
      const matchedRole = chatRolesList.value.find(r => r.id === activeChatRoleId.value) || null
      if (matchedRole) {
        systemPromptStr = `${matchedRole.systemPrompt}\n\n当前任务：请以该角色的专业设定与视角，协助解答用户的问题。`
        if (enableLocalKb.value && promptInstruction) systemPromptStr += `\n${promptInstruction}`
      }
    }

    if (enableDeepThink.value) {
      systemPromptStr += `\n\n【重要指令 - 深度思考模式已开启】：
请在回答前进行深入、细致的逐步思考和推导。在正式输出回答前，请先写出你的思考过程，以格式 \`思考过程：...\` 呈现，然后再输出最终答复。请展现严密的逻辑性，层层深入剖析问题。`
    }

    const currentModel = aiModel.value || 'Default Model'
    messages.value.push({ role: 'assistant', content: '', modelName: currentModel, roleName: activeChatRoleId.value === 'auto' ? undefined : activeChatRole.value.name })
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
      messages.value[aiMsgIndex].content = `呼叫 AI 失败: ${e.message}\n请检查您的快捷配置和 API 可用性。`
    } finally {
      sending.value = false
      scrollToBottom()
      // 保存当前会话消息到缓存和文件
      if (ai.currentSessionId) {
        sessionMessages.set(ai.currentSessionId, [...messages.value])
        schedulePersist()
      }
    }
  }

  // ═══════════════════════════════════════════
  //  7. METHODS — Agent / Plan Modes
  // ═══════════════════════════════════════════

  async function agentSendMessage(text: string) {
    messages.value.push({ role: 'user', content: text })
    userInput.value = ''
    sending.value = true
    scrollToBottom()

    try {
      await invoke('ai_configure_llm', {
        provider: aiProvider.value,
        apiKey: aiApiKey.value,
        baseUrl: aiEndpoint.value,
        model: aiModel.value,
      })
    } catch { /* ignore */ }

    try {
      if (!ai.currentSessionId) {
        const title = text.slice(0, 30) + (text.length > 30 ? '...' : '')
        await ai.createSession(title)
        addSystemBubble('📋 新会话已创建')
      }
      addSystemBubble('🧠 正在分析并拆解任务...')
      const tasks = await ai.planTask(ai.currentSessionId!, text)
      if (tasks.length > 0) {
        const list = tasks.map(t => `  • **${t.title}** _(${statusLabel(t.status)})_`).join('\n')
        addSystemBubble(`✅ 已规划 **${tasks.length}** 个子任务：\n${list}`)
        for (const task of tasks) {
          if (task.status === 'pending') {
            const role = getRole(task.assignedWorker)
            addSystemBubble(`🚀 启动 Worker：「${task.title}」... [分配角色: ${role.icon} ${role.name}]`)
            const sp = `${role.systemPrompt}\n\n当前任务描述及 ReAct 要求：${task.description}`
            await ai.spawnWorker(ai.currentSessionId!, task, sp)
          }
        }
      } else {
        addSystemBubble('⚠️ 任务规划返回空，请尝试更具体的描述。')
      }
    } catch (e: any) {
      addSystemBubble(`❌ Agent 错误: ${e}`)
    } finally {
      sending.value = false
      scrollToBottom()
      if (ai.currentSessionId) {
        sessionMessages.set(ai.currentSessionId, [...messages.value])
        schedulePersist()
      }
    }
  }

  async function planSendMessage(text: string) {
    messages.value.push({ role: 'user', content: text })
    userInput.value = ''
    sending.value = true
    scrollToBottom()

    try {
      await invoke('ai_configure_llm', {
        provider: aiProvider.value, apiKey: aiApiKey.value,
        baseUrl: aiEndpoint.value, model: aiModel.value,
      })
    } catch { /* ignore */ }

    try {
      if (!ai.currentSessionId) {
        const title = text.slice(0, 30) + (text.length > 30 ? '...' : '')
        await ai.createSession(title)
        addSystemBubble('📋 新会话已创建')
      }
      addSystemBubble('🧠 正在分析需求并规划任务...')
      const tasks = await ai.planTask(ai.currentSessionId!, text)
      if (tasks.length > 0) {
        const list = tasks.map(t =>
          `  • [${t.priority >= 0 ? `P${t.priority}` : 'P?'}] **${t.title}** - ${t.description.slice(0, 60)}${t.description.length > 60 ? '...' : ''}`
        ).join('\n')
        addSystemBubble(`📋 **任务规划完成** (${tasks.length} 项)：\n${list}\n\n💡 切换到「」模式可自动执行以上任务。`)
      } else {
        addSystemBubble('⚠️ 任务规划为空，请尝试更具体的描述。')
      }
    } catch (e: any) {
      addSystemBubble(`❌ 规划失败: ${e}`)
    } finally {
      sending.value = false
      scrollToBottom()
      if (ai.currentSessionId) {
        sessionMessages.set(ai.currentSessionId, [...messages.value])
        schedulePersist()
      }
    }
  }

  function addSystemBubble(content: string) {
    messages.value.push({ role: 'system', content })
    scrollToBottom()
  }

  function clearChat() {
    // 缓存当前会话消息
    const prevId = ai.currentSessionId
    if (prevId && messages.value.length > 0) {
      sessionMessages.set(prevId, [...messages.value])
      schedulePersist()
    }
    const newMsg: Message = { role: 'assistant', content: '对话已清空。您可以开始新的提问，或者在下方输入文本并使用快捷工具进行处理。' }
    messages.value = [newMsg]
    // 创建新会话
    if (ai.currentSessionId) {
      ai.createSession('新对话').then((newId) => {
        if (newId) {
          sessionMessages.set(newId, [{ ...newMsg }])
        }
      })
    }
  }

  // ═══════════════════════════════════════════
  //  8. METHODS — Approval & Worker
  // ═══════════════════════════════════════════

  async function handleApprove(req: ApprovalRequest) { await ai.approveRequest(req.id) }
  async function handleDeny(req: ApprovalRequest) { await ai.denyRequest(req.id) }
  async function handleApproveAll() { for (const req of ai.pendingApprovals) await ai.approveRequest(req.id) }
  async function handleDenyAll() { for (const req of ai.pendingApprovals) await ai.denyRequest(req.id) }
  async function handleKillWorker(workerId: string) { await ai.killWorker(workerId) }
  async function handleKillAllWorkers() { for (const w of ai.activeWorkers) await ai.killWorker(w.id) }

  function toggleWorkerExpand(workerId: string) {
    expandedWorkers.value[workerId] = !expandedWorkers.value[workerId]
  }

  function getTaskTitle(taskId: string): string {
    const task = ai.taskTree.find(t => t.id === taskId)
    return task ? task.title : '未命名任务'
  }

  function getWorkerRole(taskId: string) {
    const task = ai.taskTree.find(t => t.id === taskId)
    return getRole(task?.assignedWorker)
  }

  function formatTime(timestampStr: string): string {
    if (!timestampStr) return ''
    try { return new Date(timestampStr).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) }
    catch { return timestampStr }
  }

  const statusColors: Record<string, string> = {
    thinking: '#58a6ff', acting: '#f0883e', observing: '#a371f7',
    waitingApproval: '#d29922', completed: '#3fb950', failed: '#f85149',
    killed: '#8b949e', pending: '#8b949e', inProgress: '#58a6ff',
    blocked: '#d29922', active: '#3fb950', paused: '#8b949e',
  }

  const riskColors: Record<string, string> = {
    low: '#3fb950', medium: '#d29922', high: '#f0883e', critical: '#f85149',
  }

  function statusLabel(s: string): string {
    const map: Record<string, string> = {
      thinking: '思考中', acting: '执行中', observing: '观察中',
      waitingApproval: '待审批', completed: '已完成', failed: '失败',
      killed: '已终止', pending: '待处理', inProgress: '进行中',
      blocked: '阻塞', active: '活跃', paused: '暂停',
    }
    return map[s] ?? s
  }

  async function searchKnowledgeBase() {
    if (!kbSearchQuery.value.trim()) return
    try { kbSearchResults.value = await invoke<KbEntry[]>('ai_search_knowledge', { query: kbSearchQuery.value.trim(), limit: 8 }) }
    catch { /* ignore */ }
  }

  // ═══════════════════════════════════════════
  //  9. METHODS — Browser & Workspace & Session
  // ═══════════════════════════════════════════

  const manualPath = ref('')

  async function handleBrowserOpen() {
    showBrowserDialog.value = true
  }

  async function handleBrowserConfirm() {
    if (!browserUrlInput.value.trim()) return
    if (!browserUrlInput.value.startsWith('http')) browserUrlInput.value = 'https://' + browserUrlInput.value
    try {
      await invoke('ai_browser_navigate', { url: browserUrlInput.value })
      showBrowserDialog.value = false
    } catch (e: any) { console.error('打开浏览器失败:', e) }
  }

  async function handleSelectWorkspace() {
    await ai.changeWorkspaceDialog()
    manualPath.value = ai.workspaceRoot
  }

  async function handleCreateSessionPopup() {
    if (!newSessionTitle.value.trim()) return
    await ai.createSession(newSessionTitle.value.trim())
    newSessionTitle.value = ''
  }

  async function selectSession(id: string) {
    // 缓存并持久化当前会话消息
    const prevId = ai.currentSessionId
    if (prevId && messages.value.length > 0) {
      sessionMessages.set(prevId, [...messages.value])
      await saveChatMessages(prevId, messages.value)
    }
    // 切换到目标会话
    ai.currentSessionId = id
    if (id && sessionMessages.has(id)) {
      messages.value = [...sessionMessages.get(id)!]
    } else if (id) {
      // 尝试从文件加载
      const loaded = await loadChatMessages(id)
      if (loaded && loaded.length > 0) {
        messages.value = loaded
        sessionMessages.set(id, [...loaded])
      } else {
        messages.value = [{ ...defaultWelcome }]
      }
    } else {
      messages.value = [{ ...defaultWelcome }]
    }
  }

  async function setReasoningEffort(effort: 'high' | 'max' | 'off') {
    reasoningEffort.value = effort
    try { await invoke('ai_set_reasoning_effort', { effort: effort === 'off' ? '' : effort }) }
    catch { /* ignore */ }
  }

  // ═══════════════════════════════════════════
  //  10. POLLING & LIFECYCLE
  // ═══════════════════════════════════════════

  function startPolling() {
    pollTimer.value = window.setInterval(async () => {
      loadCustomModels()
      try { await ai.loadWorkers() } catch { /* ignore */ }
      try { await ai.loadPendingApprovals() } catch { /* ignore */ }
      try { await ai.loadDrafts() } catch { /* ignore */ }
    }, 3000)
  }

  function stopPolling() {
    if (pollTimer.value !== null) {
      clearInterval(pollTimer.value)
      pollTimer.value = null
    }
  }

  async function init() {
    loadCustomModels()
    loadConfig()
    chatRolesList.value = loadAllRoles()

    let hasOllama = false, hasVllm = false
    const saved = localStorage.getItem('notes-ai-models')
    if (saved) {
      try {
        const configs: Array<{ provider: string }> = JSON.parse(saved)
        hasOllama = configs.some(c => c.provider === 'ollama')
        hasVllm = configs.some(c => c.provider === 'vllm')
      } catch { /* ignore */ }
    }
    if (hasOllama) fetchOllamaModels()
    if (hasVllm) fetchVllmModels()

    await ai.loadSessions()
    await ai.loadWorkspaceRoot()
    manualPath.value = ai.workspaceRoot
    await ai.initListeners()
    await ai.registerAllFrontendTools()
    await ai.loadDrafts()
    localConfig.value = { ...ai.costConfig }

    // 从文件加载当前会话消息（如果有）
    if (ai.currentSessionId) {
      const loaded = await loadChatMessages(ai.currentSessionId)
      if (loaded && loaded.length > 0) {
        messages.value = loaded
        sessionMessages.set(ai.currentSessionId, [...loaded])
      }
    }

    workerUnlisten.value = await listen<WorkerState>('ai:worker-update', (event) => {
      const w = event.payload
      const taskTitle = ai.taskTree.find(t => t.id === w.taskId)?.title || '开发任务'
      if (w.status === 'completed') addSystemBubble(`🎉 子任务「${taskTitle}」已顺利执行完毕，所有代码变更已安全合入工作区。`)
      else if (w.status === 'failed') addSystemBubble(`❌ 子任务「${taskTitle}」执行失败。原因: ${w.terminationReason || '遇到阻碍或触发熔断'}`)
      else if (w.status === 'killed') addSystemBubble(`🛑 子任务「${taskTitle}」已被手动强制终止。`)
    })

    startPolling()
  }

  function destroy() {
    stopPolling()
    ai.destroyListeners()
    if (workerUnlisten.value) {
      workerUnlisten.value()
    }
  }

  // ═══════════════════════════════════════════
  //  RETURN
  // ═══════════════════════════════════════════

  return {
    // State
    messages, userInput, sending, inputTextarea,
    chatMode, chatModes,
    enableDeepThink, enableLocalKb, reasoningEffort,
    aiProvider, aiEndpoint, aiApiKey, aiModel,
    selectedCombinedModel, modelOptions, loadingModels,
    activeChatRoleId, chatRolesList, activeChatRole,
    isFocused, showModelSettingsModal, showSessionPopup,
    newSessionTitle, showBrowserDialog, browserUrlInput,
    isConsoleExpanded, expandedWorkers,
    kbSearchQuery, kbSearchResults, manualPath,
    localConfig, customModels, ollamaModels, vllmModels,
    // Computed
    placeholderText, workspaceShortName,
    // Methods — Config
    loadConfig, handleModelChange, saveQuickConfig,
    refreshLocalModels, loadCustomModels,
    // Methods — Streaming
    callAiStream, extractTerms, scrollToBottom, autoResizeTextarea,
    // Methods — Notes & Polish
    attachActiveNote, handleCreateNoteFromAi, polishMemo, recommendTags,
    // Methods — Send
    handleEnterKey, sendMessage, clearChat, addSystemBubble,
    // Methods — Agent & Plan
    agentSendMessage, planSendMessage,
    // Methods — Approval & Worker
    handleApprove, handleDeny, handleApproveAll, handleDenyAll,
    handleKillWorker, handleKillAllWorkers,
    toggleWorkerExpand, getTaskTitle, getWorkerRole,
    formatTime, statusColors, riskColors, statusLabel,
    searchKnowledgeBase,
    // Methods — Browser & Workspace & Session
    handleBrowserOpen, handleBrowserConfirm,
    handleSelectWorkspace, handleCreateSessionPopup, selectSession,
    setReasoningEffort,
    // Lifecycle
    init, destroy,
  }
}
