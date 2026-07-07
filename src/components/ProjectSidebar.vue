<script setup lang="ts">
import { ref, nextTick, computed, onMounted, onUnmounted } from 'vue'
import { useProjectStore } from '@/stores/project'
import { useStatusStore } from '@/stores/status'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import CommandDialog from '@/components/CommandDialog.vue'
import NoteSidebar from '@/components/notes/NoteSidebar.vue'
import type { Command } from '@/types'

const store = useProjectStore()
const activeTab = ref<'projects' | 'workflows' | 'tools' | 'notes'>('projects')
const showAdd = ref(false)
const newName = ref('')
const newDir = ref('')
const expandedProjects = ref<Set<string>>(new Set())
const detectedLang = ref('')
const detectedCmds = ref<{ name: string; command: string; workingDir: string }[]>([])
const dialogProjectId = ref('')
const editingCmd = ref<Command | null>(null)
const cmdDialogRef = ref<InstanceType<typeof CommandDialog>>()

function toggleExpand(id: string) {
  if (expandedProjects.value.has(id)) {
    expandedProjects.value.delete(id)
  } else {
    expandedProjects.value.add(id)
  }
}

async function pickDir() {
  const d = await open({
    directory: true,
    multiple: false,
    title: '选择项目目录'
  })
  if (d && typeof d === 'string') {
    newDir.value = d
    newName.value = d.split(/[\\/]/).pop() || newName.value
    const info = await store.detectProject(d)
    if (info) {
      newName.value = info.name
      detectedLang.value = info.lang
      detectedCmds.value = info.suggestCommands
    }
  }
}

async function handleAdd() {
  const n = newName.value.trim() || '新项目'
  store.addProject(n)
  const pid = store.projects[store.projects.length - 1].id
  expandedProjects.value.add(pid)
  for (const c of detectedCmds.value) {
    store.addCommand(pid, c)
  }
  newName.value = ''
  newDir.value = ''
  detectedLang.value = ''
  detectedCmds.value = []
  showAdd.value = false
}

function addQuickCmd(id: string) {
  editingCmd.value = null
  dialogProjectId.value = id
  cmdDialogRef.value?.openDialog()
}

function editCmd(pid: string, cmd: Command) {
  editingCmd.value = cmd
  dialogProjectId.value = pid
  cmdDialogRef.value?.openDialog()
}

function isRunning(pid: string, cid: string) {
  return store.runningMap[store.cmdKey(pid, cid)] === 'running'
}

// ---- Project context menu ----
const projCtxShow = ref(false)
const projCtxPos = ref({ x: 0, y: 0 })
const projCtxId = ref('')

function openProjCtx(e: MouseEvent, pid: string) {
  e.preventDefault()
  projCtxPos.value = { x: e.clientX, y: e.clientY }
  projCtxId.value = pid
  projCtxShow.value = true
}

function closeProjCtx() {
  projCtxShow.value = false
}

function ctxRenameProj() {
  editingProjId.value = projCtxId.value
  editProjName.value = store.projects.find(p => p.id === projCtxId.value)?.name || ''
  closeProjCtx()
  nextTick(() => {
    const el = document.querySelector<HTMLInputElement>('.proj-edit-input')
    el?.focus()
    el?.select()
  })
}

function confirmRenameProj() {
  const n = editProjName.value.trim()
  if (n) {
    store.updateProjectName(editingProjId.value, n)
  }
  editingProjId.value = ''
}

// 新增快速命令
function ctxAddCmd() {
  dialogProjectId.value = projCtxId.value
  editingCmd.value = null
  cmdDialogRef.value?.openDialog()
  closeProjCtx()
}

function ctxDelProj() {
  store.removeProject(projCtxId.value)
  closeProjCtx()
}

const editingProjId = ref('')
const editProjName = ref('')

// ---- Command context menu ----
const cmdCtxShow = ref(false)
const cmdCtxPos = ref({ x: 0, y: 0 })
const cmdCtxPid = ref('')
const cmdCtxCmd = ref<Command | null>(null)

function openCmdCtx(e: MouseEvent, pid: string, cmd: Command) {
  e.preventDefault()
  e.stopPropagation()
  cmdCtxPos.value = { x: e.clientX, y: e.clientY }
  cmdCtxPid.value = pid
  cmdCtxCmd.value = cmd
  cmdCtxShow.value = true
}

function closeCmdCtx() {
  cmdCtxShow.value = false
}

function ctxEditCmd() {
  if (cmdCtxCmd.value) {
    dialogProjectId.value = cmdCtxPid.value
    editingCmd.value = cmdCtxCmd.value
    cmdDialogRef.value?.openDialog()
  }
  closeCmdCtx()
}

function ctxRenameCmd() {
  const c = cmdCtxCmd.value
  if (c) {
    editingCmdId.value = cmdCtxPid.value + '::' + c.id
    editCmdName.value = c.name
  }
  closeCmdCtx()
  nextTick(() => {
    const el = document.querySelector<HTMLInputElement>('.cmd-edit-input')
    el?.focus()
    el?.select()
  })
}

function confirmRenameCmd() {
  const [pid, cid] = editingCmdId.value.split('::')
  const n = editCmdName.value.trim()
  const p = store.projects.find(p => p.id === pid)
  const c = p?.commands.find(c => c.id === cid)
  if (n && c) {
    store.updateCommand(pid, { ...c, name: n })
  }
  editingCmdId.value = ''
}

const editingCmdId = ref('')
const editCmdName = ref('')

function ctxDelCmd() {
  if (cmdCtxCmd.value) {
    store.removeCommand(cmdCtxPid.value, cmdCtxCmd.value.id)
  }
  closeCmdCtx()
}

// ── 工作流 JSON 模板 ──
const WORKFLOW_TEMPLATE_JSON = JSON.stringify({
  name: "编译并运行",
  description: "先编译项目，编译成功后再启动",
  category: "Tauri",
  steps: [
    { name: "编译", command: "cargo build", workingDir: "src-tauri" },
    { name: "运行", command: "npx tauri dev", workingDir: "." }
  ]
}, null, 2)

// ---- Workflows (多命令顺序执行，替代旧快捷方式) ----
const showWfDlg = ref(false)
const wfEditId = ref('')
const wfName = ref('')
const wfDesc = ref('')
const wfCat = ref('')
const wfSteps = ref<Array<{ name: string; command: string; workingDir: string }>>([{ name: '', command: '', workingDir: '' }])
const wfJsonMode = ref(false)
const wfJsonText = ref('')
const aiGenerating = ref(false)
const wfAiProvider = ref('')
const wfAiEndpoint = ref('')
const wfAiKey = ref('')
const wfAiModel = ref('')
const wfModelList = ref<string[]>([])
const wfModelMap = ref<Record<string, { provider: string; endpoint: string; apiKey: string; model: string }>>({})
const wfAiMsg = ref('')
const stepAiIdx = ref(-1)
const stepAiInput = ref('')

