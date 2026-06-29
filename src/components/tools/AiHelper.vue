<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import type { TaskNode, WorkerState, ApprovalRequest } from '@/types/ai'

const notesStore = useNotesStore()
const status = useStatusStore()
const ai = useAiStore()

// 消息列表结构升级，增加 modelName 属性
interface Message {
  role: 'user' | 'assistant' | 'system'
  content: string
  modelName?: string
}

const messages = ref<Message[]>([
  { role: 'assistant', content: '您好！我是您的 JC9 笔记 AI 助手。我可以帮您润色备忘、推荐标签，或者基于您的本地笔记库回答问题。' }
])

const userInput = ref('')
const sending = ref(false)

// ── DeepSeek 风格 UI 状态 ──
const enableDeepThink = ref(false)
const enableLocalKb = ref(true)
const isFocused = ref(false)

// ── AI Agent 相关变量与绑定 ──
const newSessionTitle = ref('')
const taskRequest = ref('')
const selectedTaskId = ref<string | null>(null)
const systemPrompt = ref('你是一个专业的代码助手，请使用 ReAct 模式完成任务。')
const pollTimer = ref<number | null>(null)
const manualPath = ref('')

const showSettings = ref(false)
const localConfig = ref({
  inputCachedCostPerM: 0.025,
  inputUncachedCostPerM: 3.0,
  outputCostPerM: 6.0,
  costLimit: 5.0
})

const selectedTask = computed<TaskNode | null>(() =>
  ai.taskTree.find((t) => t.id === selectedTaskId.value) ?? null,
)

const statusColors: Record<string, string> = {
  thinking: '#58a6ff',
  acting: '#f0883e',
  observing: '#a371f7',
  waitingApproval: '#d29922',
  completed: '#3fb950',
  failed: '#f85149',
  killed: '#8b949e',
  pending: '#8b949e',
  inProgress: '#58a6ff',
  blocked: '#d29922',
  active: '#3fb950',
  paused: '#8b949e',
}

const riskColors: Record<string, string> = {
  low: '#3fb950',
  medium: '#d29922',
  high: '#f0883e',
  critical: '#f85149',
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

// ── 将当前活动笔记内容快捷附加到输入框 ──
function attachActiveNote() {
  const activeNoteId = notesStore.activeNoteTabId
  if (!activeNoteId) return
  const note = notesStore.notes.find(n => n.id === activeNoteId)
  if (note) {
    const textToAttach = `\n[参考笔记: ${note.title || '无标题'}]\n${note.content}\n`
    userInput.value += textToAttach
    status.pushMessage('已成功附加当前活动笔记', 'success')
  }
}

// ── 拦截 Enter 直接发送消息，而 Shift+Enter 正常换行 ──
function handleEnterKey(e: KeyboardEvent) {
  if (e.shiftKey) return
  e.preventDefault()
  sendMessage()
}

// AI 核心配置
const aiProvider = ref('ollama')
const aiEndpoint = ref('http://127.0.0.1:11434')
const aiApiKey = ref('')
const aiModel = ref('llama3')

// ── 聊天消息区动态滚动置底 ──
function scrollToBottom() {
  nextTick(() => {
    const el = document.querySelector('.chat-messages')
    if (el) {
      el.scrollTop = el.scrollHeight
    }
  })
}

// ── 供应商专属 API Key 缓存 ──
const openaiKey = ref('')
const deepseekKey = ref('')
const geminiKey = ref('')

const selectedCombinedModel = ref('')
const ollamaModels = ref<string[]>([])
const vllmModels = ref<string[]>([])
const loadingModels = ref(false)

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
  } catch (e) {
    console.warn('无法获取 Ollama 本地模型列表')
  }
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
  } catch (e) {
    console.warn('无法获取 vLLM 本地模型列表')
  } finally {
    loadingModels.value = false
  }
}

async function refreshLocalModels() {
  loadingModels.value = true
  status.pushMessage('正在刷新本地模型列表...', 'info')
  await Promise.all([
    fetchOllamaModels(),
    fetchVllmModels()
  ])
  loadingModels.value = false
  status.pushMessage('本地模型列表刷新完毕', 'success')
  selectedCombinedModel.value = `${aiProvider.value}/${aiModel.value}`
}

function loadConfig() {
  aiProvider.value = localStorage.getItem('notes-ai-provider') || 'ollama'
  aiEndpoint.value = localStorage.getItem('notes-ai-endpoint') || 'http://127.0.0.1:11434'
  aiApiKey.value = localStorage.getItem('notes-ai-apikey') || ''
  aiModel.value = localStorage.getItem('notes-ai-model') || 'llama3'

  openaiKey.value = localStorage.getItem('notes-ai-apikey-openai') || ''
  deepseekKey.value = localStorage.getItem('notes-ai-apikey-deepseek') || ''
  geminiKey.value = localStorage.getItem('notes-ai-apikey-gemini') || ''

  if (!localStorage.getItem('notes-ai-endpoint-ollama')) {
    localStorage.setItem('notes-ai-endpoint-ollama', 'http://127.0.0.1:11434')
  }
  if (!localStorage.getItem('notes-ai-endpoint-vllm')) {
    localStorage.setItem('notes-ai-endpoint-vllm', 'http://192.168.5.100:8000/v1')
  }
}

