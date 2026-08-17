<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { save, open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { loadAllRoles, saveAllRoles, type AgentRole } from '@/config/roles'
import { useAiStore } from '@/stores/ai'
import JcModal from '@/components/ui/JcModal.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcInputNumber from '@/components/ui/JcInputNumber.vue'
import JcSelect, { type JcSelectOption } from '@/components/ui/JcSelect.vue'
import JcSwitch from '@/components/ui/JcSwitch.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const store = useNotesStore()
const status = useStatusStore()
const aiStore = useAiStore()

const activeTab = ref<'general' | 'ai' | 'ai-roles' | 'backup' | 'skills' | 'command' |'hook'|'plugin'|'mcp'>('general')

// JcSelect 选项
const formatOptions: JcSelectOption[] = [
  { label: 'Markdown (推荐)', value: 'markdown' },
  { label: '纯文本', value: 'plain' },
]
const visibilityOptions: JcSelectOption[] = [
  { label: 'PRIVATE (私有本地)', value: 'PRIVATE' },
  { label: 'PUBLIC (公开/对接远端后可见)', value: 'PUBLIC' },
]
const providerOptions: JcSelectOption[] = [
  { label: 'DeepSeek', value: 'deepseek' },
  { label: 'OpenAI', value: 'openai' },
  { label: 'Ollama (本地)', value: 'ollama' },
  { label: 'Google Gemini', value: 'gemini' },
  { label: 'vLLM (自部署)', value: 'vllm' },
]
const reasoningOptions: JcSelectOption[] = [
  { label: 'High (推荐，大多数场景)', value: 'high' },
  { label: 'Max (复杂 Agent 任务)', value: 'max' },
  { label: '关闭 (节省 Token)', value: '' },
]
const transportOptions: JcSelectOption[] = [
  { label: 'SSE（远程服务器）', value: 'sse' },
  { label: 'Stdio（本地进程）', value: 'stdio' },
]

// ── 解决拖拽选择文本触发弹窗关闭的 Bug ──
let mousedownTarget: EventTarget | null = null

function handleMousedown(e: MouseEvent) {
  mousedownTarget = e.target
}

function handleOverlayClick(e: MouseEvent) {
  if (e.target === e.currentTarget && mousedownTarget === e.currentTarget) {
    emit('close')
  }
}

// ── 设置偏好状态 (存入 localStorage) ──
const defaultFormat = ref<'markdown' | 'plain'>('markdown')
const defaultVisibility = ref<'PRIVATE' | 'PUBLIC'>('PRIVATE')
const saveOnClose = ref(localStorage.getItem('notes-save-on-close') === 'true')

// ── AI 模型配置列表 ──
interface ModelConfig {
  id: string
  name: string
  provider: 'ollama' | 'deepseek' | 'openai' | 'gemini' | 'vllm'
  endpoint: string
  apiKey: string
  model: string
  inputPrice: number
  outputPrice: number
  costLimit: number
  reasoningEffort: 'high' | 'max' | ''  // DS thinking mode 强度，空=关闭
}

const modelConfigs = ref<ModelConfig[]>([])
const showModelForm = ref(false)
const vllmModels = ref<string[]>([])
const loadingModels = ref(false)

function blankModel(): ModelConfig {
  return {
    id: '',
    name: '',
    provider: 'deepseek',
    endpoint: 'https://api.deepseek.com',
    apiKey: '',
    model: 'deepseek-v4-pro',
    inputPrice: 2.0,
    outputPrice: 4.0,
    costLimit: 10.0,
    reasoningEffort: 'high',
  }
}

const newModelForm = ref<ModelConfig>(blankModel())

// ── AI 角色管理 ──
const rolesList = ref<AgentRole[]>([])
const showRoleForm = ref(false)

function blankRole(): AgentRole {
  return {
    id: '',
    name: '',
    icon: '🤖',
    description: '',
    systemPrompt: '',
    isCustom: true
  }
}

const newRoleForm = ref<AgentRole>(blankRole())

function addRole() {
  newRoleForm.value = blankRole()
  showRoleForm.value = true
}

function editRole(role: AgentRole) {
  newRoleForm.value = { ...role }
  showRoleForm.value = true
}

function deleteRole(id: string) {
  rolesList.value = rolesList.value.filter(r => r.id !== id)
}

function saveRoleForm() {
  const r = newRoleForm.value
  if (!r.name.trim() || !r.systemPrompt.trim()) {
    status.pushMessage('请填写角色名称和专属提示词', 'warn')
    return
  }
  if (!r.id) {
    r.id = 'custom_' + Date.now().toString()
  }
  const idx = rolesList.value.findIndex(role => role.id === r.id)
  if (idx >= 0) {
    rolesList.value[idx] = { ...r }
  } else {
    rolesList.value.push({ ...r })
  }
  showRoleForm.value = false
}

function cancelRoleForm() {
  showRoleForm.value = false
}

// ── 备份与导入 ──

onMounted(async () => {
  // 从 JSON 文件同步 AI 配置到 localStorage（跨 dev/build 共享）
  try {
    const json = await invoke<string>('get_ai_config')
    const cfg = JSON.parse(json)
    for (const [k, v] of Object.entries(cfg)) {
      if (typeof v === 'string' && v) localStorage.setItem(k, v)
    }
  } catch { /* ignore */ }

  defaultFormat.value = (localStorage.getItem('notes-default-format') as any) || 'markdown'
  defaultVisibility.value = (localStorage.getItem('notes-default-visibility') as any) || 'PRIVATE'
  saveOnClose.value = localStorage.getItem('notes-save-on-close') === 'true'

  // 读取模型配置列表
  const saved = localStorage.getItem('notes-ai-models')
  if (saved) {
    try { modelConfigs.value = JSON.parse(saved) } catch { /* ignore */ }
  }
  // 兼容旧版单模型配置
  if (modelConfigs.value.length === 0) {
    const legacy: ModelConfig = {
      id: 'legacy',
      name: '默认配置',
      provider: (localStorage.getItem('notes-ai-provider') as any) || 'deepseek',
      endpoint: localStorage.getItem('notes-ai-endpoint') || 'https://api.deepseek.com',
      apiKey: localStorage.getItem('notes-ai-apikey') || '',
      model: localStorage.getItem('notes-ai-model') || 'deepseek-v4-pro',
      inputPrice: parseFloat(localStorage.getItem('notes-ai-input-price') || '3.0'),
      outputPrice: parseFloat(localStorage.getItem('notes-ai-output-price') || '6.0'),
      costLimit: parseFloat(localStorage.getItem('notes-ai-cost-limit') || '5.0'),
      reasoningEffort: 'high',
    }
    modelConfigs.value = [legacy]
  }

  // 加载 AI 角色配置
  rolesList.value = loadAllRoles()

  // 加载 MCP 服务器列表
  aiStore.loadMcpServers()
})

// 辅助：将所有 AI localStorage 键同步到 JSON 文件
async function saveAiConfigToJson() {
  const keys: string[] = []
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i)
    if (k?.startsWith('notes-ai-') || k === 'jc9-last-model') keys.push(k)
  }
  const config: Record<string, string> = {}
  for (const k of keys) {
    const v = localStorage.getItem(k)
    if (v) config[k] = v
  }
  try { await invoke('save_ai_config', { config: JSON.stringify(config) }) } catch {}
}

