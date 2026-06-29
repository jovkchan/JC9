<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
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
const enableLocalKb = ref(false)
const enableAgentMode = ref(false)
const reasoningEffort = ref<'high' | 'max' | 'off'>('high')
const isFocused = ref(false)

// ── AI Agent Store ──
const pollTimer = ref<number | null>(null)
const manualPath = ref('')
const showSettings = ref(false)
const localConfig = ref({
  inputCachedCostPerM: 0.025,
  inputUncachedCostPerM: 3.0,
  outputCostPerM: 6.0,
  costLimit: 5.0
})

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
  if (aiProvider.value === 'ollama' || aiProvider.value === 'vllm') {
    selectedCombinedModel.value = `${aiProvider.value}/${aiModel.value}`
  }
}

function loadConfig() {
  loadCustomModels()
  const lastSelected = localStorage.getItem('jc9-last-model')
  
  if (customModels.value.length > 0) {
    try {
      let target = null
      if (lastSelected) {
        if (lastSelected.includes('::')) {
          const [cfgId] = lastSelected.split('::')
          target = customModels.value.find(c => c.id === cfgId || `${c.provider}-${c.model}-${c.name}` === cfgId)
        } else {
          // 模糊向下兼容老配置格式
          target = customModels.value.find(c => lastSelected.includes(c.model) || lastSelected.startsWith(c.provider))
        }
      }
      
      const cfg = target || customModels.value[0]
      aiProvider.value = cfg.provider
      
      const subModels = cfg.model.split(',').map(m => m.trim()).filter(Boolean)
      const activeModel = (lastSelected && lastSelected.includes('::'))
        ? lastSelected.split('::')[1]
        : (subModels[0] || cfg.model)
        
      aiModel.value = activeModel
      aiEndpoint.value = cfg.endpoint
      aiApiKey.value = cfg.apiKey || ''
      
      const configId = cfg.id || `${cfg.provider}-${cfg.model}-${cfg.name}`
      selectedCombinedModel.value = `${configId}::${activeModel}`
      
      // 同步预算与费率至后端
      ai.updateCostConfig({
        inputCachedCostPerM: cfg.inputPrice ? cfg.inputPrice * 0.008 : 0.025,
        inputUncachedCostPerM: cfg.inputPrice || 2.0,
        outputCostPerM: cfg.outputPrice || 4.0,
        costLimit: cfg.costLimit || 10.0,
      })
      return
    } catch { /* ignore */ }
  }

  // 兜底旧逻辑
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

const customModels = ref<CustomModel[]>([])

function loadCustomModels() {
  const saved = localStorage.getItem('notes-ai-models')
  if (saved) {
    try {
      customModels.value = JSON.parse(saved)
    } catch {
      customModels.value = []
    }
  }
}

const modelOptions = computed(() => {
  const groups: Record<string, Array<{ id: string; name: string; label: string }>> = {}

  // 1. 分组读取用户在设置里添加的自定义模型配置，并支持多模型解析
  for (const cfg of customModels.value) {
    const provName = cfg.provider.charAt(0).toUpperCase() + cfg.provider.slice(1)
    if (!groups[provName]) groups[provName] = []
    
    // 对逗号分隔的多个模型名称进行切割
    const subModels = cfg.model.split(',').map(m => m.trim()).filter(Boolean)
    for (const m of subModels) {
      const configId = cfg.id || `${cfg.provider}-${cfg.model}-${cfg.name}`
      // 复合 value 标识为：配置唯一ID::特定模型标识符
      const selectId = `${configId}::${m}`
      groups[provName].push({
        id: selectId,
        name: m,
        label: `${cfg.name} (${m})`
      })
    }
  }

  // 2. 兜底：Ollama 本地模型
  const ollamaList: Array<{ id: string; name: string; label: string }> = []
  if (ollamaModels.value.length > 0) {
    ollamaModels.value.forEach(m => ollamaList.push({ id: `ollama/${m}`, name: m, label: m }))
  }
  if (ollamaList.length > 0 && !groups['Ollama']) groups['Ollama'] = ollamaList

  // 3. 兜底：vLLM
  const vllmList: Array<{ id: string; name: string; label: string }> = []
  if (vllmModels.value.length > 0) {
    vllmModels.value.forEach(m => vllmList.push({ id: `vllm/${m}`, name: m, label: m }))
  }
  if (vllmList.length > 0 && !groups['Vllm']) groups['Vllm'] = vllmList

  return groups
})

function handleModelChange() {
  const val = selectedCombinedModel.value
  if (!val) return

  // 1. 判断是否是动态加载的本地 Ollama / vLLM 兜底模型
  if (val.startsWith('ollama/') || val.startsWith('vllm/')) {
    const parts = val.split('/')
    const prov = parts[0]
    const model = parts.slice(1).join('/')

    aiProvider.value = prov
    aiModel.value = model

    const savedEndpoint = localStorage.getItem(`notes-ai-endpoint-${prov}`)
    if (savedEndpoint) {
      aiEndpoint.value = savedEndpoint
    } else {
      aiEndpoint.value = prov === 'ollama' ? 'http://127.0.0.1:11434' : 'http://localhost:8000/v1'
    }
    aiApiKey.value = ''
    
    saveQuickConfig()
    localStorage.setItem('jc9-last-model', val)
    return
  }

  // 2. 精准匹配用户在设置面板自定义的 AI 配置 (通过唯一 ID 并解析多模型名称)
  let configId = ''
  let selectedModelName = ''

  if (val.includes('::')) {
    const parts = val.split('::')
    configId = parts[0]
    selectedModelName = parts.slice(1).join('::')
  } else {
    // 兼容没有双冒号的旧版本缓存
    configId = val
  }

  const cfg = customModels.value.find(c => c.id === configId || `${c.provider}-${c.model}-${c.name}` === configId)
  if (cfg) {
    aiProvider.value = cfg.provider
    if (!selectedModelName) {
      // 降级选择配置中的第一个模型名称
      selectedModelName = cfg.model.split(',')[0]?.trim() || cfg.model
    }
    aiModel.value = selectedModelName
    aiEndpoint.value = cfg.endpoint
    aiApiKey.value = cfg.apiKey

    // 同步更新后端的防爆 Token 预算限额及单价
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

// ── 核心流式请求（SSE/Chunked 读取） ──
async function callAiStream(promptMessages: Message[], onChunk: (text: string) => void) {
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

  console.log(`📡 API 请求 → ${url} | Model: ${aiModel.value} | Provider: ${aiProvider.value}`)

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
  let chunkCount = 0
  let fullResponse = ''

  console.group('📡 流式响应')
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
          if (text) {
            chunkCount++
            fullResponse += text
            console.log(`  chunk #${chunkCount}:`, text)
            onChunk(text)
          }
        } catch (e) { /* ignore */ }
      } else {
        if (trimmed.startsWith('data:')) {
          const dataVal = trimmed.slice(5).trim()
          if (dataVal === '[DONE]') continue
          try {
            const json = JSON.parse(dataVal)
            const text = json.choices?.[0]?.delta?.content || ''
            if (text) {
              chunkCount++
              fullResponse += text
              console.log(`  chunk #${chunkCount}:`, text)
              onChunk(text)
            }
          } catch (e) { /* ignore */ }
        }
      }
    }
  }
  console.log(`✅ 完成，共 ${chunkCount} chunk, ${fullResponse.length} 字符`)
  console.groupEnd()
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

