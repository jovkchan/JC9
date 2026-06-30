<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, watch } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { WorkerState, ApprovalRequest, KbEntry } from '@/types/ai'
import { getRole, loadAllRoles, type AgentRole } from '@/config/roles'

const notesStore = useNotesStore()
const status = useStatusStore()
const ai = useAiStore()

// 消息列表结构升级，增加 modelName 属性
interface Message {
  role: 'user' | 'assistant' | 'system'
  content: string
  modelName?: string
  roleName?: string
  roleIcon?: string
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

// ── AI 角色切换与智能路由 ──
const activeChatRoleId = ref('auto')
const chatRolesList = ref<AgentRole[]>([])

const activeChatRole = computed(() => {
  if (activeChatRoleId.value === 'auto') {
    return { id: 'auto', name: '智能路由', icon: '🤖', description: '根据提问内容自动选择最适合的角色', systemPrompt: '' }
  }
  return chatRolesList.value.find(r => r.id === activeChatRoleId.value) || { id: 'auto', name: '智能路由', icon: '🤖', description: '', systemPrompt: '' }
})

// ── AI Agent Store ──
const pollTimer = ref<number | null>(null)
const manualPath = ref('')
const showModelSettingsModal = ref(false)
const showSessionPopup = ref(false)
const newSessionTitle = ref('')
const kbSearchQuery = ref('')
const kbSearchResults = ref<KbEntry[]>([])

async function searchKnowledgeBase() {
  if (!kbSearchQuery.value.trim()) return
  try {
    kbSearchResults.value = await invoke<KbEntry[]>('ai_search_knowledge', {
      query: kbSearchQuery.value.trim(),
      limit: 8,
    })
  } catch { /* ignore */ }
}
const isConsoleExpanded = ref(true)
const expandedWorkers = ref<Record<string, boolean>>({})

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
  try {
    const d = new Date(timestampStr)
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch {
    return timestampStr
  }
}

async function handleKillAllWorkers() {
  for (const w of ai.activeWorkers) {
    await ai.killWorker(w.id)
  }
}

const localConfig = ref({
  inputCachedCostPerM: 0.025,
  inputUncachedCostPerM: 3.0,
  outputCostPerM: 6.0,
  costLimit: 5.0
})

const inputTextarea = ref<HTMLTextAreaElement | null>(null)

function autoResizeTextarea() {
  const el = inputTextarea.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = Math.min(el.scrollHeight, 480) + 'px'
}

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

async function handleCreateNoteFromAi(text: string) {
  // 1. 检测是否包含 URL 链接
  const urlRegex = /(https?:\/\/[^\s]+)/gi
  const urlMatch = text.match(urlRegex)
  let contentText = ""

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
      if (!contentText.trim()) {
        throw new Error("抓取到的网页正文内容为空。")
      }
      messages.value[aiMsgIndex].content = `📥 网页内容抓取成功（约 ${contentText.length} 字符），正在调用 AI 智能生成总结笔记...`
    } catch (e: any) {
      messages.value[aiMsgIndex].content = `❌ 网页抓取失败: ${e.message || e}。\n我们将跳过网页抓取，直接根据您提供的信息生成笔记。`
    }
  }

  // 2. 构造 Prompt
  let prompt = ""
  if (contentText) {
    prompt = `用户指令：“${text}”\n\n抓取到的网页网页内容如下：\n${contentText}\n\n请根据以上网页内容，提取核心要点并整理出一篇结构清晰、排版优美并具有清晰层级的 Markdown 备忘笔记。
请必须仅输出一个符合以下 JSON 格式的字符串，不要包含任何 markdown 标志（如 \`\`\`json）：
{
  "title": "网页核心主题标题",
  "content": "# 标题\\n\\n## 概述\\n...内容详情...\\n使用 Markdown 格式。",
  "tags": ["标签1", "标签2"]
}`
  } else {
    // 提取历史聊天记录
    const historyText = messages.value
      .slice(0, -2) // 排除掉刚发送的 user 提问和当前的 assistant 提示气泡
      .filter(m => m.role === 'user' || m.role === 'assistant')
      .map(m => `${m.role === 'user' ? '用户' : 'AI助理'}: ${m.content}`)
      .join('\n\n')

    prompt = `用户指令：“${text}”\n\n当前对话的历史上下文如下：\n${historyText || '无上下文历史。'}\n\n请根据以上历史对话内容（如有），结合您的知识库，提取核心要点并整理出一篇结构清晰、排版优美并具有清晰层级的 Markdown 备忘笔记。
请必须仅输出一个符合以下 JSON 格式的字符串，不要包含任何 markdown 标志（如 \`\`\`json）：
{
  "title": "本篇对话核心主题标题",
  "content": "# 标题\\n\\n## 对话要点总结\\n...内容详情...\\n使用 Markdown 格式。",
  "tags": ["标签1", "标签2"]
}`
  }

  // 3. 呼叫大模型
  let responseText = ''
  try {
    const promptMsgs: Message[] = [
      { role: 'system', content: '你是一个专业的 Markdown 笔记整理专家。请必须按指定的 JSON 格式输出生成的笔记标题、正文 and 标签列表。不要输出任何 JSON 之外的多余内容或 markdown 围栏。' },
      { role: 'user', content: prompt }
    ]

    await callAiStream(promptMsgs, (chunk) => {
      responseText += chunk
      messages.value[aiMsgIndex].content = `✍️ 正在流式接收并生成笔记中...\n\n${responseText.slice(0, 1000)}${responseText.length > 1000 ? '...' : ''}`
    })

    // 解析 JSON
    const cleanedJson = responseText
      .trim()
      .replace(/^```json\s*/i, '')
      .replace(/```$/, '')
      .trim()

    let parsed: any
    try {
      parsed = JSON.parse(cleanedJson)
    } catch (err) {
      // 容错处理：如果未能直接输出 JSON，尝试提取中间的 JSON 串
      const match = cleanedJson.match(/\{[\s\S]*\}/)
      if (match) {
        parsed = JSON.parse(match[0])
      } else {
        throw err
      }
    }

    if (parsed.title && parsed.content) {
      // 创建笔记
      const newNote = await notesStore.createNote({
        title: parsed.title,
        content: parsed.content,
        format: 'markdown',
        visibility: 'PRIVATE',
        groupId: null,
        tags: parsed.tags || []
      })

      if (newNote) {
        messages.value[aiMsgIndex].content = `🎉 **本地备忘笔记已自动生成并保存成功！**\n\n- **标题**：${parsed.title}\n- **分类标签**：${(parsed.tags || []).map((t: string) => '#' + t).join(' ')}\n\n*系统已为您自动在左侧开启该笔记编辑 Tab，您可以直接查看或做进一步润色。*`
        notesStore.openNoteTab(newNote.id)
      } else {
        messages.value[aiMsgIndex].content = `❌ 笔记整理完毕，但保存至本地 SQLite 数据库失败，请检查终端日志。`
      }
    } else {
      messages.value[aiMsgIndex].content = `❌ 笔记生成失败。大模型未按预期返回 \`title\` 或 \`content\` 字段。\n\n大模型原始回复如下：\n\`\`\`\n${responseText}\n\`\`\``
    }
  } catch (e: any) {
    console.error('笔记生成失败:', e)
    messages.value[aiMsgIndex].content = `❌ 整理笔记失败: ${e.message || e}\n\n大模型原始回复：\n${responseText || '（无响应）'}`
  } finally {
    sending.value = false
    scrollToBottom()
  }
}