async function saveSettings() {
  localStorage.setItem('notes-default-format', defaultFormat.value)
  localStorage.setItem('notes-default-visibility', defaultVisibility.value)
  localStorage.setItem('notes-save-on-close', String(saveOnClose.value))
  localStorage.setItem('notes-ai-models', JSON.stringify(modelConfigs.value))
  
  // 保存 AI 角色配置
  saveAllRoles(rolesList.value)

  // 兼容旧字段
  const first = modelConfigs.value[0]
  if (first) {
    localStorage.setItem('notes-ai-provider', first.provider)
    localStorage.setItem('notes-ai-endpoint', first.endpoint)
    localStorage.setItem('notes-ai-apikey', first.apiKey)
    localStorage.setItem('notes-ai-model', first.model)
    localStorage.setItem(`notes-ai-endpoint-${first.provider}`, first.endpoint)
    localStorage.setItem(`notes-ai-apikey-${first.provider}`, first.apiKey)
    localStorage.setItem(`notes-ai-model-${first.provider}`, first.model)

    // 同步 Agent cost config
    import('@/stores/ai').then(({ useAiStore }) => {
      useAiStore().updateCostConfig({
        inputCachedCostPerM: first.inputPrice * 0.008,
        inputUncachedCostPerM: first.inputPrice,
        outputCostPerM: first.outputPrice,
        costLimit: first.costLimit,
      })
    })
  }
  await saveAiConfigToJson()
  status.pushMessage('设置保存成功', 'success')
  emit('close')
}

function addModel() {
  newModelForm.value = blankModel()
  showModelForm.value = true
}

function editModel(config: ModelConfig) {
  newModelForm.value = { ...config }
  showModelForm.value = true
}

function deleteModel(id: string) {
  modelConfigs.value = modelConfigs.value.filter(c => c.id !== id)
}

function saveModelForm() {
  const f = newModelForm.value
  if (!f.name.trim() || !f.model.trim()) {
    status.pushMessage('请填写模型名称和代号', 'warn')
    return
  }
  if (!f.id) f.id = Date.now().toString()
  const idx = modelConfigs.value.findIndex(c => c.id === f.id)
  if (idx >= 0) {
    modelConfigs.value[idx] = { ...f }
  } else {
    modelConfigs.value.push({ ...f })
  }
  showModelForm.value = false
}

function cancelModelForm() {
  showModelForm.value = false
}

function setProviderDefaults() {
  const f = newModelForm.value
  if (f.provider === 'ollama') { f.endpoint = 'http://127.0.0.1:11434'; f.model = 'llama3' }
  else if (f.provider === 'deepseek') { f.endpoint = 'https://api.deepseek.com'; f.model = 'deepseek-v4-pro' }
  else if (f.provider === 'openai') { f.endpoint = 'https://api.openai.com/v1'; f.model = 'gpt-4o-mini' }
  else if (f.provider === 'gemini') { f.endpoint = 'https://generativelanguage.googleapis.com'; f.model = 'gemini-1.5-flash' }
  else if (f.provider === 'vllm') { f.endpoint = 'http://192.168.5.100:8000/v1'; f.model = ''; fetchVllmModelsForm() }
}

async function fetchVllmModelsForm() {
  loadingModels.value = true
  vllmModels.value = []
  vllmSelectedModels.value = []
  try {
    const url = `${newModelForm.value.endpoint.replace(/\/+$/, '')}/models`
    const res = await fetch(url)
    if (res.ok) {
      const json = await res.json()
      if (json.data && Array.isArray(json.data)) {
        vllmModels.value = json.data.map((m: any) => m.id)
        status.pushMessage(`获取到 ${vllmModels.value.length} 个模型`, 'success')
        syncVllmSelection()
      }
    } else {
      status.pushMessage(`获取模型列表失败 (${res.status})`, 'error')
    }
  } catch (e) {
    status.pushMessage(`无法连接 vLLM 服务: ${e}`, 'error')
  }
  finally { loadingModels.value = false }
}

function onEndpointBlur() {
  if (newModelForm.value.provider === 'vllm' && newModelForm.value.endpoint.trim()) {
    fetchVllmModelsForm()
  }
}

const vllmSelectedModels = ref<string[]>([])

function toggleVllmModel(model: string) {
  const idx = vllmSelectedModels.value.indexOf(model)
  if (idx >= 0) {
    vllmSelectedModels.value.splice(idx, 1)
  } else {
    vllmSelectedModels.value.push(model)
  }
  newModelForm.value.model = vllmSelectedModels.value.join(',')
}

// 初始化选中状态：当 vllmModels 加载完成时，根据已有 model 字段勾选
function syncVllmSelection() {
  if (newModelForm.value.provider === 'vllm' && newModelForm.value.model) {
    const existing = newModelForm.value.model.split(',').map(m => m.trim()).filter(Boolean)
    vllmSelectedModels.value = existing.filter(m => vllmModels.value.includes(m))
  }
}

// ── MCP 服务器管理 ──
const showMcpForm = ref(false)
const connecting = ref(false)
const mcpViewMode = ref<'list' | 'json'>('list')
const mcpJsonConfig = ref('')
const applyingJson = ref(false)
const mcpJsonError = ref('')
const mcpForm = ref({
  transport: 'sse' as 'sse' | 'stdio',
  name: '',
  url: '',
  command: '',
  argsText: '',
})

function statusLabel(s: string): string {
  const map: Record<string, string> = {
    connected: '已连接',
    disconnected: '已断开',
    connecting: '连接中',
    error: '错误',
  }
  return map[s] || s
}

async function saveMcpForm() {
  const f = mcpForm.value
  if (!f.name.trim()) {
    status.pushMessage('请填写服务器名称', 'warn')
    return
  }
  connecting.value = true
  try {
    if (f.transport === 'sse') {
      if (!f.url.trim()) {
        status.pushMessage('请填写 SSE URL', 'warn')
        return
      }
      await aiStore.connectMcpServer(f.name.trim(), f.url.trim())
    } else {
      if (!f.command.trim()) {
        status.pushMessage('请填写启动命令', 'warn')
        return
      }
      const args = f.argsText.split(',').map(a => a.trim()).filter(Boolean)
      await aiStore.connectMcpServerStdio(f.name.trim(), f.command.trim(), args)
    }
    showMcpForm.value = false
    status.pushMessage(`已连接 MCP 服务器: ${f.name}`, 'success')
  } catch (e) {
    status.pushMessage(`连接失败: ${e}`, 'error')
  } finally {
    connecting.value = false
  }
}