const WORKFLOW_SCHEMA_EXAMPLE = `{
  "name": "编译并运行",
  "description": "先编译项目，编译成功后再启动",
  "category": "Tauri",
  "steps": [
    { "name": "编译", "command": "cargo build", "workingDir": "src-tauri" },
    { "name": "运行", "command": "npx tauri dev", "workingDir": "." }
  ]
}`

function openWfDlg(editId?: string) {
  showWfDlg.value = true
  wfJsonMode.value = false
  // 从 JSON 加载模型列表
  loadWfModels()
  if (editId) {
    const w = store.workflows.find(x => x.id === editId)
    if (w) {
      wfEditId.value = editId
      wfName.value = w.name
      wfDesc.value = w.description
      wfCat.value = w.category
      wfSteps.value = w.steps.map(s => ({ ...s }))
      wfJsonText.value = JSON.stringify({ name: w.name, description: w.description, category: w.category, steps: w.steps }, null, 2)
      return
    }
  }
  // 新建：预填模板
  wfEditId.value = ''
  const tpl = JSON.parse(WORKFLOW_TEMPLATE_JSON)
  wfName.value = tpl.name
  wfDesc.value = tpl.description
  wfCat.value = tpl.category
  wfSteps.value = tpl.steps.map((s: any) => ({ ...s }))
  wfJsonText.value = WORKFLOW_TEMPLATE_JSON
}

function applyJsonToForm() {
  try {
    const parsed = JSON.parse(wfJsonText.value)
    wfName.value = parsed.name || ''
    wfDesc.value = parsed.description || ''
    wfCat.value = parsed.category || ''
    wfSteps.value = (parsed.steps || []).map((s: any) => ({
      name: s.name || '',
      command: s.command || '',
      workingDir: s.workingDir || ''
    }))
    wfJsonMode.value = false
  } catch (e) {
    useStatusStore().pushMessage(`JSON 格式错误: ${e}`, 'error')
  }
}

function syncFormToJson() {
  wfJsonText.value = JSON.stringify({
    name: wfName.value,
    description: wfDesc.value,
    category: wfCat.value,
    steps: wfSteps.value.filter(s => s.command.trim())
  }, null, 2)
  wfJsonMode.value = true
}

function addStep() {
  wfSteps.value.push({ name: '', command: '', workingDir: '' })
}

function removeStep(idx: number) {
  wfSteps.value.splice(idx, 1)
}

async function pickWfDir(step: { workingDir: string }) {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const dir = await open({ directory: true, multiple: false, title: '选择工作目录' })
  if (dir && typeof dir === 'string') step.workingDir = dir
}

async function loadWfModels() {
  try {
    const json = await invoke<string>('get_ai_config')
    const cfg = JSON.parse(json)
    const modelsRaw = cfg['notes-ai-models']
    if (modelsRaw) {
      const models = JSON.parse(modelsRaw)
      const labels: string[] = []
      const map: Record<string, { provider: string; endpoint: string; apiKey: string; model: string }> = {}
      for (const m of models) {
        const subModels = (m.model || '').split(',').map((s: string) => s.trim()).filter(Boolean)
        for (const sm of subModels) {
          labels.push(`${m.name} (${sm})`)
          map[`${m.name} (${sm})`] = { provider: m.provider, endpoint: m.endpoint, apiKey: m.apiKey || '', model: sm }
        }
      }
      wfModelList.value = labels
      wfModelMap.value = map
      // 默认选中第一个
      if (labels.length > 0) {
        wfAiModel.value = labels[0]
        selectWfModel(labels[0])
      }
    }
    // fallback: 从单个配置读
    if (wfModelList.value.length === 0) {
      const p = cfg['notes-ai-provider'] || 'deepseek'
      const e = cfg['notes-ai-endpoint'] || 'https://api.deepseek.com'
      const k = cfg['notes-ai-apikey'] || ''
      const mo = cfg['notes-ai-model'] || 'deepseek-v4-pro'
      wfAiProvider.value = p
      wfAiEndpoint.value = e
      wfAiKey.value = k
      wfAiModel.value = `${p} ${mo}`
    }
  } catch { /* ignore */ }
}

function selectWfModel(label: string) {
  const m = wfModelMap.value[label]
  if (m) {
    wfAiProvider.value = m.provider
    wfAiEndpoint.value = m.endpoint
    wfAiKey.value = m.apiKey
    // wfAiModel 由 v-model 管理，这里只记录实际模型名供 AI 调用
  }
}