const modelOptions = computed(() => {
  const groups: Record<string, { name: string; label: string }[]> = {}

  const ollamaList: { name: string; label: string }[] = []
  if (ollamaModels.value.length > 0) {
    ollamaModels.value.forEach(m => ollamaList.push({ name: m, label: m }))
  }
  const userOllama = localStorage.getItem('notes-ai-model-ollama')
  if (userOllama && !ollamaList.some(x => x.name === userOllama)) {
    ollamaList.push({ name: userOllama, label: `${userOllama} (手动配置)` })
  }
  if (ollamaList.length > 0) {
    groups['Ollama'] = ollamaList
  }

  const vllmList: { name: string; label: string }[] = []
  if (vllmModels.value.length > 0) {
    vllmModels.value.forEach(m => vllmList.push({ name: m, label: m }))
  }
  const userVllm = localStorage.getItem('notes-ai-model-vllm')
  if (userVllm && !vllmList.some(x => x.name === userVllm)) {
    vllmList.push({ name: userVllm, label: `${userVllm} (手动配置)` })
  }
  if (vllmList.length > 0) {
    groups['vLLM'] = vllmList
  }

  if (deepseekKey.value || aiProvider.value === 'deepseek') {
    groups['DeepSeek'] = [
      { name: 'deepseek-chat', label: 'deepseek-chat' },
      { name: 'deepseek-coder', label: 'deepseek-coder' }
    ]
  }

  if (openaiKey.value || aiProvider.value === 'openai') {
    groups['OpenAI'] = [
      { name: 'gpt-4o-mini', label: 'gpt-4o-mini' },
      { name: 'gpt-4o', label: 'gpt-4o' }
    ]
  }

  if (geminiKey.value || aiProvider.value === 'gemini') {
    groups['Gemini'] = [
      { name: 'gemini-1.5-flash', label: 'gemini-1.5-flash' },
      { name: 'gemini-1.5-pro', label: 'gemini-1.5-pro' }
    ]
  }

  return groups
})

function handleModelChange() {
  const parts = selectedCombinedModel.value.split('/')
  if (parts.length < 2) return
  const prov = parts[0]
  const model = parts.slice(1).join('/')

  aiProvider.value = prov
  aiModel.value = model

  const savedEndpoint = localStorage.getItem(`notes-ai-endpoint-${prov}`)
  const savedApiKey = localStorage.getItem(`notes-ai-apikey-${prov}`)

  if (savedEndpoint) {
    aiEndpoint.value = savedEndpoint
  } else {
    if (prov === 'ollama') aiEndpoint.value = 'http://127.0.0.1:11434'
    else if (prov === 'vllm') aiEndpoint.value = 'http://192.168.5.100:8000/v1'
    else if (prov === 'deepseek') aiEndpoint.value = 'https://api.deepseek.com/v1'
    else if (prov === 'openai') aiEndpoint.value = 'https://api.openai.com/v1'
    else if (prov === 'gemini') aiEndpoint.value = 'https://generativelanguage.googleapis.com'
  }

  if (savedApiKey) {
    aiApiKey.value = savedApiKey
  } else {
    aiApiKey.value = ''
  }

  saveQuickConfig()
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

// ── 核心流式请求（SSE/Chunked 读取） ──
async function callAiStream(promptMessages: Message[], onChunk: (text: string) => void) {
  loadConfig()

  const headers: Record<string, string> = {
    'Content-Type': 'application/json'
  }

  if (aiProvider.value !== 'ollama' && aiProvider.value !== 'vllm' && aiApiKey.value) {
    headers['Authorization'] = `Bearer ${aiApiKey.value}`
  } else if (aiProvider.value === 'vllm' && aiApiKey.value) {
    headers['Authorization'] = `Bearer ${aiApiKey.value}`
  }

  let body = {}
  let url = ''

  if (aiProvider.value === 'ollama') {
    url = `${aiEndpoint.value}/api/chat`
    body = {
      model: aiModel.value,
      messages: promptMessages.map(m => ({ role: m.role, content: m.content })),
      stream: true
    }
  } else {
    url = `${aiEndpoint.value}/chat/completions`
    body = {
      model: aiModel.value,
      messages: promptMessages.map(m => ({ role: m.role, content: m.content })),
      temperature: 0.7,
      stream: true
    }
  }

  const res = await fetch(url, {
    method: 'POST',
    headers,
    body: JSON.stringify(body)
  })

  if (!res.ok) {
    throw new Error(`请求失败 (${res.status}): ${await res.text()}`)
  }

  const reader = res.body?.getReader()
  if (!reader) {
    throw new Error('当前环境或接口未支持流式读取 (ReadableStream 空)')
  }

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
        } catch (e) {
          // 忽略
        }
      } else {
        if (trimmed.startsWith('data:')) {
          const dataVal = trimmed.slice(5).trim()
          if (dataVal === '[DONE]') continue
          try {
            const json = JSON.parse(dataVal)
            const text = json.choices?.[0]?.delta?.content || ''
            if (text) onChunk(text)
          } catch (e) {
            // 忽略
          }
        }
      }
    }
  }
}

// ── 智能中英文切词与停用词过滤 ──
function extractTerms(query: string): string[] {
  const terms: string[] = []
  const blocks = query.toLowerCase().match(/[\u4e00-\u9fa5]+|[a-z0-9]+/g) || []
  const stopWords = new Set([
    '的', '了', '和', '是', '就', '都', '而', '及', '与', '这', '那', '有', '无', 
    '一个', '一下', '关于', '内容', '笔记', '总结', '阅读', '查找', '搜索', '一篇', 
    '这篇', '那些', '哪些', '什么', '请问', '有没有', '怎么', '如何', '帮我', '看看', 
    '谁', '哪里', '记了'
  ])
  
  for (const block of blocks) {
    if (stopWords.has(block)) continue
    if (/[\u4e00-\u9fa5]/.test(block)) {
      if (block.length > 1 && !stopWords.has(block)) {
        terms.push(block)
      }
      for (let i = 0; i < block.length - 1; i++) {
        const sub = block.slice(i, i + 2)
        if (!stopWords.has(sub)) {
          terms.push(sub)
        }
      }
      if (block.length === 1 && !stopWords.has(block)) {
        terms.push(block)
      }
    } else {
      if (block.length > 0) {
        terms.push(block)
      }
    }
  }
  return Array.from(new Set(terms))
}