async function disconnectMcp(name: string) {
  try {
    await aiStore.disconnectMcpServer(name)
    status.pushMessage(`已断开 MCP 服务器: ${name}`, 'success')
  } catch (e) {
    status.pushMessage(`断开失败: ${e}`, 'error')
  }
}

function switchToJsonMode() {
  // 将当前服务器列表导出为 JSON 格式
  const config: Record<string, any> = { mcpServers: {} }
  for (const srv of aiStore.mcpServers) {
    if (srv.transport === 'stdio') {
      config.mcpServers[srv.name] = {
        command: srv.command,
        args: srv.args,
      }
    } else {
      config.mcpServers[srv.name] = {
        url: srv.url,
      }
    }
  }
  mcpJsonConfig.value = JSON.stringify(config, null, 2)
  mcpJsonError.value = ''
  mcpViewMode.value = 'json'
}

function validateAndParseMcpJson(text: string): { mcpServers: Record<string, any> } | null {
  try {
    const parsed = JSON.parse(text)
    if (!parsed.mcpServers || typeof parsed.mcpServers !== 'object') {
      throw new Error('缺少 "mcpServers" 顶层字段')
    }
    for (const [name, cfg] of Object.entries(parsed.mcpServers)) {
      const c = cfg as any
      if (typeof c !== 'object' || c === null) {
        throw new Error(`服务器 "${name}" 配置无效`)
      }
      if (!c.url && !c.command) {
        throw new Error(`服务器 "${name}" 需要提供 "url"（SSE）或 "command"（Stdio）`)
      }
    }
    return parsed
  } catch (e: any) {
    mcpJsonError.value = `JSON 解析错误: ${e.message}`
    return null
  }
}

async function applyMcpJson() {
  applyingJson.value = true
  mcpJsonError.value = ''
  try {
    // 先断开所有现有连接
    for (const srv of aiStore.mcpServers) {
      await aiStore.disconnectMcpServer(srv.name)
    }

    // 等待断开生效
    await new Promise(r => setTimeout(r, 300))

    // 解析 JSON
    const config = validateAndParseMcpJson(mcpJsonConfig.value)
    if (!config) {
      applyingJson.value = false
      return
    }

    // 逐个连接
    let connected = 0
    let failed = 0
    for (const [name, cfg] of Object.entries(config.mcpServers)) {
      try {
        if (cfg.url) {
          await aiStore.connectMcpServer(name, cfg.url)
        } else if (cfg.command) {
          const args: string[] = cfg.args || []
          await aiStore.connectMcpServerStdio(name, cfg.command, args)
        }
        connected++
      } catch (e) {
        failed++
        console.error(`连接 MCP 服务器 "${name}" 失败:`, e)
      }
    }

    await aiStore.loadMcpServers()
    status.pushMessage(`JSON 配置应用完成: ${connected} 成功, ${failed} 失败`, failed > 0 ? 'warn' : 'success')
  } catch (e: any) {
    mcpJsonError.value = `应用配置失败: ${e.message}`
  } finally {
    applyingJson.value = false
  }
}

async function disconnectAllMcp() {
  for (const srv of aiStore.mcpServers) {
    await aiStore.disconnectMcpServer(srv.name)
  }
  await aiStore.loadMcpServers()
  status.pushMessage('已断开所有 MCP 服务器', 'success')
}

// ── 技能管理 ──
interface SystemSkill {
  id: string
  name: string
  version: string
  description: string
  path: string
  file_size: number
  enabled: boolean
  source: string
}

const systemSkills = ref<SystemSkill[]>([])
const skillsLoading = ref(false)
const skillsError = ref('')
const skillsSearch = ref('')

const filteredSkills = computed(() => {
  const q = skillsSearch.value.trim().toLowerCase()
  if (!q) return systemSkills.value
  return systemSkills.value.filter(s =>
    s.name.toLowerCase().includes(q) ||
    s.id.toLowerCase().includes(q) ||
    s.description.toLowerCase().includes(q)
  )
})

async function loadSystemSkills() {
  skillsLoading.value = true
  skillsError.value = ''
  try {
    // 确保 workspaceRoot 已从后端读取
    if (!aiStore.workspaceRoot) {
      await aiStore.loadWorkspaceRoot()
    }
    const root = aiStore.workspaceRoot || ''
    const skills = await invoke<SystemSkill[]>('list_system_skills', { workspaceRoot: root })
    systemSkills.value = skills
  } catch (e) {
    skillsError.value = `加载失败: ${e}`
  } finally {
    skillsLoading.value = false
  }
}

function formatSkillSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function openSkillFolder(path: string) {
  invoke('show_in_folder', { path })
}

// ── 备份与导出 ──
async function exportData() {
  try {
    const filePath = await save({
      filters: [{ name: 'JSON Backup', extensions: ['json'] }],
      defaultPath: 'jc9-memos-backup.json'
    })

    if (filePath) {
      const dataStr = JSON.stringify({
        notes: store.notes,
        groups: store.groups
      }, null, 2)

      const encoder = new TextEncoder()
      const binaryData = Array.from(encoder.encode(dataStr))

      await invoke('write_file_binary', { path: filePath, data: binaryData })
      status.pushMessage('备份文件导出成功！', 'success')
    }
  } catch (e) {
    status.pushMessage(`导出失败: ${e}`, 'error')
  }
}

// ── 备份与导入 ──
async function importData() {
  try {
    const selected = await open({
      filters: [{ name: 'JSON Backup', extensions: ['json'] }],
      multiple: false
    })

    if (selected && typeof selected === 'string') {
      // 引入 Rust 端读取文本文件的接口
      const content = await invoke<string>('read_file_string', { path: selected })
      const data = JSON.parse(content)

      if (!data.notes || !Array.isArray(data.notes)) {
        status.pushMessage('无效的备份文件结构', 'error')
        return
      }

      // 批量覆盖式保存到本地数据库
      for (const note of data.notes) {
        await store.saveNote(note)
      }
      if (data.groups && Array.isArray(data.groups)) {
        for (const g of data.groups) {
          await store.updateGroup(g)
        }
      }

      await store.loadAllNotes()
      await store.loadGroups()
      status.pushMessage('备份数据导入恢复成功！', 'success')
    }
  } catch (e) {
    status.pushMessage(`导入失败: ${e}`, 'error')
  }
}

</script>

