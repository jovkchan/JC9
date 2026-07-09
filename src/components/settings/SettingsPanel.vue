<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import { save, open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { loadAllRoles, saveAllRoles, type AgentRole } from '@/config/roles'

// ── 页面加载时从 JSON 同步 AI 配置（跨 dev/build 共享）──
onMounted(async () => {
  try {
    const json = await invoke<string>('get_ai_config')
    const cfg = JSON.parse(json)
    for (const [k, v] of Object.entries(cfg)) {
      if (typeof v === 'string' && v) localStorage.setItem(k, v)
    }
  } catch { /* JSON 不存在或读取失败，忽略 */ }
  // 加载 MCP Server 配置
  await loadMcpServerConfig()
  await loadMcpServerStatus()
})

const win = getCurrentWindow()
const notesStore = useNotesStore()
const status = useStatusStore()
const aiStore = useAiStore()

const maximized = ref(false)
const saveFeedback = ref('')

async function doMinimize() { try { await win.minimize() } catch {} }
async function doMaximize() {
  maximized.value = !maximized.value
  try { await win.toggleMaximize() } catch {}
}
async function doClose() { try { await win.close() } catch {} }

// ── Theme sync ──
// ── Tab ──
const activeTab = ref<'general' | 'ai' | 'ai-roles' | 'backup' | 'skills' | 'command' | 'hook' | 'plugin' | 'mcp'>('general')

// ── General settings ──
const defaultFormat = ref<'markdown' | 'plain'>('markdown')
const defaultVisibility = ref<'PRIVATE' | 'PUBLIC'>('PRIVATE')

// ── AI 模型配置 ──
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
  reasoningEffort: 'high' | 'max' | ''
}

const modelConfigs = ref<ModelConfig[]>([])
const showModelForm = ref(false)
const vllmModels = ref<string[]>([])
const loadingModels = ref(false)

function blankModel(): ModelConfig {
  return {
    id: '', name: '', provider: 'deepseek',
    endpoint: 'https://api.deepseek.com', apiKey: '',
    model: 'deepseek-v4-pro', inputPrice: 2.0, outputPrice: 4.0,
    costLimit: 10.0, reasoningEffort: 'high',
  }
}
const newModelForm = ref<ModelConfig>(blankModel())

// ── AI 角色 ──
const rolesList = ref<AgentRole[]>([])
const showRoleForm = ref(false)

function blankRole(): AgentRole {
  return { id: '', name: '', icon: '🤖', description: '', systemPrompt: '', isCustom: true }
}
const newRoleForm = ref<AgentRole>(blankRole())

function addRole() { newRoleForm.value = blankRole(); showRoleForm.value = true }
function editRole(role: AgentRole) { newRoleForm.value = { ...role }; showRoleForm.value = true }
function deleteRole(id: string) { rolesList.value = rolesList.value.filter(r => r.id !== id) }

function saveRoleForm() {
  const r = newRoleForm.value
  if (!r.name.trim() || !r.systemPrompt.trim()) { status.pushMessage('请填写角色名称和专属提示词', 'warn'); return }
  if (!r.id) r.id = 'custom_' + Date.now().toString()
  const idx = rolesList.value.findIndex(role => role.id === r.id)
  if (idx >= 0) rolesList.value[idx] = { ...r }
  else rolesList.value.push({ ...r })
  showRoleForm.value = false
}
function cancelRoleForm() { showRoleForm.value = false }

// ── Model form ──
function addModel() { newModelForm.value = blankModel(); showModelForm.value = true }
function editModel(config: ModelConfig) { newModelForm.value = { ...config }; showModelForm.value = true }
function deleteModel(id: string) { modelConfigs.value = modelConfigs.value.filter(c => c.id !== id) }

function saveModelForm() {
  const f = newModelForm.value
  if (!f.name.trim() || !f.model.trim()) { status.pushMessage('请填写模型名称和代号', 'warn'); return }
  if (!f.id) f.id = Date.now().toString()
  const idx = modelConfigs.value.findIndex(c => c.id === f.id)
  if (idx >= 0) modelConfigs.value[idx] = { ...f }
  else modelConfigs.value.push({ ...f })
  showModelForm.value = false
}
function cancelModelForm() { showModelForm.value = false }

function setProviderDefaults() {
  const f = newModelForm.value
  if (f.provider === 'ollama') { f.endpoint = 'http://127.0.0.1:11434'; f.model = 'llama3' }
  else if (f.provider === 'deepseek') { f.endpoint = 'https://api.deepseek.com'; f.model = 'deepseek-v4-pro' }
  else if (f.provider === 'openai') { f.endpoint = 'https://api.openai.com/v1'; f.model = 'gpt-4o-mini' }
  else if (f.provider === 'gemini') { f.endpoint = 'https://generativelanguage.googleapis.com'; f.model = 'gemini-1.5-flash' }
  else if (f.provider === 'vllm') { f.endpoint = 'http://192.168.5.100:8000/v1'; f.model = ''; fetchVllmModelsForm() }
}

const vllmSelectedModels = ref<string[]>([])
async function fetchVllmModelsForm() {
  loadingModels.value = true; vllmModels.value = []; vllmSelectedModels.value = []
  try {
    const url = `${newModelForm.value.endpoint.replace(/\/+$/, '')}/models`
    const res = await fetch(url)
    if (res.ok) {
      const json = await res.json()
      if (json.data && Array.isArray(json.data)) vllmModels.value = json.data.map((m: any) => m.id)
    }
  } catch { /* ignore */ }
  finally { loadingModels.value = false }
}
function onEndpointBlur() {
  if (newModelForm.value.provider === 'vllm' && newModelForm.value.endpoint.trim()) fetchVllmModelsForm()
}
function toggleVllmModel(model: string) {
  const idx = vllmSelectedModels.value.indexOf(model)
  if (idx >= 0) vllmSelectedModels.value.splice(idx, 1)
  else vllmSelectedModels.value.push(model)
  newModelForm.value.model = vllmSelectedModels.value.join(',')
}

// ── MCP ──
const showMcpForm = ref(false)
const connecting = ref(false)
const mcpViewMode = ref<'list' | 'json'>('list')
const mcpJsonConfig = ref('')
const applyingJson = ref(false)
const mcpJsonError = ref('')
const mcpForm = ref({ transport: 'sse' as 'sse' | 'stdio', name: '', url: '', command: '', argsText: '' })

function mcpStatusLabel(s: string): string {
  const map: Record<string, string> = { connected: '已连接', disconnected: '已断开', connecting: '连接中', error: '错误' }
  return map[s] || s
}

async function saveMcpForm() {
  const f = mcpForm.value
  if (!f.name.trim()) { status.pushMessage('请填写服务器名称', 'warn'); return }
  connecting.value = true
  try {
    if (f.transport === 'sse') {
      if (!f.url.trim()) { status.pushMessage('请填写 SSE URL', 'warn'); return }
      await aiStore.connectMcpServer(f.name.trim(), f.url.trim())
    } else {
      if (!f.command.trim()) { status.pushMessage('请填写启动命令', 'warn'); return }
      await aiStore.connectMcpServerStdio(f.name.trim(), f.command.trim(), f.argsText.split(',').map(a => a.trim()).filter(Boolean))
    }
    showMcpForm.value = false
    status.pushMessage(`已连接 MCP 服务器: ${f.name}`, 'success')
  } catch (e) { status.pushMessage(`连接失败: ${e}`, 'error') }
  finally { connecting.value = false }
}