// ── 发送消息（智能 RAG 感知） ──
async function sendMessage() {
  const text = userInput.value.trim()
  if (!text || sending.value) return

  messages.value.push({ role: 'user', content: text })
  userInput.value = ''
  sending.value = true
  scrollToBottom()

  let systemPromptStr = ''

  if (enableLocalKb.value) {
    await notesStore.loadAllNotes()
    const userQuery = text.toLowerCase()
    const activeNotes = notesStore.notes.filter(n => !n.isDeleted && !n.isArchived)
    const terms = extractTerms(userQuery)
    const titleMatched = activeNotes.filter(n => n.title && (userQuery.includes(n.title.toLowerCase()) || n.title.toLowerCase().includes(userQuery)))

    const scored = activeNotes.map(n => {
      let score = 0
      const titleLower = (n.title || '').toLowerCase()
      const contentLower = (n.content || '').toLowerCase()

      if (titleLower && (userQuery.includes(titleLower) || titleLower.includes(userQuery))) {
        score += 150
      }
      n.tags.forEach(t => {
        if (userQuery.includes(t.toLowerCase())) score += 50
      })
      terms.forEach(term => {
        if (titleLower && titleLower.includes(term)) score += term.length * 30
        if (contentLower && contentLower.includes(term)) score += term.length * 8
        n.tags.forEach(t => {
          if (t.toLowerCase().includes(term)) score += term.length * 15
        })
      })
      return { note: n, score }
    })

    const isSummaryRequest = /汇总|所有|全部|概括|总结我|整理我/.test(userQuery)
    let contextNotes = scored.filter(x => x.score > 0).map(x => x.note)
    
    if (contextNotes.length === 0 || isSummaryRequest) {
      contextNotes = activeNotes.slice(0, 40)
    } else if (titleMatched.length > 0) {
      contextNotes = Array.from(new Set([...titleMatched, ...contextNotes])).slice(0, 10)
    } else {
      contextNotes = scored.sort((a, b) => b.score - a.score).map(x => x.note).slice(0, 10)
    }

    const referenceText = contextNotes
      .map(n => `[笔记标题: ${n.title || '无标题'}, 更新时间: ${n.updatedAt || n.createdAt}]\n内容:\n${n.content}`)
      .join('\n\n---\n\n')

    systemPromptStr = `你是一个内置在用户本地客户端（JC9 项目与终端管理器）中的专属 AI 助理。
你拥有完全的本地笔记库读取权限。在每次对话中，系统都会自动为你检索并读取出与用户当前提问高度相关的本地笔记内容。

当前为你读取并注入的本地笔记如下：
=========================================
${referenceText}
=========================================

请注意：
1. 你当前【绝对有权限】访问这些本地笔记。不要对用户说“我无法访问你的本地文件”、“我无法读取本地笔记”、“需要你复制粘贴给我”等推阻之词。
2. 用户的所有关于“我记了什么”、“找一下某某笔记”、“总结某篇笔记”的提问，都请直接在上方提供的数据中寻找并进行回答。
3. 请以第一人称（如“我已经为您找到了关于‘标题2’的笔记，内容如下：”）来回答，表现出你确实能直接读取和管理他的笔记库。
4. 如果上述注入的内容中确实没有包含用户所要寻找的任何笔记（且你的常规知识也无法解答），你可以友好地引导用户：“在您当前的笔记库中似乎未检索到相关内容，您可以确认一下笔记标题或内容是否正确。”`
  } else {
    systemPromptStr = `你是一个内置在用户本地客户端（JC9 项目与终端管理器）中的专属 AI 助理。请以专业、清晰且对开发者友好的语气解答用户的问题。`
  }

  if (enableDeepThink.value) {
    systemPromptStr += `\n\n【重要指令 - 深度思考模式已开启】：
请在回答前进行深入、细致的逐步思考和推导。在正式输出回答前，请先写出你的思考过程，以格式 \`思考过程：...\` 呈现，然后再输出最终答复。请展现严密的逻辑性，层层深入剖析问题。`
  }

  const promptMsgs: Message[] = [
    { role: 'system', content: systemPromptStr },
    ...messages.value.filter(m => m.role === 'user' || m.role === 'assistant').slice(0, -1)
  ]

  const currentModel = aiModel.value || 'Default Model'
  messages.value.push({ role: 'assistant', content: '', modelName: currentModel })
  const aiMsgIndex = messages.value.length - 1

  try {
    await callAiStream(promptMsgs, (chunk) => {
      messages.value[aiMsgIndex].content += chunk
      scrollToBottom()
    })
  } catch (e: any) {
    messages.value[aiMsgIndex].content = `呼叫 AI 失败: ${e.message}\n请检查您的快捷配置和 API 可用性。`
  } finally {
    sending.value = false
    scrollToBottom()
  }
}

async function polishMemo() {
  const text = userInput.value.trim()
  if (!text) {
    status.pushMessage('请先在输入框中输入需要润色的文字', 'warn')
    return
  }

  sending.value = true
  messages.value.push({ role: 'user', content: `帮我润色以下内容，整理为漂亮的 Markdown 格式并保持原意：\n\n${text}` })
  userInput.value = ''
  scrollToBottom()

  const currentModel = aiModel.value || 'Default Model'
  messages.value.push({ role: 'assistant', content: '', modelName: currentModel })
  const aiMsgIndex = messages.value.length - 1

  try {
    const promptMsgs: Message[] = [
      { role: 'system', content: '你是一个文字润色专家，专门负责把粗糙的书写、碎碎念或无格式文本整理成结构清晰、排版优美并具有清晰层级的 Markdown 备忘录。' },
      { role: 'user', content: text }
    ]
    await callAiStream(promptMsgs, (chunk) => {
      messages.value[aiMsgIndex].content += chunk
      scrollToBottom()
    })
  } catch (e: any) {
    messages.value[aiMsgIndex].content = `润色失败: ${e.message}`
  } finally {
    sending.value = false
    scrollToBottom()
  }
}