<template>
  <div v-if="show" class="settings-overlay" @mousedown="handleMousedown" @click="handleOverlayClick">
    <div class="settings-modal animate-slide-in">
      <div class="settings-header">
        <span class="settings-title">设置</span>
        <button class="settings-close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="settings-container">
        <!-- 侧边 Tab 导航 -->
        <aside class="settings-nav">
          <div :class="['nav-item', { active: activeTab === 'general' }]" @click="activeTab = 'general'">
            通用设置
          </div>
          <div :class="['nav-item', { active: activeTab === 'ai' }]" @click="activeTab = 'ai'">
            模型配置
          </div>
          <div :class="['nav-item', { active: activeTab === 'ai-roles' }]" @click="activeTab = 'ai-roles'">
            智能体
          </div>
          <div :class="['nav-item', { active: activeTab === 'skills' }]" @click="activeTab = 'skills'; loadSystemSkills()">
            技能
          </div>
          <div :class="['nav-item', { active: activeTab === 'command' }]" @click="activeTab = 'command'">
            指令
          </div>
          <div :class="['nav-item', { active: activeTab === 'hook' }]" @click="activeTab = 'hook'">
            钩子
          </div>
          <div :class="['nav-item', { active: activeTab === 'plugin' }]" @click="activeTab = 'plugin'">
            插件
          </div>
          <div :class="['nav-item', { active: activeTab === 'mcp' }]" @click="activeTab = 'mcp'">
            MCP
          </div>
          <div :class="['nav-item', { active: activeTab === 'backup' }]" @click="activeTab = 'backup'">
            数据备份导入
          </div>
        </aside>

        <!-- 主内容区 -->
        <main class="settings-content">
          <!-- 1. 通用设置 -->
          <div v-if="activeTab === 'general'" class="settings-pane">
            <h3 class="pane-title">偏好设置</h3>
            <div class="form-group">
              <label>默认笔记格式</label>
              <JcSelect
                beam glow
                :model-value="defaultFormat"
                :options="formatOptions"
                style="width: 100%"
                @update:model-value="defaultFormat = $event as 'markdown' | 'plain'"
              />
              <span class="help-text">新建备忘时的默认输入解析格式</span>
            </div>

            <div class="form-group">
              <label>新建笔记默认可见性</label>
              <JcSelect
                beam glow
                :model-value="defaultVisibility"
                :options="visibilityOptions"
                style="width: 100%"
                @update:model-value="defaultVisibility = $event as 'PRIVATE' | 'PUBLIC'"
              />
              <span class="help-text">第一期完全本地化下默认均为 PRIVATE 级别</span>
            </div>

            <div class="form-group">
              <label class="toggle-row">
                <span>关闭标签时自动保存笔记</span>
                <JcSwitch v-model:checked="saveOnClose" />
              </label>
              <span class="help-text">开启后，点击标签栏 ✕ 按钮或右键关闭标签时自动保存当前编辑内容</span>
            </div>
          </div>

          <!-- 2. AI 助理配置 -->
          <div v-if="activeTab === 'ai'" class="settings-pane">
            <h3 class="pane-title">AI 模型配置</h3>
            <p class="pane-desc" style="margin-bottom:12px">管理您接入的大模型供应商，每个模型独立配置计费与熔断限额。</p>

            <!-- 模型列表 -->
            <div class="model-list">
              <div v-for="cfg in modelConfigs" :key="cfg.id" class="model-card">
                <div class="model-card-header">
                  <span class="model-card-name">{{ cfg.name }}</span>
                  <span class="model-card-provider">{{ cfg.provider }}</span>
                  <span class="model-card-model">{{ cfg.model }}</span>
                </div>
                <!-- <div class="model-card-meta">
                  <span v-if="cfg.reasoningEffort">{{ cfg.reasoningEffort }}</span>
                  <span>输入 ¥{{ cfg.inputPrice }}/M</span>
                  <span>输出 ¥{{ cfg.outputPrice }}/M</span>
                  <span>限额 ¥{{ cfg.costLimit }}</span>
                </div> -->
                <div class="model-card-actions">
                  <button class="model-btn edit" @click="editModel(cfg)">编辑</button>
                  <button class="model-btn del" @click="deleteModel(cfg.id)">删除</button>
                </div>
              </div>
              <div v-if="modelConfigs.length === 0" class="empty-hint">尚未添加任何模型</div>
            </div>

            <button class="add-model-btn" @click="addModel">+ 添加模型</button>

            <!-- 添加/编辑模型表单 -->
            <JcModal v-model:open="showModelForm" :title="(newModelForm.id ? '编辑' : '添加') + '模型配置'" width="480">
                <div class="form-group">
                  <label>配置名称</label>
                  <JcInput beam glow v-model="newModelForm.name" placeholder="例如：DeepSeek 主力" />
                </div>
                <div class="form-group">
                  <label>供应商</label>
                  <JcSelect
                    beam glow
                    :model-value="newModelForm.provider"
                    :options="providerOptions"
                    style="width: 100%"
                    @change="setProviderDefaults"
                    @update:model-value="newModelForm.provider = $event as ModelConfig['provider']"
                  />
                </div>
                <div class="form-group">
                  <label>Endpoint</label>
                  <JcInput beam glow v-model="newModelForm.endpoint" @blur="onEndpointBlur" />
                </div>
                <div class="form-group" v-if="newModelForm.provider !== 'ollama' && newModelForm.provider !== 'vllm'">
                  <label>API Key</label>
                  <JcInput beam glow v-model="newModelForm.apiKey" type="password" placeholder="sk-..." />
                </div>
                <div class="form-group">
                  <label>Model</label>
                  <div v-if="newModelForm.provider === 'vllm'" class="vllm-model-area">
                    <div class="vllm-toolbar">
                      <span class="vllm-count" v-if="!loadingModels">{{ vllmModels.length }} 个模型</span>
                      <button class="vllm-refresh-btn" :disabled="loadingModels" @click="fetchVllmModelsForm" title="刷新模型列表">
                        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" :class="{ spinning: loadingModels }">
                          <path d="M1.5 8a6.5 6.5 0 0 1 10.5-5L14 5m0-3.5V5h-3.5M14.5 8a6.5 6.5 0 0 1-10.5 5L2 11m0 3.5V11h3.5"/>
                        </svg>
                        刷新
                      </button>
                    </div>
                    <div v-if="loadingModels" class="vllm-loading">正在获取模型列表...</div>
                    <div v-else-if="vllmModels.length === 0" class="vllm-empty">点击刷新从 <code>/models</code> 获取模型列表</div>
                    <div v-else class="vllm-checklist">
                      <label v-for="m in vllmModels" :key="m" class="vllm-check-item" :class="{ checked: vllmSelectedModels.includes(m) }">
                        <input type="checkbox" :checked="vllmSelectedModels.includes(m)" @change="toggleVllmModel(m)" />
                        <span>{{ m }}</span>
                      </label>
                    </div>
                  </div>
                  <JcInput beam glow v-else v-model="newModelForm.model" placeholder="多个用英文逗号分隔，如: gemini-1.5-flash, gemini-1.5-pro" />
                  <span class="help-text" v-if="newModelForm.provider !== 'vllm'">支持输入多个模型，请使用英文逗号 <code>,</code> 分隔。</span>
                </div>
                <div class="form-group" v-if="newModelForm.provider === 'deepseek'">
                  <label>思维强度 (Thinking Mode)</label>
                  <JcSelect
                    beam glow
                    :model-value="newModelForm.reasoningEffort"
                    :options="reasoningOptions"
                    style="width: 100%"
                    @update:model-value="newModelForm.reasoningEffort = $event as ModelConfig['reasoningEffort']"
                  />
                  <span class="help-text">DeepSeek 思考模式：high 适合日常编码，max 适合复杂多步推理</span>
                </div>
                <div class="form-row">
                  <div class="form-group form-half">
                    <label>输入价格 (元/百万)</label>
                    <JcInputNumber :model-value="newModelForm.inputPrice" :min="0" :step="0.1" size="small" beam glow @update:model-value="newModelForm.inputPrice = $event ?? 0" />
                  </div>
                  <div class="form-group form-half">
                    <label>输出价格 (元/百万)</label>
                    <JcInputNumber :model-value="newModelForm.outputPrice" :min="0" :step="0.1" size="small" beam glow @update:model-value="newModelForm.outputPrice = $event ?? 0" />
                  </div>
                </div>
                <div class="form-group">
                  <label>熔断限额 (元)</label>
                  <JcInputNumber :model-value="newModelForm.costLimit" :min="0" :step="0.5" size="small" beam glow @update:model-value="newModelForm.costLimit = $event ?? 0" />
                </div>
                <template #footer>
                  <button class="footer-btn-cancel" @click="cancelModelForm">取消</button>
                  <button class="footer-btn-save" @click="saveModelForm">确定</button>
                </template>
              </JcModal>
          </div>

          <!-- 3. AI 角色管理 -->
          <div v-if="activeTab === 'ai-roles'" class="settings-pane">
            <h3 class="pane-title">AI 角色管理</h3>
            <p class="pane-desc" style="margin-bottom:12px">配置您开发中使用的 AI 角色。内置预设角色不可删除，您可以编辑其提示词；您也可以添加自定义角色。</p>

            <div class="roles-list-container">
              <div class="roles-grid">
                <div v-for="role in rolesList" :key="role.id" class="role-settings-card">
                  <div class="role-card-top">
                    <div class="role-card-info">
                      <span class="role-card-name">{{ role.name }}</span>
                      <span class="role-card-id">{{ role.id }}</span>
                    </div>
                    <span class="role-card-type" :class="{ custom: role.isCustom }">
                      {{ role.isCustom ? '自定义' : '预置' }}
                    </span>
                  </div>
                  <p class="role-card-desc">{{ role.description || '无介绍' }}</p>
                  <div class="role-card-actions">
                    <button class="role-btn edit" @click="editRole(role)">编辑</button>
                    <button v-if="role.isCustom" class="role-btn del" @click="deleteRole(role.id)">删除</button>
                  </div>
                </div>
              </div>
            </div>

            <button class="add-role-btn" @click="addRole">+ 添加角色</button>

            <!-- 添加/编辑角色表单 -->
            <JcModal v-model:open="showRoleForm" :title="(newRoleForm.id ? '编辑' : '添加') + ' AI 角色'" width="440">
                <div class="form-group">
                  <label>角色名称</label>
                  <JcInput beam glow v-model="newRoleForm.name" placeholder="例如：测试工程师" />
                </div>
                <div class="form-group">
                  <label>角色介绍</label>
                  <JcInput beam glow v-model="newRoleForm.description" placeholder="简述该角色的核心职责" />
                </div>
                <div class="form-group">
                  <label>专属系统提示词 (System Prompt)</label>
                  <JcTextarea v-model="newRoleForm.systemPrompt" beam glow :beam-size-ratio="0.6" :rows="6" placeholder="在此处输入详细的角色设定和 ReAct 指导性提示词..." />
                </div>
                <template #footer>
                  <button class="footer-btn-cancel" @click="cancelRoleForm">取消</button>
                  <button class="footer-btn-save" @click="saveRoleForm">确定</button>
                </template>
              </JcModal>
          </div>

          <!-- 4. 数据备份导入 -->
          <div v-if="activeTab === 'backup'" class="settings-pane">
            <h3 class="pane-title">数据本地备份与导入恢复</h3>
            <p class="pane-desc">因为所有备忘均保存在本地 SQLite 数据库中，您可以导出 JSON 数据包保存至本地，也可以通过 JSON 备份包将所有记录恢复至本软件中。</p>

            <div class="backup-actions">
              <button class="backup-btn export" @click="exportData">
                备份并导出 JSON
              </button>
              <button class="backup-btn import" @click="importData">
                导入并恢复 JSON
              </button>
            </div>
          </div>
          <div v-if="activeTab === 'skills'" class="settings-pane">
            <!-- <h3 class="pane-title">技能管理</h3> -->
            <!-- <p class="pane-desc" style="margin-bottom:12px">
              管理系统中的 AI 技能文件。技能位于 <code>~/.agents/skills/</code> 目录，每个技能是一个包含 <code>SKILL.md</code> 的子文件夹。
            </p> -->

            <div class="skills-toolbar">
              <JcInput beam glow v-model="skillsSearch" placeholder="搜索技能名称、ID 或描述..." style="flex: 1; min-width: 0" />
              <button class="skills-refresh-btn" :disabled="skillsLoading" @click="loadSystemSkills">
                {{ skillsLoading ? '加载中...' : '刷新' }}
              </button>
              <span class="skills-count" v-if="!skillsLoading">
                {{ filteredSkills.length }}/{{ systemSkills.length }} 个技能
              </span>
            </div>

            <div v-if="skillsError" class="skills-error">{{ skillsError }}</div>

            <div class="skills-list">
              <div v-for="skill in filteredSkills" :key="skill.id" class="skill-card">
                <div class="skill-card-header">
                  <div class="skill-card-info">
                    <span class="skill-card-name">{{ skill.name }} v{{ skill.version || '0.0.0' }}</span>
               
                  </div>
                  <div class="skill-card-badges">
                    <span class="skill-card-status" :class="{ enabled: skill.enabled }">
                      {{ skill.enabled ? '已启用' : '已禁用' }}
                    </span>
                    <span class="skill-card-source" :class="skill.source">
                      {{ skill.source === 'system' ? '全局' : '项目' }}
                    </span>
                  </div>
                </div>
                <p class="skill-card-desc" v-if="skill.description">{{ skill.description }}</p>
                <div class="skill-card-meta">
                  <span class="skill-card-size">{{ formatSkillSize(skill.file_size) }}</span>
                  <button class="skill-card-folder" @click="openSkillFolder(skill.path)" title="打开文件夹">
                    📂 {{ skill.path }}
                  </button>
                </div>
              </div>
              <div v-if="!skillsLoading && systemSkills.length === 0 && !skillsError" class="empty-hint">
                未发现任何技能文件。全局技能放入 <code>~/.agents/skills/</code>，项目技能放入 <code>.jc9/skills/</code>。
              </div>
            </div>
          </div>
          <div v-if="activeTab === 'command'" class="settings-pane">设置始终生效的指令，在整个工作区或用户配置文件中引导AI行为</div>
          <div v-if="activeTab === 'hook'" class="settings-pane">配置由保存文件或运行任务等事件触发的自动操作</div>
          <div v-if="activeTab === 'plugin'" class="settings-pane">安装和管理智能体插件，以添加更多工具，技能和集成</div>
          <!-- 7. MCP 服务器管理 -->
          <div v-if="activeTab === 'mcp'" class="settings-pane">
            <h3 class="pane-title">MCP 服务器管理</h3>
            <p class="pane-desc" style="margin-bottom:12px">连接外部 MCP（Model Context Protocol）服务器，通过自定义工具和数据源扩展 AI Agent 能力。</p>

            <!-- 模式切换 -->
            <div class="mcp-mode-toggle">
              <button :class="['mcp-mode-btn', { active: mcpViewMode === 'list' }]" @click="mcpViewMode = 'list'">服务器列表</button>
              <button :class="['mcp-mode-btn', { active: mcpViewMode === 'json' }]" @click="switchToJsonMode">JSON 配置</button>
            </div>

            <!-- ── 列表模式 ── -->
            <template v-if="mcpViewMode === 'list'">
              <div class="mcp-server-list">
                <div v-for="srv in aiStore.mcpServers" :key="srv.id" class="mcp-server-card">
                  <div class="mcp-server-top">
                    <div class="mcp-server-info">
                      <span class="mcp-server-name">{{ srv.name }}</span>
                      <span :class="['mcp-server-status', srv.status]">
                        {{ statusLabel(srv.status) }}
                      </span>
                      <span class="mcp-server-transport">{{ srv.transport === 'sse' ? 'SSE' : 'Stdio' }}</span>
                    </div>
                    <button class="mcp-server-disconnect" @click="disconnectMcp(srv.name)" title="断开连接">✕</button>
                  </div>
                  <div v-if="srv.url" class="mcp-server-url">{{ srv.url }}</div>
                  <div v-if="srv.command" class="mcp-server-url">{{ srv.command }} {{ srv.args?.join(' ') }}</div>
                  <div v-if="srv.errorMessage" class="mcp-server-error">{{ srv.errorMessage }}</div>
                  <div v-if="srv.tools && srv.tools.length > 0" class="mcp-server-tools">
                    <div class="mcp-tools-label">工具列表 ({{ srv.tools.length }})</div>
                    <div class="mcp-tools-grid">
                      <div v-for="tool in srv.tools" :key="tool.name" class="mcp-tool-chip" :title="tool.description || tool.name">
                        {{ tool.name }}
                      </div>
                    </div>
                  </div>
                  <div v-else class="mcp-server-tools">
                    <span class="mcp-tools-empty">暂无工具</span>
                  </div>
                </div>
                <div v-if="aiStore.mcpServers.length === 0" class="empty-hint">尚未连接任何 MCP 服务器</div>
              </div>

              <button class="add-mcp-btn" @click="showMcpForm = true">+ 连接 MCP 服务器</button>

              <!-- 添加 MCP 表单弹窗 -->
              <JcModal v-model:open="showMcpForm" title="连接 MCP 服务器" width="460">
                  <div class="form-group">
                    <label>连接方式</label>
                    <JcSelect
                      beam glow
                      :model-value="mcpForm.transport"
                      :options="transportOptions"
                      style="width: 100%"
                      @update:model-value="mcpForm.transport = $event as 'sse' | 'stdio'"
                    />
                  </div>
                  <div class="form-group">
                    <label>服务器名称</label>
                    <JcInput beam glow v-model="mcpForm.name" placeholder="例如：my-filesystem-server" />
                  </div>
                  <template v-if="mcpForm.transport === 'sse'">
                    <div class="form-group">
                      <label>SSE URL</label>
                      <JcInput beam glow v-model="mcpForm.url" placeholder="https://example.com/mcp" />
                    </div>
                  </template>
                  <template v-if="mcpForm.transport === 'stdio'">
                    <div class="form-group">
                      <label>启动命令</label>
                      <JcInput beam glow v-model="mcpForm.command" placeholder="例如：npx" />
                    </div>
                    <div class="form-group">
                      <label>参数</label>
                      <JcInput beam glow v-model="mcpForm.argsText" placeholder="例如：@modelcontextprotocol/server-filesystem, ." />
                      <span class="help-text">多个参数用逗号分隔</span>
                    </div>
                  </template>
                  <template #footer>
                    <button class="footer-btn-cancel" @click="showMcpForm = false">取消</button>
                    <button class="footer-btn-save" :disabled="connecting" @click="saveMcpForm">
                      {{ connecting ? '连接中...' : '连接' }}
                    </button>
                  </template>
                </JcModal>
              </template>

            <!-- ── JSON 配置模式 ── -->
            <template v-if="mcpViewMode === 'json'">
              <p class="pane-desc" style="margin-bottom:8px">
                编辑符合 MCP 规范的 JSON 配置，点击应用后自动连接所有服务器。
                格式参考：
                <code style="font-size:10px">{ "mcpServers": { "name": { "command": "...", "args": [...] } } }</code>
              </p>
              <textarea
                v-model="mcpJsonConfig"
                class="mcp-json-editor"
                spellcheck="false"
                placeholder='{\n  "mcpServers": {\n    "filesystem": {\n      "command": "npx",\n      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]\n    },\n    "remote-api": {\n      "url": "https://api.example.com/mcp"\n    }\n  }\n}'
              ></textarea>
              <div class="mcp-json-actions">
                <button class="mcp-json-apply" @click="applyMcpJson" :disabled="applyingJson">
                  {{ applyingJson ? '应用中...' : '应用 JSON 配置' }}
                </button>
                <button class="mcp-json-clear" @click="disconnectAllMcp">断开全部</button>
              </div>
              <div v-if="mcpJsonError" class="mcp-server-error">{{ mcpJsonError }}</div>
            </template>
          </div>
        </main>
      </div>

      <div class="settings-footer">
        <button class="footer-btn-cancel" @click="emit('close')">取消</button>
        <button class="footer-btn-save" @click="saveSettings">保存配置</button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-overlay {
  position: fixed;
  inset: 0;
  background: var(--jc-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1200;
}

.settings-modal {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  width: 870px;
  max-width: 95%;
  height: 580px;
  display: flex;
  flex-direction: column;
  box-shadow: var(--jc-shadow-modal);
  border-radius: 8px;
  overflow: hidden;
}

.settings-header {
  background: var(--jc-bg-panel);
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--jc-border-default);

  .settings-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--jc-text-highlight);
  }

  .settings-close-btn {
    background: none;
    border: none;
    color: var(--jc-text-secondary);
    font-size: 14px;
    cursor: pointer;

    &:hover {
      color: var(--jc-color-error);
    }
  }
}