async function disconnectMcp(name: string) {
  try { await aiStore.disconnectMcpServer(name); status.pushMessage(`已断开: ${name}`, 'success') }
  catch (e) { status.pushMessage(`断开失败: ${e}`, 'error') }
}

function switchToJsonMode() {
  const config: Record<string, any> = { mcpServers: {} }
  for (const srv of aiStore.mcpServers) {
    if (srv.transport === 'stdio') config.mcpServers[srv.name] = { command: srv.command, args: srv.args }
    else config.mcpServers[srv.name] = { url: srv.url }
  }
  mcpJsonConfig.value = JSON.stringify(config, null, 2)
  mcpJsonError.value = ''
  mcpViewMode.value = 'json'
}

function validateAndParseMcpJson(text: string): { mcpServers: Record<string, any> } | null {
  try {
    const parsed = JSON.parse(text)
    if (!parsed.mcpServers || typeof parsed.mcpServers !== 'object') throw new Error('缺少 "mcpServers" 顶层字段')
    for (const [name, cfg] of Object.entries(parsed.mcpServers)) {
      const c = cfg as any
      if (!c.url && !c.command) throw new Error(`服务器 "${name}" 需要提供 "url"（SSE）或 "command"（Stdio）`)
    }
    return parsed
  } catch (e: any) { mcpJsonError.value = `JSON 解析错误: ${e.message}`; return null }
}

async function applyMcpJson() {
  applyingJson.value = true; mcpJsonError.value = ''
  try {
    for (const srv of aiStore.mcpServers) await aiStore.disconnectMcpServer(srv.name)
    await new Promise(r => setTimeout(r, 300))
    const config = validateAndParseMcpJson(mcpJsonConfig.value)
    if (!config) { applyingJson.value = false; return }
    let connected = 0, failed = 0
    for (const [name, cfg] of Object.entries(config.mcpServers)) {
      try {
        if (cfg.url) await aiStore.connectMcpServer(name, cfg.url)
        else if (cfg.command) await aiStore.connectMcpServerStdio(name, cfg.command, cfg.args || [])
        connected++
      } catch { failed++ }
    }
    await aiStore.loadMcpServers()
    status.pushMessage(`JSON 配置应用完成: ${connected} 成功, ${failed} 失败`, failed > 0 ? 'warn' : 'success')
  } catch (e: any) { mcpJsonError.value = `应用配置失败: ${e.message}` }
  finally { applyingJson.value = false }
}

async function disconnectAllMcp() {
  for (const srv of aiStore.mcpServers) await aiStore.disconnectMcpServer(srv.name)
  await aiStore.loadMcpServers()
  status.pushMessage('已断开所有 MCP 服务器', 'success')
}

// ── JC9 内置 MCP Server ──
const mcpServerRunning = ref(false)
const mcpServerEnabled = ref(false)
const mcpServerKey = ref('')
const mcpServerUrl = ref('')
const showMcpKey = ref(false)
const mcpPortInput = ref('19799')
const mcpServerMsg = ref('')
const mcpLoading = ref(false)



async function loadMcpServerStatus() {
  try {
    const status = await invoke<{ running: boolean; enabled: boolean; port: number; host: string }>('ai_get_mcp_server_status')
    mcpServerRunning.value = status.running
    mcpServerEnabled.value = status.enabled
    mcpPortInput.value = status.port.toString()
    mcpServerUrl.value = `http://${status.host}:${status.port}`
  } catch { /* ignore */ }
}

interface McpServerConfigType {
  enabled: boolean; port: number; apiKey: string; host: string; groupIds: string[]
}

const mcpSelectedGroups = ref<string[]>([])
const mcpNoteGroups = ref<Array<{ id: string; name: string }>>([])

function toggleMcpGroup(gid: string) {
  const idx = mcpSelectedGroups.value.indexOf(gid)
  if (idx >= 0) mcpSelectedGroups.value.splice(idx, 1)
  else mcpSelectedGroups.value.push(gid)
}

async function loadMcpServerConfig() {
  try {
    const config = await invoke<McpServerConfigType>('ai_get_mcp_server_config')
    mcpServerKey.value = config.apiKey
    mcpPortInput.value = config.port.toString()
    mcpServerUrl.value = `http://${config.host}:${config.port}`
    mcpServerEnabled.value = config.enabled
    mcpSelectedGroups.value = config.groupIds || []
  } catch { /* ignore */ }
  try {
    mcpNoteGroups.value = await invoke<Array<{ id: string; name: string }>>('get_note_groups')
  } catch { /* ignore */ }
}

async function startMcpServer() {
  mcpLoading.value = true; mcpServerMsg.value = ''
  try {
    const config = await invoke<McpServerConfigType>('ai_get_mcp_server_config')
    config.enabled = true
    config.port = parseInt(mcpPortInput.value) || 19799
    config.groupIds = mcpSelectedGroups.value
    const resultMsg = await invoke<string>('ai_set_mcp_server_config', { config })
    await loadMcpServerStatus()
    await loadMcpServerConfig()
    mcpServerMsg.value = resultMsg
  } catch (e) { mcpServerMsg.value = `❌ ${e}` }
  finally { mcpLoading.value = false }
}

async function stopMcpServer() {
  mcpLoading.value = true; mcpServerMsg.value = ''
  try {
    const msg = await invoke<string>('ai_stop_mcp_server')
    await loadMcpServerStatus()
    mcpServerMsg.value = msg
  } catch (e) { mcpServerMsg.value = `❌ ${e}` }
  finally { mcpLoading.value = false }
}

async function regenerateMcpKey() {
  mcpLoading.value = true; mcpServerMsg.value = ''
  try {
    const newKey = crypto.randomUUID()
    const config = await invoke<McpServerConfigType>('ai_get_mcp_server_config')
    config.apiKey = newKey
    await invoke<string>('ai_set_mcp_server_config', { config })
    await loadMcpServerConfig()
    mcpServerMsg.value = '✅ Key 已重新生成'
  } catch (e) { mcpServerMsg.value = `❌ ${e}` }
  finally { mcpLoading.value = false }
}

function copyMcpUrl() {
  navigator.clipboard.writeText(mcpServerUrl.value + '/sse')
  status.pushMessage('MCP Server 地址已复制', 'success')
}

function copyMcpKeyToClipboard() {
  navigator.clipboard.writeText(mcpServerKey.value)
  status.pushMessage('API Key 已复制', 'success')
}

// ── Skills ──
interface SystemSkill { id: string; name: string; version: string; description: string; path: string; file_size: number; enabled: boolean; source: string }
const systemSkills = ref<SystemSkill[]>([])
const skillsLoading = ref(false)
const skillsError = ref('')
const skillsSearch = ref('')

const filteredSkills = computed(() => {
  const q = skillsSearch.value.trim().toLowerCase()
  if (!q) return systemSkills.value
  return systemSkills.value.filter(s => s.name.toLowerCase().includes(q) || s.id.toLowerCase().includes(q) || s.description.toLowerCase().includes(q))
})