async function recommendTags() {
  const text = userInput.value.trim()
  if (!text) {
    status.pushMessage('请先在输入框中输入需要提取标签的文字', 'warn')
    return
  }

  sending.value = true
  messages.value.push({ role: 'user', content: `为以下文字推荐并提取合适的主题标签（用 #标签 格式输出）：\n\n${text}` })
  userInput.value = ''
  scrollToBottom()

  const currentModel = aiModel.value || 'Default Model'
  messages.value.push({ role: 'assistant', content: '', modelName: currentModel })
  const aiMsgIndex = messages.value.length - 1

  try {
    const promptMsgs: Message[] = [
      { role: 'system', content: '你是一个标签提取专家。用户会提供一段文字，你需要输出 2-5 个代表这段文字核心主题的标签。必须采用 #标签 格式。不需要输出其他解释内容。' },
      { role: 'user', content: text }
    ]
    await callAiStream(promptMsgs, (chunk) => {
      messages.value[aiMsgIndex].content += chunk
      scrollToBottom()
    })
  } catch (e: any) {
    messages.value[aiMsgIndex].content = `提取标签失败: ${e.message}`
  } finally {
    sending.value = false
    scrollToBottom()
  }
}

function clearChat() {
  messages.value = [
    { role: 'assistant', content: '对话已清空。您可以开始新的提问，或者在下方输入文本并使用快捷工具进行处理。' }
  ]
}

const placeholderText = computed(() => {
  return `给 ${aiModel.value || 'AI'} 发送消息... (Enter 发送, Shift+Enter 换行)`
})

// ── AI Agent 业务处理 ──
async function handleCreateSession() {
  if (!newSessionTitle.value.trim()) return
  await ai.createSession(newSessionTitle.value.trim())
  newSessionTitle.value = ''
}

async function handlePlanTask() {
  if (!ai.currentSessionId || !taskRequest.value.trim()) return
  await ai.planTask(ai.currentSessionId, taskRequest.value.trim())
  taskRequest.value = ''
}

async function handleSpawnWorker(task: TaskNode) {
  if (!ai.currentSessionId) return
  await ai.spawnWorker(ai.currentSessionId, task, systemPrompt.value)
}

async function handleApprove(req: ApprovalRequest) {
  await ai.approveRequest(req.id)
}

async function handleDeny(req: ApprovalRequest) {
  await ai.denyRequest(req.id)
}

async function handleApproveAll() {
  for (const req of ai.pendingApprovals) {
    await ai.approveRequest(req.id)
  }
}

async function handleDenyAll() {
  for (const req of ai.pendingApprovals) {
    await ai.denyRequest(req.id)
  }
}

async function handlePromote(entryId: string) {
  await ai.promoteKnowledge(entryId)
}

async function saveSettings() {
  await ai.updateCostConfig(localConfig.value)
  showSettings.value = false
}

function estimateCost(tokenCount: number): string {
  const avgRate = (ai.costConfig.inputUncachedCostPerM * 0.8) + (ai.costConfig.outputCostPerM * 0.2);
  const cost = (tokenCount * avgRate) / 1_000_000.0;
  return cost.toFixed(4);
}

async function handleKillWorker(workerId: string) {
  await ai.killWorker(workerId)
}

async function handleSelectWorkspace() {
  await ai.changeWorkspaceDialog()
  manualPath.value = ai.workspaceRoot
}

async function handleManualWorkspace() {
  if (!manualPath.value.trim()) return
  await ai.changeWorkspaceManual(manualPath.value.trim())
}

function startPolling() {
  pollTimer.value = window.setInterval(async () => {
    await Promise.all([
      ai.loadWorkers(), 
      ai.loadPendingApprovals(),
      ai.loadDrafts()
    ])
  }, 2000)
}

function stopPolling() {
  if (pollTimer.value !== null) {
    clearInterval(pollTimer.value)
    pollTimer.value = null
  }
}

onMounted(async () => {
  loadConfig()
  fetchOllamaModels()
  fetchVllmModels()
  selectedCombinedModel.value = `${aiProvider.value}/${aiModel.value}`

  await ai.loadSessions()
  await ai.loadWorkspaceRoot()
  manualPath.value = ai.workspaceRoot
  await ai.initListeners()
  await ai.loadDrafts()
  localConfig.value = { ...ai.costConfig }
  startPolling()
})

onUnmounted(() => {
  stopPolling()
  ai.destroyListeners()
})
</script>

