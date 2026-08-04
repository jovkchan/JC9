<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { useAiStore } from '@/stores/ai'
import { save, open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { loadAllRoles, saveAllRoles, type AgentRole } from '@/config/roles'
import JcModal from '@/components/ui/JcModal.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const formatOptions = [
  { label: 'Markdown (推荐)', value: 'markdown' },
  { label: '纯文本', value: 'plain' }
]
const visibilityOptions = [
  { label: 'PRIVATE (私有本地)', value: 'PRIVATE' },
  { label: 'PUBLIC (公开)', value: 'PUBLIC' }
]
const providerOptions = [
  { label: 'DeepSeek', value: 'deepseek' },
  { label: 'OpenAI', value: 'openai' },
  { label: 'Ollama (本地)', value: 'ollama' },
  { label: 'Google Gemini', value: 'gemini' },
  { label: 'vLLM (自部署)', value: 'vllm' }
]
const mcpTransportOptions = [
  { label: 'SSE', value: 'sse' },
  { label: 'Stdio', value: 'stdio' }
]
const memoryTypeOptions = [
  { label: '选择类型...', value: '' },
  { label: 'decision - 架构决策', value: 'decision' },
  { label: 'bugfix - Bug 修复', value: 'bugfix' },
  { label: 'architecture - 模块结构', value: 'architecture' },
  { label: 'pattern - 编码规范', value: 'pattern' },
  { label: 'config - 配置变更', value: 'config' },
  { label: 'discovery - 调研发现', value: 'discovery' }
]

interface Memory {
  id: string; scope: string; topicKey: string; title: string; content: string;
  memoryType: string; tags: string[]; createdAt: string; updatedAt: string;
}

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
  await loadNoteShareConfig()
  await loadNoteShareStatus()
  // 释放 MCP stdio 代理脚本（内嵌模板 → exe 同目录 mcp/，自动写入当前地址/端口）
  await refreshMcpScript()
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
const activeTab = ref<'general' | 'ai' | 'ai-roles' | 'backup' | 'skills' | 'command' | 'hook' | 'plugin' | 'mcp' | 'memory'>('general')

// ── General settings ──
const defaultFormat = ref<'markdown' | 'plain'>('markdown')
const defaultVisibility = ref<'PRIVATE' | 'PUBLIC'>('PRIVATE')
const saveOnClose = ref(localStorage.getItem('notes-save-on-close') === 'true')

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
const mcpServerUrl = ref('')
const showMcpKey = ref(false)
const mcpPortInput = ref('18899')
const mcpServerMsg = ref('')
const mcpLoading = ref(false)

// ── 配置模板（仅 Stdio 接入方式）──
const mcpScriptPath = ref('')

// ── 笔记分享服务 ──
const noteShareRunning = ref(false)
const noteSharePortInput = ref('8899')
const noteShareMsg = ref('')
const noteShareLoading = ref(false)

async function loadNoteShareStatus() {
  try {
    const config = await invoke<{ port: number; host: string }>('get_note_share_status')
    noteSharePortInput.value = config.port.toString()
    noteShareRunning.value = true
  } catch { noteShareRunning.value = false }
}

async function loadNoteShareConfig() {
  try {
    const config = await invoke<{ port: number; host: string }>('get_note_share_config')
    noteSharePortInput.value = config.port.toString()
  } catch { /* 使用默认值 */ }
}

async function noteShareAction(action: 'start' | 'stop') {
  noteShareLoading.value = true; noteShareMsg.value = ''
  try {
    const msg = await invoke<string>(action === 'start' ? 'note_share_start' : 'note_share_stop')
    noteShareMsg.value = msg
    await loadNoteShareStatus()
  } catch (e) { noteShareMsg.value = `❌ ${e}` }
  finally { noteShareLoading.value = false }
}

async function saveNoteShareConfig() {
  noteShareLoading.value = true; noteShareMsg.value = ''
  try {
    const msg = await invoke<string>('save_note_share_config', {
      config: {
        port: parseInt(noteSharePortInput.value) || 8899,
        host: '0.0.0.0',
      }
    })
    noteShareMsg.value = msg
  } catch (e) { noteShareMsg.value = `❌ ${e}` }
  finally { noteShareLoading.value = false }
}



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
  enabled: boolean; port: number; host: string
}

// ── API Key 管理（独立于服务配置）──
interface ApiKeyItem { id: string; key: string; label: string; scope: string; group_ids: string[]; tools: string[] }
// MCP 工具白名单选项（对应内置 MCP Server 的 16 个工具；risk: safe=查询/绿, medium=中危/黄, danger=删改/红；新建类=黄）
const mcpToolOptions: { name: string; label: string; category: string; risk: 'safe' | 'medium' | 'danger'; description: string }[] = [
  { name: 'jc9_note_search', label: '搜索笔记', category: '笔记', risk: 'safe', description: '向量语义 + 关键词混合搜索笔记，返回最匹配列表（含预览/匹配度）' },
  { name: 'jc9_note_read', label: '读取笔记', category: '笔记', risk: 'safe', description: '读取指定笔记的完整内容（Markdown）' },
  { name: 'jc9_note_create', label: '新建笔记', category: '笔记', risk: 'medium', description: '新建笔记，自动同步知识库并生成向量嵌入（写操作）' },
  { name: 'jc9_note_update_title', label: '改标题', category: '笔记', risk: 'danger', description: '仅更新笔记标题（写操作）' },
  { name: 'jc9_note_update', label: '更新笔记', category: '笔记', risk: 'danger', description: '更新笔记标题/正文/标签，自动重建向量（写操作）' },
  { name: 'jc9_note_delete', label: '删除笔记', category: '笔记', risk: 'danger', description: '删除笔记（软删除，可在回收站恢复）' },
  { name: 'jc9_note_list', label: '笔记列表', category: '笔记', risk: 'safe', description: '列出笔记（可按分组过滤，含内容预览）' },
  { name: 'jc9_note_groups', label: '笔记分组', category: '笔记', risk: 'safe', description: '获取笔记分组列表' },
  { name: 'jc9_memory_add', label: '添加记忆', category: '记忆', risk: 'medium', description: '添加 Agent 记忆，自动向量化（记忆沉淀，写操作）' },
  { name: 'jc9_memory_update', label: '更新记忆', category: '记忆', risk: 'danger', description: '更新已有记忆（写操作）' },
  { name: 'jc9_memory_delete', label: '删除记忆', category: '记忆', risk: 'danger', description: '物理删除记忆' },
  { name: 'jc9_memory_list', label: '记忆列表', category: '记忆', risk: 'safe', description: '列出记忆（按 scope 隔离）' },
  { name: 'jc9_memory_read', label: '读取记忆', category: '记忆', risk: 'safe', description: '读取记忆完整内容' },
  { name: 'jc9_memory_compress', label: '压缩记忆', category: '记忆', risk: 'danger', description: '压缩多条记忆为一条摘要，原记忆被删除' },
  { name: 'jc9_database_stats', label: '诊断统计', category: '诊断', risk: 'safe', description: '数据库/向量索引诊断统计（只读）' },
  { name: 'jc9_reindex', label: '重建向量', category: '诊断', risk: 'medium', description: '重建全部知识条目向量嵌入（耗时，重操作）' },
]
const mcpToolCategories = ['笔记', '记忆', '诊断']
const showApiKeyManager = ref(false)
const showApiKeyForm = ref(false)
const editingApiKeyId = ref('') // '' = 添加新 Key
const newApiKey = ref({ key: '', label: '', scope: '', groupIds: [] as string[], tools: [] as string[] })
const mcpApiKeys = ref<ApiKeyItem[]>([])