async function loadSystemSkills() {
  skillsLoading.value = true; skillsError.value = ''
  try {
    if (!aiStore.workspaceRoot) await aiStore.loadWorkspaceRoot()
    const root = aiStore.workspaceRoot || ''
    systemSkills.value = await invoke<SystemSkill[]>('list_system_skills', { workspaceRoot: root })
  } catch (e) { skillsError.value = `加载失败: ${e}` }
  finally { skillsLoading.value = false }
}

function formatSkillSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
function openSkillFolder(path: string) { invoke('show_in_folder', { path }) }

// ── Backup ──
async function exportData() {
  try {
    const filePath = await save({ filters: [{ name: 'JSON Backup', extensions: ['json'] }], defaultPath: 'jc9-memos-backup.json' })
    if (filePath) {
      const dataStr = JSON.stringify({ notes: notesStore.notes, groups: notesStore.groups }, null, 2)
      const binaryData = Array.from(new TextEncoder().encode(dataStr))
      await invoke('write_file_binary', { path: filePath, data: binaryData })
      status.pushMessage('备份文件导出成功！', 'success')
    }
  } catch (e) { status.pushMessage(`导出失败: ${e}`, 'error') }
}

async function importData() {
  try {
    const selected = await open({ filters: [{ name: 'JSON Backup', extensions: ['json'] }], multiple: false })
    if (selected && typeof selected === 'string') {
      const content = await invoke<string>('read_file_string', { path: selected })
      const data = JSON.parse(content)
      if (!data.notes || !Array.isArray(data.notes)) { status.pushMessage('无效的备份文件结构', 'error'); return }
      for (const note of data.notes) await notesStore.saveNote(note)
      if (data.groups && Array.isArray(data.groups)) for (const g of data.groups) await notesStore.updateGroup(g)
      await notesStore.loadAllNotes(); await notesStore.loadGroups()
      status.pushMessage('备份数据导入恢复成功！', 'success')
    }
  } catch (e) { status.pushMessage(`导入失败: ${e}`, 'error') }
}

// ── Save ──
async function saveAllToJson() {
  // 收集所有 AI 相关的 localStorage 键写入 JSON（跨 dev/build 共享）
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
  localStorage.setItem('notes-ai-models', JSON.stringify(modelConfigs.value))
  saveAllRoles(rolesList.value)
  const first = modelConfigs.value[0]
  if (first) {
    localStorage.setItem('notes-ai-provider', first.provider)
    localStorage.setItem('notes-ai-endpoint', first.endpoint)
    localStorage.setItem('notes-ai-apikey', first.apiKey)
    localStorage.setItem('notes-ai-model', first.model)
    localStorage.setItem(`notes-ai-endpoint-${first.provider}`, first.endpoint)
    localStorage.setItem(`notes-ai-apikey-${first.provider}`, first.apiKey)
    localStorage.setItem(`notes-ai-model-${first.provider}`, first.model)
    aiStore.updateCostConfig({
      inputCachedCostPerM: first.inputPrice * 0.008,
      inputUncachedCostPerM: first.inputPrice,
      outputCostPerM: first.outputPrice,
      costLimit: first.costLimit,
    })
  }
  await saveAllToJson()
  status.pushMessage('设置保存成功', 'success')
  // 行内反馈
  saveFeedback.value = '✅ 已保存'
  setTimeout(() => { saveFeedback.value = '' }, 2000)
}

// ── Overlay click guard ──
let formMousedownTarget: EventTarget | null = null
function handleFormMousedown(e: MouseEvent) { formMousedownTarget = e.target }
function handleModelOverlayClick(e: MouseEvent) { if (e.target === e.currentTarget && formMousedownTarget === e.currentTarget) cancelModelForm() }
function handleRoleOverlayClick(e: MouseEvent) { if (e.target === e.currentTarget && formMousedownTarget === e.currentTarget) cancelRoleForm() }
function handleMcpOverlayClick(e: MouseEvent) { if (e.target === e.currentTarget && formMousedownTarget === e.currentTarget) showMcpForm.value = false }

onMounted(() => {
  defaultFormat.value = (localStorage.getItem('notes-default-format') as any) || 'markdown'
  defaultVisibility.value = (localStorage.getItem('notes-default-visibility') as any) || 'PRIVATE'
  const saved = localStorage.getItem('notes-ai-models')
  if (saved) { try { modelConfigs.value = JSON.parse(saved) } catch {} }
  if (modelConfigs.value.length === 0) {
    modelConfigs.value = [{
      id: 'legacy', name: '默认配置',
      provider: (localStorage.getItem('notes-ai-provider') as any) || 'deepseek',
      endpoint: localStorage.getItem('notes-ai-endpoint') || 'https://api.deepseek.com',
      apiKey: localStorage.getItem('notes-ai-apikey') || '',
      model: localStorage.getItem('notes-ai-model') || 'deepseek-v4-pro',
      inputPrice: parseFloat(localStorage.getItem('notes-ai-input-price') || '3.0'),
      outputPrice: parseFloat(localStorage.getItem('notes-ai-output-price') || '6.0'),
      costLimit: parseFloat(localStorage.getItem('notes-ai-cost-limit') || '5.0'),
      reasoningEffort: 'high',
    }]
  }
  rolesList.value = loadAllRoles()
  aiStore.loadMcpServers()

  // Sync theme from main window
  const savedTheme = localStorage.getItem('jc9-theme')
  if (savedTheme === 'light' || savedTheme === 'dark') document.documentElement.setAttribute('data-theme', savedTheme)
  window.setInterval(() => {
    const saved = localStorage.getItem('jc9-theme')
    if (saved) {
      const current = document.documentElement.getAttribute('data-theme')
      if (current !== saved) document.documentElement.setAttribute('data-theme', saved)
    }
  }, 2000)
})
</script>