<template>
  <div class="ai-helper-container">
    <!-- 左侧：问答与聊天主面板 -->
    <div class="ai-chat-section">
      <div class="ai-chat-area">
        <!-- 消息滚动列表 -->
        <div class="chat-messages">
          <div v-for="(msg, i) in messages" :key="i" :class="['chat-bubble', msg.role]">
            <div class="bubble-sender">
              {{ msg.role === 'user' ? '您' : (msg.role === 'system' ? '系统' : (msg.modelName || 'AI Copilot')) }}
            </div>
            <div class="bubble-content" v-html="msg.content.replace(/\n/g, '<br/>')"></div>
          </div>
        </div>

        <!-- 快捷效率胶囊工具栏 -->
        <Transition name="fade-slide">
          <div class="shortcut-pills" v-if="userInput.trim().length > 0">
            <span class="shortcut-pill-desc">针对当前输入：</span>
            <button class="shortcut-pill" @click="polishMemo" :disabled="sending">✨ 润色排版</button>
            <button class="shortcut-pill" @click="recommendTags" :disabled="sending">🏷️ 提取标签</button>
          </div>
        </Transition>

        <!-- 一体化输入卡片 -->
        <div class="ds-input-card" :class="{ 'focused': isFocused, 'has-content': userInput.trim().length > 0 }">
          <textarea 
            v-model="userInput" 
            :placeholder="placeholderText" 
            class="ds-textarea"
            @focus="isFocused = true"
            @blur="isFocused = false"
            @keydown.enter.prevent="handleEnterKey"
          ></textarea>
          
          <div class="ds-control-bar">
            <div class="ds-pills">
              <!-- 模型选择 -->
              <div class="ds-pill-select-wrap">
                <svg class="ds-pill-icon model-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="3" y="11" width="18" height="10" rx="2"></rect>
                  <circle cx="12" cy="5" r="2"></circle>
                  <path d="M12 7v4"></path>
                  <line x1="8" y1="16" x2="8" y2="16"></line>
                  <line x1="16" y1="16" x2="16" y2="16"></line>
                </svg>
                <select v-model="selectedCombinedModel" @change="handleModelChange" class="ds-pill-select" title="切换 AI 模型">
                  <optgroup v-for="(models, providerName) in modelOptions" :key="providerName" :label="providerName">
                    <option v-for="m in models" :key="m.name" :value="providerName.toLowerCase() + '/' + m.name">
                      {{ m.label }}
                    </option>
                  </optgroup>
                </select>
                <button 
                  v-if="aiProvider === 'vllm' || aiProvider === 'ollama'" 
                  class="ds-pill-refresh" 
                  @click="refreshLocalModels" 
                  :disabled="loadingModels" 
                  title="刷新本地模型列表"
                >
                  <svg viewBox="0 0 16 16" class="refresh-icon-svg" :class="{ spinning: loadingModels }">
                    <path d="M1.5 8a6.5 6.5 0 0 1 10.5-5L14 5m0-3.5V5h-3.5M14.5 8a6.5 6.5 0 0 1-10.5 5L2 11m0 3.5V11h3.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
              </div>

              <!-- 深度思考 -->
              <button 
                class="ds-pill-btn deep-think" 
                :class="{ active: enableDeepThink }" 
                @click="enableDeepThink = !enableDeepThink"
              >
                <svg class="ds-pill-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(45 12 12)"></ellipse>
                  <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(-45 12 12)"></ellipse>
                  <circle cx="12" cy="12" r="1.5" fill="currentColor"></circle>
                </svg>
                深度思考
              </button>
              
              <!-- 本地知识库 -->
              <button 
                class="ds-pill-btn local-kb" 
                :class="{ active: enableLocalKb }" 
                @click="enableLocalKb = !enableLocalKb"
              >
                <svg class="ds-pill-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="9"></circle>
                  <line x1="2" y1="12" x2="22" y2="12"></line>
                  <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                </svg>
                本地知识库
              </button>
            </div>
            
            <div class="ds-actions">
              <button v-if="notesStore.activeNoteTabId" class="ds-action-btn attach" @click="attachActiveNote" title="附件当前笔记">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path>
                </svg>
              </button>
              
              <button class="ds-action-btn clear" @click="clearChat" title="清空对话">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="3 6 5 6 21 6"></polyline>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
              </button>
              
              <button class="ds-send-btn" :disabled="sending || !userInput.trim()" @click="sendMessage">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="12" y1="19" x2="12" y2="5"></line>
                  <polyline points="5 12 12 5 19 12"></polyline>
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 右侧：AI 智能体运行及设置侧边栏 -->
    <div class="ai-agent-sidebar">
      <div class="sidebar-scroll">
        <!-- 工作空间管理 -->
        <div class="agent-section-card">
          <div class="card-meta">
            <span class="card-label">📁 工作区根目录:</span>
            <span class="card-path" :title="ai.workspaceRoot">{{ ai.workspaceRoot || '获取工作区...' }}</span>
          </div>
          <div class="workspace-controls-bar">
            <button class="btn-sidebar btn-primary-sidebar" @click="handleSelectWorkspace">选择文件夹</button>
            <div class="workspace-manual-bar">
              <input v-model="manualPath" placeholder="输入绝对路径..." class="input-sidebar" @keyup.enter="handleManualWorkspace" />
              <button class="btn-sidebar" @click="handleManualWorkspace">指定</button>
            </div>
          </div>
        </div>

        <!-- 计费与熔断防爆配置 -->
        <div class="agent-section-card border-gold">
          <div class="card-title font-gold" @click="showSettings = !showSettings" style="cursor: pointer; display: flex; align-items: center; justify-content: space-between;">
            <span>⚙️ 计费与防爆设置 (DeepSeek)</span>
            <span style="font-size: 10px;">{{ showSettings ? '折叠 ▲' : '展开 ▼' }}</span>
          </div>
          <div class="settings-body-sidebar" v-show="showSettings">
            <div class="settings-row-sidebar">
              <label>缓存未命中输入 (元/百万):</label>
              <input type="number" step="0.1" v-model="localConfig.inputUncachedCostPerM" class="input-sidebar width-60" />
            </div>
            <div class="settings-row-sidebar">
              <label>缓存命中输入 (元/百万):</label>
              <input type="number" step="0.001" v-model="localConfig.inputCachedCostPerM" class="input-sidebar width-60" />
            </div>
            <div class="settings-row-sidebar">
              <label>输出 Token 价格 (元/百万):</label>
              <input type="number" step="0.1" v-model="localConfig.outputCostPerM" class="input-sidebar width-60" />
            </div>
            <div class="settings-row-sidebar">
              <label>防爆熔断限额 (元 ¥):</label>
              <input type="number" step="0.5" v-model="localConfig.costLimit" class="input-sidebar width-60" />
            </div>
            <div class="settings-actions-sidebar">
              <button class="btn-sidebar btn-primary-sidebar" @click="saveSettings">保存</button>
            </div>
          </div>
        </div>

        <!-- 会话与任务规划 -->
        <div class="agent-section-card">
          <div class="card-title font-gold">任务规划 (LLM Planner)</div>
          <div class="session-create-bar">
            <input v-model="newSessionTitle" placeholder="新会话标题..." class="input-sidebar flex-1" @keyup.enter="handleCreateSession" />
            <button class="btn-sidebar btn-primary-sidebar" @click="handleCreateSession">创建</button>
          </div>
          <div class="session-list-bar" v-if="ai.sessions.length > 0">
            <div v-for="s in ai.sessions" :key="s.id" class="session-item-bar" :class="{ active: s.id === ai.currentSessionId }" @click="ai.currentSessionId = s.id">
              <span class="dot-bar" :style="{ background: statusColors[s.status] }"></span>
              <span class="name-bar">{{ s.title }}</span>
            </div>
          </div>
          
          <div class="planner-box" v-if="ai.currentSessionId">
            <textarea v-model="taskRequest" placeholder="描述你要完成的开发任务..." class="textarea-sidebar" rows="2"></textarea>
            <button class="btn-sidebar btn-primary-sidebar" @click="handlePlanTask" :disabled="ai.isLoading" style="width: 100%; margin-top: 6px;">
              {{ ai.isLoading ? '分析规划中...' : '开始规划任务' }}
            </button>
            
            <div class="task-tree-bar" v-if="ai.taskTree.length > 0">
              <div v-for="task in ai.taskTree" :key="task.id" class="task-node-bar" :class="{ selected: task.id === selectedTaskId }" @click="selectedTaskId = task.id">
                <div class="node-header">
                  <span class="dot-bar" :style="{ background: statusColors[task.status] }"></span>
                  <span class="node-title">{{ task.title }}</span>
                </div>
                <div class="node-desc">{{ task.description }}</div>
                <div class="node-actions" v-if="task.status === 'pending'">
                  <button class="btn-sidebar btn-sm-sidebar btn-primary-sidebar" @click.stop="handleSpawnWorker(task)">启动 Worker</button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Worker 状态控制 -->
        <div class="agent-section-card">
          <div class="card-title">Worker 并发调度池 <span class="badge-sidebar">{{ ai.workers.length }}</span></div>
          <div class="worker-list-bar" v-if="ai.workers.length > 0">
            <div v-for="w in ai.workers" :key="w.id" class="worker-item-bar">
              <div class="worker-header-bar">
                <span class="dot-bar" :style="{ background: statusColors[w.status] }"></span>
                <span class="worker-name-bar">{{ w.id.slice(0, 8) }} ({{ statusLabel(w.status) }})</span>
                <button class="btn-sidebar btn-danger-sidebar" @click="handleKillWorker(w.id)" v-if="!['killed','completed','failed'].includes(w.status)">终止</button>
              </div>
              <div class="worker-thought-bar" v-if="w.currentThought">💭 {{ w.currentThought }}</div>
              
              <div class="sandbox-box" v-if="w.cowPath">
                <span class="sandbox-lbl">📦 隔离沙箱:</span>
                <code class="sandbox-pth" :title="w.cowPath">{{ w.cowPath }}</code>
              </div>

              <div class="worker-meta-bar">
                <span>🔧 {{ w.toolCallCount }} 工具</span>
                <span class="cost-lbl">💰 ¥{{ estimateCost(w.tokenCount) }}</span>
              </div>
            </div>
          </div>
          <div class="empty-bar-hint" v-else>暂无子代 Worker 运行</div>
        </div>

        <!-- 知识经验草稿箱 -->
        <div class="agent-section-card">
          <div class="card-title">💡 知识草稿箱 <span class="badge-sidebar-blue" v-if="ai.drafts.length > 0">{{ ai.drafts.length }}</span></div>
          <div class="draft-list-bar" v-if="ai.drafts.length > 0">
            <div v-for="d in ai.drafts" :key="d.id" class="draft-item-bar">
              <div class="draft-title-bar"># {{ d.title }}</div>
              <div class="draft-content-bar">{{ d.content }}</div>
              <div class="draft-actions-bar">
                <button class="btn-sidebar btn-sm-sidebar btn-primary-sidebar" @click="handlePromote(d.id)">推广至 RAG</button>
              </div>
            </div>
          </div>
          <div class="empty-bar-hint" v-else>暂无待审核草稿</div>
        </div>
      </div>
    </div>

    <!-- 毛玻璃聚合安全审批弹窗 -->
    <div class="overlay-backdrop" v-if="ai.pendingApprovals.length > 0">
      <div class="overlay-card">
        <div class="overlay-header">
          <h3 class="overlay-title">🛡️ 聚合安全审批队列</h3>
          <span class="overlay-badge">{{ ai.pendingApprovals.length }} 项敏感操作</span>
        </div>
        <div class="overlay-body">
          <div v-for="req in ai.pendingApprovals" :key="req.id" class="approval-card-item">
            <div class="approval-card-meta">
              <span class="risk-badge" :style="{ background: riskColors[req.riskLevel] }">{{ req.riskLevel }}</span>
              <span class="approval-tool">{{ req.toolName }}</span>
              <span class="approval-worker">Worker: {{ req.workerId.slice(0, 8) }}</span>
            </div>
            <div class="approval-card-reason">{{ req.reason }}</div>
            <div class="approval-card-args" v-if="req.arguments && Object.keys(req.arguments).length > 0">
              <strong>参数:</strong> <code>{{ JSON.stringify(req.arguments) }}</code>
            </div>
            <pre class="approval-card-diff" v-if="req.diffPreview">{{ req.diffPreview }}</pre>
            <div class="approval-card-actions">
              <button class="btn btn-sm btn-success" @click="handleApprove(req)">✓ 批准执行</button>
              <button class="btn btn-sm btn-danger" @click="handleDeny(req)">✗ 拒绝</button>
            </div>
          </div>
        </div>
        <div class="overlay-footer">
          <button class="btn btn-danger" @click="handleDenyAll">✗ 一键全部拒绝 (Reject All)</button>
          <button class="btn btn-success" @click="handleApproveAll">✓ 一键全部批准</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.ai-helper-container {
  display: flex;
  flex: 1;
  height: 100%;
  overflow: hidden;
  background: var(--jc-bg-app);
}