function generateApiKey() {
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), b => b.toString(16).padStart(2,'0')).join('')
}

function openAddApiKey() {
  editingApiKeyId.value = ''
  newApiKey.value = { key: '', label: '', scope: '', groupIds: [], tools: mcpToolOptions.map(o => o.name) }
  showApiKeyForm.value = true
}

function openApiKeyManager() {
  showApiKeyManager.value = true
}

function openEditApiKey(ak: ApiKeyItem) {
  editingApiKeyId.value = ak.id
  newApiKey.value = { key: ak.key, label: ak.label, scope: ak.scope, groupIds: [...ak.group_ids], tools: [...(ak.tools || [])] }
  showApiKeyForm.value = true
}

function toggleApiKeyGroup(gid: string) {
  const arr = newApiKey.value.groupIds
  const idx = arr.indexOf(gid)
  if (idx >= 0) arr.splice(idx, 1)
  else arr.push(gid)
}

function toggleApiKeyTool(name: string) {
  const arr = newApiKey.value.tools
  const idx = arr.indexOf(name)
  if (idx >= 0) arr.splice(idx, 1)
  else arr.push(name)
}

async function saveApiKey() {
  const f = newApiKey.value
  const key = f.key || generateApiKey()
  try {
    if (editingApiKeyId.value) {
      await invoke('mcp_update_api_key', { id: editingApiKeyId.value, label: f.label, scope: f.scope, groupIds: [...f.groupIds], tools: [...f.tools] })
    } else {
      await invoke('mcp_add_api_key', { key, label: f.label, scope: f.scope, groupIds: [...f.groupIds], tools: [...f.tools] })
    }
    await loadMcpApiKeys()
  } catch (e) { console.error('保存 Key 失败:', e) }
  showApiKeyForm.value = false
}

function regenerateKeyInForm() {
  newApiKey.value.key = generateApiKey()
}

async function removeApiKey(ak: ApiKeyItem) {
  try {
    await invoke('mcp_delete_api_key', { id: ak.id })
    await loadMcpApiKeys()
  } catch (e) { console.error('删除 Key 失败:', e) }
}

function copyText(text: string) {
  navigator.clipboard.writeText(text)
  status.pushMessage('已复制到剪贴板', 'success')
}
const mcpNoteGroups = ref<Array<{ id: string; name: string; parentId: string | null }>>([])

// 分组标签列表（用于 Key 弹窗中选择白名单分组）
const mcpRootGroups = computed(() => mcpNoteGroups.value.filter(g => !g.parentId))

// 按 scope 分组显示 API Keys
const groupedApiKeys = computed(() => {
  const groups: Record<string, ApiKeyItem[]> = {}
  for (const ak of mcpApiKeys.value) {
    const scope = ak.scope || '(未分类)'
    if (!groups[scope]) groups[scope] = []
    groups[scope].push(ak)
  }
  return Object.entries(groups)
})

async function loadMcpApiKeys() {
  try {
    mcpApiKeys.value = await invoke<ApiKeyItem[]>('mcp_list_api_keys')
  } catch { mcpApiKeys.value = [] }
}

async function loadMcpServerConfig() {
  try {
    const config = await invoke<McpServerConfigType>('ai_get_mcp_server_config')
    mcpPortInput.value = (config.port ?? 18899).toString()
    mcpServerUrl.value = `http://${config.host ?? '127.0.0.1'}:${config.port ?? 18899}`
    mcpServerEnabled.value = config.enabled ?? false
  } catch { /* ignore */ }
  await loadMcpApiKeys()
  try {
    mcpNoteGroups.value = await invoke<Array<{ id: string; name: string; parentId: string | null }>>('get_note_groups')
  } catch { /* ignore */ }
}