<template>
  <div class="settings-window">
    <!-- macOS Titlebar -->
    <div class="settings-titlebar" data-tauri-drag-region>
      <div class="stb-spacer"></div>
      <div class="stb-controls">
        <button class="stb-btn" @click="doMinimize" title="最小化">
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 13h10"/></svg>
        </button>
        <button class="stb-btn" @click="doMaximize" title="最大化">
          <svg v-if="!maximized" viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/></svg>
          <svg v-else viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3.5" y="5.5" width="7" height="7" rx="1"/><path d="M5.5 5.5V3.5h7v7h-2"/></svg>
        </button>
        <button class="stb-btn stb-close" @click="doClose" title="关闭">
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
        </button>
      </div>
    </div>

    <!-- Header -->
    <div class="settings-header">
      <span class="settings-title">设置</span>
      <span class="settings-subtitle">JC9 系统配置</span>
    </div>

    <!-- Body: sidebar + content -->
    <div class="settings-body">
      <aside class="settings-nav">
        <div :class="['nav-item', { active: activeTab === 'general' }]" @click="activeTab = 'general'">通用设置</div>
        <div :class="['nav-item', { active: activeTab === 'ai' }]" @click="activeTab = 'ai'">模型配置</div>
        <div :class="['nav-item', { active: activeTab === 'ai-roles' }]" @click="activeTab = 'ai-roles'">智能体</div>
        <div :class="['nav-item', { active: activeTab === 'skills' }]" @click="activeTab = 'skills'; loadSystemSkills()">技能</div>
        <div :class="['nav-item', { active: activeTab === 'command' }]" @click="activeTab = 'command'">指令</div>
        <div :class="['nav-item', { active: activeTab === 'hook' }]" @click="activeTab = 'hook'">钩子</div>
        <div :class="['nav-item', { active: activeTab === 'plugin' }]" @click="activeTab = 'plugin'">插件</div>
        <div :class="['nav-item', { active: activeTab === 'mcp' }]" @click="activeTab = 'mcp'">MCP</div>
        <div :class="['nav-item', { active: activeTab === 'backup' }]" @click="activeTab = 'backup'">数据备份</div>
      </aside>

      <main class="settings-content">
        <!-- General -->
        <div v-if="activeTab === 'general'" class="settings-pane">
          <h3 class="pane-title">偏好设置</h3>
          <div class="form-group">
            <label>默认笔记格式</label>
            <select v-model="defaultFormat" class="form-select">
              <option value="markdown">Markdown (推荐)</option>
              <option value="plain">纯文本</option>
            </select>
            <span class="help-text">新建备忘时的默认输入解析格式</span>
          </div>
          <div class="form-group">
            <label>新建笔记默认可见性</label>
            <select v-model="defaultVisibility" class="form-select">
              <option value="PRIVATE">PRIVATE (私有本地)</option>
              <option value="PUBLIC">PUBLIC (公开)</option>
            </select>
            <span class="help-text">第一期完全本地化下默认均为 PRIVATE 级别</span>
          </div>
        </div>

        <!-- AI Model Config -->
        <div v-if="activeTab === 'ai'" class="settings-pane">
          <h3 class="pane-title">AI 模型配置</h3>
          <p class="pane-desc" style="margin-bottom:12px">管理您接入的大模型供应商，每个模型独立配置计费与熔断限额。</p>
          <div class="model-list">
            <div v-for="cfg in modelConfigs" :key="cfg.id" class="model-card">
              <div class="model-card-header">
                <span class="model-card-name">{{ cfg.name }}</span>
                <span class="model-card-provider">{{ cfg.provider }}</span>
                <span class="model-card-model">{{ cfg.model }}</span>
              </div>
              <div class="model-card-actions">
                <button class="btn-sm" @click="editModel(cfg)">编辑</button>
                <button class="btn-sm btn-danger" @click="deleteModel(cfg.id)">删除</button>
              </div>
            </div>
            <div v-if="modelConfigs.length === 0" class="empty-hint">尚未添加任何模型</div>
          </div>
          <button class="add-btn" @click="addModel">+ 添加模型</button>

          <!-- Model form overlay -->
          <div v-if="showModelForm" class="form-overlay" @mousedown="handleFormMousedown" @click="handleModelOverlayClick">
            <div class="form-card">
              <h4>{{ newModelForm.id ? '编辑' : '添加' }}模型配置</h4>
              <div class="form-group"><label>配置名称</label><input v-model="newModelForm.name" class="form-input" placeholder="例如：DeepSeek 主力" /></div>
              <div class="form-group">
                <label>供应商</label>
                <select v-model="newModelForm.provider" @change="setProviderDefaults" class="form-select">
                  <option value="deepseek">DeepSeek</option>
                  <option value="openai">OpenAI</option>
                  <option value="ollama">Ollama (本地)</option>
                  <option value="gemini">Google Gemini</option>
                  <option value="vllm">vLLM (自部署)</option>
                </select>
              </div>
              <div class="form-group"><label>Endpoint</label><input v-model="newModelForm.endpoint" class="form-input" @blur="onEndpointBlur" /></div>
              <div class="form-group" v-if="newModelForm.provider !== 'ollama' && newModelForm.provider !== 'vllm'"><label>API Key</label><input v-model="newModelForm.apiKey" type="password" class="form-input" placeholder="sk-..." /></div>
              <div class="form-group">
                <label>Model</label>
                <div v-if="newModelForm.provider === 'vllm'" class="vllm-model-area">
                  <div class="vllm-toolbar">
                    <span class="vllm-count" v-if="!loadingModels">{{ vllmModels.length }} 个模型</span>
                    <button class="vllm-refresh-btn" :disabled="loadingModels" @click="fetchVllmModelsForm">🔄 刷新</button>
                  </div>
                  <div v-if="loadingModels" class="vllm-loading">获取中...</div>
                  <div v-else-if="vllmModels.length === 0" class="vllm-empty">点击刷新从 /models 获取</div>
                  <div v-else class="vllm-checklist">
                    <label v-for="m in vllmModels" :key="m" class="vllm-check-item" :class="{ checked: vllmSelectedModels.includes(m) }">
                      <input type="checkbox" :checked="vllmSelectedModels.includes(m)" @change="toggleVllmModel(m)" /> <span>{{ m }}</span>
                    </label>
                  </div>
                </div>
                <input v-else v-model="newModelForm.model" class="form-input" placeholder="多个用英文逗号分隔" />
              </div>
              <div class="form-row">
                <div class="form-group form-half"><label>输入价格 (元/百万)</label><input v-model.number="newModelForm.inputPrice" type="number" step="0.1" class="form-input" /></div>
                <div class="form-group form-half"><label>输出价格 (元/百万)</label><input v-model.number="newModelForm.outputPrice" type="number" step="0.1" class="form-input" /></div>
              </div>
              <div class="form-group"><label>熔断限额 (元)</label><input v-model.number="newModelForm.costLimit" type="number" step="0.5" class="form-input" /></div>
              <div class="form-actions"><button class="footer-btn-cancel" @click="cancelModelForm">取消</button><button class="footer-btn-save" @click="saveModelForm">确定</button></div>
            </div>
          </div>
        </div>

        <!-- AI Roles -->
        <div v-if="activeTab === 'ai-roles'" class="settings-pane">
          <h3 class="pane-title">AI 角色管理</h3>
          <p class="pane-desc" style="margin-bottom:12px">配置 AI 角色。内置预设不可删除，可编辑提示词；也可添加自定义角色。</p>
          <div class="roles-grid">
            <div v-for="role in rolesList" :key="role.id" class="role-settings-card">
              <div class="role-card-top">
                <div class="role-card-info">
                  <span class="role-card-name">{{ role.name }}</span>
                  <span class="role-card-id">{{ role.id }}</span>
                </div>
                <span class="role-card-type" :class="{ custom: role.isCustom }">{{ role.isCustom ? '自定义' : '预置' }}</span>
              </div>
              <p class="role-card-desc">{{ role.description || '无介绍' }}</p>
              <div class="role-card-actions">
                <button class="btn-sm" @click="editRole(role)">编辑</button>
                <button v-if="role.isCustom" class="btn-sm btn-danger" @click="deleteRole(role.id)">删除</button>
              </div>
            </div>
          </div>
          <button class="add-btn" @click="addRole">+ 添加角色</button>

          <div v-if="showRoleForm" class="form-overlay" @mousedown="handleFormMousedown" @click="handleRoleOverlayClick">
            <div class="form-card">
              <h4>{{ newRoleForm.id ? '编辑' : '添加' }} AI 角色</h4>
              <div class="form-group"><label>角色名称</label><input v-model="newRoleForm.name" class="form-input" placeholder="例如：测试工程师" /></div>
              <div class="form-group"><label>角色介绍</label><input v-model="newRoleForm.description" class="form-input" placeholder="简述该角色的核心职责" /></div>
              <div class="form-group"><label>专属系统提示词</label><textarea v-model="newRoleForm.systemPrompt" class="form-textarea" rows="6"></textarea></div>
              <div class="form-actions"><button class="footer-btn-cancel" @click="cancelRoleForm">取消</button><button class="footer-btn-save" @click="saveRoleForm">确定</button></div>
            </div>
          </div>
        </div>

        <!-- Skills -->
        <div v-if="activeTab === 'skills'" class="settings-pane">
          <div class="skills-toolbar">
            <input v-model="skillsSearch" class="skills-search-input" placeholder="搜索技能..." />
            <button class="skills-refresh-btn" :disabled="skillsLoading" @click="loadSystemSkills">{{ skillsLoading ? '加载中...' : '刷新' }}</button>
            <span class="skills-count" v-if="!skillsLoading">{{ filteredSkills.length }}/{{ systemSkills.length }} 个</span>
          </div>
          <div v-if="skillsError" class="error-banner">{{ skillsError }}</div>
          <div class="skills-list">
            <div v-for="skill in filteredSkills" :key="skill.id" class="skill-card">
              <div class="skill-card-header">
                <span class="skill-card-name">{{ skill.name }} v{{ skill.version || '0.0.0' }}</span>
                <div class="skill-card-badges">
                  <span class="skill-card-status" :class="{ enabled: skill.enabled }">{{ skill.enabled ? '已启用' : '已禁用' }}</span>
                  <span class="skill-card-source" :class="skill.source">{{ skill.source === 'system' ? '全局' : '项目' }}</span>
                </div>
              </div>
              <p class="skill-card-desc" v-if="skill.description">{{ skill.description }}</p>
              <div class="skill-card-meta">
                <span>{{ formatSkillSize(skill.file_size) }}</span>
                <button class="link-btn" @click="openSkillFolder(skill.path)">📂 {{ skill.path }}</button>
              </div>
            </div>
            <div v-if="!skillsLoading && systemSkills.length === 0 && !skillsError" class="empty-hint">未发现技能文件</div>
          </div>
        </div>

        <!-- Command / Hook / Plugin (placeholders) -->
        <div v-if="activeTab === 'command'" class="settings-pane"><p class="pane-desc">设置始终生效的指令，在整个工作区或用户配置文件中引导 AI 行为。</p></div>
        <div v-if="activeTab === 'hook'" class="settings-pane"><p class="pane-desc">配置由保存文件或运行任务等事件触发的自动操作。</p></div>
        <div v-if="activeTab === 'plugin'" class="settings-pane"><p class="pane-desc">安装和管理智能体插件，以添加更多工具、技能和集成。</p></div>

        <!-- MCP -->
        <div v-if="activeTab === 'mcp'" class="settings-pane">
          <h3 class="pane-title">MCP 服务器管理</h3>
          <p class="pane-desc" style="margin-bottom:12px">管理 JC9 内置 MCP Server 和连接的外部 MCP 服务器。</p>

          <!-- JC9 内置 MCP Server -->
          <div class="builtin-mcp-section">
            <h4 style="font-size:12px;font-weight:600;color:var(--jc-text-highlight);margin:0 0 6px">🧠 JC9 MCP Server</h4>
            <p class="pane-desc" style="margin-bottom:6px">让其他 AI Agent（如 Cline、Copilot）通过 MCP 协议连接 JC9，<br/>搜索/创建/更新笔记（基于 sqlite-vec 向量语义搜索）。启动后在目标 Agent 中配置：</p>
            <div class="mcp-server-config-card">
              <div class="mcp-config-row">
                <span class="mcp-config-label">状态</span>
                <span :class="['mcp-status-badge', { running: mcpServerRunning }]">
                  {{ mcpServerRunning ? '🟢 运行中' : (mcpServerEnabled ? '🟡 已启用' : '🔴 已停止') }}
                </span>
              </div>
              <div class="mcp-config-row">
                <span class="mcp-config-label">地址</span>
                <code class="mcp-config-value">{{ mcpServerUrl }}</code>
                <button class="mcp-copy-btn" @click="copyMcpUrl">复制</button>
              </div>
              <div class="mcp-config-row">
                <span class="mcp-config-label">API Key</span>
                <code class="mcp-config-value mcp-key-text">{{ showMcpKey ? mcpServerKey : '••••••••' }}</code>
                <button class="mcp-copy-btn" @click="showMcpKey = !showMcpKey">{{ showMcpKey ? '隐藏' : '显示' }}</button>
                <button class="mcp-copy-btn" @click="copyMcpKeyToClipboard">复制</button>
              </div>
              <div class="mcp-config-row">
                <span class="mcp-config-label">端口</span>
                <div style="display:flex;align-items:center;gap:4px">
                  <input v-model="mcpPortInput" class="mcp-port-input" type="number" min="1024" max="65535" @change="mcpPortInput = Math.max(1024, Math.min(65535, Number(mcpPortInput) || 19799)).toString()" />
                  <span class="mcp-port-hint">1024-65535</span>
                </div>
              </div>
              <div class="mcp-config-row">
                <span class="mcp-config-label">白名单</span>
                <div class="mcp-group-chips">
                  <span v-for="g in mcpNoteGroups" :key="g.id"
                    :class="['mcp-chip', { active: mcpSelectedGroups.includes(g.id) }]"
                    @click="toggleMcpGroup(g.id)">
                    {{ g.name }}
                  </span>
                </div>
              </div>
              <div class="mcp-config-row" style="font-size:10px;color:var(--jc-text-secondary)">
                {{ mcpSelectedGroups.length === 0 ? '空=访问所有分组' : '已选 ' + mcpSelectedGroups.length + ' 个分组' }}
              </div>
              <div class="mcp-config-actions">
                <button v-if="!mcpServerRunning" class="mcp-start-btn" @click="startMcpServer" :disabled="mcpLoading">启动</button>
                <button v-if="mcpServerRunning" class="mcp-stop-btn" @click="stopMcpServer" :disabled="mcpLoading">停止</button>
                <button class="mcp-copy-btn" @click="regenerateMcpKey" :disabled="mcpLoading">重新生成 Key</button>
                <span v-if="mcpLoading" style="font-size:11px;color:var(--jc-text-secondary)">处理中...</span>
              </div>
              <div v-if="mcpServerMsg" :class="['mcp-server-msg', { success: mcpServerMsg.startsWith('✅'), error: mcpServerMsg.startsWith('❌') }]">{{ mcpServerMsg }}</div>
              <details style="margin-top:6px;font-size:11px">
                <summary style="cursor:pointer;color:var(--jc-text-secondary)">📖 在其他 AI Agent 中配置（点击展开）</summary>
                <div style="margin-top:6px;background:var(--jc-bg-elevated);padding:8px;border-radius:4px;font-size:10px;line-height:1.6">
                  <p><b>Cline / Claude Desktop 配置：</b></p>
                  <pre style="background:var(--jc-bg-input);padding:6px;border-radius:3px;overflow-x:auto;white-space:pre-wrap">{
  "mcpServers": {
    "jc9": {
      "url": "{{ mcpServerUrl }}/sse",
      "headers": {
        "Authorization": "Bearer {{ showMcpKey ? mcpServerKey : 'YOUR_API_KEY' }}"
      }
    }
  }
}</pre>
                  <p style="margin-top:4px;color:var(--jc-text-secondary)">将以上 JSON 添加到目标工具的 MCP 配置文件中即可连接。</p>
                </div>
              </details>
            </div>
          </div>

          <hr style="border:none;border-top:1px solid var(--jc-border-default);margin:12px 0" />

          <!-- 外部 MCP 服务器 -->
          <h4 style="font-size:12px;font-weight:600;color:var(--jc-text-highlight);margin:0 0 6px">🔗 外部 MCP 服务器</h4>
          <p class="pane-desc" style="margin-bottom:6px">连接外部 MCP 服务器获取更多工具，扩展 AI Agent 能力。</p>
          <div class="mode-toggle">
            <button :class="['mode-btn', { active: mcpViewMode === 'list' }]" @click="mcpViewMode = 'list'">列表</button>
            <button :class="['mode-btn', { active: mcpViewMode === 'json' }]" @click="switchToJsonMode">JSON</button>
          </div>

          <template v-if="mcpViewMode === 'list'">
            <div class="mcp-server-list">
              <div v-for="srv in aiStore.mcpServers" :key="srv.id" class="mcp-server-card">
                <div class="mcp-server-top">
                  <div class="mcp-server-info">
                    <span class="mcp-server-name">{{ srv.name }}</span>
                    <span :class="['mcp-server-status', srv.status]">{{ mcpStatusLabel(srv.status) }}</span>
                    <span class="mcp-server-transport">{{ srv.transport === 'sse' ? 'SSE' : 'Stdio' }}</span>
                  </div>
                  <button class="mcp-server-disconnect" @click="disconnectMcp(srv.name)" title="断开">✕</button>
                </div>
                <div v-if="srv.url" class="mcp-server-url">{{ srv.url }}</div>
                <div v-if="srv.command" class="mcp-server-url">{{ srv.command }} {{ srv.args?.join(' ') }}</div>
                <div v-if="srv.errorMessage" class="mcp-server-error">{{ srv.errorMessage }}</div>
                <div class="mcp-server-tools">
                  <span class="mcp-tools-label">工具 ({{ srv.tools?.length || 0 }})</span>
                  <div class="mcp-tools-grid" v-if="srv.tools?.length">
                    <span v-for="tool in srv.tools" :key="tool.name" class="mcp-tool-chip" :title="tool.description">{{ tool.name }}</span>
                  </div>
                  <span v-else class="mcp-tools-empty">暂无工具</span>
                </div>
              </div>
              <div v-if="aiStore.mcpServers.length === 0" class="empty-hint">尚未连接任何 MCP 服务器</div>
            </div>
            <button class="add-btn" @click="showMcpForm = true">+ 连接 MCP 服务器</button>
            <div v-if="showMcpForm" class="form-overlay" @mousedown="handleFormMousedown" @click="handleMcpOverlayClick">
              <div class="form-card">
                <h4>连接 MCP 服务器</h4>
                <div class="form-group"><label>方式</label><select v-model="mcpForm.transport" class="form-select"><option value="sse">SSE</option><option value="stdio">Stdio</option></select></div>
                <div class="form-group"><label>名称</label><input v-model="mcpForm.name" class="form-input" placeholder="my-server" /></div>
                <template v-if="mcpForm.transport === 'sse'"><div class="form-group"><label>SSE URL</label><input v-model="mcpForm.url" class="form-input" placeholder="https://example.com/mcp" /></div></template>
                <template v-if="mcpForm.transport === 'stdio'">
                  <div class="form-group"><label>命令</label><input v-model="mcpForm.command" class="form-input" placeholder="npx" /></div>
                  <div class="form-group"><label>参数 (逗号分隔)</label><input v-model="mcpForm.argsText" class="form-input" placeholder="arg1, arg2" /></div>
                </template>
                <div class="form-actions"><button class="footer-btn-cancel" @click="showMcpForm = false">取消</button><button class="footer-btn-save" :disabled="connecting" @click="saveMcpForm">{{ connecting ? '连接中...' : '连接' }}</button></div>
              </div>
            </div>
          </template>

          <template v-if="mcpViewMode === 'json'">
            <textarea v-model="mcpJsonConfig" class="mcp-json-editor" spellcheck="false" placeholder='{"mcpServers":{...}}'></textarea>
            <div class="mcp-json-actions">
              <button class="mcp-json-apply" @click="applyMcpJson" :disabled="applyingJson">{{ applyingJson ? '应用中...' : '应用 JSON 配置' }}</button>
              <button class="mcp-json-clear" @click="disconnectAllMcp">断开全部</button>
            </div>
            <div v-if="mcpJsonError" class="mcp-server-error">{{ mcpJsonError }}</div>
          </template>
        </div>

        <!-- Backup -->
        <div v-if="activeTab === 'backup'" class="settings-pane">
          <h3 class="pane-title">数据本地备份与导入恢复</h3>
          <p class="pane-desc">所有备忘保存在本地 SQLite 数据库中，可导出 JSON 备份或从 JSON 恢复。</p>
          <div class="backup-actions">
            <button class="backup-btn export" @click="exportData">📤 备份并导出 JSON</button>
            <button class="backup-btn import" @click="importData">📥 导入并恢复 JSON</button>
          </div>
        </div>
      </main>
    </div>

    <!-- Footer -->
    <div class="settings-footer">
      <button class="footer-btn-cancel" @click="doClose">取消</button>
      <button class="footer-btn-save" @click="saveSettings">保存配置</button>
      <span v-if="saveFeedback" class="save-feedback">{{ saveFeedback }}</span>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-window {
  display: flex; flex-direction: column; height: 100vh;
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  background: var(--jc-bg-app); color: var(--jc-text-primary); overflow: hidden;
}