/* 左侧聊天区域 */
.ai-chat-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  height: 100%;
}

.ai-chat-area {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 14px;
  gap: 12px;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  border-radius: 8px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;

  &::-webkit-scrollbar {
    width: 4px;
  }
  &::-webkit-scrollbar-thumb {
    background: var(--jc-border-default);
    border-radius: 2px;
  }
}

.chat-bubble {
  max-width: 85%;
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-self: flex-start;

  .bubble-sender {
    font-size: 11px;
    color: var(--jc-text-secondary);
    font-weight: 500;
  }

  .bubble-content {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 13px;
    line-height: 1.6;
    padding: 10px 14px;
    border-radius: 8px;
    word-break: break-word;
  }

  &.user {
    align-self: flex-end;
    align-items: flex-end;

    .bubble-content {
      background: var(--jc-bg-selected);
      border-color: var(--jc-color-accent);
    }
  }
}

.shortcut-pills {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 2px;
  flex-wrap: wrap;
  padding: 0 4px;

  .shortcut-pill-desc {
    font-size: 11px;
    color: var(--jc-text-secondary);
    font-weight: 500;
  }

  .shortcut-pill {
    background: var(--jc-bg-btn);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 20px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;

    &:hover:not(:disabled) {
      background: var(--jc-bg-hover);
      border-color: var(--jc-color-accent);
      color: var(--jc-color-accent);
    }
    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.fade-slide-enter-from, .fade-slide-leave-to {
  opacity: 0;
  transform: translateY(6px) scale(0.98);
}

.ds-input-card {
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 16px;
  padding: 10px 14px;
  gap: 8px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.05);

  &:hover { border-color: var(--jc-border-strong); }
  &.focused {
    border-color: var(--jc-color-accent, #8a58ff);
    box-shadow: 0 0 12px rgba(138, 88, 255, 0.15);
  }

  .ds-textarea {
    width: 100%;
    min-height: 48px;
    max-height: 120px;
    background: transparent;
    border: none;
    resize: none;
    outline: none;
    color: var(--jc-text-primary);
    font-size: 13.5px;
    font-family: inherit;
    line-height: 1.6;
    padding: 2px 0;
    overflow-y: auto;
  }

  .ds-control-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .ds-pills { display: flex; gap: 8px; align-items: center; }

  .ds-pill-select-wrap {
    display: inline-flex;
    align-items: center;
    background: var(--jc-bg-btn);
    border: 1px solid var(--jc-border-default);
    padding: 0 10px;
    height: 28px;
    border-radius: 20px;
    color: var(--jc-text-secondary);
    position: relative;

    .model-icon { width: 14px; height: 14px; margin-right: 4px; flex-shrink: 0; }
    .ds-pill-select {
      background: transparent;
      border: none;
      outline: none;
      color: inherit;
      font-size: 11.5px;
      font-weight: 500;
      cursor: pointer;
      padding: 0 14px 0 0;
      max-width: 120px;
      appearance: none;
      background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 24 24' fill='none' stroke='gray' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'></polyline></svg>");
      background-repeat: no-repeat;
      background-position: right center;
      background-size: 8px;

      optgroup, option { background: var(--jc-bg-panel); color: var(--jc-text-primary); }
    }
    .ds-pill-refresh {
      background: transparent;
      border: none;
      outline: none;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      padding: 0 0 0 6px;
      margin-left: 2px;
      border-left: 1px solid var(--jc-border-default);
      color: var(--jc-text-secondary);
      height: 14px;
      .refresh-icon-svg { width: 11px; height: 11px; &.spinning { animation: spin-anim 1s linear infinite; } }
    }
  }

  .ds-pill-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--jc-bg-btn);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-secondary);
    padding: 5px 12px;
    font-size: 11.5px;
    border-radius: 20px;
    cursor: pointer;
    font-weight: 500;

    .ds-pill-icon { width: 14px; height: 14px; flex-shrink: 0; }
    &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
    &.active {
      background: rgba(138, 88, 255, 0.09);
      border-color: var(--jc-color-accent, #8a58ff);
      color: var(--jc-color-accent, #8a58ff);
    }
  }

  .ds-actions { display: flex; align-items: center; gap: 8px; }
  .ds-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--jc-text-secondary);
    cursor: pointer;
    svg { width: 17px; height: 17px; }
    &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
    &.clear:hover { color: var(--jc-color-error); background: rgba(220, 38, 38, 0.08); }
    &.attach:hover { color: var(--jc-color-success); background: rgba(46, 204, 113, 0.08); }
  }

  .ds-send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: none;
    background: var(--jc-border-default);
    color: var(--jc-bg-app);
    cursor: not-allowed;
    svg { width: 16px; height: 16px; }
    &:not(:disabled) {
      background: var(--jc-color-accent, #8a58ff);
      color: #ffffff;
      cursor: pointer;
      &:hover { transform: scale(1.06); }
    }
  }
}