.settings-container {
  display: flex;
  flex: 1;
  min-height: 0;
}

.settings-nav {
  width: 160px;
  background: var(--jc-bg-panel);
  border-right: 1px solid var(--jc-border-default);
  padding: 12px 0;
  display: flex;
  flex-direction: column;
  gap: 2px;

  .nav-item {
    padding: 8px 16px;
    font-size: 12px;
    color: var(--jc-text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;

    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
    }

    &.active {
      background: var(--jc-bg-selected);
      color: var(--jc-color-accent);
      font-weight: 600;
    }
  }
}

.settings-content {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}

.settings-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;

  .pane-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--jc-text-highlight);
    margin-bottom: 4px;
    border-bottom: 1px solid var(--jc-border-default);
    padding-bottom: 6px;
  }

  .pane-desc {
    font-size: 11px;
    color: var(--jc-text-secondary);
    line-height: 1.6;
  }
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 4px;

  label {
    font-size: 11px;
    font-weight: 500;
    color: var(--jc-text-primary);
  }

  .form-input {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 12px;
    padding: 6px 10px;
    border-radius: 4px;
    outline: none;

    &:focus {
      border-color: var(--jc-color-accent);
    }
  }

  .help-text {
    font-size: 10px;
    color: var(--jc-text-secondary);
    opacity: 0.8;
  }

  /* 开关行：标签 + toggle 并排 */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    font-size: 11px;
    font-weight: 500;
    color: var(--jc-text-primary);
    cursor: pointer;
  }
}