// ── 发送消息（智能 RAG 感知 + Agent 自动管道）──
async function sendMessage() {
  const text = userInput.value.trim()
  if (!text || sending.value) return

  // Agent 模式：自动触发任务规划+Worker 管道
  if (enableAgentMode.value) {
    await sendAgentMessage(text)
    return
  }

  await doSendMessage(text)
}

async function doSendMessage(text: string) {
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

    systemPromptStr = `你是通用 AI 助手。以下是用户本地笔记库中相关的笔记供参考：
${referenceText ? `\n${referenceText}\n` : ''}
请直接回答用户的问题，有笔记相关内容时可引用，但不强制。`
  } else {
    systemPromptStr = '你是一个通用 AI 助手。请直接、简洁地回答用户的问题。不要说"作为XX助手"之类的话，直接回答问题即可。'
  }

  if (enableDeepThink.value) {
    systemPromptStr += `\n\n【重要指令 - 深度思考模式已开启】：
请在回答前进行深入、细致的逐步思考和推导。在正式输出回答前，请先写出你的思考过程，以格式 \`思考过程：...\` 呈现，然后再输出最终答复。请展现严密的逻辑性，层层深入剖析问题。`
  }

  const promptMsgs: Message[] = [
    { role: 'system', content: systemPromptStr },
    ...messages.value.filter(m => m.role === 'user' || m.role === 'assistant').slice(0, -1)
  ]

  // 🔍 完整调试日志
  console.group(`🤖 AI 请求 | ${aiProvider.value} / ${aiModel.value}`)
  console.log('📋 System Prompt:\n', systemPromptStr)
  console.log('💬 Messages (', promptMsgs.length, '条):')
  promptMsgs.forEach((m, i) => {
    console.log(`  [${i}] ${m.role}:`, m.content.length > 500 ? m.content.slice(0, 500) + '...' : m.content)
  })
  console.groupEnd()

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