/* 右侧 AI 智能体侧边栏 */
.ai-agent-sidebar {
  width: 360px;
  height: 100%;
  border-left: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sidebar-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb { background: var(--jc-border-default); border-radius: 2px; }
}

.agent-section-card {
  background: rgba(255, 255, 255, 0.015);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  
  &.border-gold { border-color: rgba(240, 136, 62, 0.25); }
  
  .card-meta { display: flex; flex-direction: column; gap: 4px; }
  .card-label { font-size: 11px; font-weight: 600; color: var(--jc-text-secondary); }
  .card-path { font-family: monospace; font-size: 11px; color: #58a6ff; word-break: break-all; }
  .card-title { font-size: 12px; font-weight: 700; color: var(--jc-text-primary); }
}

.workspace-controls-bar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  
  .workspace-manual-bar {
    display: flex;
    gap: 6px;
    align-items: center;
  }
}

.settings-body-sidebar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px dashed var(--jc-border-default);
}
.settings-row-sidebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  label { color: var(--jc-text-secondary); }
}
.width-60 { width: 80px; text-align: right; }
.settings-actions-sidebar { display: flex; justify-content: flex-end; }

.session-create-bar {
  display: flex;
  gap: 6px;
}
.session-list-bar {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 120px;
  overflow-y: auto;
}
.session-item-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  background: rgba(255,255,255,0.01);
  border: 1px solid transparent;
  &:hover { background: var(--jc-bg-hover); }
  &.active { background: var(--jc-bg-active); border-color: rgba(240, 136, 62, 0.2); }
  
  .dot-bar { width: 6px; height: 6px; border-radius: 50%; }
  .name-bar { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
}

.planner-box {
  margin-top: 6px;
  border-top: 1px solid var(--jc-border-default);
  padding-top: 8px;
}
.textarea-sidebar {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-primary);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 12px;
  resize: vertical;
}

.task-tree-bar {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 10px;
  max-height: 240px;
  overflow-y: auto;
}
.task-node-bar {
  padding: 8px 10px;
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  cursor: pointer;
  background: rgba(0,0,0,0.08);
  &:hover { border-color: var(--jc-color-accent); }
  &.selected { border-color: var(--jc-color-accent); background: var(--jc-bg-active); }
  
  .node-header { display: flex; align-items: center; gap: 6px; }
  .node-title { font-size: 11.5px; font-weight: 600; }
  .node-desc { font-size: 11px; color: var(--jc-text-secondary); margin-top: 3px; }
  .node-actions { margin-top: 6px; }
}