.backup-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}

.backup-btn {
  flex: 1;
  padding: 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: opacity 0.2s;

  &.export {
    background: rgba(var(--jc-color-accent-rgb, 0, 102, 204), 0.1);
    color: var(--jc-color-accent);
    border: 1px solid var(--jc-color-accent);
  }

  &.import {
    background: rgba(var(--jc-color-success-rgb, 0, 109, 50), 0.1);
    color: var(--jc-color-success);
    border: 1px solid var(--jc-color-success);
  }

  &:hover {
    opacity: 0.9;
  }
}

.settings-footer {
  padding: 12px 16px;
  background: var(--jc-bg-panel);
  border-top: 1px solid var(--jc-border-default);
  display: flex;
  justify-content: flex-end;
  gap: 8px;

  .footer-btn-cancel {
    background: var(--jc-bg-btn);
    color: var(--jc-text-secondary);
    border: none;
    padding: 6px 14px;
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;

    &:hover {
      color: var(--jc-text-primary);
    }
  }

  .footer-btn-save {
    background: var(--jc-color-accent);
    color: #fff;
    border: none;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;

    &:hover {
      opacity: 0.9;
    }
  }
}

// 动画
.animate-slide-in {
  animation: slideIn 0.2s ease-out;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(20px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* ── 模型列表 ── */
.model-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 420px;
  overflow-y: auto;
  margin-bottom: 10px;
}

.model-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  font-size: 12px;
}