// ── DS 思维强度切换（即时生效）──
async function setReasoningEffort(effort: 'high' | 'max' | 'off') {
  reasoningEffort.value = effort
  const val = effort === 'off' ? '' : effort
  try { await invoke('ai_set_reasoning_effort', { effort: val }) } catch { /* ignore */ }
}

// ── Agent 模式：自然语言一键触发规划+Worker ──
async function sendAgentMessage(text: string) {
  console.group('🤖 Agent 模式')
  console.log('任务:', text)

  // 意图检测：非编码任务走普通对话
  const codingKeywords = /编写|添加|修复|创建|修改|重构|实现|优化|配置|安装|部署|调试|写一个|改一下|帮我写|生成|代码|文件|组件|模块|接口|函数|类|类型|测试|构建|打包|编译|提交|合并|分支|npm|cargo|git|vue|react|rust|typescript|python|依赖|import|export|package|运行|启动|检查|报错|错误/i
  if (!codingKeywords.test(text)) {
    console.log('📋 非编码任务，走普通对话')
    console.groupEnd()
    await doSendMessage(text)
    return
  }

  console.log('模型:', `${aiProvider.value}/${aiModel.value}`)

  // 同步 LLM 配置到后端
  try {
    await invoke('ai_configure_llm', {
      provider: aiProvider.value,
      apiKey: aiApiKey.value,
      baseUrl: aiEndpoint.value,
      model: aiModel.value,
    })
    console.log('✅ LLM 配置已同步')
  } catch (e) {
    console.warn('⚠️ 同步 LLM 配置失败:', e)
  }

  messages.value.push({ role: 'user', content: text })
  userInput.value = ''
  sending.value = true
  scrollToBottom()

  try {
    if (!ai.currentSessionId) {
      const title = text.slice(0, 30) + (text.length > 30 ? '...' : '')
      await ai.createSession(title)
      addAgentBubble('📋 新会话已创建')
    }
    addAgentBubble('🧠 正在分析并拆解任务...')
    const tasks = await ai.planTask(ai.currentSessionId!, text)
    console.log('规划结果:', tasks.length, '个子任务')
    if (tasks.length > 0) {
      const list = tasks.map(t => `  • **${t.title}** _(${statusLabel(t.status)})_`).join('\n')
      addAgentBubble(`✅ 已规划 **${tasks.length}** 个子任务：\n${list}`)
      for (const task of tasks) {
        if (task.status === 'pending') {
          addAgentBubble(`🚀 启动 Worker：「${task.title}」...`)
          const sp = `你是一个专业的代码助手，使用 ReAct 模式。任务：${task.description}`
          await ai.spawnWorker(ai.currentSessionId!, task, sp)
        }
      }
    } else {
      addAgentBubble('⚠️ 任务规划返回空，请尝试更具体的描述。')
    }
  } catch (e: any) {
    console.error('❌ Agent 错误:', e)
    addAgentBubble(`❌ Agent 错误: ${e}`)
  } finally {
    console.groupEnd()
    sending.value = false
    scrollToBottom()
  }
}

function addAgentBubble(content: string) {
  messages.value.push({ role: 'system', content })
  scrollToBottom()
}

const placeholderText = computed(() => {
  if (enableAgentMode.value) return '描述开发任务，Agent 自动规划并执行... (Enter 发送)'
  return `给 ${aiModel.value || 'AI'} 发送消息... (Enter 发送, Shift+Enter 换行)`
})

const workspaceShortName = computed(() => {
  const p = ai.workspaceRoot
  if (!p) return '📁 未设置'
  const parts = p.replace(/\\/g, '/').split('/')
  return '📁 ' + (parts[parts.length - 1] || p)
})