async function startMcpServer() {
  mcpLoading.value = true; mcpServerMsg.value = ''
  try {
    const resultMsg = await invoke<string>('ai_set_mcp_server_config', {
      config: {
        enabled: true,
        port: parseInt(mcpPortInput.value) || 18899,
        host: '127.0.0.1',
      }
    })
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

// (regenerateMcpKey removed — keys managed independently via add/edit/delete)

// 基于当前 exe 路径动态释放 MCP stdio 代理脚本并返回其绝对路径
async function refreshMcpScript() {
  try { mcpScriptPath.value = await invoke<string>('ai_prepare_mcp_script') } catch { /* 忽略 */ }
}

function handleConfigExpand(e: Event) {
  const details = e.target as HTMLDetailsElement
  if (details?.open) {
    loadMcpServerStatus()
    // 展开配置模板时基于当前 exe 路径刷新脚本地址（端口/地址变化会重新写入）
    refreshMcpScript()
  }
}

// 可复制的单 server 配置（键值对，粘贴到已有 mcpServers 中即可）
const mcpServerEntryPreview = computed(() => {
  const key = showMcpKey.value ? (mcpApiKeys.value[0]?.key ?? '') : 'YOUR_API_KEY'
  const scriptPath = mcpScriptPath.value || '<释放的 jc9-mcp.mjs 绝对路径>'
  const server = { command: 'node', args: [scriptPath], env: { key } }
  return `"jc9": ${JSON.stringify(server, null, 2)}`
})

function copyMcpConfigJson() {
  navigator.clipboard.writeText(mcpServerEntryPreview.value)
  status.pushMessage('MCP server 配置已复制', 'success')
}

function copyMcpUrl() {
  navigator.clipboard.writeText(mcpServerUrl.value + '/sse')
  status.pushMessage('MCP Server 地址已复制', 'success')
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
  localStorage.setItem('notes-save-on-close', String(saveOnClose.value))
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
  saveFeedback.value = '已保存'
  setTimeout(() => { saveFeedback.value = '' }, 2000)
}

onMounted(() => {
  defaultFormat.value = (localStorage.getItem('notes-default-format') as any) || 'markdown'
  defaultVisibility.value = (localStorage.getItem('notes-default-visibility') as any) || 'PRIVATE'
  saveOnClose.value = localStorage.getItem('notes-save-on-close') === 'true'
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

// ── 记忆管理 ──
const memoryList = ref<Memory[]>([])
const memoryScope = ref('') // 当前选中的 scope Tab，空=全部
const memoryForm = ref({ id: '', title: '', content: '', topicKey: '', type: '', scope: '' })
const compressSelected = ref<string[]>([])

// 搜索 & 分页
const memorySearch = ref('')
const memoryPage = ref(1)
const memoryPageSize = 20
const memoryTotal = ref(0)
const memoryTotalPages = computed(() => Math.max(1, Math.ceil(memoryTotal.value / memoryPageSize)))

// 记忆弹窗
const showMemoryModal = ref(false)
const memoryModalMode = ref<'view' | 'edit'>('view')  // 默认查看模式
const viewingMemory = ref<Memory | null>(null)

// 记忆按 scope 分组（首次加载全部时缓存，避免筛选后标签消失）
const allMemoryScopes = ref<string[]>([])
const memoryScopes = computed(() => allMemoryScopes.value)

async function loadMemoryList() {
  try {
    const result = await invoke<{ items: Memory[]; total: number }>('get_memories', {
      search: memorySearch.value,
      page: memoryPage.value,
      pageSize: memoryPageSize,
      scope: memoryScope.value,
    })
    memoryList.value = result.items
    memoryTotal.value = result.total
    // 加载"全部"时缓存所有 scope，让标签页始终可见
    if (!memoryScope.value) {
      const scopes = new Set<string>()
      for (const m of result.items) { if (m.scope) scopes.add(m.scope) }
      allMemoryScopes.value = Array.from(scopes).sort()
    } else if (allMemoryScopes.value.length === 0) {
      // scope 标签未缓存时（首次打开就带筛选的场景），先加载一次全部来填充
      const full = await invoke<{ items: Memory[]; total: number }>('get_memories', {
        search: '', page: 1, pageSize: 1, scope: '',
      })
      // 再请求一次获取所有 scope
      const all = await invoke<{ items: Memory[]; total: number }>('get_memories', {
        search: '', page: 1, pageSize: Math.max(full.total, 1), scope: '',
      })
      const scopes = new Set<string>()
      for (const m of all.items) { if (m.scope) scopes.add(m.scope) }
      allMemoryScopes.value = Array.from(scopes).sort()
    }
  } catch { memoryList.value = []; memoryTotal.value = 0 }
}

function searchMemory() {
  memoryPage.value = 1
  compressSelected.value = []
  loadMemoryList()
}

function goToPage(page: number) {
  if (page < 1 || page > memoryTotalPages.value) return
  memoryPage.value = page
  compressSelected.value = []
  loadMemoryList()
}

function resetMemoryForm() {
  memoryForm.value = { id: '', title: '', content: '', topicKey: '', type: '', scope: memoryScope.value }
}

function openCreateMemory() {
  resetMemoryForm()
  viewingMemory.value = null
  memoryModalMode.value = 'edit'
  showMemoryModal.value = true
}

function viewMemory(mem: Memory) {
  viewingMemory.value = mem
  memoryForm.value = {
    id: mem.id,
    title: mem.title,
    content: mem.content,
    topicKey: mem.topicKey,
    type: mem.memoryType,
    scope: mem.scope || '',
  }
  memoryModalMode.value = 'view'
  showMemoryModal.value = true
}

function editMemoryFromView() {
  memoryModalMode.value = 'edit'
}

function closeMemoryModal() {
  showMemoryModal.value = false
  viewingMemory.value = null
  resetMemoryForm()
}

async function saveMemory() {
  const f = memoryForm.value
  if (!f.title.trim() || !f.content.trim()) { status.pushMessage('标题和内容不能为空', 'error'); return }
  try {
    const mem = {
      id: f.id || crypto.randomUUID(),
      scope: f.scope || memoryScope.value,
      topicKey: f.topicKey, title: f.title, content: f.content,
      memoryType: f.type || 'discovery', tags: ['memory'],
      createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
    }
    await invoke('save_memory', { memory: mem })
    status.pushMessage('记忆已保存', 'success')
    closeMemoryModal()
    await loadMemoryList()
  } catch (e: any) { status.pushMessage('保存失败: ' + e, 'error') }
}

async function compressMemories() {
  const ids = compressSelected.value
  if (ids.length < 2) return
  try {
    await invoke('compress_memories', { ids })
    status.pushMessage(`已压缩 ${ids.length} 条记忆`, 'success')
    compressSelected.value = []
    await loadMemoryList()
  } catch (e: any) { status.pushMessage('压缩失败: ' + e, 'error') }
}
</script>

<template>
  <div class="settings-window">
    <!-- macOS Titlebar -->
    <div class="settings-titlebar" data-tauri-drag-region>
      <div class="stb-spacer">  <div class="settings-header">
      <span class="settings-title">设置</span>
      <span class="settings-subtitle">JC9 系统配置</span>
    </div></div>
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
        <div :class="['nav-item', { active: activeTab === 'mcp' }]" @click="activeTab = 'mcp'; loadMcpServerConfig()">MCP</div>
        <div :class="['nav-item', { active: activeTab === 'memory' }]" @click="activeTab = 'memory'; loadMemoryList()">记忆</div>
        <div :class="['nav-item', { active: activeTab === 'backup' }]" @click="activeTab = 'backup'">数据备份</div>
      </aside>

      <main class="settings-content">
        <!-- General -->
        <div v-if="activeTab === 'general'" class="settings-pane">
          <h3 class="pane-title">偏好设置</h3>
          <div class="form-group">
            <label>默认笔记格式</label>
            <JcSelect beam :model-value="defaultFormat" :options="formatOptions" style="width: 100%" @update:model-value="(v) => defaultFormat = v as 'markdown' | 'plain'" />
            <span class="help-text">新建备忘时的默认输入解析格式</span>
          </div>
          <div class="form-group">
            <label>新建笔记默认可见性</label>
            <JcSelect beam :model-value="defaultVisibility" :options="visibilityOptions" style="width: 100%" @update:model-value="(v) => defaultVisibility = v as 'PRIVATE' | 'PUBLIC'" />
            <span class="help-text">第一期完全本地化下默认均为 PRIVATE 级别</span>
          </div>
          <div class="form-group">
            <label class="toggle-row">
              <span>关闭标签时自动保存笔记</span>
              <label class="toggle-switch">
                <input type="checkbox" v-model="saveOnClose" />
                <span class="toggle-slider"></span>
              </label>
            </label>
            <span class="help-text">开启后，点击标签栏 ✕ 按钮或右键关闭标签时自动保存当前编辑内容</span>
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
          <JcModal v-model:open="showModelForm" :title="(newModelForm.id ? '编辑' : '添加') + '模型配置'" width="480">
              <div class="form-group"><label>配置名称</label><JcInput beam v-model="newModelForm.name" placeholder="例如：DeepSeek 主力" /></div>
              <div class="form-group">
                <label>供应商</label>
                <JcSelect beam :model-value="newModelForm.provider" :options="providerOptions" style="width: 100%" @update:model-value="(v) => { newModelForm.provider = v as 'ollama' | 'deepseek' | 'openai' | 'gemini' | 'vllm'; setProviderDefaults() }" />
              </div>
              <div class="form-group"><label>Endpoint</label><JcInput beam v-model="newModelForm.endpoint" @blur="onEndpointBlur" /></div>
              <div class="form-group" v-if="newModelForm.provider !== 'ollama' && newModelForm.provider !== 'vllm'"><label>API Key</label><JcInput beam v-model="newModelForm.apiKey" type="password" placeholder="sk-..." /></div>
              <div class="form-group">
                <label>Model</label>
                <div v-if="newModelForm.provider === 'vllm'" class="vllm-model-area">
                  <div class="vllm-toolbar">
                    <span class="vllm-count" v-if="!loadingModels">{{ vllmModels.length }} 个模型</span>
                    <button class="vllm-refresh-btn" :disabled="loadingModels" @click="fetchVllmModelsForm">刷新</button>
                  </div>
                  <div v-if="loadingModels" class="vllm-loading">获取中...</div>
                  <div v-else-if="vllmModels.length === 0" class="vllm-empty">点击刷新从 /models 获取</div>
                  <div v-else class="vllm-checklist">
                    <label v-for="m in vllmModels" :key="m" class="vllm-check-item" :class="{ checked: vllmSelectedModels.includes(m) }">
                      <input type="checkbox" :checked="vllmSelectedModels.includes(m)" @change="toggleVllmModel(m)" /> <span>{{ m }}</span>
                    </label>
                  </div>
                </div>
                <JcInput beam v-else v-model="newModelForm.model" placeholder="多个用英文逗号分隔" />
              </div>
              <div class="form-row">
                <div class="form-group form-half"><label>输入价格 (元/百万)</label><input v-model.number="newModelForm.inputPrice" type="number" step="0.1" class="form-input" /></div>
                <div class="form-group form-half"><label>输出价格 (元/百万)</label><input v-model.number="newModelForm.outputPrice" type="number" step="0.1" class="form-input" /></div>
              </div>
              <div class="form-group"><label>熔断限额 (元)</label><input v-model.number="newModelForm.costLimit" type="number" step="0.5" class="form-input" /></div>
              <template #footer>
                <button class="footer-btn-cancel" @click="cancelModelForm">取消</button>
                <button class="footer-btn-save" @click="saveModelForm">确定</button>
              </template>
            </JcModal>
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

          <JcModal v-model:open="showRoleForm" :title="(newRoleForm.id ? '编辑' : '添加') + ' AI 角色'" width="440">
              <div class="form-group"><label>角色名称</label><JcInput beam v-model="newRoleForm.name" placeholder="例如：测试工程师" /></div>
              <div class="form-group"><label>角色介绍</label><JcInput beam v-model="newRoleForm.description" placeholder="简述该角色的核心职责" /></div>
              <div class="form-group"><label>专属系统提示词</label><JcTextarea v-model="newRoleForm.systemPrompt" beam :beam-size-ratio="0.6" :rows="6" /></div>
              <template #footer>
                <button class="footer-btn-cancel" @click="cancelRoleForm">取消</button>
                <button class="footer-btn-save" @click="saveRoleForm">确定</button>
              </template>
            </JcModal>
        </div>

        <!-- Skills -->
        <div v-if="activeTab === 'skills'" class="settings-pane">
          <div class="skills-toolbar">
            <JcInput beam v-model="skillsSearch" placeholder="搜索技能..." style="flex: 1; min-width: 0" />
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
            <h4 style="font-size:12px;font-weight:600;color:var(--jc-text-highlight);margin:0 0 8px">JC9 MCP Server</h4>

            <!-- 三行紧凑布局 -->
            <div class="mcp-server-compact">
              <!-- 第1行：状态 + 操作按钮 -->
              <div class="mcp-compact-row">
                <span class="mcp-compact-label">状态</span>
                <span :class="['mcp-status-badge', { running: mcpServerRunning }]">
                  {{ mcpServerRunning ? '🟢 运行中' : (mcpServerEnabled ? '🟡 已启用' : '🔴 已停止') }}
                </span>
                <span style="flex:1"></span>
                <button v-if="!mcpServerRunning" class="mcp-start-btn" @click="startMcpServer" :disabled="mcpLoading">启动</button>
                <button v-if="mcpServerRunning" class="mcp-stop-btn" @click="stopMcpServer" :disabled="mcpLoading">停止</button>
                <span v-if="mcpLoading" style="font-size:11px;color:var(--jc-text-secondary)">处理中...</span>
                <span class="mcp-compact-sep">|</span>
                <button class="mcp-action-btn" @click="openApiKeyManager">🔑 API KEY 管理 ({{ mcpApiKeys.length }})</button>
              </div>
              <div v-if="mcpServerMsg" :class="['mcp-server-msg', { success: mcpServerMsg.startsWith('✅'), error: mcpServerMsg.startsWith('❌') }]" style="margin:4px 0">{{ mcpServerMsg }}</div>

              <!-- 第2行：地址 + 端口 -->
              <div class="mcp-compact-row">
                <span class="mcp-compact-label">地址</span>
                <code class="mcp-compact-value">{{ mcpServerUrl }}/sse</code>
                <button class="mcp-copy-btn" @click="copyMcpUrl">复制</button>
                <span class="mcp-compact-sep">|</span>
                <span class="mcp-compact-label">端口</span>
                <input v-model="mcpPortInput" class="mcp-port-input" type="number" min="1024" max="65535" @change="mcpPortInput = Math.max(1024, Math.min(65535, Number(mcpPortInput) || 18899)).toString()" />
              </div>

              <!-- 第3行：配置模板（仅 Stdio 接入方式）-->
              <details class="mcp-config-details" @toggle="handleConfigExpand">
                <summary>
                  配置模板（点击展开）
                  <button class="mcp-copy-btn" @click.stop.prevent="copyMcpConfigJson" title="复制配置 JSON">复制</button>
                </summary>
                <div class="mcp-config-preview">
                  <p style="margin:0 0 4px;font-size:10px"><b>VS Code / Cline / Claude Desktop 配置（Stdio，推荐）：</b>command=node、args 指向释放的 jc9-mcp.mjs（自动写入当前地址/端口），env 传 key（对齐 MCP 接入规范，均不含 type）</p>
                  <JcTextarea :model-value="mcpServerEntryPreview" readonly mono :spellcheck="false" :rows="12" />
                  <p style="margin:4px 0 0;font-size:10px;color:var(--jc-text-secondary)">全选上方内容复制，粘贴到目标工具的 mcpServers 字段中即可。通过 env 的 key 认证，权限与所选 Key 绑定；脚本由 JC9 启动时释放到可执行文件同目录 mcp/ 下。</p>
                </div>
              </details>
            </div>
          </div>

          <!-- API Key 管理弹窗 -->
          <JcModal v-model:open="showApiKeyManager" title="🔑 API Key 管理" width="520">
              <!-- Key 列表 -->
              <div style="margin-bottom:8px">
                <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px">
                  <span style="font-size:11px;color:var(--jc-text-secondary)">{{ mcpApiKeys.length === 0 ? '暂无 API Key，服务启动后无 Key 的请求将被拒绝' : '共 ' + mcpApiKeys.length + ' 个 Key' }}</span>
                  <button class="footer-btn-save" style="font-size:11px;padding:3px 8px" @click="openAddApiKey">+ 添加</button>
                </div>
                <template v-for="[scope, keys] in groupedApiKeys" :key="scope">
                  <div style="font-size:10px;color:var(--jc-text-secondary);padding:8px 0 2px;font-weight:600;text-transform:uppercase;letter-spacing:0.5px">
                    {{ scope }}
                    <span style="font-weight:400;opacity:0.7">({{ keys.length }})</span>
                  </div>
                  <div v-for="ak in keys" :key="ak.id" class="api-key-item" style="display:flex;align-items:center;gap:8px;padding:5px 6px;font-size:11px;border:1px solid var(--jc-border-default);border-radius:4px;margin-bottom:3px">
                    <code style="color:var(--jc-color-success);font-size:10px;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{{ ak.key }}</code>
                    <span style="color:var(--jc-text-highlight);font-size:10px;white-space:nowrap">{{ ak.label }}</span>
                    <span v-if="ak.tools && ak.tools.length" style="font-size:10px;color:var(--jc-text-secondary);white-space:nowrap" :title="ak.tools.join(', ')">工具 {{ ak.tools.length }}/16</span>
                    <span v-else style="font-size:10px;color:var(--jc-color-success);white-space:nowrap">全部工具</span>
                    <button class="mcp-copy-btn" @click="copyText(ak.key)" title="复制">复制</button>
                    <button class="btn-sm" @click="openEditApiKey(ak)">编辑</button>
                    <button class="btn-sm btn-danger" @click="removeApiKey(ak)">删除</button>
                  </div>
                </template>
              </div>

              <!-- 添加/编辑 Key 子表单 -->
              <div v-if="showApiKeyForm" style="border-top:1px solid var(--jc-border-default);padding-top:10px;margin-top:4px">
                <h5 style="margin:0 0 8px;font-size:12px">{{ editingApiKeyId ? '编辑' : '添加' }} API Key</h5>
                <div class="form-group">
                  <label>Key</label>
                  <div style="display:flex;gap:4px">
                    <JcInput beam v-model="newApiKey.key" style="flex:1;min-width:0" :placeholder="editingApiKeyId ? '' : '自动生成'" readonly />
                    <button class="btn-sm" @click="regenerateKeyInForm">生成</button>
                  </div>
                </div>
                <div class="form-group">
                  <label>备注</label>
                  <JcInput beam v-model="newApiKey.label" placeholder="如：JCGO 项目" />
                </div>
                <div class="form-group">
                  <label>Scope（项目标识）</label>
                  <JcInput beam v-model="newApiKey.scope" placeholder="如：JCGO" />
                </div>
                <div class="form-group">
                  <label>白名单分组</label>
                  <div class="mcp-group-chips" style="margin-top:4px">
                    <span v-for="g in mcpRootGroups" :key="g.id"
                      :class="['mcp-chip', { active: newApiKey.groupIds.includes(g.id) }]"
                      @click="toggleApiKeyGroup(g.id)">
                      {{ g.name }}
                    </span>
                  </div>
                  <span style="font-size:10px;color:var(--jc-text-secondary);margin-top:2px">{{ newApiKey.groupIds.length === 0 ? '空=访问所有分组' : '已选 ' + newApiKey.groupIds.length + ' 个根分组' }}</span>
                </div>
                <div class="form-group">
                  <label>工具权限（开关：开=允许该工具；默认全开=允许全部；颜色：<span class="risk-dot" style="background:#52c41a"></span>查询 <span class="risk-dot" style="background:#faad14"></span>中危 <span class="risk-dot" style="background:#ff4d4f"></span>危险）</label>
                  <div class="mcp-tool-list" style="margin-top:4px">
                    <template v-for="cat in mcpToolCategories" :key="cat">
                      <div class="mcp-tool-cat">{{ cat }}</div>
                      <div v-for="t in mcpToolOptions.filter(o => o.category === cat)" :key="t.name"
                        class="mcp-tool-row"
                        :title="t.description"
                        @click="toggleApiKeyTool(t.name)">
                        <span class="mcp-tool-toggle" :class="['risk-' + t.risk, { on: newApiKey.tools.includes(t.name) }]">
                          <span class="mcp-tool-knob"></span>
                        </span>
                        <span class="mcp-tool-name">{{ t.label }}</span>
                        <span class="mcp-tool-code">{{ t.name }}</span>
                      </div>
                    </template>
                  </div>
                  <span style="font-size:10px;color:var(--jc-text-secondary);margin-top:2px">{{ newApiKey.tools.length === 0 ? '全关 = 不限制（允许全部）' : '已选 ' + newApiKey.tools.length + ' 个工具' }}</span>
                </div>
              </div>
              <template #footer>
                <template v-if="showApiKeyForm">
                  <button class="footer-btn-cancel" @click="showApiKeyForm = false">取消</button>
                  <button class="footer-btn-save" @click="saveApiKey">{{ editingApiKeyId ? '保存' : '生成并添加' }}</button>
                </template>
                <template v-else>
                  <button class="footer-btn-cancel" @click="showApiKeyManager = false">关闭</button>
                </template>
              </template>
            </JcModal>

          <!-- 笔记分享服务器（独立端口配置） -->
          <div class="builtin-mcp-section" style="margin-top:8px">
            <h4 style="font-size:12px;font-weight:600;color:var(--jc-text-highlight);margin:0 0 8px">📝 笔记分享服务</h4>
            <div class="mcp-compact-row">
              <span class="mcp-compact-label">状态</span>
              <span :class="['mcp-status-badge', { running: noteShareRunning }]">
                {{ noteShareRunning ? '🟢 运行中' : '🔴 已停止' }}
              </span>
              <span style="flex:1"></span>
              <button v-if="!noteShareRunning" class="mcp-start-btn" @click="noteShareAction('start')" :disabled="noteShareLoading">启动</button>
              <button v-if="noteShareRunning" class="mcp-stop-btn" @click="noteShareAction('stop')" :disabled="noteShareLoading">停止</button>
              <span v-if="noteShareLoading" style="font-size:11px;color:var(--jc-text-secondary)">处理中...</span>
              <span class="mcp-compact-sep">|</span>
              <span class="mcp-compact-label">端口</span>
              <input v-model="noteSharePortInput" class="mcp-port-input" type="number" min="1024" max="65535" @change="noteSharePortInput = Math.max(1024, Math.min(65535, Number(noteSharePortInput) || 8899)).toString()" />
              <span style="flex:1"></span>
              <button class="mcp-action-btn" @click="saveNoteShareConfig" :disabled="noteShareLoading">{{ noteShareLoading ? '保存中...' : '保存' }}</button>
            </div>
            <div v-if="noteShareMsg" :class="['mcp-server-msg', { success: noteShareMsg.startsWith('✅'), error: noteShareMsg.startsWith('❌') }]" style="margin:4px 0">{{ noteShareMsg }}</div>
          </div>

          <hr style="border:none;border-top:1px solid var(--jc-border-default);margin:12px 0" />

          <!-- 外部 MCP 服务器 -->
          <h4 style="font-size:12px;font-weight:600;color:var(--jc-text-highlight);margin:0 0 6px">外部 MCP 服务器</h4>
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
            <button class="add-btn" @click="showMcpForm = true">连接 MCP 服务器</button>
            <JcModal v-model:open="showMcpForm" title="连接 MCP 服务器" width="440">
                <div class="form-group"><label>方式</label><JcSelect beam :model-value="mcpForm.transport" :options="mcpTransportOptions" style="width: 100%" @update:model-value="(v) => mcpForm.transport = v as 'sse' | 'stdio'" /></div>
                <div class="form-group"><label>名称</label><JcInput beam v-model="mcpForm.name" placeholder="my-server" /></div>
                <template v-if="mcpForm.transport === 'sse'"><div class="form-group"><label>SSE URL</label><JcInput beam v-model="mcpForm.url" placeholder="https://example.com/mcp" /></div></template>
                <template v-if="mcpForm.transport === 'stdio'">
                  <div class="form-group"><label>命令</label><JcInput beam v-model="mcpForm.command" placeholder="npx" /></div>
                  <div class="form-group"><label>参数 (逗号分隔)</label><JcInput beam v-model="mcpForm.argsText" placeholder="arg1, arg2" /></div>
                </template>
                <template #footer>
                  <button class="footer-btn-cancel" @click="showMcpForm = false">取消</button>
                  <button class="footer-btn-save" :disabled="connecting" @click="saveMcpForm">{{ connecting ? '连接中...' : '连接' }}</button>
                </template>
              </JcModal>
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

        <!-- 记忆 -->
        <div v-if="activeTab === 'memory'" class="settings-pane" style="display:flex;flex-direction:column;overflow:hidden;height:100%">
          <!-- 固定头部：标题、scope 标签、搜索 -->
          <div style="flex-shrink:0">
            <div style="display:flex;align-items:center;justify-content:space-between">
              <div>
                <h3 class="pane-title" style="border:none;padding:0;margin:0">Agent 记忆管理</h3>
                <p class="pane-desc" style="margin:2px 0 0">基于 AI Agent 记忆系统设计。记忆按 scope（项目）分组隔离。</p>
              </div>
              <button class="footer-btn-save" style="font-size:11px;padding:4px 10px" @click="openCreateMemory">+ 添加记忆</button>
            </div>

            <!-- Scope Tabs -->
            <div class="memory-scope-tabs" style="display:flex;gap:4px;margin-top:10px;flex-wrap:wrap">
              <span :class="['memory-scope-tab', { active: !memoryScope }]" @click="memoryScope = ''; resetMemoryForm(); memoryPage = 1; loadMemoryList()">
                全部
              </span>
              <span v-for="sc in memoryScopes" :key="sc"
                :class="['memory-scope-tab', { active: memoryScope === sc }]"
                @click="memoryScope = sc; resetMemoryForm(); memoryPage = 1; loadMemoryList()">
                {{ sc }}
              </span>
            </div>

            <!-- 搜索 -->
            <div style="display:flex;gap:6px;margin-top:8px">
              <JcInput beam v-model="memorySearch" placeholder="搜索标题或内容..." style="flex:1;min-width:0" @keyup.enter="searchMemory" />
              <button class="footer-btn-save" style="font-size:11px;padding:4px 10px" @click="searchMemory">搜索</button>
              <button v-if="memorySearch" class="footer-btn-cancel" style="font-size:11px;padding:4px 8px" @click="memorySearch = ''; searchMemory()">清除</button>
            </div>
          </div>

          <!-- 滚动列表区域 -->
          <div class="memory-section" style="margin-top:10px;flex:1;overflow-y:auto;min-height:0">
            <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:4px">
              <h4 style="margin:0">📋 记忆列表 ({{ memoryTotal }})</h4>
              <button v-if="compressSelected.length > 1" class="footer-btn-save" style="font-size:11px;padding:3px 8px" @click="compressMemories">🗜 压缩选中 ({{ compressSelected.length }})</button>
            </div>
            <div v-if="memoryList.length === 0" style="color:var(--jc-text-secondary);font-size:12px;padding:8px">
              {{ memorySearch ? '无匹配结果' : '暂无记忆' }}
            </div>
            <div v-for="m in memoryList" :key="m.id" class="memory-item" style="display:flex;align-items:center;gap:8px">
              <input type="checkbox" :value="m.id" v-model="compressSelected" style="flex-shrink:0;accent-color:var(--jc-color-accent)" @click.stop />
              <div style="flex:1;min-width:0;cursor:pointer" @click="viewMemory(m)">
                <div style="display:flex;align-items:center;gap:6px">
                  <span style="color:var(--jc-color-accent);font-size:10px">{{ m.id.slice(0,8) }}</span>
                  <span style="font-weight:500">{{ m.title }}</span>
                  <span v-if="m.scope && !memoryScope" style="font-size:9px;background:rgba(88,166,255,0.15);color:#58a6ff;padding:0 3px;border-radius:2px">{{ m.scope }}</span>
                  <span style="font-size:10px;color:var(--jc-color-success)">{{ m.memoryType }}</span>
                  <span v-if="m.topicKey" style="font-size:10px;color:var(--jc-text-secondary)">#{{ m.topicKey }}</span>
                </div>
                <span style="font-size:10px;color:var(--jc-text-secondary)">{{ (m.content || '').slice(0, 80) }}...</span>
              </div>
            </div>

            <!-- 分页 -->
            <div v-if="memoryTotalPages > 1" style="display:flex;align-items:center;justify-content:center;gap:6px;margin-top:10px">
              <button class="page-btn" :disabled="memoryPage <= 1" @click="goToPage(memoryPage - 1)">‹ 上一页</button>
              <span v-for="p in memoryTotalPages" :key="p">
                <button v-if="p === 1 || p === memoryTotalPages || Math.abs(p - memoryPage) <= 2"
                  :class="['page-btn', { active: p === memoryPage }]" @click="goToPage(p)">{{ p }}</button>
                <span v-else-if="p === memoryPage - 3 || p === memoryPage + 3" style="color:var(--jc-text-secondary);padding:0 2px">…</span>
              </span>
              <button class="page-btn" :disabled="memoryPage >= memoryTotalPages" @click="goToPage(memoryPage + 1)">下一页 ›</button>
            </div>
          </div>

          <!-- 记忆弹窗 -->
          <JcModal v-model:open="showMemoryModal" :title="memoryModalMode === 'edit' ? (memoryForm.id ? '编辑记忆' : '创建记忆') : '查看记忆'" width="580" @cancel="closeMemoryModal">
            <template #title>
              <span style="display:inline-flex;gap:6px">
                {{ memoryModalMode === 'edit' ? (memoryForm.id ? '编辑记忆' : '创建记忆') : '查看记忆' }}
                <button v-if="memoryModalMode === 'view'" class="footer-btn-save" style="font-size:11px;padding:3px 8px" @click="editMemoryFromView">✏️ 编辑</button>
              </span>
            </template>

              <!-- 查看模式 -->
              <template v-if="memoryModalMode === 'view'">
                <div class="memory-detail">
                  <div class="memory-detail-field">
                    <span class="memory-detail-label">标题</span>
                    <span class="memory-detail-value">{{ memoryForm.title }}</span>
                  </div>
                  <div class="memory-detail-row">
                    <div class="memory-detail-field">
                      <span class="memory-detail-label">Scope</span>
                      <span class="memory-detail-value">{{ memoryForm.scope || '—' }}</span>
                    </div>
                    <div class="memory-detail-field">
                      <span class="memory-detail-label">类型</span>
                      <span class="memory-detail-value" style="color:var(--jc-color-success)">{{ memoryForm.type || '—' }}</span>
                    </div>
                    <div class="memory-detail-field">
                      <span class="memory-detail-label">Topic Key</span>
                      <span class="memory-detail-value">{{ memoryForm.topicKey || '—' }}</span>
                    </div>
                  </div>
                  <div class="memory-detail-field">
                    <span class="memory-detail-label">内容</span>
                    <pre class="memory-detail-content">{{ memoryForm.content }}</pre>
                  </div>
                </div>
              </template>

              <!-- 编辑模式 -->
              <template v-else>
                <div class="memory-form">
                  <JcInput beam v-model="memoryForm.title" placeholder="记忆标题" />
                  <div style="display:flex;gap:6px;margin-top:6px">
                    <JcInput beam v-model="memoryForm.scope" placeholder="scope（项目标识）" style="flex:1;min-width:0" />
                    <JcSelect beam v-model="memoryForm.type" :options="memoryTypeOptions" style="flex:1;min-width:0" />
                  </div>
                  <JcInput beam v-model="memoryForm.topicKey" placeholder="topic_key（去重用）" style="margin-top:6px" />
                  <JcTextarea v-model="memoryForm.content" beam :beam-size-ratio="0.6" :rows="10" placeholder="**What**: 做了什么
**Why**: 为什么
**Where**: 涉及文件
**Learned**: 踩坑记录" style="margin-top:6px" />
                </div>
              </template>
              <template #footer>
                <template v-if="memoryModalMode === 'edit'">
                  <button class="footer-btn-cancel" @click="closeMemoryModal">取消</button>
                  <button class="footer-btn-save" @click="saveMemory">{{ memoryForm.id ? '更新' : '创建' }}</button>
                </template>
                <template v-else>
                  <button class="footer-btn-cancel" @click="closeMemoryModal">关闭</button>
                </template>
              </template>
            </JcModal>
        </div>

        <!-- Backup -->
        <div v-if="activeTab === 'backup'" class="settings-pane">
          <h3 class="pane-title">数据本地备份与导入恢复</h3>
          <p class="pane-desc">所有备忘保存在本地 SQLite 数据库中，可导出 JSON 备份或从 JSON 恢复。</p>
          <div class="backup-actions">
            <button class="backup-btn export" @click="exportData">备份并导出 JSON</button>
            <button class="backup-btn import" @click="importData">导入并恢复 JSON</button>
          </div>
        </div>
      </main>
    </div>

    <!-- Footer -->
    <div class="settings-footer">
      <button class="footer-btn-save" @click="saveSettings">{{ saveFeedback || '保存配置' }}</button>
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
  .settings-title { font-size: 16px; font-weight: 700; }
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

/* Toggle switch */
.form-group .toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  font-size: 11px;
  font-weight: 500;
  color: var(--jc-text-primary);
  cursor: pointer;
}
.toggle-switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
  cursor: pointer;
  input { opacity: 0; width: 0; height: 0; }
  .toggle-slider {
    position: absolute; inset: 0;
    background: var(--jc-border-default, #444);
    border-radius: 20px;
    transition: background 0.2s;
    &::before {
      content: '';
      position: absolute;
      width: 16px; height: 16px;
      left: 2px; bottom: 2px;
      background: #fff;
      border-radius: 50%;
      transition: transform 0.2s;
    }
  }
  input:checked + .toggle-slider { background: var(--jc-color-accent, #58a6ff); }
  input:checked + .toggle-slider::before { transform: translateX(16px); }
}
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

/* JC9 内置 MCP Server — 紧凑三行布局 */
.builtin-mcp-section { margin-bottom: 8px; }
.mcp-server-compact {
  background: var(--jc-bg-elevated); border: 1px solid var(--jc-border-default); border-radius: 6px;
  padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; font-size: 12px;
}
.mcp-compact-row {
  display: flex; align-items: center; gap: 6px;
}
.mcp-compact-label {
  font-size: 11px; color: var(--jc-text-secondary); flex-shrink: 0;
}
.mcp-compact-value {
  font-family: monospace; font-size: 10px; background: var(--jc-bg-input); padding: 2px 6px; border-radius: 3px; flex: 1;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.mcp-compact-sep {
  color: var(--jc-border-default); font-size: 11px; user-select: none;
}
.mcp-action-btn {
  background: none; border: 1px solid var(--jc-border-default); color: var(--jc-text-secondary);
  font-size: 11px; padding: 3px 8px; border-radius: 4px; cursor: pointer; transition: all 0.15s;
  &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); }
}
.mcp-config-details {
  font-size: 11px;
  summary {
    cursor: pointer; color: var(--jc-text-secondary); display: inline-flex; align-items: center; gap: 8px;
    &:hover { color: var(--jc-text-primary); }
  }
}
.mcp-config-preview {
  margin-top: 6px; background: var(--jc-bg-elevated); padding: 8px; border-radius: 4px;
  font-size: 10px; line-height: 1.6;
}
.mcp-key-text { max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
.mcp-copy-btn { background: none; border: 1px solid var(--jc-border-default); color: var(--jc-text-secondary); font-size: 10px; padding: 1px 6px; border-radius: 3px; cursor: pointer; flex-shrink: 0; &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); } }
.mcp-status-badge { font-size: 11px; font-weight: 500; }
.mcp-port-input { width: 70px; background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-size: 11px; padding: 2px 4px; border-radius: 3px; outline: none; text-align: center; &:focus { border-color: var(--jc-color-accent); } }
.mcp-port-hint { font-size: 9px; color: var(--jc-text-secondary); }
.mcp-group-chips { display:flex; flex-wrap:wrap; gap:4px; }
.mcp-chip { font-size:10px; padding:2px 6px; border-radius:3px; background:var(--jc-bg-input); border:1px solid var(--jc-border-default); color:var(--jc-text-secondary); cursor:pointer; &.active { background:rgba(63,185,80,0.15); border-color:#3fb950; color:#3fb950; } &:hover { border-color:var(--jc-color-accent); } }
.risk-dot { display:inline-block; width:8px; height:8px; border-radius:50%; vertical-align:middle; margin:0 2px 0 4px; }
.mcp-tool-list { display:flex; flex-direction:column; gap:2px; max-height:220px; overflow-y:auto; padding:4px 6px; border:1px solid var(--jc-border-default); border-radius:4px; background:var(--jc-bg-input); }
.mcp-tool-cat { font-size:10px; color:var(--jc-text-secondary); font-weight:600; margin:6px 0 2px; }
.mcp-tool-row { display:flex; align-items:center; gap:8px; padding:3px 6px; border-radius:3px; cursor:pointer; &:hover { background:var(--jc-bg-elevated); } }
.mcp-tool-name { font-size:11px; color:var(--jc-text-primary); flex-shrink:0; }
.mcp-tool-code { font-size:9px; color:var(--jc-text-secondary); opacity:0.7; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.mcp-tool-toggle { position:relative; width:28px; height:16px; border-radius:8px; background:var(--jc-bg-elevated); border:1px solid var(--jc-border-default); flex-shrink:0; transition:background 0.2s; .mcp-tool-knob { position:absolute; top:1px; left:1px; width:12px; height:12px; border-radius:50%; background:var(--jc-text-secondary); transition:left 0.2s, background 0.2s; } &.on { .mcp-tool-knob { left:13px; background:#fff; } } &.risk-safe.on { background:#52c41a; border-color:#52c41a; } &.risk-medium.on { background:#faad14; border-color:#faad14; } &.risk-danger.on { background:#ff4d4f; border-color:#ff4d4f; } }
.mcp-config-actions { display: flex; gap: 6px; align-items: center; margin-top: 4px; }
.mcp-start-btn { background: var(--jc-color-success); color: #fff; border: none; padding: 4px 14px; border-radius: 4px; font-size: 11px; font-weight: 600; cursor: pointer; &:hover { opacity: 0.9; } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.mcp-stop-btn { background: #f85149; color: #fff; border: none; padding: 4px 14px; border-radius: 4px; font-size: 11px; font-weight: 600; cursor: pointer; &:hover { opacity: 0.9; } &:disabled { opacity: 0.5; cursor: not-allowed; } }
.mcp-server-msg { font-size: 11px; padding: 4px 8px; border-radius: 4px; &.success { background: rgba(63,185,80,0.1); color: #3fb950; } &.error { background: rgba(248,81,73,0.1); color: #f85149; } }

/* Backup */
// 记忆管理
.memory-section {
  h4 { font-size: 12px; font-weight: 600; color: var(--jc-text-highlight); margin: 0 0 6px; }
}
.memory-scope-tab {
  padding: 3px 10px;
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;
  color: var(--jc-text-secondary);
  background: var(--jc-bg-elevated);
  border: 1px solid transparent;
  transition: all 0.15s;
  &:hover { color: var(--jc-text-primary); background: var(--jc-bg-hover); }
  &.active {
    color: var(--jc-color-accent);
    background: rgba(88,166,255,0.1);
    border-color: rgba(88,166,255,0.3);
  }
}
.memory-form { display: flex; flex-direction: column; }
.memory-item {
  padding: 8px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  margin-bottom: 4px;
  &:hover { background: var(--jc-bg-hover); }
}
.memory-detail {
  display: flex; flex-direction: column; gap: 8px;
}
.memory-detail-field {
  display: flex; flex-direction: column; gap: 2px;
}
.memory-detail-label {
  font-size: 10px; font-weight: 600; color: var(--jc-text-secondary); text-transform: uppercase; letter-spacing: 0.5px;
}
.memory-detail-value {
  font-size: 13px; color: var(--jc-text-primary);
}
.memory-detail-row {
  display: flex; gap: 12px;
  .memory-detail-field { flex: 1; min-width: 0; }
}
.memory-detail-content {
  margin: 0;
  padding: 10px;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  font-size: 12px; line-height: 1.6; color: var(--jc-text-primary);
  white-space: pre-wrap; word-break: break-word;
  max-height: 260px; overflow-y: auto;
  font-family: inherit;
}
.backup-actions { display: flex; gap: 10px; margin-top: 8px; }

// 分页
.page-btn {
  background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-secondary);
  font-size: 11px; padding: 3px 8px; border-radius: 4px; cursor: pointer; transition: all 0.15s;
  &:hover:not(:disabled) { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
  &:disabled { opacity: 0.4; cursor: default; }
  &.active { background: var(--jc-color-accent); color: #fff; border-color: var(--jc-color-accent); }
}

// API Key 列表
.api-key-item {
  &:hover { background: var(--jc-bg-hover); border-radius: 3px; }
}
.backup-btn { flex: 1; padding: 8px; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; transition: opacity 0.2s; &.export { background: rgba(0,102,204,0.1); color: var(--jc-color-accent); border: 1px solid var(--jc-color-accent); } &.import { background: rgba(0,109,50,0.1); color: var(--jc-color-success); border: 1px solid var(--jc-color-success); } &:hover { opacity: 0.9; } }

/* Common */
.empty-hint { text-align: center; color: var(--jc-text-secondary); font-size: 12px; padding: 12px; }
</style>