/* Titlebar */
.settings-titlebar {
  display: flex; align-items: center; justify-content: space-between; height: 32px;
  flex-shrink: 0; -webkit-app-region: drag; padding: 0 4px;
}
.stb-spacer { flex: 1; }
.stb-controls { display: flex; gap: 1px; height: 100%; align-items: center; -webkit-app-region: no-drag; }
.stb-btn {
  width: 34px; height: 100%; display: flex; align-items: center; justify-content: center;
  background: none; border: none; color: var(--jc-text-secondary); cursor: pointer; border-radius: 0; transition: background 80ms;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
}
.stb-close:hover { background: #e81123 !important; color: #fff !important; }

/* Header */
.settings-header {
  padding: 10px 16px 8px;
  .settings-title { font-size: 16px; font-weight: 700; color: var(--jc-color-accent); }
  .settings-subtitle { font-size: 11px; color: var(--jc-text-secondary); margin-left: 8px; }
}

/* Body */
.settings-body { display: flex; flex: 1; min-height: 0; }
.settings-nav {
  width: 140px; background: var(--jc-bg-panel); border-right: 1px solid var(--jc-border-default);
  padding: 8px 0; display: flex; flex-direction: column; gap: 2px;
  .nav-item {
    padding: 7px 14px; font-size: 12px; color: var(--jc-text-secondary); cursor: pointer; transition: all 0.15s;
    &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
    &.active { background: var(--jc-bg-selected); color: var(--jc-color-accent); font-weight: 600; }
  }
}
.settings-content { flex: 1; padding: 12px 16px; overflow-y: auto; }

/* Panes */
.settings-pane { display: flex; flex-direction: column; gap: 10px; }
.pane-title { font-size: 13px; font-weight: 600; color: var(--jc-text-highlight); border-bottom: 1px solid var(--jc-border-default); padding-bottom: 6px; margin: 0; }
.pane-desc { font-size: 11px; color: var(--jc-text-secondary); line-height: 1.6; margin: 0; }

/* Form */
.form-group { display: flex; flex-direction: column; gap: 3px; }
.form-group label { font-size: 11px; font-weight: 500; color: var(--jc-text-primary); }
.form-select, .form-input {
  background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary);
  font-size: 12px; padding: 5px 8px; border-radius: 4px; outline: none;
  &:focus { border-color: var(--jc-color-accent); }
}
.help-text { font-size: 10px; color: var(--jc-text-secondary); opacity: 0.8; }
.form-row { display: flex; gap: 8px; }
.form-half { flex: 1; }
.form-textarea { background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-size: 11px; padding: 6px 8px; border-radius: 4px; outline: none; resize: vertical; font-family: monospace; &:focus { border-color: var(--jc-color-accent); } }
.form-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }

/* Overlay for sub-forms */
.form-overlay { position: absolute; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 10; }
.form-card {
  background: var(--jc-bg-elevated); border: 1px solid var(--jc-border-strong); border-radius: 8px;
  padding: 14px; width: 340px; max-height: 90%; overflow-y: auto;
  display: flex; flex-direction: column; gap: 8px; box-shadow: 0 8px 30px rgba(0,0,0,0.4);
  h4 { margin: 0; font-size: 13px; color: var(--jc-text-primary); }
}

/* Buttons */
.add-btn { width: 100%; padding: 5px; border: 1px dashed var(--jc-border-default); border-radius: 6px; background: transparent; color: var(--jc-text-secondary); font-size: 12px; cursor: pointer; &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); } }
.btn-sm { padding: 2px 8px; border: 1px solid var(--jc-border-default); border-radius: 4px; font-size: 10px; cursor: pointer; background: var(--jc-bg-elevated); color: var(--jc-text-primary); &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); } }
.btn-danger { &:hover { border-color: #f85149 !important; color: #f85149 !important; } }
.footer-btn-cancel { background: var(--jc-bg-btn); color: var(--jc-text-secondary); border: none; padding: 6px 14px; font-size: 12px; border-radius: 4px; cursor: pointer; &:hover { color: var(--jc-text-primary); } }
.footer-btn-save { background: var(--jc-color-accent); color: #fff; border: none; padding: 6px 14px; font-size: 12px; font-weight: 600; border-radius: 4px; cursor: pointer; &:hover { opacity: 0.9; } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.save-feedback { font-size: 12px; color: var(--jc-color-success); margin-left: 8px; }

/* Footer bar */
.settings-footer { padding: 10px 16px; background: var(--jc-bg-panel); border-top: 1px solid var(--jc-border-default); display: flex; justify-content: flex-end; gap: 8px; }

/* Model list */
.model-list { display: flex; flex-direction: column; gap: 6px; max-height: 300px; overflow-y: auto; margin-bottom: 8px; }
.model-card { display: flex; align-items: center; justify-content: space-between; padding: 6px 10px; border: 1px solid var(--jc-border-default); border-radius: 6px; font-size: 12px; }
.model-card-header { display: flex; align-items: center; gap: 6px; flex: 1; min-width: 0; }
.model-card-name { font-weight: 600; font-size: 12px; color: var(--jc-text-highlight); }
.model-card-provider { font-size: 10px; padding: 1px 5px; border-radius: 3px; background: rgba(255,255,255,0.08); }
.model-card-model { font-family: monospace; font-size: 11px; color: #58a6ff; font-weight: 600; }

/* vLLM */
.vllm-model-area { display: flex; flex-direction: column; gap: 4px; }
.vllm-toolbar { display: flex; align-items: center; justify-content: space-between; }
.vllm-refresh-btn { padding: 3px 8px; border: 1px solid var(--jc-border-default); border-radius: 4px; background: var(--jc-bg-elevated); color: var(--jc-text-secondary); font-size: 10px; cursor: pointer; &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.vllm-checklist { display: flex; flex-direction: column; gap: 2px; max-height: 150px; overflow-y: auto; border: 1px solid var(--jc-border-default); border-radius: 4px; padding: 4px; }
.vllm-check-item { display: flex; align-items: center; gap: 5px; padding: 3px 6px; border-radius: 3px; font-size: 11px; cursor: pointer; font-family: monospace; &:hover { background: var(--jc-bg-hover); } &.checked { color: var(--jc-color-accent); } }
.vllm-loading, .vllm-empty { font-size: 11px; color: var(--jc-text-secondary); padding: 6px 4px; }

/* Roles */
.roles-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px; max-height: 300px; overflow-y: auto; }
.role-settings-card { border: 1px solid var(--jc-border-default); border-radius: 6px; padding: 6px 10px; display: flex; flex-direction: column; gap: 4px; font-size: 11px; }
.role-card-top { display: flex; align-items: center; gap: 6px; position: relative; }
.role-card-info { display: flex; flex-direction: column; min-width: 0; }
.role-card-name { font-weight: 600; color: var(--jc-text-primary); font-size: 12px; }
.role-card-id { font-family: monospace; font-size: 9px; color: var(--jc-text-secondary); }
.role-card-type { font-size: 9px; padding: 0 4px; border-radius: 3px; background: rgba(255,255,255,0.06); color: var(--jc-text-secondary); position: absolute; right: 0; top: 50%; transform: translateY(-50%); &.custom { background: rgba(138,88,255,0.15); color: var(--jc-color-accent); } }
.role-card-desc { margin: 0; color: var(--jc-text-secondary); line-height: 1.4; }
.role-card-actions { display: flex; gap: 4px; justify-content: flex-end; border-top: 1px solid rgba(255,255,255,0.03); padding-top: 4px; }

/* Skills */
.skills-toolbar { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.skills-search-input { flex: 1; background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-size: 11px; padding: 4px 8px; border-radius: 4px; outline: none; &:focus { border-color: var(--jc-color-accent); } }
.skills-refresh-btn { padding: 4px 10px; border: 1px solid var(--jc-border-default); border-radius: 4px; background: var(--jc-bg-elevated); color: var(--jc-text-primary); font-size: 11px; cursor: pointer; &:hover { border-color: var(--jc-color-accent); } &:disabled { opacity: 0.6; cursor: not-allowed; } }
.skills-count { font-size: 11px; color: var(--jc-text-secondary); }
.error-banner { font-size: 11px; color: #f85149; background: rgba(248,81,73,0.08); padding: 5px 8px; border-radius: 4px; margin-bottom: 6px; }
.skills-list { display: flex; flex-direction: column; gap: 6px; max-height: 320px; overflow-y: auto; }
.skill-card { border: 1px solid var(--jc-border-default); border-radius: 6px; padding: 5px 8px; display: flex; flex-direction: column; gap: 3px; }
.skill-card-header { display: flex; align-items: center; justify-content: space-between; }
.skill-card-name { font-weight: 600; font-size: 12px; color: var(--jc-text-highlight); }
.skill-card-badges { display: flex; gap: 4px; }
.skill-card-status { font-size: 9px; padding: 1px 5px; border-radius: 3px; background: rgba(139,148,158,0.1); color: var(--jc-text-secondary); &.enabled { background: rgba(63,185,80,0.12); color: #3fb950; } }
.skill-card-source { font-size: 9px; padding: 1px 5px; border-radius: 3px; &.system { background: rgba(88,166,255,0.1); color: #58a6ff; } &.project { background: rgba(210,153,34,0.12); color: #d29922; } }
.skill-card-desc { margin: 0; font-size: 11px; color: var(--jc-text-secondary); }
.skill-card-meta { display: flex; align-items: center; gap: 8px; font-size: 10px; }
.link-btn { background: none; border: none; color: var(--jc-text-secondary); font-size: 10px; cursor: pointer; padding: 0; text-align: left; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; &:hover { color: var(--jc-color-accent); } }

/* MCP */
.mode-toggle { display: flex; gap: 4px; margin-bottom: 6px; }
.mode-btn { flex: 1; padding: 4px 8px; border: 1px solid var(--jc-border-default); border-radius: 4px; background: transparent; color: var(--jc-text-secondary); font-size: 11px; cursor: pointer; &:hover { border-color: var(--jc-color-accent); } &.active { background: rgba(88,166,255,0.1); border-color: var(--jc-color-accent); color: var(--jc-color-accent); font-weight: 600; } }
.mcp-server-list { display: flex; flex-direction: column; gap: 6px; max-height: 250px; overflow-y: auto; margin-bottom: 8px; }
.mcp-server-card { border: 1px solid var(--jc-border-default); border-radius: 6px; padding: 8px 10px; display: flex; flex-direction: column; gap: 4px; }
.mcp-server-top { display: flex; align-items: center; justify-content: space-between; }
.mcp-server-info { display: flex; align-items: center; gap: 6px; }
.mcp-server-name { font-weight: 600; font-size: 12px; color: var(--jc-text-highlight); }
.mcp-server-status { font-size: 9px; padding: 1px 5px; border-radius: 3px; font-weight: 500; &.connected { background: rgba(63,185,80,0.15); color: #3fb950; } &.connecting { background: rgba(210,153,34,0.15); color: #d29922; } &.error { background: rgba(248,81,73,0.15); color: #f85149; } }
.mcp-server-transport { font-size: 9px; padding: 1px 4px; border-radius: 3px; background: rgba(255,255,255,0.06); color: var(--jc-text-secondary); font-family: monospace; }
.mcp-server-disconnect { background: none; border: 1px solid transparent; color: var(--jc-text-secondary); font-size: 11px; cursor: pointer; &:hover { color: #f85149; border-color: #f85149; } }
.mcp-server-url { font-size: 10px; font-family: monospace; color: var(--jc-text-secondary); word-break: break-all; }
.mcp-server-error { font-size: 10px; color: #f85149; background: rgba(248,81,73,0.08); padding: 4px 8px; border-radius: 4px; }
.mcp-server-tools { display: flex; flex-direction: column; gap: 3px; }
.mcp-tools-label { font-size: 10px; color: var(--jc-text-secondary); }
.mcp-tools-grid { display: flex; flex-wrap: wrap; gap: 3px; }
.mcp-tool-chip { font-size: 9px; padding: 2px 5px; border-radius: 3px; background: rgba(88,166,255,0.1); color: #58a6ff; font-family: monospace; }
.mcp-tools-empty { font-size: 10px; color: var(--jc-text-secondary); opacity: 0.6; }
.mcp-json-editor { width: 100%; min-height: 200px; background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-family: monospace; font-size: 11px; padding: 8px; border-radius: 4px; resize: vertical; outline: none; &:focus { border-color: var(--jc-color-accent); } }
.mcp-json-actions { display: flex; gap: 8px; margin-top: 6px; }
.mcp-json-apply { flex: 1; padding: 5px 10px; background: var(--jc-color-accent); color: #fff; border: none; border-radius: 4px; font-size: 11px; font-weight: 600; cursor: pointer; &:hover { opacity: 0.9; } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.mcp-json-clear { padding: 5px 10px; background: transparent; color: var(--jc-text-secondary); border: 1px solid var(--jc-border-default); border-radius: 4px; font-size: 11px; cursor: pointer; &:hover { color: #f85149; } }

/* JC9 内置 MCP Server */
.builtin-mcp-section { margin-bottom: 8px; }
.mcp-server-config-card {
  background: var(--jc-bg-elevated); border: 1px solid var(--jc-border-default); border-radius: 6px;
  padding: 10px; display: flex; flex-direction: column; gap: 6px; font-size: 12px;
}
.mcp-config-row { display: flex; align-items: center; gap: 6px; }
.mcp-config-label { font-size: 11px; color: var(--jc-text-secondary); min-width: 50px; flex-shrink: 0; }
.mcp-config-value { font-family: monospace; font-size: 10px; background: var(--jc-bg-input); padding: 2px 6px; border-radius: 3px; word-break: break-all; flex: 1; }
.mcp-key-text { max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
.mcp-copy-btn { background: none; border: 1px solid var(--jc-border-default); color: var(--jc-text-secondary); font-size: 10px; padding: 1px 6px; border-radius: 3px; cursor: pointer; flex-shrink: 0; &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); } }
.mcp-status-badge { font-size: 11px; font-weight: 500; }
.mcp-port-input { width: 70px; background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-size: 11px; padding: 2px 4px; border-radius: 3px; outline: none; text-align: center; &:focus { border-color: var(--jc-color-accent); } }
.mcp-port-hint { font-size: 9px; color: var(--jc-text-secondary); }
.mcp-group-chips { display:flex; flex-wrap:wrap; gap:4px; }
.mcp-chip { font-size:10px; padding:2px 6px; border-radius:3px; background:var(--jc-bg-input); border:1px solid var(--jc-border-default); color:var(--jc-text-secondary); cursor:pointer; &.active { background:rgba(63,185,80,0.15); border-color:#3fb950; color:#3fb950; } &:hover { border-color:var(--jc-color-accent); } }
.mcp-config-actions { display: flex; gap: 6px; align-items: center; margin-top: 4px; }
.mcp-start-btn { background: var(--jc-color-success); color: #fff; border: none; padding: 4px 14px; border-radius: 4px; font-size: 11px; font-weight: 600; cursor: pointer; &:hover { opacity: 0.9; } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.mcp-stop-btn { background: #f85149; color: #fff; border: none; padding: 4px 14px; border-radius: 4px; font-size: 11px; font-weight: 600; cursor: pointer; &:hover { opacity: 0.9; } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.mcp-server-msg { font-size: 11px; padding: 4px 8px; border-radius: 4px; &.success { background: rgba(63,185,80,0.1); color: #3fb950; } &.error { background: rgba(248,81,73,0.1); color: #f85149; } }

/* Backup */
.backup-actions { display: flex; gap: 10px; margin-top: 8px; }
.backup-btn { flex: 1; padding: 8px; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; transition: opacity 0.2s; &.export { background: rgba(0,102,204,0.1); color: var(--jc-color-accent); border: 1px solid var(--jc-color-accent); } &.import { background: rgba(0,109,50,0.1); color: var(--jc-color-success); border: 1px solid var(--jc-color-success); } &:hover { opacity: 0.9; } }

/* Common */
.empty-hint { text-align: center; color: var(--jc-text-secondary); font-size: 12px; padding: 12px; }
</style>
