<script setup lang="ts">
import { ref, onMounted, nextTick, computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'

const notesStore = useNotesStore()
const status = useStatusStore()

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

  // Ollama
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

  // vLLM
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

  // DeepSeek
  if (deepseekKey.value || aiProvider.value === 'deepseek') {
    groups['DeepSeek'] = [
      { name: 'deepseek-chat', label: 'deepseek-chat' },
      { name: 'deepseek-coder', label: 'deepseek-coder' }
    ]
  }

  // OpenAI
  if (openaiKey.value || aiProvider.value === 'openai') {
    groups['OpenAI'] = [
      { name: 'gpt-4o-mini', label: 'gpt-4o-mini' },
      { name: 'gpt-4o', label: 'gpt-4o' }
    ]
  }

  // Gemini
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

onMounted(() => {
  loadConfig()
  fetchOllamaModels()
  fetchVllmModels()
  selectedCombinedModel.value = `${aiProvider.value}/${aiModel.value}`
})

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
    // OpenAI / DeepSeek / vLLM
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
          // 忽略非 JSON 部分
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
            // 忽略非合法 JSON 分块
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

// ── 动作 1: 发送普通对话 (具备智能 RAG 本地笔记感知) ──
async function sendMessage() {
  const text = userInput.value.trim()
  if (!text || sending.value) return

  messages.value.push({ role: 'user', content: text })
  userInput.value = ''
  sending.value = true
  scrollToBottom()

  let systemPrompt = ''

  if (enableLocalKb.value) {
    // 1. 确保每次问答前同步最新的本地笔记
    await notesStore.loadAllNotes()

    // 2. 提取与用户当前输入最相关的笔记 (智能 RAG 检索召回)
    const userQuery = text.toLowerCase()
    const activeNotes = notesStore.notes.filter(n => !n.isDeleted && !n.isArchived)
    
    // 智能提取核心检索关键字
    const terms = extractTerms(userQuery)

    // 标题包含或被包含在查询中均算直接完全匹配
    const titleMatched = activeNotes.filter(n => n.title && (userQuery.includes(n.title.toLowerCase()) || n.title.toLowerCase().includes(userQuery)))

    // 对所有笔记计算相关性得分
    const scored = activeNotes.map(n => {
      let score = 0
      const titleLower = (n.title || '').toLowerCase()
      const contentLower = (n.content || '').toLowerCase()

      // 标题直接整体匹配
      if (titleLower && (userQuery.includes(titleLower) || titleLower.includes(userQuery))) {
        score += 150
      }
      
      // 标签匹配
      n.tags.forEach(t => {
        const tagLower = t.toLowerCase()
        if (userQuery.includes(tagLower)) {
          score += 50
        }
      })
      
      // 多关键字匹配
      terms.forEach(term => {
        if (titleLower && titleLower.includes(term)) {
          score += term.length * 30
        }
        if (contentLower && contentLower.includes(term)) {
          score += term.length * 8
        }
        n.tags.forEach(t => {
          if (t.toLowerCase().includes(term)) {
            score += term.length * 15
          }
        })
      })

      return { note: n, score }
    })

    // 整理出高相关度笔记列表。
    // 如果提问中带有统括性词语（如汇总、所有），且激活了本地知识库，我们直接拉取最多前 40 篇做全局汇总分析
    const isSummaryRequest = /汇总|所有|全部|概括|总结我|整理我/.test(userQuery)
    let contextNotes = scored.filter(x => x.score > 0).map(x => x.note)
    
    if (contextNotes.length === 0 || isSummaryRequest) {
      contextNotes = activeNotes.slice(0, 40)
    } else if (titleMatched.length > 0) {
      // 若有标题完全匹配，确保完全匹配的排在最前并去重限制
      contextNotes = Array.from(new Set([...titleMatched, ...contextNotes])).slice(0, 10)
    } else {
      contextNotes = scored.sort((a, b) => b.score - a.score).map(x => x.note).slice(0, 10)
    }

    // 拼接上下文
    const referenceText = contextNotes
      .map(n => `[笔记标题: ${n.title || '无标题'}, 更新时间: ${n.updatedAt || n.createdAt}]\n内容:\n${n.content}`)
      .join('\n\n---\n\n')

    systemPrompt = `你是一个内置在用户本地客户端（JC9 项目与终端管理器）中的专属 AI 助理。
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
    systemPrompt = `你是一个内置在用户本地客户端（JC9 项目与终端管理器）中的专属 AI 助理。请以专业、清晰且对开发者友好的语气解答用户的问题。`
  }

  // 深度思考 Prompt 附加
  if (enableDeepThink.value) {
    systemPrompt += `\n\n【重要指令 - 深度思考模式已开启】：
请在回答前进行深入、细致的逐步思考和推导。在正式输出回答前，请先写出你的思考过程，以格式 \`思考过程：...\` 呈现，然后再输出最终答复。请展现严密的逻辑性，层层深入剖析问题。`
  }

  // 挂载包含 RAG 提示消息和普通历史的发送队列 (排除最后的空占位消息)
  const promptMsgs: Message[] = [
    { role: 'system', content: systemPrompt },
    ...messages.value.filter(m => m.role === 'user' || m.role === 'assistant').slice(0, -1)
  ]

  // 挂载空的 assistant 占位消息，标记当前的 ModelName
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


// ── 动作 3: 润色备忘内容 ──
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

// ── 动作 4: 提取/推荐标签 ──
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
</script>

<template>
  <div class="ai-helper">
    <div class="ai-chat-area">
      <!-- 消息滚动列表 -->
      <div class="chat-messages">
        <div v-for="(msg, i) in messages" :key="i" :class="['chat-bubble', msg.role]">
          <div class="bubble-sender">
            <!-- 未指定模型名时 fallback 显示 AI Copilot -->
            {{ msg.role === 'user' ? '您' : (msg.role === 'system' ? '系统' : (msg.modelName || 'AI Copilot')) }}
          </div>
          <div class="bubble-content" v-html="msg.content.replace(/\n/g, '<br/>')"></div>
        </div>
      </div>

      <!-- 快捷效率胶囊工具栏 (当输入框有文字时平滑滑出) -->
      <Transition name="fade-slide">
        <div class="shortcut-pills" v-if="userInput.trim().length > 0">
          <span class="shortcut-pill-desc">针对当前输入：</span>
          <button class="shortcut-pill" @click="polishMemo" :disabled="sending">✨ 润色排版</button>
          <button class="shortcut-pill" @click="recommendTags" :disabled="sending">🏷️ 提取标签</button>
        </div>
      </Transition>

      <!-- DeepSeek 风格圆角一体化输入卡片 -->
      <div class="ds-input-card" :class="{ 'focused': isFocused, 'has-content': userInput.trim().length > 0 }">
        <!-- 文本输入区 -->
        <textarea 
          v-model="userInput" 
          :placeholder="placeholderText" 
          class="ds-textarea"
          @focus="isFocused = true"
          @blur="isFocused = false"
          @keydown.enter.prevent="handleEnterKey"
        ></textarea>
        
        <!-- 底部控制栏 -->
        <div class="ds-control-bar">
          <!-- 左侧药丸选择器 -->
          <div class="ds-pills">
            <!-- 模型选择胶囊 (放到深度思考左边) -->
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
              <!-- 本地模型的一键刷新按钮 -->
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

            <!-- 深度思考药丸 -->
            <button 
              class="ds-pill-btn deep-think" 
              :class="{ active: enableDeepThink }" 
              @click="enableDeepThink = !enableDeepThink"
              title="开启后将引导 AI 进行深度思考与思考链逐步 analysis"
            >
              <svg class="ds-pill-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(45 12 12)"></ellipse>
                <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(-45 12 12)"></ellipse>
                <circle cx="12" cy="12" r="1.5" fill="currentColor"></circle>
              </svg>
              深度思考
            </button>
            
            <!-- 本地知识库药丸 -->
            <button 
              class="ds-pill-btn local-kb" 
              :class="{ active: enableLocalKb }" 
              @click="enableLocalKb = !enableLocalKb"
              title="开启后将自动关联匹配您本地的备忘录笔记进行智能问答"
            >
              <svg class="ds-pill-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="9"></circle>
                <line x1="2" y1="12" x2="22" y2="12"></line>
                <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
              </svg>
              本地知识库
            </button>
          </div>
          
          <!-- 右侧动作按钮区 -->
          <div class="ds-actions">
            <!-- 导入当前编辑的笔记快捷键 (回形针) -->
            <button 
              v-if="notesStore.activeNoteTabId" 
              class="ds-action-btn attach" 
              @click="attachActiveNote" 
              title="将当前正在编辑的笔记内容插入输入框"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path>
              </svg>
            </button>
            
            <!-- 清除历史按钮 (垃圾桶) -->
            <button 
              class="ds-action-btn clear" 
              @click="clearChat" 
              title="清空聊天记录，开启新对话"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3 6 5 6 21 6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
            </button>
            
            <!-- 发送按钮 -->
            <button 
              class="ds-send-btn" 
              :disabled="sending || !userInput.trim()" 
              @click="sendMessage"
              title="发送消息"
            >
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
</template>

<style scoped lang="scss">
.ai-helper {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  background: var(--jc-bg-app);
}


.ai-chat-area {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 12px;
  gap: 10px;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  border-radius: 6px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;

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
    font-size: 10px;
    color: var(--jc-text-secondary);
    font-weight: 500;
  }

  .bubble-content {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 12.5px;
    line-height: 1.6;
    padding: 8px 12px;
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

// ── 快捷效率胶囊工具栏 ──
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
    display: inline-flex;
    align-items: center;
    gap: 4px;

    &:hover:not(:disabled) {
      background: var(--jc-bg-hover);
      border-color: var(--jc-color-accent);
      color: var(--jc-color-accent);
      transform: translateY(-1px);
    }

    &:active:not(:disabled) {
      transform: translateY(0);
    }

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

// ── 胶囊栏滑入滑出过渡动效 ──
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(6px) scale(0.98);
}
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(6px) scale(0.98);
}

// ── DeepSeek 风格圆角一体化输入卡片 ──
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

  &:hover {
    border-color: var(--jc-border-strong);
  }

  &.focused {
    border-color: var(--jc-color-accent, #8a58ff);
    box-shadow: 0 0 12px rgba(var(--jc-color-accent-rgb, 138, 88, 255), 0.15);
  }

  .ds-textarea {
    width: 100%;
    min-height: 48px;
    max-height: 160px;
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

    &::placeholder {
      color: var(--jc-text-secondary);
      opacity: 0.65;
    }

    &::-webkit-scrollbar {
      width: 4px;
    }
    &::-webkit-scrollbar-thumb {
      background: var(--jc-border-default);
      border-radius: 2px;
    }
  }

  .ds-control-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid transparent; // 占位
  }

  .ds-pills {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .ds-pill-select-wrap {
    display: inline-flex;
    align-items: center;
    background: var(--jc-bg-btn);
    border: 1px solid var(--jc-border-default);
    padding: 0 10px;
    height: 28px;
    border-radius: 20px;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    color: var(--jc-text-secondary);
    position: relative;

    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
      border-color: var(--jc-border-strong);
    }
    
    &:focus-within {
      border-color: var(--jc-color-accent, #8a58ff);
      color: var(--jc-color-accent, #8a58ff);
    }

    .model-icon {
      width: 14px;
      height: 14px;
      margin-right: 4px;
      flex-shrink: 0;
    }

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
      text-overflow: ellipsis;
      white-space: nowrap;
      overflow: hidden;
      appearance: none; // 去掉原生外观
      -webkit-appearance: none;
      
      // 用自定义小三角代替
      background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 24 24' fill='none' stroke='gray' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'></polyline></svg>");
      background-repeat: no-repeat;
      background-position: right center;
      background-size: 8px;

      optgroup {
        background: var(--jc-bg-panel);
        color: var(--jc-text-primary);
      }
      option {
        background: var(--jc-bg-panel);
        color: var(--jc-text-primary);
      }
    }

    .ds-pill-refresh {
      background: transparent;
      border: none;
      outline: none;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding: 0;
      margin-left: 2px;
      border-left: 1px solid var(--jc-border-default);
      padding-left: 6px;
      color: var(--jc-text-secondary);
      height: 14px;

      &:hover {
        color: var(--jc-color-accent);
      }

      .refresh-icon-svg {
        width: 11px;
        height: 11px;
        &.spinning {
          animation: spin-anim 1s linear infinite;
        }
      }
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
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

    .ds-pill-icon {
      width: 14px;
      height: 14px;
      flex-shrink: 0;
    }

    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
      border-color: var(--jc-border-strong);
    }

    &.active {
      background: rgba(138, 88, 255, 0.09);
      border-color: var(--jc-color-accent, #8a58ff);
      color: var(--jc-color-accent, #8a58ff);
      box-shadow: 0 1px 6px rgba(138, 88, 255, 0.1);
    }
  }

  .ds-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

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
    transition: all 0.2s;

    svg {
      width: 17px;
      height: 17px;
    }

    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
    }
    
    &.attach {
      &:hover {
        color: var(--jc-color-success);
        background: rgba(var(--jc-color-success-rgb, 46, 204, 113), 0.08);
      }
    }

    &.clear {
      &:hover {
        color: var(--jc-color-error);
        background: rgba(var(--jc-color-error-rgb, 220, 38, 38), 0.08);
      }
    }
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
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);

    svg {
      width: 16px;
      height: 16px;
    }

    &:not(:disabled) {
      background: var(--jc-color-accent, #8a58ff);
      color: #ffffff;
      cursor: pointer;

      &:hover {
        transform: scale(1.06);
        filter: brightness(1.1);
        box-shadow: 0 2px 8px rgba(138, 88, 255, 0.35);
      }

      &:active {
        transform: scale(0.94);
      }
    }
  }
}

@keyframes spin-anim {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