async function aiGenerateWorkflow() {
  wfAiMsg.value = ''
  const desc = wfDesc.value.trim()
  if (!desc) {
    wfAiMsg.value = '⚠️ 请先填写「说明」'
    return
  }
  const provider = wfAiProvider.value || 'deepseek'
  const endpoint = wfAiEndpoint.value || 'https://api.deepseek.com'
  const apiKey = wfAiKey.value
  const selectedCfg = wfModelMap.value[wfAiModel.value]
  const model = selectedCfg?.model || ''

  if (!apiKey && provider !== 'ollama') {
    wfAiMsg.value = '⚠️ 该模型未配置 API Key'
    return
  }
  if (!model && !wfAiModel.value) {
    wfAiMsg.value = '⚠️ 请选择一个模型'
    return
  }
  aiGenerating.value = true
  wfAiMsg.value = '⏳ 正在生成...'
  try {
    // 系统环境信息
    const ua = navigator.userAgent
    const isWin = ua.includes('Windows')
    const isWin11 = ua.includes('Windows NT 10.0') && ua.includes('.0') // Win11 also reports NT 10.0
    const osName = isWin
      ? (isWin11 ? 'Windows 11' : (ua.match(/Windows NT (\d+\.\d+)/)?.[1] === '10.0' ? 'Windows 10' : 'Windows'))
      : (ua.includes('Mac') ? 'macOS' : (ua.includes('Linux') ? 'Linux' : 'Unknown'))
    const arch = ua.includes('Win64') || ua.includes('x64') ? 'x86_64' : (ua.includes('ARM') ? 'ARM64' : 'x86')

    const prompt = `你是一个工作流 JSON 生成器。根据用户的描述，生成符合以下 schema 的 JSON：

## 运行环境
- 操作系统：${osName} (${arch})
- 默认 Shell：PowerShell（也兼容 cmd.exe）
- 可用工具：git, node, npm, npx, cargo, rustc, go, python, robocopy, xcopy, curl, tar, 7z
- 路径分隔符：\\（PowerShell 中路径可直接使用）
- 换行符：CRLF (\\r\\n)
- 编码：UTF-8

## JSON Schema
{
  "name": "工作流名称",
  "description": "描述",
  "category": "分类",
  "steps": [
    { "name": "步骤名", "command": "要执行的命令", "workingDir": "工作目录" }
  ]
}

## 示例
${WORKFLOW_SCHEMA_EXAMPLE}

## 用户需求
${desc}

## 约束
- 只输出 JSON，不要包含任何解释、代码块标记或其他文字
- 确保 steps 数组至少有一项
- 使用 PowerShell 语法（如 Copy-Item 而非 copy，New-Item 而非 mkdir）
- 文件名含空格或特殊字符时请用双引号包裹路径
- PowerShell 中路径的反斜杠无需转义`

    const headers: Record<string, string> = { 'Content-Type': 'application/json' }
    if (provider !== 'ollama') headers['Authorization'] = `Bearer ${apiKey}`

    const body = provider === 'ollama'
      ? JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], stream: false })
      : JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], temperature: 0.3, stream: false })

    const responseText = await invoke<string>('proxy_ai_request', {
      url: provider === 'ollama' ? `${endpoint}/api/chat` : `${endpoint}/chat/completions`,
      method: 'POST',
      headers: Object.entries(headers),
      body,
    })

    // 解析 AI 响应
    let jsonStr = ''
    if (provider === 'ollama') {
      const parsed = JSON.parse(responseText)
      jsonStr = parsed.message?.content || ''
    } else {
      const parsed = JSON.parse(responseText)
      jsonStr = parsed.choices?.[0]?.message?.content || ''
    }

    // 提取 JSON（去掉可能的代码块标记）
    const jsonMatch = jsonStr.match(/```(?:json)?\s*([\s\S]*?)```/) || jsonStr.match(/{[\s\S]*}/)
    const cleanJson = jsonMatch ? jsonMatch[1] || jsonMatch[0] : jsonStr

    // 验证 JSON 合法性
    const parsed = JSON.parse(cleanJson)
    if (!parsed.steps || !Array.isArray(parsed.steps)) {
      throw new Error('生成的 JSON 缺少 steps 数组')
    }

    // 填充到表单
    wfJsonText.value = JSON.stringify(parsed, null, 2)
    wfName.value = parsed.name || wfName.value
    wfDesc.value = parsed.description || wfDesc.value
    wfCat.value = parsed.category || wfCat.value
    wfSteps.value = (parsed.steps || []).map((s: any) => ({
      name: s.name || '',
      command: s.command || '',
      workingDir: s.workingDir || ''
    }))

    useStatusStore().pushMessage('✅ 工作流 JSON 已生成', 'success')
    wfAiMsg.value = ''
  } catch (e) {
    useStatusStore().pushMessage(`AI 生成失败: ${e}`, 'error')
    wfAiMsg.value = `❌ ${e}`
  } finally {
    aiGenerating.value = false
  }
}

const stepAiMsg = ref('')