// ── 发送消息（智能 RAG 感知 + Agent 自动管道）──
async function sendMessage() {
  const text = userInput.value.trim()
  if (!text || sending.value) return

  // 检查是否匹配写/总结笔记意图
  const writeNoteKeywords = /总结.*到笔记|总结一下.*添加.*笔记|添加.*笔记|归纳.*到笔记|总结.*添加到笔记|生成.*笔记/i
  if (writeNoteKeywords.test(text)) {
    await handleCreateNoteFromAi(text)
    return
  }

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
  let promptInstruction = ''
  const userQuery = text.toLowerCase()
  const hasNoteKeywords = /笔记|备忘|文档|知识|记录|草稿|我的/.test(userQuery)
  const shouldRetrieve = enableLocalKb.value || hasNoteKeywords

  if (shouldRetrieve) {
    await notesStore.loadAllNotes()
    const activeNotes = notesStore.notes.filter(n => !n.isDeleted && !n.isArchived)
    
    // 如果是宽泛的“查看笔记/所有笔记/有哪些笔记”指令，我们直接把最近修改的 15 篇笔记的标题和摘要组装进 Prompt
    const isListNotesRequest = /查看笔记|列出笔记|有什么笔记|所有笔记|我的笔记|有哪些笔记|有哪些文档|有什么备忘|找下笔记/.test(userQuery)

    if (isListNotesRequest) {
      const sortedRecent = [...activeNotes].sort((a, b) => {
        const tA = new Date(a.updatedAt || a.createdAt).getTime()
        const tB = new Date(b.updatedAt || b.createdAt).getTime()
        return tB - tA
      }).slice(0, 15)
      
      const listText = sortedRecent.map((n, idx) => {
        const brief = n.content ? (n.content.slice(0, 80) + (n.content.length > 80 ? '...' : '')) : '无内容'
        return `${idx + 1}. 【${n.title || '无标题'}】(更新于: ${n.updatedAt || n.createdAt}) - 简述: ${brief}`
      }).join('\n')
      
      promptInstruction = `\n[系统感知] 用户提出了查看或列出其笔记的请求。以下是用户最近更新的 15 篇笔记列表摘要：\n${listText}\n请直接向用户展现此列表，并温柔、主动地询问用户想要详细阅读或处理哪一篇。\n`
    } else {
      // 正常的关键词检索
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
        contextNotes = activeNotes.slice(0, 10)
      } else if (titleMatched.length > 0) {
        contextNotes = Array.from(new Set([...titleMatched, ...contextNotes])).slice(0, 5)
      } else {
        contextNotes = scored.sort((a, b) => b.score - a.score).map(x => x.note).slice(0, 5)
      }
  
      const referenceText = contextNotes
        .map(n => `[笔记标题: ${n.title || '无标题'}, 更新时间: ${n.updatedAt || n.createdAt}]\n内容:\n${n.content}`)
        .join('\n\n---\n\n')
  
      promptInstruction = referenceText
        ? `\n以下是用户本地笔记库中相关的笔记供参考：\n${referenceText}\n请结合这些参考笔记直接且简洁地回答用户的问题，如果有相关内容可进行引用或说明。\n`
        : '\n未找到明确相关的本地笔记内容。请正常回答用户，或告诉用户您的本地笔记库中目前可能还没有相关内容。\n'
    }

    systemPromptStr = `你是通用 AI 助手，也是用户的本地备忘笔记助理。${promptInstruction}请直接、友好地回答用户。`
  } else {
    systemPromptStr = '你是一个通用 AI 助手。请直接、简洁地回答用户的问题。不要说"作为XX助手"之类的话，直接回答问题即可。'
  }

  // ── 智能路由：根据选定的角色匹配专属提示词 ──
  let matchedRole: AgentRole | null = null
  if (activeChatRoleId.value === 'auto') {
    // 不做角色分配，直接简洁回答
    systemPromptStr = `你是 JC9 本地桌面应用中的 AI 助手，运行在用户的电脑上。
你可以直接读取、写入用户电脑上的本地文件（通过 Agent 模式的工具）。
如果用户请求涉及操作本地文件、代码、终端命令，请告知用户开启"Agent 模式"（顶部的闪电按钮）来执行。
如果用户只是提问或聊天，请直接、简洁地回答问题，不要说"作为XX助手"之类的话。`
    if (enableLocalKb.value && promptInstruction) {
      systemPromptStr += `\n\n以下是用户本地笔记库中相关的参考内容：\n${promptInstruction}`
    }
  } else {
    matchedRole = chatRolesList.value.find(r => r.id === activeChatRoleId.value) || null
    if (matchedRole) {
      let roleInstructions = `${matchedRole.systemPrompt}\n\n当前任务：请以该角色的专业设定与视角，协助解答用户的问题。`
      if (enableLocalKb.value && promptInstruction) {
        roleInstructions += `\n${promptInstruction}`
      }
      systemPromptStr = roleInstructions
    }
  }

  if (enableDeepThink.value) {
    systemPromptStr += `\n\n【重要指令 - 深度思考模式已开启】：
请在回答前进行深入、细致的逐步思考和推导。在正式输出回答前，请先写出你的思考过程，以格式 \`思考过程：...\` 呈现，然后再输出最终答复。请展现严密的逻辑性，层层深入剖析问题。`
  }

  const promptMsgs: Message[] = [
    { role: 'system', content: systemPromptStr },
    ...messages.value.filter(m => m.role === 'user' || m.role === 'assistant')
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
  const isAuto = activeChatRoleId.value === 'auto'
  messages.value.push({ 
    role: 'assistant', 
    content: '', 
    modelName: currentModel,
    roleName: isAuto ? undefined : activeChatRole.value.name
  })
  const aiMsgIndex = messages.value.length - 1

  try {
    let receivedHeader = false
    let currentContent = ''
    await callAiStream(promptMsgs, (chunk) => {
      currentContent += chunk
      
      // 检测并解析首行的 "选择角色：[前端工程师]"
      if (!receivedHeader && activeChatRoleId.value === 'auto') {
        const firstLineEnd = currentContent.indexOf('\n')
        const line = firstLineEnd !== -1 ? currentContent.slice(0, firstLineEnd) : currentContent
        if (line.includes('选择角色：')) {
          const match = line.match(/选择角色：\[(.*?)\]/)
          if (match && match[1]) {
            messages.value[aiMsgIndex].roleName = match[1].trim()
            receivedHeader = true
          }
        }
      }

      // 过滤掉第一行 "选择角色：[xxx]" 在 UI 的渲染
      let displayContent = currentContent
      if (activeChatRoleId.value === 'auto' && displayContent.includes('选择角色：')) {
        const firstLineEnd = displayContent.indexOf('\n')
        if (firstLineEnd !== -1) {
          displayContent = displayContent.slice(firstLineEnd).trimStart()
        } else {
          displayContent = ''
        }
      }

      messages.value[aiMsgIndex].content = displayContent
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

  // 意图检测：Agent 模式下全部走 Agent 管道，不再判断是否编码任务
  // 用户主动开启了 Agent 模式，就应该始终使用 Agent

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
          const role = getRole(task.assignedWorker)
          addAgentBubble(`🚀 启动 Worker：「${task.title}」... [分配角色: ${role.icon} ${role.name}]`)
          const sp = `${role.systemPrompt}\n\n当前任务描述及 ReAct 要求：${task.description}`
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
  if (!p) return '未设置'
  const parts = p.replace(/\\/g, '/').split('/')
  return (parts[parts.length - 1] || p)
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

async function handleKillWorker(workerId: string) {
  await ai.killWorker(workerId)
}

function selectSession(id: string) {
  ai.currentSessionId = id
  showSessionPopup.value = false
}

async function handleCreateSessionPopup() {
  if (!newSessionTitle.value.trim()) return
  await ai.createSession(newSessionTitle.value.trim())
  newSessionTitle.value = ''
}

async function handleSelectWorkspace() {
  await ai.changeWorkspaceDialog()
  manualPath.value = ai.workspaceRoot
}

const browserUrlInput = ref('https://google.com')
const showBrowserDialog = ref(false)

async function handleBrowserOpen() {
  // 如果已经有打开过的 URL，直接使用；否则弹出对话框
  if (browserUrlInput.value && browserUrlInput.value.startsWith('http')) {
    try {
      await invoke('ai_browser_navigate', { url: browserUrlInput.value })
    } catch {
      showBrowserDialog.value = true
    }
  } else {
    showBrowserDialog.value = true
  }
}

async function handleBrowserConfirm() {
  if (!browserUrlInput.value.trim()) return
  if (!browserUrlInput.value.startsWith('http')) {
    browserUrlInput.value = 'https://' + browserUrlInput.value
  }
  try {
    await invoke('ai_browser_navigate', { url: browserUrlInput.value })
    showBrowserDialog.value = false
  } catch (e: any) {
    console.error('打开浏览器失败:', e)
  }
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
  chatRolesList.value = loadAllRoles()
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
  await ai.registerAllFrontendTools()
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

</script>

<template><div class="ai-helper-container">
    <div class="ai-top-bar">
      <button class="top-session-btn" @click="showSessionPopup = true" :title="ai.currentSession?.title || '选择会话'">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:15px;height:15px">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
      </button>
      <button class="top-browser-btn" @click="handleBrowserOpen" title="打开浏览器">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:15px;height:15px">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="2" y1="12" x2="22" y2="12"></line>
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
        </svg>
      </button>
    </div>
    <!-- 左侧：问答与聊天主面板 -->
    <div class="ai-chat-section">
      <div class="ai-chat-area">
        <!-- Cline 风格智能体控制台 (聊天区域顶部) -->
        <div class="cline-agent-console" v-if="ai.workers.length > 0">
          <div class="console-header" @click="isConsoleExpanded = !isConsoleExpanded">
            <div class="console-title">
              <span class="console-icon">🤖</span>
              <span>智能开发控制台</span>
              <span class="console-badge" v-if="ai.activeWorkers.length > 0">
                {{ ai.activeWorkers.length }} 个活跃代理
              </span>
              <span class="console-badge completed" v-else>
                运行结束
              </span>
            </div>
            <div class="console-actions">
              <button 
                v-if="ai.activeWorkers.length > 0" 
                class="console-btn kill-all" 
                @click.stop="handleKillAllWorkers"
              >
                ✕ 全部强杀
              </button>
              <span class="chevron-icon" :class="{ rotated: isConsoleExpanded }">▼</span>
            </div>
          </div>
          
          <div class="console-body" v-show="isConsoleExpanded">
            <div v-for="w in ai.workers" :key="w.id" class="agent-card" :class="w.status">
              <div class="agent-card-header" @click="toggleWorkerExpand(w.id)">
                <div class="agent-info">
                  <span class="agent-dot" :style="{ background: statusColors[w.status] }"></span>
                  <span class="agent-role-badge" :title="getWorkerRole(w.taskId).description">
                    {{ getWorkerRole(w.taskId).icon }} {{ getWorkerRole(w.taskId).name }}
                  </span>
                  <span class="agent-name">Worker-{{ w.id.slice(0, 8) }}</span>
                  <span class="agent-task-title" :title="getTaskTitle(w.taskId)">
                    「{{ getTaskTitle(w.taskId) }}」
                  </span>
                  <span class="agent-status-label" :style="{ color: statusColors[w.status] }">
                    {{ statusLabel(w.status) }}
                  </span>
                </div>
                <div class="agent-actions">
                  <span class="agent-stats">
                    Cost: ¥{{ (w.tokenCount * 0.000005).toFixed(4) }} ({{ w.toolCallCount }} 工具)
                  </span>
                  <button 
                    v-if="w.status !== 'completed' && w.status !== 'failed' && w.status !== 'killed'"
                    class="agent-kill-btn" 
                    @click.stop="handleKillWorker(w.id)"
                  >
                    终止
                  </button>
                  <span class="chevron-icon" :class="{ rotated: expandedWorkers[w.id] }">▼</span>
                </div>
              </div>
              
              <div class="agent-card-body" v-show="expandedWorkers[w.id]">
                <!-- ReAct 详细轨迹 -->
                <div class="agent-history-log">
                  <div v-for="step in w.history" :key="step.iteration" class="log-step">
                    <div class="step-header">
                      <span class="step-num">#{{ step.iteration }} 轮迭代</span>
                      <span class="step-time">{{ formatTime(step.timestamp) }}</span>
                    </div>
                    
                    <div class="step-section thought" v-if="step.thought">
                      <div class="section-title">🧠 Thought</div>
                      <pre class="section-content">{{ step.thought }}</pre>
                    </div>
                    
                    <div class="step-section action" v-if="step.action">
                      <div class="section-title">🔧 Call Tool: <code>{{ step.action.toolName }}</code></div>
                      <div class="tool-args" v-if="step.action.arguments && Object.keys(step.action.arguments).length > 0">
                        <strong>参数:</strong> <code>{{ JSON.stringify(step.action.arguments) }}</code>
                      </div>
                    </div>
                    
                    <div class="step-section observation" v-if="step.observation">
                      <div class="section-title">👁️ Observation</div>
                      <pre class="section-content">{{ step.observation }}</pre>
                    </div>
                  </div>
                  
                  <div class="log-current" v-if="w.currentThought && w.status !== 'completed' && w.status !== 'failed' && w.status !== 'killed'">
                    <div class="step-header">
                      <span class="step-num">正在执行/思考中...</span>
                    </div>
                    <div class="step-section thought">
                      <pre class="section-content">{{ w.currentThought }}</pre>
                    </div>
                  </div>

                  <div class="log-failed-reason" v-if="w.terminationReason">
                    <div class="failed-reason-title">❌ 终止原因 / 异常信息</div>
                    <pre class="failed-reason-content">{{ w.terminationReason }}</pre>
                  </div>
                  
                  <div class="log-empty" v-if="(!w.history || w.history.length === 0) && !w.currentThought">
                    暂无迭代历史日志
                  </div>
                </div>
              </div>
            </div>

            <!-- ── 任务树面板 ── -->
            <div class="console-section" v-if="ai.taskTree.length > 0">
              <div class="console-section-title">📋 任务树 ({{ ai.taskTree.length }})</div>
              <div class="task-tree-compact">
                <div v-for="task in ai.taskTree" :key="task.id" class="task-node-compact">
                  <span :class="['task-status-dot', task.status]"></span>
                  <span class="task-title-compact">{{ task.title }}</span>
                  <span class="task-status-label">{{ statusLabel(task.status) }}</span>
                </div>
              </div>
            </div>

            <!-- ── 知识库搜索面板 ── -->
            <div class="console-section" v-if="ai.workers.length > 0">
              <div class="console-section-title">🔍 知识库</div>
              <div class="kb-search-compact">
                <input v-model="kbSearchQuery" class="kb-search-input" placeholder="搜索知识库..." @keyup.enter="searchKnowledgeBase" />
                <button class="kb-search-btn" @click="searchKnowledgeBase">搜索</button>
              </div>
              <div v-for="entry in kbSearchResults" :key="entry.id" class="kb-result-item">
                <div class="kb-result-title">{{ entry.title }}</div>
                <div class="kb-result-preview">{{ entry.content.slice(0, 80) }}...</div>
              </div>
            </div>
          </div>
        </div>

        <!-- 消息滚动列表 -->
        <div class="chat-messages">
          <div v-for="(msg, i) in messages" :key="i" :class="['chat-bubble', msg.role]">
            <div class="bubble-sender">
              <span v-if="msg.roleName" class="bubble-role-badge">
                {{ msg.roleName }}
              </span>
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
            ref="inputTextarea"
            v-model="userInput" 
            :placeholder="placeholderText" 
            class="ds-textarea"
            @focus="isFocused = true"
            @blur="isFocused = false"
            @keydown.enter.prevent="handleEnterKey"
            @input="autoResizeTextarea"
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
                    <option v-for="m in models" :key="m.name" :value="m.id">
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
              <!-- 会话居中弹窗 -->
              <div v-if="showSessionPopup" class="session-overlay" @click="showSessionPopup = false">
                <div class="session-modal" @click.stop>
                  <div class="session-modal-header">
                    <span>会话列表</span>
                    <button class="session-modal-close" @click="showSessionPopup = false">✕</button>
                  </div>
                  <div class="session-modal-body">
                    <div class="session-create-row">
                      <input v-model="newSessionTitle" class="session-input" placeholder="新建会话名称..." @keyup.enter="handleCreateSessionPopup" />
                      <button class="session-create-btn" @click="handleCreateSessionPopup">＋</button>
                    </div>
                    <div v-for="s in ai.sessions" :key="s.id" :class="['session-item', { active: s.id === ai.currentSessionId }]" @click="selectSession(s.id)">
                      <span class="session-dot" :class="s.status"></span>
                      <span class="session-name">{{ s.title }}</span>
                      <span class="session-date">{{ new Date(s.updatedAt).toLocaleDateString() }}</span>
                    </div>
                    <div v-if="ai.sessions.length === 0" class="session-empty">暂无会话，请新建</div>
                  </div>
                </div>
              </div>

              <!-- 浏览器 URL 输入弹窗 -->
              <div v-if="showBrowserDialog" class="session-overlay" @click="showBrowserDialog = false">
                <div class="browser-modal" @click.stop>
                  <div class="session-modal-header">
                    <span>🌐 打开浏览器</span>
                    <button class="session-modal-close" @click="showBrowserDialog = false">✕</button>
                  </div>
                  <div class="browser-modal-body">
                    <input v-model="browserUrlInput" class="browser-url-input" placeholder="输入 URL..."
                      @keyup.enter="handleBrowserConfirm" @click.stop />
                    <button class="browser-go-btn" @click="handleBrowserConfirm">打开</button>
                  </div>
                </div>
              </div>

              <!-- 工作区 -->
              <div class="ds-pill-select-wrap workspace">
                <button class="ds-pill-inline-btn workspace-btn" @click="handleSelectWorkspace" :title="ai.workspaceRoot || '选择工作区'">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                  </svg>
                  {{ workspaceShortName }}
                </button>
              </div>

              <!-- 深度思考 -->
              <div class="ds-pill-select-wrap" :class="{ active: enableDeepThink }">
                <button class="ds-pill-inline-btn" @click="enableDeepThink = !enableDeepThink" :title="enableDeepThink ? '关闭深度思考' : '开启深度思考'">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                    <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(45 12 12)"></ellipse>
                    <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(-45 12 12)"></ellipse>
                    <circle cx="12" cy="12" r="1.5" fill="currentColor"></circle>
                  </svg>
                  深度思考
                </button>
              </div>
              
              <!-- 本地知识库 -->
              <div class="ds-pill-select-wrap" :class="{ active: enableLocalKb }">
                <button class="ds-pill-inline-btn" @click="enableLocalKb = !enableLocalKb" :title="enableLocalKb ? '关闭本地知识库' : '开启本地知识库'">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                    <circle cx="12" cy="12" r="9"></circle>
                    <line x1="2" y1="12" x2="22" y2="12"></line>
                    <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                  </svg>
                  本地知识库
                </button>
              </div>

              <!-- Agent 模式 -->
              <div class="ds-pill-select-wrap" :class="{ active: enableAgentMode }">
                <button class="ds-pill-inline-btn" @click="enableAgentMode = !enableAgentMode" title="开启后直接描述编码任务，自动规划并执行">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                  </svg>
                  Agent
                </button>
              </div>
               <!-- AI 角色选择 / 智能体团队状态 -->
              <div v-if="!enableAgentMode" class="ds-pill-select-wrap">
                <select v-model="activeChatRoleId" class="ds-pill-select" style="max-width:92px" title="切换当前对话角色">
                  <option value="auto">智能路由</option>
                  <option v-for="r in chatRolesList" :key="r.id" :value="r.id">
                    {{ r.name }}
                  </option>
                </select>
              </div>
              <div v-else class="ds-pill-select-wrap" style="color: var(--jc-color-accent); font-weight: bold; border-color: rgba(138, 88, 255, 0.3)">
                <span class="ds-pill-text" style="font-size:10.5px;padding: 0 4px">智能体团队 (多角色协同)</span>
              </div>
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
  flex-direction: column;
  flex: 1;
  height: 100%;
  overflow: hidden;
  background: var(--jc-bg-app);
}

.ai-top-bar {
  display: flex;
  align-items: center;
  padding: 2px 14px;
  background: var(--jc-bg-panel);
  border-bottom: 1px solid var(--jc-border-default);
  gap: 8px;
  flex-shrink: 0;
      justify-content: flex-end;
}

.top-session-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  background: transparent;
  border: none;
  color: var(--jc-text-secondary);
  cursor: pointer;
  padding: 0;
  transition: all 0.15s;
}
.top-session-btn:hover {
  color: var(--jc-color-accent);
}

.top-browser-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  background: transparent;
  border: none;
  color: var(--jc-text-secondary);
  cursor: pointer;
  padding: 0;
  transition: all 0.15s;
  margin-left: 4px;
}
.top-browser-btn:hover {
  color: var(--jc-color-accent);
}

.browser-modal {
  background: var(--jc-surface);
  border: 1px solid var(--jc-border);
  border-radius: 8px;
  padding: 0;
  width: 400px;
  max-width: 90vw;
  box-shadow: var(--jc-shadow-lg);
}
.browser-modal-body {
  display: flex;
  gap: 8px;
  padding: 12px 16px 16px;
}
.browser-url-input {
  flex: 1;
  background: var(--jc-bg-secondary);
  border: 1px solid var(--jc-border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--jc-text);
  font-size: 13px;
  outline: none;
}
.browser-url-input:focus {
  border-color: var(--jc-color-accent);
}
.browser-go-btn {
  background: var(--jc-color-accent);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 8px 16px;
  font-size: 13px;
  cursor: pointer;
  white-space: nowrap;
  transition: opacity 0.15s;
}
.browser-go-btn:hover {
  opacity: 0.85;
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
  padding: 2px;
  gap: 2px;
}

/* Cline 风格智能体控制台 */
.cline-agent-console {
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  overflow: hidden;
  margin-bottom: 4px;
  display: flex;
  flex-direction: column;
  max-height: 280px;
  flex-shrink: 0;

  .console-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--jc-bg-input);
    cursor: pointer;
    border-bottom: 1px solid var(--jc-border-default);
    user-select: none;

    &:hover {
      background: var(--jc-bg-hover);
    }
  }

  .console-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--jc-text-highlight);

    .console-icon {
      font-size: 14px;
    }

    .console-badge {
      font-size: 10px;
      padding: 1px 6px;
      border-radius: 10px;
      background: rgba(138, 88, 255, 0.15);
      color: var(--jc-color-accent);
      font-weight: 500;

      &.completed {
        background: rgba(63, 185, 80, 0.15);
        color: #3fb950;
      }
    }
  }

  .console-actions {
    display: flex;
    align-items: center;
    gap: 10px;

    .console-btn.kill-all {
      background: rgba(248, 81, 73, 0.1);
      border: 1px solid rgba(248, 81, 73, 0.3);
      color: #f85149;
      font-size: 10px;
      padding: 2px 6px;
      border-radius: 4px;
      cursor: pointer;
      font-weight: 500;

      &:hover {
        background: #f85149;
        color: #fff;
      }
    }
  }

  .chevron-icon {
    font-size: 8px;
    color: var(--jc-text-secondary);
    transition: transform 0.2s;

    &.rotated {
      transform: rotate(180deg);
    }
  }

  .console-body {
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--jc-bg-panel);

    &::-webkit-scrollbar {
      width: 4px;
    }
    &::-webkit-scrollbar-thumb {
      background: var(--jc-border-default);
      border-radius: 2px;
    }
  }

  .agent-card {
    border: 1px solid var(--jc-border-default);
    border-radius: 6px;
    overflow: hidden;
    background: var(--jc-bg-input);

    &.thinking { border-left: 3px solid #58a6ff; }
    &.acting { border-left: 3px solid #f0883e; }
    &.observing { border-left: 3px solid #a371f7; }
    &.waitingApproval { border-left: 3px solid #d29922; }
    &.completed { border-left: 3px solid #3fb950; opacity: 0.85; }
    &.failed { border-left: 3px solid #f85149; }
    &.killed { border-left: 3px solid #8b949e; opacity: 0.75; }

    .agent-card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 6px 10px;
      cursor: pointer;
      font-size: 11px;
      user-select: none;

      &:hover {
        background: var(--jc-bg-hover);
      }
    }

    .agent-info {
      display: flex;
      align-items: center;
      gap: 6px;
      min-width: 0;

      .agent-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        flex-shrink: 0;
      }

      .agent-role-badge {
        font-size: 10px;
        padding: 1px 6px;
        background: rgba(var(--jc-color-accent-rgb, 138, 88, 255), 0.1);
        border: 1px solid rgba(var(--jc-color-accent-rgb, 138, 88, 255), 0.2);
        color: var(--jc-color-accent);
        border-radius: 4px;
        font-weight: 500;
        white-space: nowrap;
      }

      .agent-name {
        font-weight: 600;
        color: var(--jc-text-primary);
        font-family: monospace;
      }

      .agent-task-title {
        color: var(--jc-text-secondary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 150px;
      }

      .agent-status-label {
        font-weight: 500;
        font-size: 10px;
      }
    }

    .agent-actions {
      display: flex;
      align-items: center;
      gap: 8px;

      .agent-stats {
        font-size: 10px;
        color: var(--jc-text-secondary);
      }

      .agent-kill-btn {
        background: transparent;
        border: 1px solid var(--jc-border-default);
        color: var(--jc-text-secondary);
        font-size: 9px;
        padding: 1px 4px;
        border-radius: 3px;
        cursor: pointer;

        &:hover {
          background: rgba(248, 81, 73, 0.1);
          border-color: #f85149;
          color: #f85149;
        }
      }
    }

    .agent-card-body {
      border-top: 1px solid var(--jc-border-default);
      background: var(--jc-bg-panel);
      padding: 8px;
    }

    .agent-history-log {
      display: flex;
      flex-direction: column;
      gap: 10px;
      max-height: 200px;
      overflow-y: auto;
      font-family: monospace;
      font-size: 10.5px;
      padding-right: 4px;

      &::-webkit-scrollbar {
        width: 3px;
      }
      &::-webkit-scrollbar-thumb {
        background: var(--jc-border-default);
        border-radius: 1.5px;
      }
    }

    .log-step {
      border-bottom: 1px dashed var(--jc-border-default);
      padding-bottom: 8px;
      
      &:last-child {
        border-bottom: none;
        padding-bottom: 0;
      }
    }

    .step-header {
      display: flex;
      justify-content: space-between;
      color: var(--jc-text-secondary);
      margin-bottom: 4px;
      font-size: 10px;

      .step-num {
        font-weight: bold;
        color: var(--jc-color-accent);
      }
    }

    .step-section {
      margin-top: 4px;
      display: flex;
      flex-direction: column;
      gap: 2px;

      .section-title {
        font-weight: 600;
        color: var(--jc-text-primary);
        font-size: 10.5px;
      }

      .section-content {
        margin: 0;
        padding: 4px 6px;
        background: var(--jc-bg-input);
        border: 1px solid var(--jc-border-default);
        border-radius: 4px;
        white-space: pre-wrap;
        word-break: break-all;
        color: var(--jc-text-primary);
        max-height: 100px;
        overflow-y: auto;
      }
    }

    .log-current {
      padding-top: 4px;
      .step-num {
        color: #58a6ff;
        animation: pulse 1.5s infinite;
      }
      .section-content {
        border-left: 2px solid #58a6ff;
      }
    }

    .log-failed-reason {
      margin-top: 6px;
      padding: 6px;
      background: rgba(248, 81, 73, 0.05);
      border: 1px solid rgba(248, 81, 73, 0.2);
      border-radius: 4px;

      .failed-reason-title {
        font-weight: bold;
        color: #f85149;
        margin-bottom: 2px;
      }
      .failed-reason-content {
        margin: 0;
        white-space: pre-wrap;
        color: var(--jc-text-primary);
      }
    }

    .log-empty {
      text-align: center;
      color: var(--jc-text-secondary);
      padding: 12px;
    }
  }
}

@keyframes pulse {
  0% { opacity: 0.6; }
  50% { opacity: 1; }
  100% { opacity: 0.6; }
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  border-radius: 2px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 6px;

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
    display: flex;
    align-items: center;
  }

  .bubble-role-badge {
    font-size: 10px;
    padding: 1px 6px;
    background: rgba(var(--jc-color-accent-rgb, 138, 88, 255), 0.12);
    border: 1px solid rgba(var(--jc-color-accent-rgb, 138, 88, 255), 0.25);
    color: var(--jc-color-accent);
    border-radius: 4px;
    font-weight: bold;
    margin-right: 4px;
    display: inline-flex;
    align-items: center;
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
  border-radius: 4px;
  padding: 5px 14px;
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
    max-height: 200px;
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
    gap: 3px;

    padding: 0 6px;
    height: 22px;
    color: var(--jc-text-secondary);
    position: relative;
    transition: all 0.15s;

    &:hover { color: var(--jc-text-primary); border-color: var(--jc-border-strong); }
    &.active {
      border-color: var(--jc-color-accent, #8a58ff);
      color: var(--jc-color-accent, #8a58ff);
    }
    &.workspace { max-width: 130px; overflow: hidden; }

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

  .ds-pill-inline-btn {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: transparent;
    border: none;
    color: inherit;
    font-size: 10.5px;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
    white-space: nowrap;
    outline: none;
  }
  .workspace-btn {
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

/* ── 会话居中弹窗 ── */
.session-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  padding: 20px;
}

.session-modal {
  width: 100%;
  max-width: 420px;
  max-height: 70vh;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  border-radius: 10px;
  box-shadow: 0 12px 40px rgba(0,0,0,0.4);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.session-modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--jc-border-default);
  font-size: 13px;
  font-weight: 600;
  color: var(--jc-text-primary);
}

.session-modal-close {
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  cursor: pointer;
  font-size: 14px;
  padding: 2px 6px;
  border-radius: 4px;
  &:hover { color: var(--jc-color-error); background: rgba(248,81,73,0.08); }
}

.session-modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
}

.session-create-row {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}

.session-input {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-primary);
  font-size: 12px;
  padding: 6px 10px;
  border-radius: 5px;
  outline: none;
  &:focus { border-color: var(--jc-color-accent); }
}

.session-create-btn {
  background: var(--jc-color-accent);
  color: #fff;
  border: none;
  width: 30px;
  border-radius: 5px;
  cursor: pointer;
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  &:hover { opacity: 0.9; }
}

.session-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 5px;
  cursor: pointer;
  font-size: 12px;
  color: var(--jc-text-primary);
  transition: background 0.15s;
  &:hover { background: var(--jc-bg-hover); }
  &.active { background: var(--jc-bg-selected); color: var(--jc-color-accent); }
}

.session-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  &.active { background: #3fb950; }
  &.completed { background: #8b949e; }
  &.failed { background: #f85149; }
}

.session-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-date {
  font-size: 10px;
  color: var(--jc-text-secondary);
  flex-shrink: 0;
}

.session-empty {
  text-align: center;
  color: var(--jc-text-secondary);
  font-size: 12px;
  padding: 24px;
}

/* ── 控制台内嵌面板 ── */
.console-section {
  border-top: 1px solid var(--jc-border-default);
  padding: 8px 10px;
}

.console-section-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
}

/* 任务树 */
.task-tree-compact {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.task-node-compact {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  background: var(--jc-bg-input);
  border-radius: 4px;
  font-size: 10px;
}

.task-status-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  flex-shrink: 0;
  &.pending { background: #8b949e; }
  &.inProgress { background: #58a6ff; }
  &.completed { background: #3fb950; }
  &.failed { background: #f85149; }
  &.blocked { background: #d29922; }
}

.task-title-compact {
  flex: 1;
  color: var(--jc-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-status-label {
  font-size: 9px;
  color: var(--jc-text-secondary);
}

/* 知识库搜索 */
.kb-search-compact {
  display: flex;
  gap: 4px;
  margin-bottom: 6px;
}

.kb-search-input {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-primary);
  font-size: 10px;
  padding: 4px 6px;
  border-radius: 4px;
  outline: none;
  &:focus { border-color: var(--jc-color-accent); }
}

.kb-search-btn {
  padding: 4px 8px;
  background: var(--jc-bg-btn);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  color: var(--jc-text-secondary);
  font-size: 10px;
  cursor: pointer;
  &:hover { color: var(--jc-text-primary); border-color: var(--jc-color-accent); }
}

.kb-result-item {
  padding: 4px 6px;
  border-bottom: 1px solid var(--jc-border-default);
  &:last-child { border-bottom: none; }
}

.kb-result-title {
  font-size: 10px;
  font-weight: 600;
  color: var(--jc-text-primary);
}

.kb-result-preview {
  font-size: 9px;
  color: var(--jc-text-secondary);
  margin-top: 1px;
}
</style>