.model-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.model-card-name {
  font-weight: 600;
  font-size: 13px;
  color: var(--jc-text-highlight);
}

.model-card-provider {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--jc-text-primary);
  font-weight: 500;
}

.model-card-model {
  font-family: monospace;
  font-size: 12px;
  color: #58a6ff;
  font-weight: 600;
}

.model-card-meta {
  display: flex;
  gap: 10px;
  font-size: 10px;
  color: var(--jc-text-secondary);
  margin: 0 12px;
  white-space: nowrap;
}

.model-card-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.model-btn {
  padding: 2px 8px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  font-size: 10px;
  cursor: pointer;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-primary);

  &.edit:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }

  &.del:hover {
    border-color: #f85149;
    color: #f85149;
  }
}

.vllm-model-area {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.vllm-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.vllm-count {
  font-size: 10.5px;
  color: var(--jc-text-secondary);
}

.vllm-refresh-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-secondary);
  font-size: 10.5px;
  cursor: pointer;
  white-space: nowrap;
  font-family: inherit;
  transition: all 0.15s;
  flex-shrink: 0;

  &:hover:not(:disabled) {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.vllm-loading {
  font-size: 11px;
  color: var(--jc-text-secondary);
  padding: 8px 4px;
}

.vllm-empty {
  font-size: 11px;
  color: var(--jc-text-secondary);
  padding: 8px 4px;
  opacity: 0.7;
  code {
    font-size: 10px;
    background: rgba(255,255,255,0.06);
    padding: 1px 4px;
    border-radius: 3px;
  }
}

.vllm-checklist {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  padding: 4px;

  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb { background: var(--jc-border-default); border-radius: 2px; }
}

.vllm-check-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: 3px;
  font-size: 11.5px;
  color: var(--jc-text-primary);
  cursor: pointer;
  transition: background 0.1s;
  font-family: monospace;

  &:hover { background: var(--jc-bg-hover); }
  &.checked { color: var(--jc-color-accent); }

  input[type="checkbox"] {
    accent-color: var(--jc-color-accent);
    margin: 0;
    flex-shrink: 0;
  }

  span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.spinning {
  animation: spin 1s linear infinite;
}

.add-model-btn {
  width: 100%;
  padding: 6px;
  border: 1px dashed var(--jc-border-default);
  border-radius: 6px;
  background: transparent;
  color: var(--jc-text-secondary);
  font-size: 12px;
  cursor: pointer;

  &:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }
}

.empty-hint {
  text-align: center;
  color: var(--jc-text-secondary);
  font-size: 12px;
  padding: 16px;
}

/* ── 模型表单弹窗 ── */
.model-form-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.model-form-card {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  border-radius: 8px;
  padding: 16px;
  width: 360px;
  max-height: 90%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4);

  h4 {
    margin: 0;
    font-size: 13px;
    color: var(--jc-text-primary);
  }
}

.model-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.pane-subtitle {
  font-size: 11px;
  font-weight: 600;
  color: #f0883e;
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--jc-border-default);
}

.form-row {
  display: flex;
  gap: 8px;
}

.form-half {
  flex: 1;
}

/* ── AI 角色管理样式 ── */
.roles-list-container {
  max-height: 420px;
  overflow-y: auto;
  margin-bottom: 10px;
  padding-right: 4px;

  &::-webkit-scrollbar {
    width: 4px;
  }
  &::-webkit-scrollbar-thumb {
    background: var(--jc-border-default);
    border-radius: 2px;
  }
}

.roles-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.role-settings-card {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 11px;
}

.role-card-top {
  display: flex;
  align-items: center;
  gap: 8px;
  position: relative;
}

.role-card-icon {
  font-size: 16px;
}

.role-card-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.role-card-name {
  font-weight: 600;
  color: var(--jc-text-primary);
  font-size: 12px;
}

.role-card-id {
  font-family: monospace;
  font-size: 9px;
  color: var(--jc-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100px;
}

.role-card-type {
  font-size: 9px;
  padding: 0px 4px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.06);
  color: var(--jc-text-secondary);
  position: absolute;
  right: 0;
  top: 50%;
  transform: translateY(-50%);

  &.custom {
    background: rgba(138, 88, 255, 0.15);
    color: var(--jc-color-accent);
  }
}

.role-card-desc {
  margin: 0;
  color: var(--jc-text-secondary);
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  min-height: 30px;
}

.role-card-actions {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
  margin-top: auto;
  border-top: 1px solid rgba(255, 255, 255, 0.03);
  padding-top: 4px;
}