async function aiGenStep(step: { name: string; command: string; workingDir: string }) {
  const desc = stepAiInput.value.trim()
  if (!desc) { stepAiMsg.value = '⚠️ 输入描述'; return }
  stepAiMsg.value = '⏳ 生成中...'
  const provider = wfAiProvider.value || 'deepseek'
  const endpoint = wfAiEndpoint.value || 'https://api.deepseek.com'
  const apiKey = wfAiKey.value
  const selectedCfg = wfModelMap.value[wfAiModel.value]
  const model = selectedCfg?.model || ''
  if (!apiKey && provider !== 'ollama') { stepAiMsg.value = '⚠️ 未配置 Key'; return }

  const prompt = `你是一个命令生成助手。用户需要一条在 Windows PowerShell 中执行的命令。
原命令（如果有）：${step.command || '无'}
用户需求：${desc}

请只输出命令本身，不要包含解释、代码块标记或其他文字。命令应在 PowerShell 中可执行。`

  try {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' }
    if (provider !== 'ollama') headers['Authorization'] = `Bearer ${apiKey}`
    const body = provider === 'ollama'
      ? JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], stream: false })
      : JSON.stringify({ model, messages: [{ role: 'user', content: prompt }], temperature: 0.3, stream: false })
    const url = provider === 'ollama' ? `${endpoint}/api/chat` : `${endpoint}/chat/completions`
    const text = await invoke<string>('proxy_ai_request', { url, method: 'POST', headers: Object.entries(headers), body })
    const parsed = JSON.parse(text)
    const raw = provider === 'ollama' ? (parsed.message?.content || '') : (parsed.choices?.[0]?.message?.content || '')
    const cmd = raw.replace(/```[\s\S]*?```/g, '').replace(/`([^`]+)`/g, '$1').trim().split('\n')[0].trim()
    if (cmd) { step.command = cmd; stepAiIdx = -1; stepAiMsg.value = ''; useStatusStore().pushMessage('✅ 命令已生成', 'success') }
    else { stepAiMsg.value = '⚠️ AI 未返回有效命令' }
  } catch (e) { stepAiMsg.value = `❌ ${e}` }
}

function saveWf() {
  let n = wfName.value.trim()
  if (!n) { useStatusStore().pushMessage('请输入工作流名称', 'warn'); return }
  let steps = wfSteps.value.filter(s => s.command.trim())
  if (wfJsonMode.value) {
    try {
      const parsed = JSON.parse(wfJsonText.value)
      n = parsed.name || n
      steps = (parsed.steps || []).filter((s: any) => s.command)
    } catch { useStatusStore().pushMessage('JSON 格式错误', 'error'); return }
  }
  if (steps.length === 0) { useStatusStore().pushMessage('请至少添加一个命令步骤', 'warn'); return }
  if (wfEditId.value) {
    // 直接替换整个 workflow 对象（强制触发响应式更新）
    const idx = store.workflows.findIndex(x => x.id === wfEditId.value)
    if (idx >= 0) {
      store.workflows[idx] = {
        ...store.workflows[idx],
        name: n,
        description: wfDesc.value.trim(),
        category: wfCat.value.trim() || '自定义',
        steps
      }
      // 立即持久化
      invoke('save_workflows', { workflowsJson: JSON.stringify(store.workflows) }).catch(e => console.error(e))
    }
    useStatusStore().pushMessage(`工作流「${n}」已保存`, 'success')
  } else {
    store.addWorkflow({
      name: n,
      description: wfDesc.value.trim(),
      category: wfCat.value.trim() || '自定义',
      steps
    })
  }
  showWfDlg.value = false
}

const wfSearch = ref('')
const wfTab = ref<'all' | 'freq' | 'fav'>('all')
const expandedCat = ref('')

const wfCats = computed(() => [...new Set(store.workflows.map(w => w.category))])

function wfsByCat(cat: string) {
  return store.workflows.filter(w => w.category === cat && matchesSearch(w))
}

function matchesSearch(w: { name: string; description: string; steps: Array<{ command: string }> }) {
  const q = wfSearch.value
  if (!q) return true
  return w.name.includes(q) || w.description.includes(q) || w.steps.some(s => s.command.includes(q))
}

const filteredFreq = computed(() =>
  store.frequentWorkflows.filter(w => matchesSearch(w))
)

const filteredFav = computed(() =>
  store.favWorkflows.filter(w => matchesSearch(w))
)

// Context menu
const wfCtxShow = ref(false)
const wfCtxPos = ref({ x: 0, y: 0 })
const wfCtxItem = ref<import('@/types').Workflow | null>(null)

function openWfCtx(e: MouseEvent, w: import('@/types').Workflow) {
  e.preventDefault()
  wfCtxPos.value = { x: e.clientX, y: e.clientY }
  wfCtxItem.value = w
  wfCtxShow.value = true
}

function closeWfCtx() { wfCtxShow.value = false }

function wfCtxEdit() {
  const w = wfCtxItem.value
  if (w) openWfDlg(w.id)
  closeWfCtx()
}

function wfCtxDel() {
  if (wfCtxItem.value) store.removeWorkflow(wfCtxItem.value.id)
  closeWfCtx()
}

function wfCtxFav() {
  if (wfCtxItem.value) store.toggleWfFav(wfCtxItem.value.id)
  closeWfCtx()
}

const allTools = [
  { type: 'json', name: 'JSON 格式化', desc: 'JSON 美化/压缩与校验', category: 'code', icon: 'json' },
  { type: 'regex', name: '正则测试器', desc: '正则表达式实时高亮测试', category: 'code', icon: 'regex' },
  { type: 'base64', name: 'Base64 转换', desc: 'Base64 字符串编码解码', category: 'code', icon: 'base64' },
  { type: 'uuid', name: 'UUID 生成器', desc: '批量生成 UUID v4', category: 'code', icon: 'uuid' },
  { type: 'url', name: 'URL 编解码', desc: 'URL encode / decode 转换', category: 'code', icon: 'base64' },
  { type: 'unicode', name: 'Unicode 转换', desc: 'Unicode / ASCII 互转查询', category: 'code', icon: 'base64' },
  { type: 'jwt', name: 'JWT 解码器', desc: '解析 JWT Token Header & Payload', category: 'code', icon: 'json' },
  { type: 'hash', name: '哈希计算', desc: 'MD5 / SHA 系列散列值计算', category: 'code', icon: 'uuid' },
  { type: 'html', name: 'HTML 转义', desc: '实体编码 &lt; &gt; &amp; 互转', category: 'code', icon: 'base64' },
  { type: 'sql', name: 'SQL 格式/压缩', desc: 'SQL 语句一键美化缩进与压缩', category: 'code', icon: 'json' },
  { type: 'diff', name: '代码对比 (Diff)', desc: '文本/配置文件双栏差异对比', category: 'code', icon: 'diff' },
  { type: 'color', name: '颜色转换器', desc: 'HEX/RGB/HSL 互转与取色预览', category: 'code', icon: 'color' },
  { type: 'img-base64', name: '图片转 Base64', desc: '本地图片与 Base64 互转及还原', category: 'code', icon: 'base64' },
  { type: 'qr', name: '二维码工具', desc: '指定内容生成与上传图片解析', category: 'code', icon: 'qr' },
  { type: 'port', name: '端口释放器', desc: '精准释放指定端口占用的进程', category: 'network', icon: 'network' },
  { type: 'dns', name: 'DNS 解析查询', desc: '域名 A/CNAME/AAAA/MX/TXT 解析 dig 查询', category: 'network', icon: 'network' },
  { type: 'env', name: '环境变量查看', desc: '查看系统所有环境变量并过滤', category: 'system', icon: 'system' },
  { type: 'timestamp', name: '时间戳转换', desc: 'Unix时间戳与本地日期互转', category: 'system', icon: 'timestamp' },
  { type: 'time-calc', name: '时间计算器', desc: '工作日偏移及日期时间差计算', category: 'system', icon: 'timestamp' },
  { type: 'cron', name: 'Cron 表达式生成', desc: '可视化 Cron 表达式点选生成与直白中文解析', category: 'system', icon: 'timestamp' },
  { type: 'radix', name: '进制转换', desc: '二/八/十/十六进制高精度转换', category: 'code', icon: 'uuid' },
  { type: 'case', name: '命名风格转换', desc: '下划线/驼峰/帕斯卡/烤串/常量命名互转', category: 'code', icon: 'base64' },
  { type: 'lorem', name: '占位假文生成', desc: '一键生成中英文假文段落填充UI', category: 'code', icon: 'json' },
  { type: 'lines', name: '文本行操作器', desc: '多行文本排序、去重、拆分与合并', category: 'code', icon: 'diff' },
  { type: 'aes-des', name: '对称加解密 (AES/DES)', desc: 'AES/DES 在线加解密与编码转换', category: 'code', icon: 'aes' },
  { type: 'rsa', name: '非对称加密 (RSA)', desc: 'RSA 密钥对生成、加解密与签名验签', category: 'code', icon: 'rsa' },
  { type: 'css', name: 'CSS 单位换算', desc: 'PX、REM、EM、VW、VH 实时联动转换', category: 'code', icon: 'color' },
  { type: 'svg', name: 'SVG 预览与优化', desc: 'SVG 实时图形渲染预览与源码精简压缩', category: 'code', icon: 'color' },
  { type: 'ssh', name: 'SSH 密钥生成', desc: '生成安全多算法 SSH 密钥对', category: 'system', icon: 'key' },
  { type: 'ssl', name: 'SSL 证书生成', desc: '生成开发测试用自签名 SSL 证书对', category: 'system', icon: 'cert' },
  { type: 'icon-generator', name: '图标生成器', desc: '一键生成多平台/尺寸 ICO/PNG/ICNS 图标包', category: 'code', icon: 'image' }
]

const toolSearchQuery = ref('')

const filteredTools = computed(() => {
  const q = toolSearchQuery.value.trim().toLowerCase()
  if (!q) return allTools
  return allTools.filter(t => t.name.includes(q) || t.desc.includes(q))
})

const categorizedTools = computed(() => {
  const map: Record<string, typeof allTools> = { code: [], network: [], system: [] }
  filteredTools.value.forEach(t => {
    if (map[t.category]) {
      map[t.category].push(t)
    }
  })
  return map
})

const recentUsedTools = computed(() => {
  return store.recentTools.map(type => allTools.find(t => t.type === type)).filter(Boolean) as typeof allTools
})

function switchTab(tab: 'projects'|'workflows'|'tools'|'notes') {
  activeTab.value = tab
  store.sidebarTab = tab
}

function handleGlobalClick() {
  closeProjCtx()
  closeCmdCtx()
  closeWfCtx()
}

onMounted(() => {
  document.addEventListener('click', handleGlobalClick)
})

onUnmounted(() => {
  document.removeEventListener('click', handleGlobalClick)
})
</script>

<template>
  <aside class="side">
    <div class="side-head"></div>
    <div class="tabs" role="tablist">
      <div :class="['tab',{on:activeTab==='projects'}]" role="tab" :aria-selected="activeTab==='projects'" tabindex="0" @click="switchTab('projects')" @keyup.enter="switchTab('projects')">项目</div>
      <div :class="['tab',{on:activeTab==='workflows'}]" role="tab" :aria-selected="activeTab==='workflows'" tabindex="0" @click="switchTab('workflows')" @keyup.enter="switchTab('workflows')">快捷</div>
      <div :class="['tab',{on:activeTab==='tools'}]" role="tab" :aria-selected="activeTab==='tools'" tabindex="0" @click="switchTab('tools')" @keyup.enter="switchTab('tools')">工具</div>
      <div :class="['tab',{on:activeTab==='notes'}]" role="tab" :aria-selected="activeTab==='notes'" tabindex="0" @click="switchTab('notes')" @keyup.enter="switchTab('notes')">笔记</div>
    </div>

    <!-- Projects -->
    <div v-show="activeTab==='projects'" class="panel">
      <div class="bar"><button class="btn" @click="showAdd=!showAdd">{{ showAdd?'收起':'+ 添加项目' }}</button></div>
      <div v-if="showAdd" class="add-panel">
        <input v-model="newName" placeholder="项目名称" @keyup.enter="handleAdd" />
        <div class="row"><input v-model="newDir" placeholder="项目目录" style="flex:1;min-width:0" /><button class="btn" @click="pickDir">...</button></div>
        <div v-if="detectedLang" style="font-size:11px;color:var(--jc-color-success)">识别: {{ detectedLang }} · {{ detectedCmds.length }} 命令</div>
        <button class="btn pri" @click="handleAdd">添加</button>
      </div>
      <div class="tree">
        <div v-for="p in store.projects" :key="p.id">
          <div class="proj" :class="{sel:store.selectedProjectId===p.id}" @click="toggleExpand(p.id);store.selectedProjectId=p.id" @contextmenu="openProjCtx($event,p.id)">
            <template v-if="editingProjId===p.id">
              <input class="proj-edit-input" v-model="editProjName" @keyup.enter="confirmRenameProj" @keyup.escape="editingProjId=''" @blur="confirmRenameProj" @click.stop />
            </template>
            <template v-else>
            <span class="arrow">{{ expandedProjects.has(p.id)?'▾':'▸' }}</span><span class="pn">{{ p.name }}</span><span class="pc">{{ p.commands.length }}</span>
            <button class="del" @click.stop="store.removeProject(p.id)">✕</button>
            </template>
          </div>
          <div v-if="expandedProjects.has(p.id)" class="cmds">
            <div v-for="cmd in p.commands" :key="cmd.id" class="cmd" :class="{on:isRunning(p.id,cmd.id)}" @contextmenu="openCmdCtx($event,p.id,cmd)">
              <template v-if="editingCmdId===p.id+'::'+cmd.id">
                <input class="cmd-edit-input" v-model="editCmdName" @keyup.enter="confirmRenameCmd" @keyup.escape="editingCmdId=''" @blur="confirmRenameCmd" @click.stop />
              </template>
              <template v-else>
              <span class="dot" :class="{live:isRunning(p.id,cmd.id)}"></span>
              <span class="cn" @click="store.startCommand(p.id,cmd)" @dblclick="editCmd(p.id,cmd)" :title="cmd.command">{{ cmd.name }}</span>
              <button v-if="isRunning(p.id,cmd.id)" class="stop" @click.stop="store.stopCommand(p.id,cmd.id)">■</button>
              <button class="del" @click.stop="store.removeCommand(p.id,cmd.id)">✕</button>
              </template>
            </div>
            <button class="addc" @click="addQuickCmd(p.id)">+ 命令</button>
          </div>
        </div>
        <div v-if="store.projects.length===0&&!showAdd" class="empty">点击 + 添加项目</div>
      </div>
      <CommandDialog ref="cmdDialogRef" :project-id="dialogProjectId" :editing="editingCmd" @close="editingCmd=null" />
    </div>

    <!-- Workflows (多命令顺序执行) -->
    <div v-show="activeTab==='workflows'" class="panel" style="display:flex;flex-direction:column">
      <div class="bar">
        <button class="btn" @click="openWfDlg()">+ 新建工作流</button>
        <span v-if="store.workflowRunning" class="wf-badge">运行中...</span>
      </div>
      <div class="tabs">
        <div :class="['tab',{on:wfTab==='all'}]" @click="wfTab='all'">全部</div>
        <div :class="['tab',{on:wfTab==='freq'}]" @click="wfTab='freq'">常用</div>
        <div :class="['tab',{on:wfTab==='fav'}]" @click="wfTab='fav'">收藏</div>
      </div>
      <div style="flex:1;overflow-y:auto">
        <template v-if="wfTab==='all'">
          <div v-for="cat in wfCats" :key="cat" style="border-bottom:1px solid var(--jc-border-default)">
            <div class="scat" @click="expandedCat = expandedCat===cat?'':cat">{{ expandedCat===cat?'▾':'▸'}} {{ cat }}</div>
            <div v-if="expandedCat===cat">
              <div v-for="w in wfsByCat(cat)" :key="w.id" class="sc" @click="store.runWorkflow(w.id)" @contextmenu="openWfCtx($event,w)">
                <span class="fav-star" v-if="w.favorite">★</span>
                <span class="scc">{{ w.name }}</span>
                <span class="scd">{{ w.steps.length }}步</span>
              </div>
            </div>
          </div>
        </template>
        <template v-if="wfTab==='freq'">
          <div v-for="w in filteredFreq" :key="w.id" class="sc" @click="store.runWorkflow(w.id)" @contextmenu="openWfCtx($event,w)">
            <span class="fav-star" v-if="w.favorite">★</span>
            <span class="scc">{{ w.name }}</span><span class="scd">{{ w.useCount }}次</span>
          </div>
        </template>
        <template v-if="wfTab==='fav'">
          <div v-for="w in filteredFav" :key="w.id" class="sc" @click="store.runWorkflow(w.id)" @contextmenu="openWfCtx($event,w)">
            <span class="fav-star">★</span>
            <span class="scc">{{ w.name }}</span>
          </div>
        </template>
      </div>
      <div style="padding:4px 6px;border-top:1px solid var(--jc-border-default);flex-shrink:0">
        <input v-model="wfSearch" placeholder="搜索工作流..." style="width:100%;font-size:11px;padding:3px 6px" />
      </div>
    </div>

    <!-- Tools -->
    <div v-show="activeTab==='tools'" class="panel" style="display:flex;flex-direction:column">
      <!-- 搜索过滤 -->
      <div class="search-bar">
        <input v-model="toolSearchQuery" placeholder="搜索实用工具..." class="tool-search-input" />
      </div>

      <div class="tools-list-container">
        <!-- 最近使用 -->
        <div v-if="recentUsedTools.length > 0 && !toolSearchQuery" class="tools-section">
          <div class="section-title">最近使用</div>
          <div class="tools-row-grid">
            <button v-for="t in recentUsedTools" :key="'rec-'+t.type" class="tool-item-card" @click="store.openTool(t.type, t.name)" :title="t.desc">
              <span class="tool-icon">
                <svg v-if="t.icon === 'json'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 19a1 1 0 0 1-1 1H7a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2a1 1 0 0 1 1 1"/><path d="M14 19a1 1 0 0 0 1 1h2a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2a1 1 0 0 0-1 1"/></svg>
                <svg v-else-if="t.icon === 'regex'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m5 5 14 14"/></svg>
                <svg v-else-if="t.icon === 'base64'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 3 4 4-4 4M21 7H3M7 21l-4-4 4-4M3 17h18"/></svg>
                <svg v-else-if="t.icon === 'uuid'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 21V3M17 21V3M3 12h18"/></svg>
                <svg v-else-if="t.icon === 'diff'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2z"/><path d="M9 7v10M5 12h8"/></svg>
                <svg v-else-if="t.icon === 'color'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
                <svg v-else-if="t.icon === 'qr'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="8" x="3" y="3" rx="1"/><rect width="8" height="8" x="13" y="3" rx="1"/><rect width="8" height="8" x="3" y="13" rx="1"/><path d="M13 13h1v1h-1zM18 13h3v3h-3zM13 18h3v3h-3zM18 18h1v1h-1z"/></svg>
                <svg v-else-if="t.icon === 'network'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>
                <svg v-else-if="t.icon === 'system'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8M12 17v4M6 8l4 4-4 4"/></svg>
                <svg v-else-if="t.icon === 'timestamp'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
                <svg v-else-if="t.icon === 'key'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 1.5 1.5M15.5 7.5 14 6"/></svg>
                <svg v-else-if="t.icon === 'cert'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                <svg v-else-if="t.icon === 'image'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
                <svg v-else-if="t.icon === 'aes'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20 L12 4 L20 20 M8 13 H16" /></svg>
                <svg v-else-if="t.icon === 'rsa'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20 V4 H13 C17 4 17 11 13 11 H4 M12 11 L19 20" /></svg>
              </span>
              <div class="tool-info">
                <div class="tool-name">{{ t.name }}</div>
              </div>
            </button>
          </div>
        </div>

        <!-- 编码工具 -->
        <div v-if="categorizedTools.code.length > 0" class="tools-section">
          <div class="section-title">编码工具 (CODE)</div>
          <div class="tools-flex-list">
            <button v-for="t in categorizedTools.code" :key="t.type" class="tool-item-line" @click="store.openTool(t.type, t.name)">
              <div class="tool-meta-left">
                <span class="tool-icon">
                  <svg v-if="t.icon === 'json'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 19a1 1 0 0 1-1 1H7a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2a1 1 0 0 1 1 1"/><path d="M14 19a1 1 0 0 0 1 1h2a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-2a1 1 0 0 0-1 1"/></svg>
                  <svg v-else-if="t.icon === 'regex'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m5 5 14 14"/></svg>
                  <svg v-else-if="t.icon === 'base64'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m17 3 4 4-4 4M21 7H3M7 21l-4-4 4-4M3 17h18"/></svg>
                  <svg v-else-if="t.icon === 'uuid'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 21V3M17 21V3M3 12h18"/></svg>
                  <svg v-else-if="t.icon === 'diff'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2z"/><path d="M9 7v10M5 12h8"/></svg>
                  <svg v-else-if="t.icon === 'color'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
                  <svg v-else-if="t.icon === 'qr'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="8" height="8" x="3" y="3" rx="1"/><rect width="8" height="8" x="13" y="3" rx="1"/><rect width="8" height="8" x="3" y="13" rx="1"/><path d="M13 13h1v1h-1zM18 13h3v3h-3zM13 18h3v3h-3zM18 18h1v1h-1z"/></svg>
                  <svg v-else-if="t.icon === 'image'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
                  <svg v-else-if="t.icon === 'aes'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20 L12 4 L20 20 M8 13 H16" /></svg>
                  <svg v-else-if="t.icon === 'rsa'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20 V4 H13 C17 4 17 11 13 11 H4 M12 11 L19 20" /></svg>
                </span>
                <div class="tool-text-wrap">
                  <div class="tool-name">{{ t.name }}</div>
                  <div class="tool-desc">{{ t.desc }}</div>
                </div>
              </div>
              <span class="tool-shortcut-tag">Alt+{{ allTools.findIndex(x => x.type === t.type) + 1 }}</span>
            </button>
          </div>
        </div>

        <!-- 网络工具 -->
        <div v-if="categorizedTools.network.length > 0" class="tools-section">
          <div class="section-title">网络工具 (NETWORK)</div>
          <div class="tools-flex-list">
            <button v-for="t in categorizedTools.network" :key="t.type" class="tool-item-line" @click="store.openTool(t.type, t.name)">
              <div class="tool-meta-left">
                <span class="tool-icon">
                  <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>
                </span>
                <div class="tool-text-wrap">
                  <div class="tool-name">{{ t.name }}</div>
                  <div class="tool-desc">{{ t.desc }}</div>
                </div>
              </div>
              <span class="tool-shortcut-tag">Alt+{{ allTools.findIndex(x => x.type === t.type) + 1 }}</span>
            </button>
          </div>
        </div>

        <!-- 系统工具 -->
        <div v-if="categorizedTools.system.length > 0" class="tools-section">
          <div class="section-title">系统工具 (SYSTEM)</div>
          <div class="tools-flex-list">
            <button v-for="t in categorizedTools.system" :key="t.type" class="tool-item-line" @click="store.openTool(t.type, t.name)">
              <div class="tool-meta-left">
                <span class="tool-icon">
                  <svg v-if="t.icon === 'system'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8M12 17v4M6 8l4 4-4 4"/></svg>
                  <svg v-else-if="t.icon === 'timestamp'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
                  <svg v-else-if="t.icon === 'key'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 1.5 1.5M15.5 7.5 14 6"/></svg>
                  <svg v-else-if="t.icon === 'cert'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
                </span>
                <div class="tool-text-wrap">
                  <div class="tool-name">{{ t.name }}</div>
                  <div class="tool-desc">{{ t.desc }}</div>
                </div>
              </div>
              <span class="tool-shortcut-tag">Alt+{{ allTools.findIndex(x => x.type === t.type) + 1 }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Notes -->
    <div v-show="activeTab==='notes'" class="panel">
      <NoteSidebar />
    </div>

    <Teleport to="body">
      <div v-if="projCtxShow" class="ctx" :style="{left:projCtxPos.x+'px',top:projCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxRenameProj">重命名</div>
        <div class="ci" @click="ctxAddCmd">新增命令</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelProj">删除项目</div>
      </div>
    </Teleport>
    <Teleport to="body">
      <div v-if="cmdCtxShow" class="ctx" :style="{left:cmdCtxPos.x+'px',top:cmdCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="ctxEditCmd">编辑</div>
        <div class="ci" @click="ctxRenameCmd">重命名</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="ctxDelCmd">删除</div>
      </div>
    </Teleport>
    <!-- 工作流编辑对话框 -->
    <Teleport to="body">
      <div v-if="showWfDlg" class="mbg" @mousedown.self="showWfDlg=false">
        <div class="mw" style="width:560px">
          <div class="mt" style="display:flex;align-items:center;justify-content:space-between">
            <span>{{ wfEditId ? '编辑工作流' : '新建工作流' }}</span>
            <button class="btn" style="font-size:10px;padding:2px 8px" @click="wfJsonMode ? applyJsonToForm() : syncFormToJson()">
              {{ wfJsonMode ? '📋 应用 JSON' : '✏ JSON 编辑' }}
            </button>
          </div>
          <div class="mb">
            <template v-if="!wfJsonMode">
              <div class="fld"><label>名称</label><input v-model="wfName" class="wf-input" placeholder="如: 编译并运行" autofocus /></div>
              <div class="fld"><label>分类</label><input v-model="wfCat" class="wf-input" placeholder="如: Go / Tauri" /></div>
              <div class="fld">
                <label>说明</label>
                <div style="display:flex;gap:4px">
                  <input v-model="wfDesc" class="wf-input" placeholder="描述需求，让 AI 生成工作流" style="flex:1" />
                  <button class="btn" style="font-size:11px;white-space:nowrap;padding:2px 10px" @click="aiGenerateWorkflow" :disabled="aiGenerating">
                    {{ aiGenerating ? '⏳...' : '🤖 AI 生成' }}
                  </button>
                </div>
                <div style="display:flex;gap:4px;margin-top:4px;align-items:center">
                  <select v-if="wfModelList.length > 0" v-model="wfAiModel" style="flex:1;background:var(--jc-bg-input);color:var(--jc-text-primary);border:1px solid var(--jc-border-strong);font-size:11px;padding:3px 4px" @change="selectWfModel(wfAiModel)">
                    <option v-for="l in wfModelList" :key="l" :value="l">{{ l }}</option>
                  </select>
                  <span v-else style="font-size:10px;color:var(--jc-text-secondary)">未找到模型配置，请先在设置中添加</span>
                </div>
                <div v-if="wfAiMsg" style="margin-top:4px;font-size:11px;color:var(--jc-text-highlight)">{{ wfAiMsg }}</div>
              </div>
              <div class="wf-section-label">命令步骤（顺序执行）</div>
              <div v-for="(step, idx) in wfSteps" :key="idx" class="wf-step-card">
                <div class="wf-step-header">
                  <span class="wf-step-num">#{{ idx+1 }}</span>
                  <input v-model="step.name" class="wf-input wf-step-name" placeholder="步骤名称" />
                  <button class="wf-step-del" @click="removeStep(idx)">✕</button>
                </div>
                <textarea v-model="step.command" class="wf-textarea" placeholder="命令（如 go build -o app.exe .）" rows="2" />
                <button class="wf-step-ai" @click="stepAiIdx = idx; stepAiInput = step.command">🤖 AI</button>
                <div class="wf-step-footer">
                  <input v-model="step.workingDir" class="wf-input wf-dir" placeholder="工作目录（点击📁选择）" />
                  <button class="wf-dir-pick" @click="pickWfDir(step)">📁</button>
                </div>
                <div v-if="stepAiIdx === idx" class="wf-step-ai-box">
                  <input v-model="stepAiInput" placeholder="描述需要的命令" class="wf-input" style="flex:1;font-size:11px" />
                  <button class="btn" style="font-size:10px;padding:2px 6px" @click="aiGenStep(step)">生成</button>
                  <button class="btn" style="font-size:10px;padding:2px 6px" @click="stepAiIdx = -1">✕</button>
                </div>
              </div>
              <button class="btn wf-add-step" @click="addStep">+ 添加步骤</button>
            </template>
            <template v-else>
              <div class="fld"><label>名称</label><input v-model="wfName" class="wf-input" placeholder="工作流名称" /></div>
              <div class="fld"><label>说明</label><input v-model="wfDesc" class="wf-input" placeholder="描述需求，让 AI 生成 JSON" /></div>
              <div style="margin-top:6px">
                <div class="wf-section-label">JSON 定义</div>
                <textarea v-model="wfJsonText" class="wf-textarea wf-json-editor" rows="12" spellcheck="false" />
                <button class="btn" style="width:100%;margin-top:4px;font-size:11px" @click="aiGenerateWorkflow" :disabled="aiGenerating">
                  {{ aiGenerating ? '⏳ 生成中...' : 'AI 按描述生成' }}
                </button>
              </div>
            </template>
            <div class="acts wf-acts">
              <button class="btn" @click="showWfDlg=false">取消</button>
              <button class="btn pri" @click="saveWf">{{ wfEditId ? '保存' : '创建' }}</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
    <!-- 工作流右键菜单 -->
    <Teleport to="body">
      <div v-if="wfCtxShow" class="ctx" :style="{left:wfCtxPos.x+'px',top:wfCtxPos.y+'px'}" @click.stop>
        <div class="ci" @click="wfCtxEdit">编辑</div>
        <div class="ci" @click="wfCtxFav">{{ wfCtxItem?.favorite?'取消收藏':'收藏' }}</div>
        <div class="ci" style="color:var(--jc-color-error)" @click="wfCtxDel">删除</div>
      </div>
    </Teleport>
  </aside>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;
.side { width:210px; min-width:210px; height:100%; background:var(--jc-bg-panel); display:flex; flex-direction:column; overflow:hidden; user-select:none; }
.side-head { height:2px; background:var(--jc-color-accent); }
.tabs { display:flex; }
.tab { @include tab-base; }
.panel { @include flex-panel; }
.bar { padding:6px 10px; border-bottom:1px solid var(--jc-border-default); }
.btn { @include btn-base; }
.btn.pri { @include btn-primary; }
.btn:disabled { opacity:.5; }
.add-panel { padding:8px 10px; display:flex; flex-direction:column; gap:5px; border-bottom:1px solid var(--jc-border-default);
  input { @include input-base; }
}
.row { display:flex; gap:4px; }
.tree { flex:1; overflow-y:auto; padding:4px 0; }
.proj { display:flex; align-items:center; gap:4px; padding:4px 10px; cursor:pointer; font-size:12px;
  &:hover { background:var(--jc-bg-hover); }
  &.sel { background:var(--jc-bg-selected); }
}
.arrow { font-size:9px; color:var(--jc-text-secondary); width:12px; flex-shrink:0; }
.pn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.pc { font-size:10px; color:var(--jc-text-secondary); background:var(--jc-bg-btn); padding:0 4px; border-radius:3px; }
.del { display:none; background:none; color:var(--jc-text-secondary); font-size:12px; padding:0 4px; cursor:pointer;
  &:hover { color:var(--jc-color-error); }
}
.proj:hover .del,.cmd:hover .del { display:inline; }
.cmds { padding-left:12px; }
.cmd { display:flex; align-items:center; gap:4px; padding:3px 10px; font-size:12px;
  &:hover { background:var(--jc-bg-hover); }
  &.on { background:var(--jc-bg-selected); }
}
.dot { @include dot; }
.cn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; cursor:pointer;
  &:hover { color:var(--jc-color-success); }
}
.stop { background:none; color:var(--jc-color-error); font-size:10px; padding:0 3px; cursor:pointer; }
.addc { display:block; width:100%; text-align:left; background:none; border:none; color:var(--jc-text-secondary); font-size:11px; padding:3px 10px; cursor:pointer;
  &:hover { color:var(--jc-color-success); }
}
.empty { padding:20px; text-align:center; font-size:11px; color:var(--jc-text-secondary); }
input { @include input-base; }
.ctx { @include ctx-menu; }
.ci { @include ctx-item; }
.proj-edit-input, .cmd-edit-input { background:var(--jc-bg-input); border:1px solid var(--jc-color-accent); color:var(--jc-text-primary); padding:1px 4px; font-size:12px; width:100%; outline:none; }
.scat { padding:6px 10px; font-size:11px; font-weight:600; color:var(--jc-text-highlight); cursor:pointer; background:var(--jc-bg-elevated);
  &:hover { background:var(--jc-bg-selected); }
}
.sc { padding:4px 10px 4px 20px; font-size:11px; cursor:pointer; color:var(--jc-text-secondary); display:flex; align-items:center;
  &:hover { background:var(--jc-bg-hover); color:var(--jc-color-success); }
}
.scc { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-family:'Cascadia Code',Consolas,monospace; }
.scd { font-size:9px; color:var(--jc-text-secondary); margin-left:4px; white-space:nowrap; }
.fav-star { color:var(--jc-color-favorite); margin-right:2px; font-size:10px; }
.mbg { position:fixed; inset:0; background:var(--jc-bg-overlay); display:flex; align-items:center; justify-content:center; z-index:1000; }
.mw { background:var(--jc-bg-elevated); border:1px solid var(--jc-border-strong); min-width:400px; box-shadow:var(--jc-shadow-modal); }
.mt { background:var(--jc-bg-panel); padding:10px 16px; font-size:14px; font-weight:600; color:var(--jc-text-highlight); border-bottom:1px solid var(--jc-border-default); }
.mb { padding:16px; display:flex; flex-direction:column; gap:12px; max-height:70vh; overflow-y:auto; }
.fld { display:flex; flex-direction:column; gap:4px;
  label { font-size:11px; color:var(--jc-text-secondary); text-transform:uppercase; letter-spacing:.5px; }
  input { @include input-base; padding:6px 10px; font-size:13px; }
}

// ── 工作流对话框（Teleport 到 body，用 :global 确保样式穿透）──
:global(.wf-input) { @include input-base; }
:global(.wf-textarea) { @include input-base; font-family:'Cascadia Code',Consolas,monospace; resize:vertical; width:100%; font-size:11px; }
:global(.wf-json-editor) { margin-top:4px; }
:global(.wf-step-card) { border:1px solid var(--jc-border-default); border-radius:4px; padding:6px; margin-bottom:4px; }
:global(.wf-step-header) { display:flex; gap:4px; align-items:center; margin-bottom:4px; }
:global(.wf-step-num) { font-size:10px; color:var(--jc-text-secondary); }
:global(.wf-step-name) { flex:1; }
:global(.wf-step-del) { background:none; border:none; font-size:12px; color:var(--jc-color-error); cursor:pointer; padding:2px 6px; }
:global(.wf-step-footer) { margin-top:4px; display:flex; gap:4px; align-items:center; }
:global(.wf-dir) { flex:1; font-size:10px; }
:global(.wf-dir-pick) { background:none; border:none; cursor:pointer; font-size:13px; padding:2px 4px; flex-shrink:0; }
:global(.wf-step-ai) { background:none; border:none; color:var(--jc-color-accent); cursor:pointer; font-size:11px; padding:2px 6px; margin-top:2px; width:100%; text-align:left; }
:global(.wf-step-ai):hover { background:var(--jc-bg-hover); }
:global(.wf-step-ai-box) { display:flex; gap:4px; align-items:center; margin-top:4px; }
:global(.wf-section-label) { font-size:11px; font-weight:600; color:var(--jc-text-highlight); margin:8px 0 4px; }
:global(.wf-add-step) { width:100%; margin-top:4px; }
:global(.wf-acts) { margin-top:8px; }

.acts { display:flex; justify-content:flex-end; gap:8px; margin-top:4px; }
.search-bar {
  padding: 6px 10px;
  border-bottom: 1px solid var(--jc-border-default);
  flex-shrink: 0;
}
.tool-search-input {
  width: 100%;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 4px 8px;
  font-size: 11px;
  outline: none;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.tools-list-container {
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.tools-section {
  display: flex;
  flex-direction: column;
}
.section-title {
  font-size: 9px;
  font-weight: 600;
  color: var(--jc-text-secondary);
  padding: 0 10px 4px 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.tools-row-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 6px;
  padding: 0 10px;
}
.tool-item-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  cursor: pointer;
  color: var(--jc-text-primary);
  width: 100%;
  &:hover {
    background: var(--jc-bg-hover);
    border-color: var(--jc-color-accent);
    .tool-icon {
      color: var(--jc-color-accent-hover);
    }
  }
  .tool-icon {
    color: var(--jc-color-accent);
    display: flex;
    align-items: center;
  }
  .tool-name {
    font-size: 10px;
    font-weight: 600;
    text-align: center;
  }
}
.tools-flex-list {
  display: flex;
  flex-direction: column;
}
.tool-item-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: none;
  border: none;
  width: 100%;
  cursor: pointer;
  color: var(--jc-text-primary);
  text-align: left;
  &:hover {
    background: var(--jc-bg-hover);
    .tool-icon {
      color: var(--jc-color-accent-hover);
    }
    .tool-name {
      color: var(--jc-text-highlight);
    }
  }
}
.tool-meta-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  width: 80%;
}
.tool-icon {
  color: var(--jc-text-secondary);
  display: flex;
  align-items: center;
  flex-shrink: 0;
}
.tool-text-wrap {
  display: flex;
  flex-direction: column;
  min-width: 0;
  width: 100%;
}
.tool-name {
  font-size: 11px;
  font-weight: 600;
  color: var(--jc-text-primary);
}
.tool-desc {
  font-size: 9px;
  color: var(--jc-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  width: 100%;
}
.tool-shortcut-tag {
  font-size: 8px;
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-secondary);
  padding: 1px 3px;
  border-radius: 3px;
  font-family: Consolas, monospace;
}
</style>