// ── 审批处理 ──
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
    loadCustomModels()
    try {
      await ai.loadWorkers()
    } catch { /* ignore */ }
    try {
      await ai.loadPendingApprovals()
    } catch { /* ignore */ }
    try {
      await ai.loadDrafts()
    } catch { /* ignore */ }
  }, 3000)
}

function stopPolling() {
  if (pollTimer.value !== null) {
    clearInterval(pollTimer.value)
    pollTimer.value = null
  }
}

const workerUnlisten = ref<(() => void) | null>(null)
const lastDraftIds = ref<string[]>([])

watch(() => ai.drafts, (newDrafts) => {
  const newItems = newDrafts.filter(d => !lastDraftIds.value.includes(d.id))
  for (const item of newItems) {
    addAgentBubble(`💡 **【二脑知识沉淀】** 已自动提炼并沉淀草稿备忘《${item.title}》，可信度为 ${(item.confidence * 100).toFixed(0)}%。`)
  }
  lastDraftIds.value = newDrafts.map(d => d.id)
}, { deep: true })

onMounted(async () => {
  loadCustomModels()
  loadConfig()
  // 只在 notes-ai-models 中有 ollama/vllm 配置时才拉取
  let hasOllama = false, hasVllm = false
  const saved = localStorage.getItem('notes-ai-models')
  if (saved) {
    try {
      const configs: Array<{provider:string}> = JSON.parse(saved)
      hasOllama = configs.some(c => c.provider === 'ollama')
      hasVllm = configs.some(c => c.provider === 'vllm')
    } catch {}
  }
  if (hasOllama) fetchOllamaModels()
  if (hasVllm) fetchVllmModels()

  await ai.loadSessions()
  await ai.loadWorkspaceRoot()
  manualPath.value = ai.workspaceRoot
  await ai.initListeners()
  await ai.loadDrafts()
  lastDraftIds.value = ai.drafts.map(d => d.id)
  localConfig.value = { ...ai.costConfig }
  
  // 额外监听后端状态，在终态时直接在聊天框发送通知气泡
  workerUnlisten.value = await listen<WorkerState>('ai:worker-update', (event) => {
    const w = event.payload
    const taskTitle = ai.taskTree.find(t => t.id === w.taskId)?.title || '开发任务'
    if (w.status === 'completed') {
      addAgentBubble(`🎉 子任务「${taskTitle}」已顺利执行完毕，所有代码变更已安全合入工作区。`)
    } else if (w.status === 'failed') {
      const reason = w.terminationReason || '遇到阻碍或触发熔断'
      addAgentBubble(`❌ 子任务「${taskTitle}」执行失败。原因: ${reason}`)
    } else if (w.status === 'killed') {
      addAgentBubble(`🛑 子任务「${taskTitle}」已被手动强制终止。`)
    }
  })

  startPolling()
})

onUnmounted(() => {
  stopPolling()
  ai.destroyListeners()
  if (workerUnlisten.value) {
    workerUnlisten.value()
  }
})

const newSessionTitle = ref('')
const plannerInput = ref('')

async function handleCreateSession() {
  const title = newSessionTitle.value.trim()
  if (!title) return
  const id = await ai.createSession(title)
  if (id) {
    newSessionTitle.value = ''
    status.pushMessage('会话创建成功', 'success')
  }
}

async function handlePlanTask() {
  if (!ai.currentSessionId) {
    status.pushMessage('请先选择或创建一个 Agent 会话', 'warn')
    return
  }
  const request = plannerInput.value.trim()
  if (!request) return
  sending.value = true
  try {
    addAgentBubble('🧠 正在分析并拆解任务...')
    const tasks = await ai.planTask(ai.currentSessionId, request)
    if (tasks.length > 0) {
      plannerInput.value = ''
      const list = tasks.map(t => `  • **${t.title}** _(${statusLabel(t.status)})_`).join('\n')
      addAgentBubble(`✅ 已规划 **${tasks.length}** 个子任务：\n${list}`)
    } else {
      addAgentBubble('⚠️ 任务规划返回空，请尝试更具体的描述。')
    }
  } catch (e: any) {
    status.pushMessage(`规划任务失败: ${e.message}`, 'error')
  } finally {
    sending.value = false
  }
}

