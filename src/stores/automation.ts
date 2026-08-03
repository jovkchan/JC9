import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Automation, BlockNode, Edge } from '@/types/automation'
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

function blankAutomation(): Automation {
  const now = new Date().toISOString()
  return {
    id: genId(),
    name: '未命名自动化',
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

  /** 手动保存（无自动保存） */
  function save() {
    const a = current.value
    if (!a) return
    a.updatedAt = new Date().toISOString()
    dirty.value = false
    useStatusStore().pushMessage(`已保存「${a.name}」`, 'success')
  }

  function load() {
    if (automations.value.length === 0) {
      automations.value.push(blankAutomation())
    }
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

  function remove(id: string) {
    automations.value = automations.value.filter(x => x.id !== id)
    if (currentId.value === id) {
      currentId.value = null
      editing.value = false
    }
  }

  function closeEditor() {
    editing.value = false
    currentId.value = null
  }

  function rename(id: string, name: string) {
    const a = automations.value.find(x => x.id === id)
    if (a) { a.name = name; a.updatedAt = new Date().toISOString(); markDirty() }
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

  /** 运行（F1a：轻量 mock；正式 mock 引擎见 runtime/preview.ts） */
  async function run(id: string): Promise<string | null> {
    const a = automations.value.find(x => x.id === id)
    if (!a) return null
    const status = useStatusStore()
    if (a.nodes.length === 0) {
      status.pushMessage('自动化为空，请先添加积木', 'warn')
      return null
    }
    const runId = genId()
    status.pushMessage(`▶ 运行「${a.name}」`, 'info')
    for (const n of a.nodes) {
      await new Promise(r => setTimeout(r, 300))
      status.pushMessage(`  ├ ${getBlockLabel(n.type)}`, 'info')
    }
    status.pushMessage(`🏁 「${a.name}」执行完成（mock）`, 'success')
    return runId
  }

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

  return {
    automations, currentId, current, editing, search, filtered, dirty, canUndo, canRedo,
    load, open, create, remove, closeEditor, rename, addNode, duplicate, run,
    beginEdit: pushHistory, undo, redo, save, markDirty,
    removeNode, addEdge, removeEdge, moveNode, updateNodeConfig, connectEdge,
  }
})
