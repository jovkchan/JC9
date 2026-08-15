import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Automation, BlockNode, Edge, CredentialMeta, CredentialInput, RunLog, RunStepLog } from '@/types/automation'
import { getBlockDef, defaultsFromFields, getBlockLabel } from '@/components/automation/blocks/palette'
import { useStatusStore } from '@/stores/status'

/** 结构级快照（撤回/重做用） */
interface Snapshot {
  name: string
  description: string
  nodes: BlockNode[]
  edges: Edge[]
  variables: Automation['variables']
}

function genId() { return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2, 9)}` }

// ── F1b 真实持久化：Tauri 可用 → 走 Rust（~/.jc9/data/*.json）；否则 localStorage 兜底（dev/浏览器）──
const LS_AUTOMATIONS = 'jc9-automations'
const LS_CREDENTIALS = 'jc9-credentials'
const LS_LOGS = 'jc9-automation-logs'
function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function blankAutomation(): Automation {
  const now = new Date().toISOString()
  return {
    id: genId(),
    name: '未命名工作积木',
    description: '',
    version: 2,
    nodes: [],
    edges: [],
    variables: {},
    createdAt: now,
    updatedAt: now,
  }
}

/**
 * 自动化板块 store（独立，不依赖 project.ts 的工作流逻辑）
 * F1a：本地内存 mock；F1b 接 invokeTransport（见 docs/plans §7.3/§7.4）
 */
export const useAutomationStore = defineStore('automation', () => {
  const automations = ref<Automation[]>([])
  const currentId = ref<string | null>(null)
  /** false = 列表视图，true = 编辑器视图 */
  const editing = ref(false)
  const search = ref('')

  const current = computed(() => automations.value.find(a => a.id === currentId.value) ?? null)
  const filtered = computed(() => {
    const s = search.value.trim().toLowerCase()
    if (!s) return automations.value
    return automations.value.filter(a =>
      a.name.toLowerCase().includes(s) || a.description.toLowerCase().includes(s)
    )
  })

  // ── 历史（撤回/重做，≤50 步，结构级快照）──
  const past = ref<Snapshot[]>([])
  const future = ref<Snapshot[]>([])
  const dirty = ref(false)
  const HISTORY_LIMIT = 50

  function markDirty() { dirty.value = true }

  function snapshot(): Snapshot {
    const a = current.value
    return {
      name: a?.name ?? '',
      description: a?.description ?? '',
      nodes: JSON.parse(JSON.stringify(a?.nodes ?? [])),
      edges: JSON.parse(JSON.stringify(a?.edges ?? [])),
      variables: JSON.parse(JSON.stringify(a?.variables ?? {})),
    }
  }

  function restore(s: Snapshot) {
    const a = current.value
    if (!a) return
    a.name = s.name
    a.description = s.description
    a.nodes = s.nodes
    a.edges = s.edges
    a.variables = s.variables
    a.updatedAt = new Date().toISOString()
    dirty.value = true
  }

  /** 每次用户编辑前调用：把当前状态压入 past，清空 future */
  function pushHistory() {
    const a = current.value
    if (!a) return
    past.value.push(snapshot())
    if (past.value.length > HISTORY_LIMIT) past.value.shift()
    future.value = []
    dirty.value = true
  }

  function undo() {
    if (past.value.length === 0) return
    future.value.push(snapshot())
    const s = past.value.pop()
    if (s) restore(s)
  }

  function redo() {
    if (future.value.length === 0) return
    past.value.push(snapshot())
    if (past.value.length > HISTORY_LIMIT) past.value.shift()
    const s = future.value.pop()
    if (s) restore(s)
  }

  const canUndo = computed(() => past.value.length > 0)
  const canRedo = computed(() => future.value.length > 0)

  /** 手动保存（无自动保存）：真实写入后端 JSON / localStorage */
  async function save() {
    const a = current.value
    if (!a) return
    a.updatedAt = new Date().toISOString()
    dirty.value = false
    await persist()
    useStatusStore().pushMessage(`已保存「${a.name}」`, 'success')
  }

  /** 结构校验（保存/返回前提示）：返回问题列表（空 = 通过）
   * 规则：必须有入口（start 或 manual-trigger，否则无法运行）；start 至多一个；建议有 end */
  function structureIssues(): string[] {
    const a = current.value
    if (!a || a.nodes.length === 0) return []
    const issues: string[] = []
    const types = a.nodes.map(n => n.type)
    const startCount = types.filter(t => t === 'start').length
    const hasManual = types.includes('manual-trigger')
    const hasEnd = types.includes('end')
    if (startCount === 0 && !hasManual) issues.push('缺少「开始」或「手动触发」入口块（运行需要入口）')
    if (startCount > 1) issues.push(`有 ${startCount} 个「开始」块（运行只取第一个）`)
    if (!hasEnd) issues.push('缺少「结束」块（建议各分支都连到结束）')
    return issues
  }

  /** 把全部自动化的最新数据写入后端/localStorage */
  async function persist() {
    if (isTauri()) {
      try {
        await invoke('automation_save', { automationsJson: JSON.stringify(automations.value) })
        return
      } catch (e) { console.error('automation_save failed', e) }
    }
    try { localStorage.setItem(LS_AUTOMATIONS, JSON.stringify(automations.value)) } catch (e) { console.error(e) }
  }

  /** 启动加载：从后端 / localStorage 恢复自动化列表（已加载则跳过，避免覆盖内存编辑） */
  const loaded = ref(false)
  async function load(force = false) {
    if (loaded.value && !force) return
    let data: Automation[] | null = null
    if (isTauri()) {
      try {
        const json = await invoke<string>('automation_list')
        data = JSON.parse(json)
      } catch (e) { console.error('automation_list failed', e) }
    }
    if (!data) {
      const ls = localStorage.getItem(LS_AUTOMATIONS)
      try { data = ls ? JSON.parse(ls) : null } catch { data = null }
    }
    if (data && Array.isArray(data)) automations.value = data
    else automations.value = []
    if (automations.value.length === 0) automations.value.push(blankAutomation())
    loaded.value = true
  }

  function open(id: string) {
    currentId.value = id
    editing.value = true
    past.value = []
    future.value = []
    dirty.value = false
  }

  function create() {
    const a = blankAutomation()
    automations.value.push(a)
    currentId.value = a.id
    editing.value = true
    past.value = []
    future.value = []
    dirty.value = false
    return a
  }

  async function remove(id: string) {
    automations.value = automations.value.filter(x => x.id !== id)
    if (currentId.value === id) {
      currentId.value = null
      editing.value = false
    }
    if (isTauri()) {
      try { await invoke('automation_delete', { id }) } catch (e) { console.error('automation_delete failed', e) }
    }
    await persist()
  }

  function closeEditor() {
    editing.value = false
    currentId.value = null
  }

  function rename(id: string, name: string) {
    const a = automations.value.find(x => x.id === id)
    if (a) { a.name = name; a.updatedAt = new Date().toISOString(); markDirty() }
  }

  /** 更新工作积木描述（编辑器标题栏可编辑） */
  function setDescription(id: string, desc: string) {
    const a = automations.value.find(x => x.id === id)
    if (a) { a.description = desc; a.updatedAt = new Date().toISOString(); markDirty() }
  }

  /** 从积木面板添加一个积木到当前自动化画布 */
  function addNode(type: string): BlockNode | null {
    const a = current.value
    if (!a) return null
    pushHistory()
    const def = getBlockDef(type)
    const node: BlockNode = {
      id: genId(),
      type,
      category: def?.category ?? 'terminal',
      x: 120 + (a.nodes.length % 5) * 220,
      y: 80 + Math.floor(a.nodes.length / 5) * 140,
      config: defaultsFromFields(def?.fields ?? []),
    }
    a.nodes.push(node)
    a.updatedAt = new Date().toISOString()
    return node
  }

  /** 拖拽到画布指定位置添加积木（world 坐标，位置用户控制） */
  function addNodeAt(type: string, x: number, y: number): BlockNode | null {
    const a = current.value
    if (!a) return null
    pushHistory()
    const def = getBlockDef(type)
    const node: BlockNode = {
      id: genId(), type, category: def?.category ?? 'terminal',
      x, y, config: defaultsFromFields(def?.fields ?? []),
    }
    a.nodes.push(node)
    a.updatedAt = new Date().toISOString()
    return node
  }

  // 调色板拖拽 → 编辑器画布落点（编辑器注册处理器做坐标转换与添加）
  type DropHandler = (p: { type: string; clientX: number; clientY: number }) => void
  let dropHandler: DropHandler | null = null
  function setDropHandler(fn: DropHandler | null) { dropHandler = fn }
  function dropBlock(p: { type: string; clientX: number; clientY: number }) { dropHandler?.(p) }

  /** 复制自动化（深拷贝，重新生成节点/边 id） */
  function duplicate(id: string): Automation | null {
    const a = automations.value.find(x => x.id === id)
    if (!a) return null
    const now = new Date().toISOString()
    const copy: Automation = { ...JSON.parse(JSON.stringify(a)), id: genId(), name: `${a.name} 副本`, createdAt: now, updatedAt: now }
    const idMap = new Map<string, string>()
    copy.nodes = a.nodes.map(n => {
      const nid = genId()
      idMap.set(n.id, nid)
      return { ...JSON.parse(JSON.stringify(n)), id: nid }
    })
    copy.edges = a.edges.map(e => ({
      ...e,
      id: genId(),
      fromBlock: idMap.get(e.fromBlock) ?? e.fromBlock,
      toBlock: idMap.get(e.toBlock) ?? e.toBlock,
    }))
    automations.value.push(copy)
    return copy
  }

  /** 运行（F2：Tauri → Rust 引擎；entry 可选手动触发入口；否则前端 mock 顺序高亮） */
  async function run(id: string, entry?: string): Promise<string | null> {
    const a = automations.value.find(x => x.id === id)
    if (!a) return null
    const status = useStatusStore()
    if (a.nodes.length === 0) {
      status.pushMessage('工作积木为空，请先添加积木', 'warn')
      return null
    }
    if (isTauri()) {
      try {
        const runId = await invoke<string>('automation_run', { id, entry: entry ?? null })
        status.pushMessage(`运行「${a.name}」...`, 'info')
        // started/done/stopped 事件已由引擎驱动 runState；这里仅在事件未处理时兜底
        // （否则 invoke 返回后会把 done 覆盖回 running → 流光永不结束）
        if (!runState.value[id]) {
          runState.value[id] = { runId, status: 'running' }
          runState.value = { ...runState.value }
          liveRunId.value = runId
          liveSteps.value = []
        }
        return runId
      } catch (e) {
        status.pushMessage(`运行失败: ${e}`, 'error')
        return null
      }
    }
    const runId = genId()
    runState.value[id] = { runId, status: 'running' }
    runState.value = { ...runState.value }
    liveRunId.value = runId
    liveSteps.value = []
    currentBlockId.value = null
    failBlockId.value = null
    status.pushMessage(`运行「${a.name}」`, 'info')
    let idx = 0
    for (const n of a.nodes) {
      idx += 1
      currentBlockId.value = n.id
      await new Promise(r => setTimeout(r, 300))
      status.pushMessage(`  ├ ${getBlockLabel(n.type)}`, 'info')
      liveSteps.value.push({
        blockId: n.id, blockType: n.type, name: getBlockLabel(n.type), index: idx,
        status: 'ok', startedAt: Date.now() - 300, endedAt: Date.now(), durationMs: 300,
        exitCode: 0, stdoutTail: '', detail: getBlockLabel(n.type), cwd: '', auth: '',
      })
    }
    runState.value[id] = { runId, status: 'done' }
    runState.value = { ...runState.value }
    liveRunId.value = null
    currentBlockId.value = null
    status.pushMessage(`「${a.name}」执行完成（mock）`, 'success')
    return runId
  }

  /** 停止运行（F2：Tauri → automation_stop 置取消位；本地预览模式不支持中断） */
  async function stop(id: string, runId: string): Promise<void> {
    const a = automations.value.find(x => x.id === id)
    const status = useStatusStore()
    if (isTauri()) {
      try {
        await invoke('automation_stop', { runId })
        status.pushMessage(`已请求停止「${a?.name ?? ''}」`, 'info')
      } catch (e) {
        status.pushMessage(`停止失败: ${e}`, 'error')
      }
    } else {
      status.pushMessage('本地预览模式不支持中断', 'warn')
    }
  }

  // ── 运行态（列表卡片视觉 + 实时日志，MainPanel 全局监听事件写入）──
  const runState = ref<Record<string, { runId: string; status: 'running' | 'done' | 'failed' | 'stopped' }>>({})
  const liveRunId = ref<string | null>(null)
  const liveSteps = ref<RunStepLog[]>([])
  /** 当前正在执行的积木 id（外层图形化高亮） */
  const currentBlockId = ref<string | null>(null)
  /** 最近失败的积木 id */
  const failBlockId = ref<string | null>(null)
  /** 实时命令输出流（pty-output，仿终端；processId 匹配 liveRunId） */
  const liveOutput = ref('')
  /** 当前画布缩放（调色板拖拽预览按实际大小显示） */
  const canvasScale = ref(1)

  function runStateOf(id: string) {
    return runState.value[id] ?? null
  }

  /** 处理 automation-event（started/step_start/step_done/step_fail/done/error/stopped） */
  function onRunEvent(p: Record<string, unknown>) {
    const type = String(p.type ?? '')
    const aid = String(p.automationId ?? '')
    const runId = String(p.runId ?? '')
    const bid = String(p.blockId ?? '')
    if (type === 'started') {
      if (aid) {
        runState.value[aid] = { runId, status: 'running' }
        runState.value = { ...runState.value }
      }
      liveRunId.value = runId || null
      liveSteps.value = []
      liveOutput.value = ''
      currentBlockId.value = null
      failBlockId.value = null
    } else if (type === 'step_start') {
      currentBlockId.value = bid || null
    } else if (type === 'step_done') {
      if (currentBlockId.value === bid) currentBlockId.value = null
    } else if (type === 'step_fail') {
      failBlockId.value = bid || null
      if (currentBlockId.value === bid) currentBlockId.value = null
    } else if (type === 'done' || type === 'error' || type === 'stopped') {
      const status = type === 'done' ? 'done' : type === 'stopped' ? 'stopped' : 'failed'
      // 失败/异常：把详细错误暴露给用户（Toast + 实时日志），否则运行失败无法排查
      if (type === 'error') {
        const msg = String(p.error ?? '执行失败')
        useStatusStore().pushMessage(`❌ ${msg}`, 'error')
        if (liveRunId.value === runId) {
          liveOutput.value = (liveOutput.value ? liveOutput.value + '\n' : '') + `[错误] ${msg}`
        }
      }
      if (aid && runState.value[aid]) {
        runState.value[aid] = { runId, status }
        runState.value = { ...runState.value }
      }
      if (liveRunId.value === runId) liveRunId.value = null
      currentBlockId.value = null
    }
  }

  /** 处理 step_log（单步实时日志） */
  function onStepLog(p: Record<string, unknown>) {
    liveSteps.value.push({
      blockId: String(p.blockId ?? ''),
      blockType: String(p.blockType ?? ''),
      name: String(p.name ?? ''),
      index: Number(p.index ?? 0),
      status: p.status === 'fail' ? 'fail' : 'ok',
      startedAt: Number(p.startedAt ?? 0),
      endedAt: Number(p.endedAt ?? 0),
      durationMs: Number(p.durationMs ?? 0),
      exitCode: p.exitCode === null || p.exitCode === undefined ? null : Number(p.exitCode),
      stdoutTail: String(p.stdoutTail ?? ''),
      detail: String(p.detail ?? ''),
      cwd: String(p.cwd ?? ''),
      auth: String(p.auth ?? ''),
      iteration: p.iteration === undefined ? undefined : Number(p.iteration),
    })
  }

  function clearLive() {
    liveSteps.value = []
    liveRunId.value = null
    liveOutput.value = ''
  }

  /** 处理 pty-output（实时命令输出，仿终端） */
  function onPtyOutput(p: Record<string, unknown>) {
    if (String(p.processId ?? '') !== (liveRunId.value ?? '')) return
    const data = p.data
    if (!Array.isArray(data)) return
    try {
      liveOutput.value += new TextDecoder().decode(new Uint8Array(data as number[]))
      if (liveOutput.value.length > 200000) liveOutput.value = liveOutput.value.slice(-200000)
    } catch { /* 忽略解码错误 */ }
  }

  /** 画布缩放同步（拖拽预览按实际大小） */
  function setCanvasScale(v: number) { canvasScale.value = v }

  /** 删除块（级联删除其边） */
  function removeNode(nodeId: string) {
    const a = current.value
    if (!a) return
    pushHistory()
    a.nodes = a.nodes.filter(n => n.id !== nodeId)
    a.edges = a.edges.filter(e => e.fromBlock !== nodeId && e.toBlock !== nodeId)
    a.updatedAt = new Date().toISOString()
  }

  /** 新增连线（同两端重复连接则忽略） */
  function addEdge(fromBlock: string, fromPort: string, toBlock: string, toPort: string) {
    const a = current.value
    if (!a) return
    if (a.edges.some(e => e.fromBlock === fromBlock && e.fromPort === fromPort && e.toBlock === toBlock && e.toPort === toPort)) return
    pushHistory()
    a.edges.push({ id: genId(), fromBlock, fromPort, toBlock, toPort })
    a.updatedAt = new Date().toISOString()
  }

  /** 删除连线 */
  function removeEdge(edgeId: string) {
    const a = current.value
    if (!a) return
    pushHistory()
    a.edges = a.edges.filter(e => e.id !== edgeId)
    a.updatedAt = new Date().toISOString()
  }

  /** 移动块（拖拽中高频调用不记历史；拖拽开始前调用 beginEdit()） */
  function moveNode(nodeId: string, x: number, y: number) {
    const n = current.value?.nodes.find(n => n.id === nodeId)
    if (n) { n.x = Math.round(x); n.y = Math.round(y); markDirty() }
  }

  /** 更新块配置 */
  function updateNodeConfig(nodeId: string, config: Record<string, unknown>) {
    const a = current.value
    const n = a?.nodes.find(n => n.id === nodeId)
    if (!n || !a) return
    pushHistory()
    n.config = { ...config }
    a.updatedAt = new Date().toISOString()
  }

  /** 切换积木「固定」：锁定后不可拖拽 */
  function toggleLock(nodeId: string) {
    const a = current.value
    const n = a?.nodes.find(n => n.id === nodeId)
    if (!n || !a) return
    pushHistory()
    n.locked = !n.locked
    a.updatedAt = new Date().toISOString()
  }

  /** 连线（单入边：目标 in 端口已有入边则先替换；一次历史） */
  function connectEdge(fromBlock: string, fromPort: string, toBlock: string, toPort: string) {
    const a = current.value
    if (!a) return
    if (a.edges.some(e => e.fromBlock === fromBlock && e.fromPort === fromPort && e.toBlock === toBlock && e.toPort === toPort)) return
    pushHistory()
    a.edges = a.edges.filter(e => !(e.toBlock === toBlock && e.toPort === toPort))
    a.edges.push({ id: genId(), fromBlock, fromPort, toBlock, toPort })
    a.updatedAt = new Date().toISOString()
  }

  // ── 执行日志（结构化，每个积木执行都记录；automation_logs.json，最新在前）──
  const logs = ref<RunLog[]>([])
  let logsLoaded = false

  /** 加载历史执行日志（Tauri → automation_logs_list；否则 localStorage 兜底） */
  async function logsLoad(force = false) {
    if (!force && logsLoaded) return
    logsLoaded = true
    let data: RunLog[] | null = null
    if (isTauri()) {
      try {
        const json = await invoke<string>('automation_logs_list')
        data = JSON.parse(json)
      } catch (e) { console.error('automation_logs_list failed', e) }
    }
    if (!data) {
      const ls = localStorage.getItem(LS_LOGS)
      try { data = ls ? JSON.parse(ls) : [] } catch { data = [] }
    }
    logs.value = Array.isArray(data) ? data : []
  }

  /** 追加/更新一条运行日志（列表视图历史 + localStorage 兜底） */
  function logsPush(run: RunLog) {
    const i = logs.value.findIndex(x => x.id === run.id)
    if (i >= 0) logs.value[i] = run
    else logs.value.unshift(run)
    logs.value = logs.value.slice(0, 200)
    try { localStorage.setItem(LS_LOGS, JSON.stringify(logs.value)) } catch { /* ignore */ }
  }

  // ── 凭据（登录）管理：前端仅持有掩码 meta；明文一次性经 credential_upsert 写入后端（F3 加密）──
  const credentials = ref<CredentialMeta[]>([])

  /** 启动加载凭据（仅掩码，剥离 fields 明文） */
  async function credentialLoad() {
    let data: Array<CredentialMeta & { fields?: Record<string, string> }> | null = null
    if (isTauri()) {
      try {
        const json = await invoke<string>('credential_list')
        data = JSON.parse(json)
      } catch (e) { console.error('credential_list failed', e) }
    }
    if (!data) {
      const ls = localStorage.getItem(LS_CREDENTIALS)
      try { data = ls ? JSON.parse(ls) : null } catch { data = null }
    }
    credentials.value = (Array.isArray(data) ? data : [])
      .map(c => ({ id: c.id, name: c.name, kind: c.kind, platform: c.platform, masked: c.masked, createdAt: c.createdAt }))
  }

  /** 保存凭据（一次性明文 → 后端 upsert；前端只留掩码 meta） */
  async function credentialSave(input: CredentialInput) {
    const now = new Date().toISOString()
    const id = input.id ?? genId()
    const meta: CredentialMeta = {
      id,
      name: input.name,
      kind: input.kind,
      platform: input.platform,
      masked: Object.fromEntries(
        Object.entries(input.fields)
          .filter(([, v]) => !!v)
          .map(([k, v]) => [k, maskValue(String(v))])
      ),
      createdAt: now,
    }
    // 完整记录（含明文 fields）只发后端，不回存前端 store
    const record = { ...meta, fields: input.fields }
    if (isTauri()) {
      try { await invoke('credential_upsert', { credentialJson: JSON.stringify(record) }) }
      catch (e) { console.error('credential_upsert failed', e) }
    } else {
      try {
        const ls = JSON.parse(localStorage.getItem(LS_CREDENTIALS) || '[]')
        const arr = ls.filter((c: Record<string, unknown>) => c.id !== id)
        arr.push(record)
        localStorage.setItem(LS_CREDENTIALS, JSON.stringify(arr))
      } catch (e) { console.error(e) }
    }
    const idx = credentials.value.findIndex(c => c.id === meta.id)
    if (idx >= 0) credentials.value[idx] = meta
    else credentials.value.push(meta)
    return meta
  }

  async function credentialDelete(id: string) {
    credentials.value = credentials.value.filter(c => c.id !== id)
    if (isTauri()) {
      try { await invoke('credential_delete', { id }) } catch (e) { console.error('credential_delete failed', e) }
    } else {
      try {
        const ls = JSON.parse(localStorage.getItem(LS_CREDENTIALS) || '[]')
        localStorage.setItem(LS_CREDENTIALS, JSON.stringify(ls.filter((c: Record<string, unknown>) => c.id !== id)))
      } catch (e) { console.error(e) }
    }
  }

  /** 掩码工具：保留首 4 位 + 尾 4 位 */
  function maskValue(v: string): string {
    if (v.length <= 8) return '****'
    return `${v.slice(0, 4)}****${v.slice(-4)}`
  }

  /** 导出当前自动化为完整 JSON（含 nodes/edges/config/variables，见方案 §4.6） */
  function exportCurrentJson(): string {
    const a = current.value
    if (!a) return ''
    return JSON.stringify(a, null, 2)
  }

  /** 导入自动化 JSON：同名覆盖（overwrite）或另存副本（asCopy）；返回 'conflict' 表示存在同名需确认 */
  function importAutomationJson(json: string, opts?: { overwrite?: boolean; asCopy?: boolean }): Automation | 'conflict' | null {
    try {
      const obj = JSON.parse(json)
      if (!obj || typeof obj !== 'object') throw new Error('不是有效的 JSON 对象')
      if (!Array.isArray(obj.nodes) || !Array.isArray(obj.edges)) throw new Error('缺少 nodes / edges')
      const now = new Date().toISOString()
      const name = obj.name ? String(obj.name) : '导入的工作积木'
      const existing = automations.value.find(a => a.name === name)
      if (existing && !opts?.overwrite && !opts?.asCopy) {
        return 'conflict'
      }
      const idMap = new Map<string, string>()
      const nodes: BlockNode[] = (obj.nodes as BlockNode[]).map(n => {
        const nid = genId()
        idMap.set(n.id, nid)
        return { ...n, id: nid }
      })
      const edges: Edge[] = (obj.edges as Edge[]).map(e => ({
        ...e,
        id: genId(),
        fromBlock: idMap.get(e.fromBlock) ?? e.fromBlock,
        toBlock: idMap.get(e.toBlock) ?? e.toBlock,
      }))
      if (existing && opts?.overwrite) {
        // 覆盖现有（保留原 id / createdAt）
        existing.name = name
        existing.description = obj.description ?? ''
        existing.version = 2
        existing.nodes = nodes
        existing.edges = edges
        existing.variables = obj.variables ?? {}
        existing.updatedAt = now
        return existing
      }
      const imported: Automation = {
        id: genId(),
        name: opts?.asCopy && existing ? `${name}（副本）` : name,
        description: obj.description ?? '',
        version: 2,
        nodes,
        edges,
        variables: obj.variables ?? {},
        createdAt: now,
        updatedAt: now,
      }
      automations.value.push(imported)
      return imported
    } catch (e) {
      console.error('import automation failed', e)
      return null
    }
  }

  /** 用 JSON 内容直接更新当前正在编辑的工作积木（替换节点/连线/变量/描述/名称，保留 id） */
  function applyJsonToCurrent(json: string): boolean {
    const a = current.value
    if (!a) return false
    try {
      const obj = JSON.parse(json)
      if (!obj || typeof obj !== 'object') return false
      if (!Array.isArray(obj.nodes) || !Array.isArray(obj.edges)) return false
      const now = new Date().toISOString()
      const idMap = new Map<string, string>()
      const nodes: BlockNode[] = (obj.nodes as BlockNode[]).map(n => {
        const nid = genId()
        idMap.set(n.id, nid)
        return { ...n, id: nid }
      })
      const edges: Edge[] = (obj.edges as Edge[]).map(e => ({
        ...e,
        id: genId(),
        fromBlock: idMap.get(e.fromBlock) ?? e.fromBlock,
        toBlock: idMap.get(e.toBlock) ?? e.toBlock,
      }))
      pushHistory()
      a.name = obj.name ? String(obj.name) : a.name
      a.description = obj.description ?? a.description
      a.version = 2
      a.nodes = nodes
      a.edges = edges
      a.variables = obj.variables ?? {}
      a.updatedAt = now
      markDirty()
      return true
    } catch (e) {
      console.error('apply json failed', e)
      return false
    }
  }

  return {
    automations, currentId, current, editing, search, filtered, dirty, canUndo, canRedo,
    credentials,
    load, open, create, remove, closeEditor, rename, setDescription, applyJsonToCurrent, addNode, addNodeAt, duplicate, run, stop,
    beginEdit: pushHistory, undo, redo, save, persist, markDirty, structureIssues,
    removeNode, addEdge, removeEdge, moveNode, updateNodeConfig, connectEdge, toggleLock,
    credentialLoad, credentialSave, credentialDelete,
    logs, logsLoad, logsPush,
    runState, runStateOf, liveRunId, liveSteps, currentBlockId, failBlockId, liveOutput, canvasScale, onRunEvent, onStepLog, onPtyOutput, clearLive, setDropHandler, dropBlock, setCanvasScale,
    exportCurrentJson, importAutomationJson,
  }
})