async function handleStartTaskWorker(task: TaskNode) {
  if (!ai.currentSessionId) return
  addAgentBubble(`🚀 启动 Worker 执行任务：「${task.title}」...`)
  const systemPrompt = `你是一个专业的代码助手，使用 ReAct 模式。任务：${task.description}`
  try {
    await invoke('ai_configure_llm', {
      provider: aiProvider.value,
      apiKey: aiApiKey.value,
      baseUrl: aiEndpoint.value,
      model: aiModel.value,
    })
    await ai.spawnWorker(ai.currentSessionId, task, systemPrompt)
  } catch (e: any) {
    status.pushMessage(`启动 Worker 失败: ${e.message}`, 'error')
  }
}
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
                <button 
                  class="ds-pill-settings-btn" 
                  @click="showModelSettingsModal = true" 
                  title="自定义 AI 模型管理"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="settings-gear-svg">
                    <circle cx="12" cy="12" r="3"></circle>
                    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                  </svg>
                </button>
              </div>
 <!-- DS 思维强度（仅 DS 模型显示） -->
              <div v-if="aiProvider === 'deepseek'" class="ds-pill-select-wrap" style="gap:4px">
                
                <select v-model="reasoningEffort" @change="setReasoningEffort(reasoningEffort)" class="ds-pill-select" style="max-width:56px;font-size:10px" title="思维强度">
                  <option value="high">标准</option>
                  <option value="max">深度</option>
                  <option value="off">关闭</option>
                </select>
              </div>
              <!-- 工作区 -->
              <button 
                class="ds-pill-btn workspace-btn" 
                @click="handleSelectWorkspace"
                :title="ai.workspaceRoot || '选择工作区'"
              >
                <svg class="ds-pill-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                </svg>
                {{ workspaceShortName }}
              </button>

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

              <!-- Agent 模式 -->
              <button 
                class="ds-pill-btn agent-mode" 
                :class="{ active: enableAgentMode }" 
                @click="enableAgentMode = !enableAgentMode"
                title="开启后直接描述编码任务，自动规划并执行"
              >
                <svg class="ds-pill-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                </svg>
                Agent
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

        <!-- 极简活跃智能体进程状态栏 -->
        <Transition name="fade-slide">
          <div class="active-agents-ticker" v-if="ai.activeWorkers.length > 0">
            <div v-for="w in ai.activeWorkers" :key="w.id" class="ticker-item">
              <span class="ticker-dot" :style="{ background: statusColors[w.status] }"></span>
              <span class="ticker-text">
                <span class="ticker-worker-name">Worker-{{ w.id.slice(0, 8) }}</span>
                <span class="ticker-worker-status">{{ statusLabel(w.status) }}</span>:
                <span class="ticker-thought" v-if="w.currentThought">{{ w.currentThought }}</span>
                <span class="ticker-thought" v-else>正在分配/思考中...</span>
              </span>
              <button class="ticker-kill-btn" @click="handleKillWorker(w.id)" title="强制终止">✕ 强杀</button>
            </div>
          </div>
        </Transition>
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

  &.system {
    .bubble-sender { color: #f0883e; }
    .bubble-content {
      border-left: 3px solid #f0883e;
      font-size: 12px;
      opacity: 0.9;
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
    &.agent-mode.active {
      background: rgba(240, 136, 62, 0.12);
      border-color: #f0883e;
      color: #f0883e;
    }
    &.workspace-btn {
      max-width: 130px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    &.model-settings-btn {
      font-size: 14px;
      padding: 4px 8px;
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

/* 极简活跃智能体进程状态栏 */
.active-agents-ticker {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  padding: 8px 12px;
  margin-top: 6px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);

  .ticker-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .ticker-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .ticker-text {
    flex: 1;
    color: var(--jc-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;

    .ticker-worker-name {
      font-family: monospace;
      font-weight: 600;
      color: #58a6ff;
    }

    .ticker-worker-status {
      font-weight: 600;
      color: var(--jc-text-secondary);
      margin-left: 4px;
    }

    .ticker-thought {
      color: var(--jc-text-secondary);
      font-style: italic;
      margin-left: 6px;
    }
  }

  .ticker-kill-btn {
    background: transparent;
    border: 1px solid #f85149;
    color: #f85149;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.2s;

    &:hover {
      background: rgba(248, 81, 73, 0.1);
    }
  }
}
</style>