.role-btn {
  padding: 2px 6px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  font-size: 9.5px;
  cursor: pointer;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-primary);

  &.edit:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }

  &.del:hover {
    border-color: #f85149;
    color: #f85149;
  }
}

.add-role-btn {
  width: 100%;
  padding: 6px;
  border: 1px dashed var(--jc-border-default);
  border-radius: 6px;
  background: transparent;
  color: var(--jc-text-secondary);
  font-size: 12px;
  cursor: pointer;

  &:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }
}

/* ── 角色配置表单弹窗 ── */
.role-form-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.role-form-card {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  border-radius: 8px;
  padding: 16px;
  width: 380px;
  max-height: 90%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4);

  h4 {
    margin: 0;
    font-size: 13px;
    color: var(--jc-text-primary);
  }
}

.role-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

/* ── MCP 服务器管理 ── */
.mcp-server-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
  margin-bottom: 10px;
}

.mcp-server-card {
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.02);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.mcp-server-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.mcp-server-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mcp-server-name {
  font-weight: 600;
  font-size: 13px;
  color: var(--jc-text-highlight);
}

.mcp-server-status {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 500;

  &.connected { background: rgba(63, 185, 80, 0.15); color: #3fb950; }
  &.connecting { background: rgba(210, 153, 34, 0.15); color: #d29922; }
  &.error { background: rgba(248, 81, 73, 0.15); color: #f85149; }
  &.disconnected { background: rgba(139, 148, 158, 0.15); color: #8b949e; }
}

.mcp-server-transport {
  font-size: 9px;
  padding: 1px 4px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.06);
  color: var(--jc-text-secondary);
  font-family: monospace;
}

.mcp-server-disconnect {
  background: none;
  border: 1px solid transparent;
  color: var(--jc-text-secondary);
  font-size: 11px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;

  &:hover {
    color: #f85149;
    border-color: #f85149;
  }
}

.mcp-server-url {
  font-size: 10px;
  font-family: monospace;
  color: var(--jc-text-secondary);
  word-break: break-all;
}

.mcp-server-error {
  font-size: 10px;
  color: #f85149;
  background: rgba(248, 81, 73, 0.08);
  padding: 4px 8px;
  border-radius: 4px;
}

.mcp-server-tools {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.mcp-tools-label {
  font-size: 10px;
  color: var(--jc-text-secondary);
  font-weight: 500;
}

.mcp-tools-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.mcp-tool-chip {
  font-size: 9px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(88, 166, 255, 0.1);
  color: #58a6ff;
  font-family: monospace;
  cursor: default;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  &:hover {
    background: rgba(88, 166, 255, 0.2);
  }
}

.mcp-tools-empty {
  font-size: 10px;
  color: var(--jc-text-secondary);
  opacity: 0.6;
}

.add-mcp-btn {
  width: 100%;
  padding: 6px;
  border: 1px dashed var(--jc-border-default);
  border-radius: 6px;
  background: transparent;
  color: var(--jc-text-secondary);
  font-size: 12px;
  cursor: pointer;

  &:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }
}

/* MCP 表单弹窗 */
.mcp-form-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.mcp-form-card {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  border-radius: 8px;
  padding: 16px;
  width: 360px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4);

  h4 {
    margin: 0;
    font-size: 13px;
    color: var(--jc-text-primary);
  }
}

.mcp-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.footer-btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* ── MCP 模式切换 ── */
.mcp-mode-toggle {
  display: flex;
  gap: 4px;
  margin-bottom: 8px;
}

.mcp-mode-btn {
  flex: 1;
  padding: 5px 10px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: transparent;
  color: var(--jc-text-secondary);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;

  &:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-text-primary);
  }

  &.active {
    background: rgba(88, 166, 255, 0.1);
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
    font-weight: 600;
  }
}

/* ── MCP JSON 编辑器 ── */
.mcp-json-editor {
  width: 100%;
  min-height: 260px;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-default);
  color: var(--jc-text-primary);
  font-family: monospace;
  font-size: 11px;
  padding: 10px;
  border-radius: 4px;
  resize: vertical;
  outline: none;
  line-height: 1.5;
  tab-size: 2;

  &:focus {
    border-color: var(--jc-color-accent);
  }

  &::placeholder {
    color: var(--jc-text-secondary);
    opacity: 0.4;
  }
}

.mcp-json-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.mcp-json-apply {
  flex: 1;
  padding: 6px 12px;
  background: var(--jc-color-accent);
  color: #fff;
  border: none;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;

  &:hover { opacity: 0.9; }
  &:disabled { opacity: 0.5; cursor: not-allowed; }
}

.mcp-json-clear {
  padding: 6px 12px;
  background: transparent;
  color: var(--jc-text-secondary);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;

  &:hover {
    color: #f85149;
    border-color: #f85149;
  }
}

/* ── 技能管理 ── */
.skills-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.skills-refresh-btn {
  padding: 5px 12px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-primary);
  font-size: 11px;
  cursor: pointer;
  transition: all 0.15s;

  &:hover:not(:disabled) {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }

  &:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
}

.skills-count {
  font-size: 11px;
  color: var(--jc-text-secondary);
}

.skills-error {
  font-size: 11px;
  color: #f85149;
  background: rgba(248, 81, 73, 0.08);
  padding: 6px 10px;
  border-radius: 4px;
  margin-bottom: 8px;
}

.skills-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 380px;
  overflow-y: auto;
}

.skill-card {
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  padding: 5px 6px;
  background: rgba(255, 255, 255, 0.02);
  display: flex;
  flex-direction: column;
  gap: 4px;
  transition: border-color 0.15s;

  &:hover {
    border-color: var(--jc-border-strong);
  }
}

.skill-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.skill-card-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.skill-card-name {
  font-weight: 600;
  font-size: 13px;
  color: var(--jc-text-highlight);
}

.skill-card-id {
  font-family: monospace;
  font-size: 9px;
  color: var(--jc-text-secondary);
  margin-top: 1px;
}

.skill-card-badges {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.skill-card-status {
  font-size: 9px;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 500;
  background: rgba(139, 148, 158, 0.1);
  color: var(--jc-text-secondary);
  flex-shrink: 0;

  &.enabled {
    background: rgba(63, 185, 80, 0.12);
    color: #3fb950;
  }
}

.skill-card-source {
  font-size: 9px;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 500;
  flex-shrink: 0;

  &.system {
    background: rgba(88, 166, 255, 0.1);
    color: #58a6ff;
  }

  &.project {
    background: rgba(210, 153, 34, 0.12);
    color: #d29922;
  }
}

.skill-card-desc {
  margin: 0;
  font-size: 11px;
  color: var(--jc-text-secondary);
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
}

.skill-card-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 10px;
}

.skill-card-size {
  color: var(--jc-text-secondary);
  font-family: monospace;
}

.skill-card-folder {
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  font-size: 10px;
  font-family: monospace;
  cursor: pointer;
  text-align: left;
  padding: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 320px;

  &:hover {
    color: var(--jc-color-accent);
    text-decoration: underline;
  }
}
</style>