.worker-list-bar, .draft-list-bar {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.worker-item-bar, .draft-item-bar {
  padding: 8px 10px;
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  background: rgba(0,0,0,0.1);
  font-size: 11.5px;
}
.worker-header-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  .worker-name-bar { flex: 1; font-family: monospace; font-weight: 600; color: #58a6ff; }
}
.worker-thought-bar {
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-style: italic;
  background: rgba(255,255,255,0.02);
  padding: 4px;
  margin-top: 4px;
  border-radius: 4px;
  border-left: 2px solid #a371f7;
}
.sandbox-box {
  margin-top: 4px;
  background: rgba(0,0,0,0.25);
  border-radius: 4px;
  padding: 4px 6px;
  font-size: 10.5px;
  display: flex;
  gap: 4px;
  .sandbox-lbl { color: var(--jc-text-secondary); flex-shrink: 0; }
  .sandbox-pth { font-family: monospace; color: #a371f7; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
}
.worker-meta-bar {
  display: flex;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 10.5px;
  color: var(--jc-text-secondary);
  .cost-lbl { color: #3fb950; font-weight: 700; }
}

.draft-title-bar { font-weight: 600; color: var(--jc-text-primary); }
.draft-content-bar { font-size: 11px; color: var(--jc-text-secondary); margin: 4px 0; max-height: 80px; overflow-y: auto; background: rgba(0,0,0,0.15); padding: 4px; border-radius: 4px; }
.draft-actions-bar { display: flex; justify-content: flex-end; }

.badge-sidebar, .badge-sidebar-blue {
  padding: 1px 5px;
  border-radius: 8px;
  font-size: 10px;
  background: var(--jc-bg-elevated);
  font-weight: bold;
}
.badge-sidebar-blue { background: #58a6ff; color: white; }

.input-sidebar {
  padding: 4px 8px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: var(--jc-bg-input);
  color: var(--jc-text-primary);
  font-size: 11.5px;
  outline: none;
  &:focus { border-color: var(--jc-color-accent); }
}

.btn-sidebar {
  padding: 4px 10px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-primary);
  font-size: 11.5px;
  cursor: pointer;
  font-weight: 500;
  &:hover { background: var(--jc-bg-hover); }
  &.btn-primary-sidebar { background: var(--jc-color-accent); color: white; border-color: var(--jc-color-accent); }
  &.btn-danger-sidebar { background: #da3633; color: white; border-color: #f85149; }
  &.btn-sm-sidebar { padding: 2px 6px; font-size: 10.5px; }
}
.empty-bar-hint { font-size: 11px; color: var(--jc-text-secondary); text-align: center; padding: 10px; border: 1px dashed var(--jc-border-default); border-radius: 6px; }

/* “反重力”毛玻璃聚合安全审批弹窗 */
.overlay-backdrop { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.55); backdrop-filter: blur(10px); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 20px; }
.overlay-card { width: 100%; max-width: 580px; max-height: 85vh; background: var(--jc-bg-app); border: 1px solid var(--jc-border-default); border-radius: 12px; box-shadow: 0 12px 40px rgba(0,0,0,0.4); display: flex; flex-direction: column; overflow: hidden; }
.overlay-header { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--jc-border-default); background: rgba(255,255,255,0.01); }
.overlay-title { font-size: 15px; font-weight: 700; margin: 0; color: #f0883e; }
.overlay-badge { font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 10px; background: rgba(248,81,73,0.15); color: #f85149; }
.overlay-body { flex: 1; overflow-y: auto; padding: 18px; display: flex; flex-direction: column; gap: 14px; }
.approval-card-item { border: 1px solid var(--jc-border-default); border-radius: 8px; padding: 12px; background: rgba(255,255,255,0.02); }
.approval-card-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.risk-badge { padding: 2px 7px; border-radius: 8px; font-size: 9px; font-weight: 700; color: #fff; text-transform: uppercase; }
.approval-tool { flex: 1; font-weight: 600; font-size: 13px; }
.approval-worker { font-family: monospace; font-size: 11px; }
.approval-card-reason { font-size: 12px; color: var(--jc-text-secondary); line-height: 1.4; }
.approval-card-args { font-size: 11px; margin-top: 6px; background: rgba(0,0,0,0.1); padding: 4px 8px; border-radius: 4px; word-break: break-all; }
.approval-card-diff { margin-top: 8px; padding: 8px; background: #0c1117; border: 1px solid var(--jc-border-default); border-radius: 6px; font-size: 11px; font-family: monospace; overflow-x: auto; max-height: 180px; color: #c9d1d9; white-space: pre-wrap; line-height: 1.4; }
.approval-card-actions { display: flex; gap: 8px; margin-top: 10px; justify-content: flex-end; }
.overlay-footer { padding: 14px 18px; border-top: 1px solid var(--jc-border-default); background: rgba(255,255,255,0.01); display: flex; justify-content: space-between; gap: 12px; }

.btn { padding: 6px 14px; border: 1px solid var(--jc-border-default); border-radius: 6px; background: var(--jc-bg-elevated); color: var(--jc-text-primary); font-size: 13px; cursor: pointer; font-weight: 500; &:hover { background: var(--jc-bg-hover); } }
.btn-primary { background: var(--jc-color-accent); color: white; border-color: var(--jc-color-accent); }
.btn-success { background: #238636; color: white; border-color: #2ea043; &:hover { background: #2ea043; } }
.btn-danger { background: #da3633; color: white; border-color: #f85149; &:hover { background: #f85149; } }
.btn-sm { padding: 3px 10px; font-size: 12px; }

.font-gold { color: #f0883e !important; }
.font-gray { color: var(--jc-text-secondary) !important; }
.flex-1 { flex: 1; }

@keyframes spin-anim {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
