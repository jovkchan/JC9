<script setup lang="ts">
import { nextTick, ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { useAutomationStore } from '@/stores/automation'
import { useStatusStore } from '@/stores/status'
import { getBlockDef, getBlockColor } from '@/components/automation/blocks/palette'
import { BLOCK_W, blockHeight, blockSummary, MAX_SUMMARY_LINES } from '@/components/automation/blocks/summary'
import type { BlockNode, Port, Edge } from '@/types/automation'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcModal from '@/components/ui/JcModal.vue'
import JcContextMenu from '@/components/ui/JcContextMenu.vue'
import type { JcContextMenuItem } from '@/components/ui/JcContextMenu.vue'
import InspectorPanel from './editor/InspectorPanel.vue'
import LoginDialog from './editor/LoginDialog.vue'

const store = useAutomationStore()
const status = useStatusStore()

// ── Canvas 视口（F1a 骨架：世界↔屏幕 + 平移/缩放 + 网格）──
// 后续积木/端口/连线全部由 renderer 在此绘制（见 docs/plans §4.7）
const canvasRef = ref<HTMLCanvasElement | null>(null)
let ctx2d: CanvasRenderingContext2D | null = null
let dpr = 1
const view = { scale: 1, ox: 0, oy: 0 }
let raf = 0
let resizeObs: ResizeObserver | null = null
let themeObs: MutationObserver | null = null

function toWorld(sx: number, sy: number) {
  return { x: (sx - view.ox) / view.scale, y: (sy - view.oy) / view.scale }
}

function schedule() {
  if (raf) return
  raf = requestAnimationFrame(() => { raf = 0; draw() })
}

// ── 交互状态 ──
const selectedId = ref<string | null>(null)
type Mode = 'idle' | 'pan' | 'drag' | 'connect'
let mode: Mode = 'idle'
let dragId = ''
let dragOffX = 0, dragOffY = 0
let dragPushed = false
let connFrom: { blockId: string; port: Port } | null = null
const connCursor = ref({ x: 0, y: 0 })
const PORT_HIT_R = 12        // 端口命中半径（屏幕像素）

// ── 右键菜单（删除 / 编辑 / 固定 / 登录；连线删除）──
const ctxShow = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)
const ctxNode = ref<BlockNode | null>(null)
/** 当前选中的连线（单击选中 + Delete 删除 / 右键删除） */
const selectedEdgeId = ref<string | null>(null)
/** 右键命中的连线（连线菜单） */
const ctxEdgeId = ref<string | null>(null)
const edgeMenuItems: JcContextMenuItem[] = [
  { label: '删除连线', value: 'delete-edge', danger: true },
]
const ctxMenuItems = computed<JcContextMenuItem[]>(() =>
  ctxEdgeId.value ? edgeMenuItems : ctxItems.value,
)

/** 打开编辑面板：选中块 + 记录要编辑的目标 */
const inspectNode = ref<BlockNode | null>(null)
/** 登录弹窗目标 */
const loginNode = ref<BlockNode | null>(null)
const loginOpen = ref(false)

function openContext(e: MouseEvent, node: BlockNode) {
  e.preventDefault()
  selectedId.value = node.id
  selectedEdgeId.value = null
  ctxNode.value = node
  ctxEdgeId.value = null
  ctxX.value = e.clientX
  ctxY.value = e.clientY
  ctxShow.value = true
  schedule()
}

/** 右键连线 → 删除连线菜单 */
function openEdgeContext(e: MouseEvent, edge: Edge) {
  e.preventDefault()
  selectedEdgeId.value = edge.id
  ctxNode.value = null
  ctxEdgeId.value = edge.id
  ctxX.value = e.clientX
  ctxY.value = e.clientY
  ctxShow.value = true
  schedule()
}

const ctxItems = computed<JcContextMenuItem[]>(() => {
  const n = ctxNode.value
  const items: JcContextMenuItem[] = [
    { label: '编辑', value: 'edit' },
    { label: n?.locked ? '取消固定' : '固定', value: 'lock' },
  ]
  // 手动触发块可单独触发该分支（F2，无需依赖「开始」）
  if (n?.type === 'manual-trigger') {
    items.push({ label: '触发此分支', value: 'trigger' })
  }
  // 凭据是独立积木：右键提供「配置凭据」；普通块不再绑登录，凭据通过连线引用
  if (n?.type === 'credential') {
    items.push({ label: '配置凭据', value: 'login' })
  }
  items.push({ label: '复制块 ID', value: 'copy-block-id' })
  items.push({ label: '删除', value: 'delete', danger: true })
  return items
})

function onCtxSelect(item: JcContextMenuItem) {
  const node = ctxNode.value
  ctxShow.value = false
  if (item.value === 'delete-edge') {
    if (ctxEdgeId.value) {
      store.removeEdge(ctxEdgeId.value)
      if (selectedEdgeId.value === ctxEdgeId.value) selectedEdgeId.value = null
    }
    ctxEdgeId.value = null
    return
  }
  if (!node) return
  switch (item.value) {
    case 'edit':
      inspectNode.value = node
      break
    case 'lock':
      store.toggleLock(node.id)
      break
    case 'trigger':
      // 手动触发：以该块为入口运行（多个手动触发块各自触发各自分支）
      if (store.current) doRun(store.current.id, node.id)
      break
    case 'login':
      // 配置凭据块（选择/新建凭据 → 写入 config.credentialId）
      loginNode.value = node
      loginOpen.value = true
      break
    case 'copy-block-id':
      navigator.clipboard.writeText(node.id)
        .then(() => status.pushMessage(`已复制块 ID：${node.id}`, 'success'))
        .catch(e => status.pushMessage(`复制失败: ${e}`, 'error'))
      break
    case 'delete':
      store.removeNode(node.id)
      if (selectedId.value === node.id) selectedId.value = null
      if (inspectNode.value?.id === node.id) inspectNode.value = null
      break
  }
}

/** InspectorPanel「配置凭据」按钮 → 打开 LoginDialog（针对当前编辑的凭据块） */
function onConfigureCredential() {
  if (inspectNode.value?.type === 'credential') {
    loginNode.value = inspectNode.value
    loginOpen.value = true
  }
}

// ── 导出（完整 JSON，见方案 §4.6）──
const exportOpen = ref(false)
const exportText = computed(() => store.exportCurrentJson())

function openExport() { exportOpen.value = true }

async function copyExport() {
  try {
    await navigator.clipboard.writeText(exportText.value)
    status.pushMessage('已复制工作积木 JSON', 'success')
  } catch (e) { status.pushMessage(`复制失败: ${e}`, 'error') }
}

/** 复制当前工作积木 ID（供 MCP / 外部按 ID 触发） */
async function copyCurrentId() {
  const id = store.current?.id
  if (!id) return
  try {
    await navigator.clipboard.writeText(id)
    status.pushMessage(`已复制工作积木 ID：${id}`, 'success')
  } catch (e) { status.pushMessage(`复制失败: ${e}`, 'error') }
}

// ── 保存 / 返回前校验提示（缺开始/结束 + 未保存）──
interface ConfirmBox {
  title: string
  items: string[]
  confirmText: string
  /** 提供「保存并返回」按钮（返回弹窗且存在未保存修改时） */
  withSave?: boolean
  onConfirm: () => void
}
const confirmBox = ref<ConfirmBox | null>(null)

/** 保存：结构有问题（缺开始/结束）先弹确认 */
function saveWithCheck() {
  const issues = store.structureIssues()
  if (issues.length > 0) {
    confirmBox.value = {
      title: '保存前检查',
      items: issues,
      confirmText: '仍要保存',
      onConfirm: () => { confirmBox.value = null; store.save() },
    }
  } else {
    store.save()
  }
}

/** 返回：未保存修改 / 结构有问题先弹确认 */
function backWithCheck() {
  const items: string[] = []
  if (store.dirty) items.push('有未保存的修改，返回后不会保留')
  items.push(...store.structureIssues())
  if (items.length === 0) {
    store.closeEditor()
    return
  }
  confirmBox.value = {
    title: '返回确认',
    items,
    confirmText: '直接返回',
    withSave: store.dirty,
    onConfirm: () => { confirmBox.value = null; store.closeEditor() },
  }
}

/** 保存并返回（返回弹窗） */
function confirmSaveAndBack() {
  store.save().then(() => {
    confirmBox.value = null
    store.closeEditor()
  })
}

async function saveExportFile() {
  try {
    const name = store.current?.name ? `${store.current.name}.json` : 'automation.json'
    const filePath = await save({ filters: [{ name: '工作积木 JSON', extensions: ['json'] }], defaultPath: name })
    if (!filePath) return
    const data = Array.from(new TextEncoder().encode(exportText.value))
    await invoke('write_file_binary', { path: filePath, data })
    status.pushMessage(`已导出到 ${filePath}`, 'success')
  } catch (e) { status.pushMessage(`导出失败: ${e}`, 'error') }
}

// ── 运行态（Rust 引擎 automation-event 驱动）──
const runningId = ref<string | null>(null)
const failId = ref<string | null>(null)
const runStep = ref(0)
const runTotal = ref(0)
const runName = ref('')
const runTail = ref('')
const runVars = ref<Record<string, unknown>>({})
const runningRunId = ref<string | null>(null)
const runIter = ref(0)
let unlistenAuto: UnlistenFn | null = null

function fmtDur(ms: number) {
  if (!ms || ms < 1000) return `${ms ?? 0}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

// ── 运行日志分割面板（上部画布 + 下部日志，分隔条拖拽调整高度）──
const mainRef = ref<HTMLElement | null>(null)
const runLogH = ref(220)
const showRunLog = computed(() => !!runningRunId.value || store.liveSteps.length > 0 || !!store.liveOutput)
/** 编辑器内实时命令输出区：追加后自动滚动到底跟随最新 */
const runlogOutEl = ref<HTMLPreElement | null>(null)
watch(() => store.liveOutput, () => {
  nextTick(() => { if (runlogOutEl.value) runlogOutEl.value.scrollTop = runlogOutEl.value.scrollHeight })
})
/** 新增积木后自动平移到可视范围（超出画布右/下边界的块平移进入视野） */
function ensureNodeVisible(n: BlockNode) {
  const cv = canvasRef.value
  if (!cv) return
  const W = cv.clientWidth, H = cv.clientHeight
  const h = blockHeight(n.type, (n.config ?? {}) as Record<string, unknown>)
  const sx = n.x * view.scale + view.ox
  const sy = n.y * view.scale + view.oy
  const sw = BLOCK_W * view.scale, sh = h * view.scale
  let dx = 0, dy = 0
  if (sx < 8) dx = 8 - sx
  else if (sx + sw > W - 8) dx = W - 8 - (sx + sw)
  if (sy < 8) dy = 8 - sy
  else if (sy + sh > H - 8) dy = H - 8 - (sy + sh)
  if (dx !== 0 || dy !== 0) { view.ox += dx; view.oy += dy; schedule() }
}
// 添加积木（length 增加）→ 平移到新块可见
watch(
  () => store.current?.nodes.length ?? 0,
  (n, o) => {
    if (n > o && store.current) {
      const nodes = store.current.nodes
      const last = nodes[nodes.length - 1]
      if (last) ensureNodeVisible(last)
    }
  },
)
let logDragging = false
function onLogBarDown(e: MouseEvent) {
  e.preventDefault()
  logDragging = true
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'row-resize'
  window.addEventListener('mousemove', onLogBarMove)
  window.addEventListener('mouseup', onLogBarUp)
}
function onLogBarMove(e: MouseEvent) {
  if (!logDragging || !mainRef.value) return
  const rect = mainRef.value.getBoundingClientRect()
  const h = rect.bottom - e.clientY
  runLogH.value = Math.min(480, Math.max(100, Math.round(h)))
}
function onLogBarUp() {
  logDragging = false
  document.body.style.userSelect = ''
  document.body.style.cursor = ''
  window.removeEventListener('mousemove', onLogBarMove)
  window.removeEventListener('mouseup', onLogBarUp)
}

async function doRun(id?: string, entry?: string) {
  const a = store.current
  if (!a) return
  const runId = await store.run(id ?? a.id, entry)
  if (runId) runningRunId.value = runId
}

async function doStop() {
  const a = store.current
  if (!a || !runningRunId.value) return
  await store.stop(a.id, runningRunId.value)
  runningRunId.value = null
}

async function onAutoEvent(e: { payload: Record<string, unknown> }) {
  const p = e.payload
  const type = String(p.type ?? '')
  const bid = (p.blockId as string) ?? null
  if (type === 'started') { runningId.value = null; failId.value = null; runTail.value = ''; runIter.value = 0 }
  if (type === 'step_start') {
    runningId.value = bid
    failId.value = null
    runStep.value = Number(p.step ?? 0)
    runTotal.value = Number(p.total ?? 0)
    runName.value = String(p.name ?? '')
  } else if (type === 'step_done') {
    if (runningId.value === bid) runningId.value = null
  } else if (type === 'step_fail') {
    failId.value = bid
    if (runningId.value === bid) runningId.value = null
    if (typeof p.stdoutTail === 'string') runTail.value = p.stdoutTail
  } else if (type === 'loop_iter') {
    runIter.value = Number(p.iteration ?? 0)
  } else if (type === 'var_change' || type === 'done') {
    if (p.vars && typeof p.vars === 'object') runVars.value = p.vars as Record<string, unknown>
  }
  if (type === 'done' || type === 'error' || type === 'stopped') {
    runningId.value = null
    runningRunId.value = null
    if (type === 'done') failId.value = null
  }
  schedule()
}

function snap(v: number) { return Math.round(v / 8) * 8 }

// ── 端口几何 ──
function getPorts(block: { type: string }) {
  const def = getBlockDef(block.type)
  return { inputs: def?.inputs ?? [], outputs: def?.outputs ?? [] }
}
function portPos(block: BlockNode, p: Port) {
  const { inputs, outputs } = getPorts(block)
  const h = blockHeight(block.type, (block.config ?? {}) as Record<string, unknown>)
  if (p.direction === 'in') {
    const i = inputs.findIndex(x => x.id === p.id)
    return { x: block.x, y: block.y + (h / (inputs.length + 1)) * (i + 1) }
  }
  const j = outputs.findIndex(x => x.id === p.id)
  return { x: block.x + BLOCK_W, y: block.y + (h / (outputs.length + 1)) * (j + 1) }
}

// ── 命中检测 ──
function hitBlock(wx: number, wy: number): BlockNode | null {
  const nodes = store.current?.nodes ?? []
  for (let k = nodes.length - 1; k >= 0; k--) {
    const n = nodes[k]
    const h = blockHeight(n.type, (n.config ?? {}) as Record<string, unknown>)
    if (wx >= n.x && wx <= n.x + BLOCK_W && wy >= n.y && wy <= n.y + h) return n
  }
  return null
}
function hitPort(wx: number, wy: number): { block: BlockNode; port: Port } | null {
  const nodes = store.current?.nodes ?? []
  const r = PORT_HIT_R / view.scale
  for (const n of nodes) {
    const { inputs, outputs } = getPorts(n)
    for (const p of [...inputs, ...outputs]) {
      const pos = portPos(n, p)
      const dx = wx - pos.x, dy = wy - pos.y
      if (dx * dx + dy * dy <= r * r) return { block: n, port: p }
    }
  }
  return null
}
function compatible(a: Port, b: Port): boolean {
  if (a.direction === b.direction) return false
  if (a.dataType !== b.dataType) return false
  return true
}

/** 连线贝塞尔采样点（命中检测用） */
function edgePathPoints(e: Edge, samples = 24): Array<{ x: number; y: number }> {
  const a = store.current
  if (!a) return []
  const fb = a.nodes.find(n => n.id === e.fromBlock)
  const tb = a.nodes.find(n => n.id === e.toBlock)
  if (!fb || !tb) return []
  const fPort = [...getPorts(fb).inputs, ...getPorts(fb).outputs].find(p => p.id === e.fromPort)
  const tPort = [...getPorts(tb).inputs, ...getPorts(tb).outputs].find(p => p.id === e.toPort)
  if (!fPort || !tPort) return []
  const p0 = portPos(fb, fPort), p3 = portPos(tb, tPort)
  const mx = (p0.x + p3.x) / 2
  const pts: Array<{ x: number; y: number }> = []
  for (let i = 0; i <= samples; i++) {
    const t = i / samples
    const mt = 1 - t
    pts.push({
      x: mt * mt * mt * p0.x + 3 * mt * mt * t * mx + 3 * mt * t * t * mx + t * t * t * p3.x,
      y: mt * mt * mt * p0.y + 3 * mt * mt * t * p0.y + 3 * mt * t * t * p3.y + t * t * t * p3.y,
    })
  }
  return pts
}

/** 命中连线：点到贝塞尔最近距离 < 阈值 */
function hitEdge(wx: number, wy: number): Edge | null {
  const a = store.current
  if (!a || a.edges.length === 0) return null
  const thr = 10 / view.scale
  let best: Edge | null = null
  let bestD = thr
  for (const e of a.edges) {
    for (const p of edgePathPoints(e)) {
      const d = Math.hypot(wx - p.x, wy - p.y)
      if (d < bestD) { bestD = d; best = e }
    }
  }
  return best
}

const MULTI_COLOR = '#e6a23c'   // 可连多条线的端口高亮色（琥珀）
function portKey(blockId: string, portId: string) { return `${blockId}:${portId}` }
function portColor(p: Port, count = 0): string {
  return p.multi || count >= 2 ? MULTI_COLOR : p.color
}
function edgeCounts(): Map<string, number> {
  const a = store.current
  const m = new Map<string, number>()
  if (!a) return m
  for (const e of a.edges) {
    const k = portKey(e.fromBlock, e.fromPort)
    m.set(k, (m.get(k) ?? 0) + 1)
  }
  return m
}

function draw() {
  const cv = canvasRef.value
  if (!cv || !ctx2d) return
  const w = cv.clientWidth, h = cv.clientHeight
  if (w === 0 || h === 0) return
  if (cv.width !== Math.round(w * dpr) || cv.height !== Math.round(h * dpr)) {
    cv.width = Math.round(w * dpr)
    cv.height = Math.round(h * dpr)
  }
  ctx2d.setTransform(dpr * view.scale, 0, 0, dpr * view.scale, dpr * view.ox, dpr * view.oy)
  // 世界空间矩形清屏
  const x0 = -view.ox / view.scale, y0 = -view.oy / view.scale
  ctx2d.clearRect(x0 - 1, y0 - 1, w / view.scale + 2, h / view.scale + 2)
  drawGrid(w, h)
  drawEdges()
  drawBlocks()
  drawArrows()
  drawConnectPreview()
  drawMinimap()
}

// ── 小地图（Minimap）：右下角缩略图，点击/拖动快速定位视口；画幅（面板尺寸）1×/2× 可切换 ──
const MM_BASE_W = 235
const MM_BASE_H = 130
const mmRef = ref<HTMLCanvasElement | null>(null)
/** 小地图显示开关（可隐藏；隐藏后右下角浮动图标打开） */
const mmVisible = ref(true)
/** 小地图画幅（预览面板）缩放倍率：1× = 235×130，2× = 470×260；内容仍 fit 面板，画幅变大看得更清 */
const mmZoom = ref(1)
const MM_W = computed(() => MM_BASE_W * mmZoom.value)
const MM_H = computed(() => MM_BASE_H * mmZoom.value)
let mmDragging = false

// 重新打开小地图时立即重绘：v-if 重建 canvas 后新画布是空白，需主动触发一次 draw
watch(mmVisible, (v) => { if (v) schedule() })
// 切换小地图倍率后重绘
watch(mmZoom, () => schedule())

interface MmProjection {
  minX: number
  minY: number
  bw: number
  bh: number
  mmScale: number
  offX: number
  offY: number
}

/** 所有节点世界包围盒 → 小地图映射参数（无节点返回 null） */
function minimapProjection(): MmProjection | null {
  const a = store.current
  if (!a || a.nodes.length === 0) return null
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const n of a.nodes) {
    const cfg = (n.config ?? {}) as Record<string, unknown>
    const h = blockHeight(n.type, cfg)
    if (n.x < minX) minX = n.x
    if (n.y < minY) minY = n.y
    if (n.x + BLOCK_W > maxX) maxX = n.x + BLOCK_W
    if (n.y + h > maxY) maxY = n.y + h
  }
  const pad = 30
  const bw = maxX - minX + pad * 2
  const bh = maxY - minY + pad * 2
  // 内容比例固定为基准画幅（235×130）的 fit：面板变大时图保持原大小、居中显示（不放大）
  const mmScale = Math.min(MM_BASE_W / bw, MM_BASE_H / bh)
  return {
    minX: minX - pad, minY: minY - pad, bw, bh, mmScale,
    offX: (MM_W.value - bw * mmScale) / 2,
    offY: (MM_H.value - bh * mmScale) / 2,
  }
}

function drawMinimap() {
  const mc = mmRef.value
  const cv = canvasRef.value
  if (!mc || !cv) return
  if (mc.width !== Math.round(MM_W.value * dpr) || mc.height !== Math.round(MM_H.value * dpr)) {
    mc.width = Math.round(MM_W.value * dpr)
    mc.height = Math.round(MM_H.value * dpr)
  }
  const mctx = mc.getContext('2d')
  if (!mctx) return
  mctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  mctx.clearRect(0, 0, MM_W.value, MM_H.value)
  // 主题感知：背景/文字/视口框随明暗模式（与主画布一致）
  const mmBg = cssVar('--jc-bg-elevated') || 'rgba(24,24,26,0.82)'
  const mmText = cssVar('--jc-text-secondary') || 'rgba(150,150,150,0.55)'
  const mmView = cssVar('--jc-text-primary') || 'rgba(255,255,255,0.9)'
  mctx.fillStyle = mmBg
  mctx.fillRect(0, 0, MM_W.value, MM_H.value)
  const proj = minimapProjection()
  const a = store.current
  if (!proj || !a || a.nodes.length === 0) {
    mctx.fillStyle = mmText
    mctx.font = '11px system-ui'
    mctx.textAlign = 'center'
    mctx.fillText('暂无积木', MM_W.value / 2, MM_H.value / 2)
    return
  }
  const { minX, minY, mmScale, offX, offY } = proj
  const mm = (wx: number, wy: number) => ({ x: (wx - minX) * mmScale + offX, y: (wy - minY) * mmScale + offY })
  // 连线标注（源块颜色半透明细线，画在色块之下）
  const nodeById = new Map(a.nodes.map(n => [n.id, n]))
  mctx.lineWidth = 1
  for (const edge of a.edges) {
    const fb = nodeById.get(edge.fromBlock), tb = nodeById.get(edge.toBlock)
    if (!fb || !tb) continue
    const fcfg = (fb.config ?? {}) as Record<string, unknown>
    const tcfg = (tb.config ?? {}) as Record<string, unknown>
    const p1 = mm(fb.x + BLOCK_W / 2, fb.y + blockHeight(fb.type, fcfg) / 2)
    const p2 = mm(tb.x + BLOCK_W / 2, tb.y + blockHeight(tb.type, tcfg) / 2)
    mctx.strokeStyle = getBlockColor(fb.type)
    mctx.globalAlpha = 0.5
    mctx.beginPath()
    mctx.moveTo(p1.x, p1.y)
    mctx.lineTo(p2.x, p2.y)
    mctx.stroke()
    mctx.globalAlpha = 1
  }
  // 积木色块（运行中/失败高亮）
  for (const n of a.nodes) {
    const cfg = (n.config ?? {}) as Record<string, unknown>
    const h = blockHeight(n.type, cfg)
    const color = getBlockColor(n.type)
    const p = mm(n.x, n.y)
    mctx.fillStyle = color
    mctx.globalAlpha = n.id === runningId.value ? 0.95 : n.id === failId.value ? 0.7 : 0.55
    mctx.fillRect(p.x, p.y, Math.max(2, BLOCK_W * mmScale), Math.max(2, h * mmScale))
    mctx.globalAlpha = 1
  }
  // 视口矩形（当前可视区域）
  const vx0 = -view.ox / view.scale, vy0 = -view.oy / view.scale
  const vx1 = vx0 + cv.clientWidth / view.scale, vy1 = vy0 + cv.clientHeight / view.scale
  const p0 = mm(vx0, vy0), p1 = mm(vx1, vy1)
  mctx.strokeStyle = mmView
  mctx.lineWidth = 1
  mctx.strokeRect(p0.x, p0.y, Math.max(1, p1.x - p0.x), Math.max(1, p1.y - p0.y))
}

/** 小地图坐标 → 主视口中心平移到对应世界位置（保持缩放不变） */
function minimapJump(mx: number, my: number) {
  const proj = minimapProjection()
  const cv = canvasRef.value
  if (!proj || !cv) return
  const wx = proj.minX + (mx - proj.offX) / proj.mmScale
  const wy = proj.minY + (my - proj.offY) / proj.mmScale
  view.ox = cv.clientWidth / 2 - wx * view.scale
  view.oy = cv.clientHeight / 2 - wy * view.scale
  store.setCanvasScale(view.scale)
  schedule()
}

function onMinimapDown(e: PointerEvent) {
  const mc = mmRef.value
  if (!mc) return
  mmDragging = true
  mc.setPointerCapture(e.pointerId)
  const rect = mc.getBoundingClientRect()
  minimapJump(e.clientX - rect.left, e.clientY - rect.top)
}

function onMinimapMove(e: PointerEvent) {
  if (!mmDragging || !mmRef.value) return
  const rect = mmRef.value.getBoundingClientRect()
  minimapJump(e.clientX - rect.left, e.clientY - rect.top)
}

function onMinimapUp() {
  mmDragging = false
}

function drawGrid(w: number, h: number) {
  if (!ctx2d) return
  const step = 24
  const { x: x0, y: y0 } = toWorld(0, 0)
  const { x: x1, y: y1 } = toWorld(w, h)
  ctx2d.strokeStyle = 'rgba(128,128,128,0.14)'
  ctx2d.lineWidth = 1 / view.scale
  ctx2d.beginPath()
  for (let x = Math.floor(x0 / step) * step; x <= x1; x += step) { ctx2d.moveTo(x, y0); ctx2d.lineTo(x, y1) }
  for (let y = Math.floor(y0 / step) * step; y <= y1; y += step) { ctx2d.moveTo(x0, y); ctx2d.lineTo(x1, y) }
  ctx2d.stroke()
}

function bezier(c: CanvasRenderingContext2D, p1: { x: number; y: number }, p2: { x: number; y: number }): { c1: { x: number; y: number }; c2: { x: number; y: number } } {
  const dx = Math.max(48, Math.abs(p2.x - p1.x) * 0.5)
  const c1 = { x: p1.x + dx, y: p1.y }
  const c2 = { x: p2.x - dx, y: p2.y }
  c.beginPath()
  c.moveTo(p1.x, p1.y)
  c.bezierCurveTo(c1.x, c1.y, c2.x, c2.y, p2.x, p2.y)
  return { c1, c2 }
}

/** 在曲线终点 to 处绘制箭头（朝向 from 方向，即入端切线） */
function drawArrow(c: CanvasRenderingContext2D, from: { x: number; y: number }, to: { x: number; y: number }) {
  const angle = Math.atan2(to.y - from.y, to.x - from.x)
  const size = 9 / view.scale
  c.beginPath()
  c.moveTo(to.x, to.y)
  c.lineTo(to.x - size * Math.cos(angle - Math.PI / 6), to.y - size * Math.sin(angle - Math.PI / 6))
  c.lineTo(to.x - size * Math.cos(angle + Math.PI / 6), to.y - size * Math.sin(angle + Math.PI / 6))
  c.closePath()
  c.fill()
}

function drawEdges() {
  const a = store.current
  const c = ctx2d
  if (!a || !c || a.edges.length === 0) return
  const s = view.scale
  const nodeById = new Map(a.nodes.map(n => [n.id, n]))
  const ec = edgeCounts()
  c.lineWidth = 2 / s
  for (const edge of a.edges) {
    const fb = nodeById.get(edge.fromBlock), tb = nodeById.get(edge.toBlock)
    if (!fb || !tb) continue
    const fPort = [...getPorts(fb).inputs, ...getPorts(fb).outputs].find(p => p.id === edge.fromPort)
    const tPort = [...getPorts(tb).inputs, ...getPorts(tb).outputs].find(p => p.id === edge.toPort)
    if (!fPort || !tPort) continue
    const p1 = portPos(fb, fPort), p2 = portPos(tb, tPort)
    const sel = edge.id === selectedEdgeId.value
    c.strokeStyle = sel ? '#8a58ff' : portColor(fPort, ec.get(portKey(edge.fromBlock, edge.fromPort)) ?? 0)
    c.lineWidth = (sel ? 4 : 2) / s
    c.globalAlpha = sel ? 1 : 0.85
    bezier(c, p1, p2)
    c.stroke()
    c.globalAlpha = 1
  }
}

/** 箭头单独绘制（在块/端口之上，保证层级高于连接点） */
function drawArrows() {
  const a = store.current
  const c = ctx2d
  if (!a || !c || a.edges.length === 0) return
  const nodeById = new Map(a.nodes.map(n => [n.id, n]))
  const ec = edgeCounts()
  for (const edge of a.edges) {
    const fb = nodeById.get(edge.fromBlock), tb = nodeById.get(edge.toBlock)
    if (!fb || !tb) continue
    const fPort = [...getPorts(fb).inputs, ...getPorts(fb).outputs].find(p => p.id === edge.fromPort)
    const tPort = [...getPorts(tb).inputs, ...getPorts(tb).outputs].find(p => p.id === edge.toPort)
    if (!fPort || !tPort) continue
    const p1 = portPos(fb, fPort), p2 = portPos(tb, tPort)
    const dx = Math.max(48, Math.abs(p2.x - p1.x) * 0.5)
    const c2 = { x: p2.x - dx, y: p2.y }
    c.fillStyle = portColor(fPort, ec.get(portKey(edge.fromBlock, edge.fromPort)) ?? 0)
    drawArrow(c, c2, p2)
  }
}

function drawConnectPreview() {
  const c = ctx2d
  const a = store.current
  if (!c || !a || !connFrom) return
  const s = view.scale
  const fb = a.nodes.find(n => n.id === connFrom!.blockId)
  if (!fb) return
  const ec = edgeCounts()
  const lineColor = portColor(connFrom!.port, ec.get(portKey(connFrom!.blockId, connFrom!.port.id)) ?? 0)
  const p1 = portPos(fb, connFrom!.port)
  c.strokeStyle = lineColor
  c.globalAlpha = 0.8
  c.lineWidth = 2 / s
  const pts = bezier(c, p1, connCursor.value)
  c.stroke()
  c.fillStyle = lineColor
  drawArrow(c, pts.c2, connCursor.value)
  c.globalAlpha = 1
  // 高亮兼容目标端口（可连则按端口色，多线端口琥珀）
  const r = PORT_HIT_R / s
  for (const n of a.nodes) {
    if (n.id === connFrom!.blockId) continue
    const { inputs, outputs } = getPorts(n)
    for (const p of [...inputs, ...outputs]) {
      if (!compatible(connFrom!.port, p)) continue
      const pos = portPos(n, p)
      c.beginPath()
      c.arc(pos.x, pos.y, r, 0, Math.PI * 2)
      c.strokeStyle = portColor(p, ec.get(portKey(n.id, p.id)) ?? 0)
      c.lineWidth = 2 / s
      c.globalAlpha = 0.7
      c.stroke()
      c.globalAlpha = 1
    }
  }
}

function drawHintText() {
  const cv = canvasRef.value
  if (!cv || !ctx2d) return
  const w = cv.clientWidth, h = cv.clientHeight
  const s = view.scale
  ctx2d.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx2d.fillStyle = 'rgba(140,140,140,0.55)'
  ctx2d.font = '12px system-ui'
  ctx2d.textAlign = 'center'
  ctx2d.fillText('从左侧「积木」面板点击添加积木', w / 2, h / 2 - 10)
  ctx2d.fillText('拖动端口连线 · 拖拽块移动 · 滚轮缩放 · 空白拖拽平移', w / 2, h / 2 + 12)
  ctx2d.setTransform(dpr * s, 0, 0, dpr * s, dpr * view.ox, dpr * view.oy)
}

function ellipsisText(c: CanvasRenderingContext2D, text: string, maxW: number): string {
  if (c.measureText(text).width <= maxW) return text
  let t = text
  while (t.length > 1 && c.measureText(t + '…').width > maxW) t = t.slice(0, -1)
  return t + '…'
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function drawBlocks() {
  const a = store.current
  if (!a) return
  const c = ctx2d
  if (!c) return
  if (a.nodes.length === 0) { drawHintText(); return }
  const s = view.scale
  const ec = edgeCounts()
  // 缩放内容分级：小尺度下隐藏文字，避免块缩小后文字挤在一起（只留色块轮廓 + 端口）
  const showTitle = s >= 0.45
  const showSummary = s >= 0.7
  // 主题感知：块背景/文字随明暗模式切换
  const bg = cssVar('--jc-bg-elevated') || '#2d2d30'
  const text = cssVar('--jc-text-primary') || '#e6e6e6'
  const sub = cssVar('--jc-text-secondary') || 'rgba(196,196,196,0.78)'
  for (const n of a.nodes) {
    const def = getBlockDef(n.type)
    const color = getBlockColor(n.type)
    const isSel = n.id === selectedId.value
    const cfg = (n.config ?? {}) as Record<string, unknown>
    const h = blockHeight(n.type, cfg)
    const summary = blockSummary(n.type, cfg).slice(0, MAX_SUMMARY_LINES)
    c.fillStyle = bg
    c.strokeStyle = color   // 边框始终用块主题色；选中仅加粗 + 同色阴影
    c.lineWidth = isSel ? 2 / s : 1 / s
    if (isSel) { c.shadowColor = color; c.shadowBlur = 16 / s }
    roundRect(n.x, n.y, BLOCK_W, h, 8)
    c.fill()
    c.stroke()
    c.shadowColor = 'transparent'
    c.shadowBlur = 0
    // 运行态高亮：当前执行块绿描边 + 淡绿填充；失败块红描边
    if (runningId.value === n.id) {
      c.fillStyle = 'rgba(82,196,26,0.12)'
      c.strokeStyle = '#52c41a'
      c.lineWidth = 2 / s
      roundRect(n.x, n.y, BLOCK_W, h, 8)
      c.fill()
      c.stroke()
    } else if (failId.value === n.id) {
      c.fillStyle = 'rgba(255,77,79,0.12)'
      c.strokeStyle = '#ff4d4f'
      c.lineWidth = 2 / s
      roundRect(n.x, n.y, BLOCK_W, h, 8)
      c.fill()
      c.stroke()
    }
    c.fillStyle = text
    c.font = `500 ${13 / s}px system-ui`
    c.textBaseline = 'middle'
    c.textAlign = 'left'
    // 凭据块标题显示所绑定凭据名；其余显示类型 label
    const title = n.type === 'credential'
      ? (String(cfg.credentialName ?? '') || '凭据')
      : (def?.label ?? n.type)
    if (showTitle) c.fillText(title, n.x + 14 / s, n.y + 14 / s)
    // 配置摘要（次级色多行，超宽省略；块随行数拉高；小尺度隐藏）
    if (showSummary && summary.length) {
      c.fillStyle = sub
      c.font = `400 ${11.5 / s}px system-ui`
      const maxW = BLOCK_W - 26 / s
      summary.forEach((line, i) => {
        c.fillText(ellipsisText(c, line, maxW), n.x + 14 / s, n.y + 34 / s + i * (14 / s))
      })
    }
    // 右上角状态标记（纯几何圆点，非 emoji）：固定=金色 / 凭据已配置=绿色
    let badgeX = n.x + BLOCK_W - 10 / s
    if (n.locked) {
      c.beginPath()
      c.arc(badgeX, n.y + 10 / s, 3.5 / s, 0, Math.PI * 2)
      c.fillStyle = '#e6a23c'
      c.fill()
      badgeX -= 12 / s
    }
    if (n.type === 'credential' && (n.config as Record<string, unknown>)?.credentialName) {
      c.beginPath()
      c.arc(badgeX, n.y + 10 / s, 3.5 / s, 0, Math.PI * 2)
      c.fillStyle = '#52c41a'
      c.fill()
    }
    const { inputs, outputs } = getPorts(n)
    inputs.forEach((p, i) => {
      drawPort(n.x, n.y + (h / (inputs.length + 1)) * (i + 1), portColor(p, ec.get(portKey(n.id, p.id)) ?? 0), 'in')
    })
    outputs.forEach((p, i) => {
      drawPort(n.x + BLOCK_W, n.y + (h / (outputs.length + 1)) * (i + 1), portColor(p, ec.get(portKey(n.id, p.id)) ?? 0), 'out')
    })
  }
  c.textBaseline = 'alphabetic'
}

function roundRect(x: number, y: number, w: number, h: number, r: number) {
  const c = ctx2d
  if (!c) return
  c.beginPath()
  c.moveTo(x + r, y)
  c.arcTo(x + w, y, x + w, y + h, r)
  c.arcTo(x + w, y + h, x, y + h, r)
  c.arcTo(x, y + h, x, y, r)
  c.arcTo(x, y, x + w, y, r)
  c.closePath()
}

function drawPort(x: number, y: number, color: string, dir: 'in' | 'out') {
  const c = ctx2d
  if (!c) return
  const s = view.scale
  const r = 7 / s
  // 半圆连接点：圆心在块的边框线上，叠加在边框上；半圆自身盖住边框线穿过的一段
  const start = dir === 'in' ? -Math.PI / 2 : Math.PI / 2
  const end = dir === 'in' ? Math.PI / 2 : Math.PI * 3 / 2
  c.beginPath()
  c.arc(x, y, r, start, end)
  c.closePath()
  c.fillStyle = color
  c.fill()
}

function onWheel(e: WheelEvent) {
  e.preventDefault()
  const cv = canvasRef.value
  if (!cv) return
  const rect = cv.getBoundingClientRect()
  const sx = e.clientX - rect.left, sy = e.clientY - rect.top
  const world = toWorld(sx, sy)
  const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1
  const ns = Math.min(2, Math.max(0.25, view.scale * factor))
  view.ox = sx - world.x * ns
  view.oy = sy - world.y * ns
  view.scale = ns
  store.setCanvasScale(ns)
  schedule()
}

let lastX = 0, lastY = 0

function onPointerDown(e: PointerEvent) {
  const cv = canvasRef.value
  if (!cv) return
  cv.focus()
  const rect = cv.getBoundingClientRect()
  const w = toWorld(e.clientX - rect.left, e.clientY - rect.top)
  lastX = e.clientX; lastY = e.clientY

  // 1) 端口 → 连线
  const hp = hitPort(w.x, w.y)
  if (hp) {
    mode = 'connect'
    connFrom = { blockId: hp.block.id, port: hp.port }
    connCursor.value = { x: w.x, y: w.y }
    cv.setPointerCapture(e.pointerId)
    schedule()
    return
  }
  // 2) 块 → 选中 + 直接打开参数面板（不影响拖拽）
  const blk = hitBlock(w.x, w.y)
  if (blk) {
    selectedId.value = blk.id
    selectedEdgeId.value = null
    inspectNode.value = blk
    if (e.button === 0 && !blk.locked) {
      mode = 'drag'
      dragId = blk.id
      dragOffX = w.x - blk.x
      dragOffY = w.y - blk.y
      dragPushed = false
      cv.setPointerCapture(e.pointerId)
    }
    schedule()
    return
  }
  // 2.5) 连线 → 选中（Delete 删除 / 右键删除）
  const edge = hitEdge(w.x, w.y)
  if (edge) {
    selectedEdgeId.value = edge.id
    selectedId.value = null
    if (e.button === 0) {
      mode = 'pan'
      cv.setPointerCapture(e.pointerId)
    }
    schedule()
    return
  }
  // 3) 空白 → 平移 + 取消选中/收起参数
  selectedId.value = null
  selectedEdgeId.value = null
  inspectNode.value = null
  if (e.button === 0 || e.button === 1) {
    mode = 'pan'
    cv.setPointerCapture(e.pointerId)
  }
  schedule()
}

function onPointerMove(e: PointerEvent) {
  const cv = canvasRef.value
  if (!cv) return
  const rect = cv.getBoundingClientRect()
  const w = toWorld(e.clientX - rect.left, e.clientY - rect.top)

  if (mode === 'pan') {
    view.ox += e.clientX - lastX
    view.oy += e.clientY - lastY
    lastX = e.clientX; lastY = e.clientY
    schedule()
  } else if (mode === 'drag') {
    const node = store.current?.nodes.find(n => n.id === dragId)
    if (!node) return
    if (!dragPushed) {
      const moved = Math.hypot((w.x - dragOffX) - node.x, (w.y - dragOffY) - node.y) * view.scale
      if (moved > 2) { store.beginEdit(); dragPushed = true }
    }
    if (dragPushed) {
      store.moveNode(dragId, snap(w.x - dragOffX), snap(w.y - dragOffY))
    }
  } else if (mode === 'connect') {
    connCursor.value = { x: w.x, y: w.y }
    schedule()
  }
}

function onPointerUp(e: PointerEvent) {
  const cv = canvasRef.value
  if (cv) { try { cv.releasePointerCapture(e.pointerId) } catch { /* ignore */ } }
  if (mode === 'connect' && connFrom) {
    const rect = cv!.getBoundingClientRect()
    const w = toWorld(e.clientX - rect.left, e.clientY - rect.top)
    const hit = hitPort(w.x, w.y)
    if (hit && hit.block.id !== connFrom.blockId && compatible(connFrom.port, hit.port)) {
      store.connectEdge(connFrom.blockId, connFrom.port.id, hit.block.id, hit.port.id)
    }
  }
  mode = 'idle'
  dragId = ''
  dragPushed = false
  connFrom = null
  schedule()
}

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
    e.preventDefault()
    if (e.shiftKey) store.redo(); else store.undo()
    return
  }
  if (e.key === 'Delete' || e.key === 'Backspace') {
    if (selectedEdgeId.value) {
      store.removeEdge(selectedEdgeId.value)
      selectedEdgeId.value = null
      e.preventDefault()
    } else if (selectedId.value) {
      store.removeNode(selectedId.value)
      selectedId.value = null
      inspectNode.value = null
      e.preventDefault()
    }
  }
}

/** 右键：命中积木 → 操作菜单；命中连线 → 删除连线菜单；空白 → 阻止默认菜单 */
function onContextMenu(e: MouseEvent) {
  const cv = canvasRef.value
  if (!cv) return
  const rect = cv.getBoundingClientRect()
  const w = toWorld(e.clientX - rect.left, e.clientY - rect.top)
  const blk = hitBlock(w.x, w.y)
  if (blk) { openContext(e, blk); return }
  const edge = hitEdge(w.x, w.y)
  if (edge) { openEdgeContext(e, edge); return }
  e.preventDefault()
}

// 积木/连线变化 → 重绘
watch(() => store.current?.nodes, () => schedule(), { deep: true })
watch(() => store.current?.edges, () => schedule(), { deep: true })

/** 调色板拖拽落点：画布内则在该位置添加积木（world 坐标，位置用户指定） */
function handleDrop(p: { type: string; clientX: number; clientY: number }) {
  const cv = canvasRef.value
  if (!cv) return
  const rect = cv.getBoundingClientRect()
  if (p.clientX < rect.left || p.clientX > rect.right || p.clientY < rect.top || p.clientY > rect.bottom) return
  const w = toWorld(p.clientX - rect.left, p.clientY - rect.top)
  const node = store.addNodeAt(p.type, w.x, w.y)
  if (node) ensureNodeVisible(node)
}

onMounted(() => {
  const cv = canvasRef.value
  if (!cv) return
  ctx2d = cv.getContext('2d')
  dpr = window.devicePixelRatio || 1
  cv.addEventListener('wheel', onWheel, { passive: false })
  cv.addEventListener('pointerdown', onPointerDown)
  cv.addEventListener('pointermove', onPointerMove)
  cv.addEventListener('pointerup', onPointerUp)
  cv.addEventListener('keydown', onKeydown)
  cv.addEventListener('contextmenu', onContextMenu)
  resizeObs = new ResizeObserver(() => schedule())
  resizeObs.observe(cv)
  // 主题（明暗）切换时重绘，块背景/文字跟随 --jc-* 变量
  themeObs = new MutationObserver(() => schedule())
  themeObs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
  // 凭据（登录）数据刷新（automation 数据由列表视图 load 一次，避免覆盖内存编辑）
  store.credentialLoad()
  // 订阅运行事件（画布高亮 + 状态条）；实时日志由 MainPanel 全局监听写入 store.liveSteps
  listen<Record<string, unknown>>('automation-event', onAutoEvent)
    .then(fn => { unlistenAuto = fn })
    .catch(e => console.error('automation-event listen failed', e))
  store.setDropHandler(handleDrop)
  store.setCanvasScale(view.scale)
  schedule()
})

onBeforeUnmount(() => {
  store.setDropHandler(null)
  if (raf) cancelAnimationFrame(raf)
  unlistenAuto?.()
  resizeObs?.disconnect()
  themeObs?.disconnect()
  const cv = canvasRef.value
  if (cv) {
    cv.removeEventListener('wheel', onWheel)
    cv.removeEventListener('pointerdown', onPointerDown)
    cv.removeEventListener('pointermove', onPointerMove)
    cv.removeEventListener('pointerup', onPointerUp)
    cv.removeEventListener('keydown', onKeydown)
    cv.removeEventListener('contextmenu', onContextMenu)
  }
})
</script>

<template>
  <div class="automation-editor">
    <div class="ae-bar">
      <JcButton size="small" @click="backWithCheck">← 返回</JcButton>
      <JcInput
        beam glow
        v-if="store.current"
        :model-value="store.current.name"
        class="ae-name"
        @update:model-value="(v) => store.rename(store.current!.id, String(v))"
      />
      <span v-if="store.dirty" class="ae-dirty" title="有未保存的修改">●</span>
      <div class="ae-acts">
        <JcButton size="small" :disabled="!store.canUndo" @click="store.undo()">撤回</JcButton>
        <JcButton size="small" :disabled="!store.canRedo" @click="store.redo()">重做</JcButton>
        <JcButton size="small" type="primary" @click="saveWithCheck">保存</JcButton>
        <JcButton size="small" @click="openExport()">导出</JcButton>
        <JcButton size="small" @click="copyCurrentId" :title="store.current?.id">复制 ID</JcButton>
        <JcButton size="small" @click="doRun()">运行</JcButton>
        <JcButton size="small" :disabled="!runningRunId" @click="doStop()">停止</JcButton>
      </div>
    </div>
    <div ref="mainRef" class="ae-main">
      <div class="ae-top">
        <div class="ae-canvas-wrap">
          <canvas ref="canvasRef" class="ae-canvas" tabindex="0"></canvas>
          <template v-if="mmVisible">
            <button class="ae-mm-close" title="隐藏小地图" @click="mmVisible = false" :style="{ bottom: MM_H - 20 + 'px' }">✕</button>
            <button class="ae-mm-zoom" title="切换小地图画幅（1×/2×）" @click="mmZoom = mmZoom === 1 ? 2 : 1" :style="{ bottom: MM_H - 20 + 'px' }">{{ mmZoom }}×</button>
            <canvas ref="mmRef" class="ae-minimap" :style="{ width: MM_W + 'px', height: MM_H + 'px' }" @pointerdown="onMinimapDown" @pointermove="onMinimapMove" @pointerup="onMinimapUp"></canvas>
          </template>
          <button v-else class="ae-mm-toggle" title="打开小地图" @click="mmVisible = true">
            <svg viewBox="0 0 1024 1024" width="28" height="28" xmlns="http://www.w3.org/2000/svg">
              <path d="M356.864 577.92a31.146667 31.146667 0 0 0 25.429333 13.226667h0.085334a31.146667 31.146667 0 0 0 25.472-13.397334c28.416-40.704 121.856-175.104 131.413333-193.706666a178.56 178.56 0 0 0 19.584-80.256c0-98.474667-79.36-178.56-176.938667-178.56-97.536 0-176.896 80.085333-176.896 178.56 0 27.221333 6.485333 54.144 19.328 79.914666 10.24 20.394667 104.021333 153.813333 132.522667 194.218667z m545.109333-389.12a30.933333 30.933333 0 0 1 25.514667 6.826667c7.082667 5.973333 11.178667 14.848 11.178667 24.192V849.92a31.402667 31.402667 0 0 1-25.6 30.976l-228.949334 42.026667a31.658667 31.658667 0 0 1-14.037333-0.64l-290.048-82.218667-258.645333 40.96a30.890667 30.890667 0 0 1-25.130667-7.168 31.573333 31.573333 0 0 1-10.922667-23.978667V219.818667c0-17.408 13.994667-31.530667 31.232-31.530667 17.237333 0 31.189333 14.08 31.189334 31.530667v593.237333l208.170666-32.981333V660.906667a20.906667 20.906667 0 0 1 20.778667-20.992c11.52 0 20.821333 9.386667 20.821333 20.992v118.698666l270.592 76.714667v-263.68l-141.44-43.946667a21.034667 21.034667 0 0 1-13.738666-26.24 20.821333 20.821333 0 0 1 26.026666-13.909333l129.152 40.106667V292.949333a31.786667 31.786667 0 0 1-4.650666-1.152l-65.322667-21.930666a31.573333 31.573333 0 0 1-19.797333-39.850667 31.104 31.104 0 0 1 39.509333-19.925333l57.813333 19.370666 226.304-40.618666zM709.717333 854.186667l166.528-30.592v-258.090667l-166.528 29.269333v259.413334z m166.528-331.306667V257.365333l-166.528 29.866667v264.832l166.528-29.226667zM381.866667 188.245333c63.146667 0 114.474667 51.84 114.474666 115.541334 0 17.066667-4.522667 35.754667-12.458666 51.114666-5.461333 10.197333-52.693333 79.36-101.76 150.101334C337.066667 440.661333 286.037333 366.677333 280.106667 355.285333a115.072 115.072 0 0 1-12.672-51.498666c0-63.701333 51.370667-115.541333 114.474666-115.541334z m26.026666 161.066667a51.626667 51.626667 0 0 1-52.053333 0 52.608 52.608 0 0 1-26.026667-45.482667c0-18.773333 9.941333-36.096 26.026667-45.482666a51.626667 51.626667 0 0 1 52.053333 0c16.085333 9.386667 26.026667 26.709333 26.026667 45.482666 0 18.773333-9.941333 36.096-26.026667 45.482667z" fill="currentColor"></path>
            </svg>
          </button>
        </div>
        <div v-if="inspectNode" class="ae-inspector">
          <div class="ae-ins-head">
            <span>参数</span>
            <button class="ae-ins-close" title="关闭" @click="inspectNode = null">✕</button>
          </div>
          <InspectorPanel :node="inspectNode" @configure-credential="onConfigureCredential" />
        </div>
      </div>
      <!-- 分割条：拖拽调整日志高度 -->
      <div v-if="showRunLog" class="ae-splitbar" @mousedown="onLogBarDown" title="拖拽调整日志高度"></div>
      <!-- 运行日志（下方：每个积木执行实时记录，来自全局 store.liveSteps） -->
      <div v-if="showRunLog" class="ae-runlog" :style="{ height: runLogH + 'px' }">
        <div class="ae-runlog-head">
          <span class="ae-runlog-title">运行日志</span>
          <button class="ae-runlog-clear" @click="store.clearLive()">清空</button>
        </div>
        <!-- 实时命令输出（仿终端）：长命令执行中滚动显示，不再假死 -->
        <pre ref="runlogOutEl" v-if="store.liveOutput" class="ae-runlog-out">{{ store.liveOutput }}</pre>
        <div class="ae-runlog-body">
          <div v-for="(s, i) in store.liveSteps" :key="i" class="ae-rl-step" :class="s.status">
            <span class="ae-rl-dot" :style="{ background: getBlockColor(s.blockType) }"></span>
            <span class="ae-rl-name">{{ s.name }}</span>
            <span class="ae-rl-status" :class="s.status">{{ s.status === 'ok' ? 'OK' : 'FAIL' }}</span>
            <span class="ae-rl-dur">{{ fmtDur(s.durationMs) }}</span>
            <span v-if="s.exitCode !== null" class="ae-rl-code">码 {{ s.exitCode }}</span>
            <span v-if="s.auth" class="ae-rl-auth" :title="`凭据：${s.auth}`">鉴权 {{ s.auth }}</span>
            <span v-if="s.iteration !== undefined" class="ae-rl-iter">#{{ s.iteration }}</span>
            <div class="ae-rl-detail">
              <div v-if="s.detail" class="ae-rl-line"><span class="lbl">执行</span>{{ s.detail }}</div>
              <div v-if="s.cwd" class="ae-rl-line"><span class="lbl">目录</span>{{ s.cwd }}</div>
              <div v-if="s.stdoutTail" class="ae-rl-line"><span class="lbl">输出</span><span class="mono">{{ s.stdoutTail }}</span></div>
            </div>
          </div>
        </div>
      </div>
      <!-- 运行状态条 -->
      <div v-if="runStep > 0 || failId" class="ae-runbar">
        <span class="ae-run-step">{{ failId ? '失败' : '运行中' }} {{ runStep }}/{{ runTotal }} {{ runName }}</span>
        <span v-if="runIter > 0" class="ae-run-iter">循环 #{{ runIter }}</span>
        <span v-if="runTail" class="ae-run-tail">{{ runTail }}</span>
        <span v-if="Object.keys(runVars).length" class="ae-run-vars">{{ JSON.stringify(runVars) }}</span>
      </div>
    </div>

    <!-- 画布右键菜单 -->
    <JcContextMenu
      v-model:show="ctxShow"
      :x="ctxX"
      :y="ctxY"
      :items="ctxMenuItems"
      @select="onCtxSelect"
    />

    <!-- 登录 / 凭据绑定 -->
    <LoginDialog v-model:open="loginOpen" :node="loginNode" />

    <!-- 导出完整 JSON -->
    <JcModal :open="exportOpen" title="导出工作积木" width="560" @update:open="exportOpen = $event">
      <JcTextarea :model-value="exportText" :rows="14" readonly :spellcheck="false" />
      <template #footer>
        <JcButton @click="copyExport">复制</JcButton>
        <JcButton type="primary" @click="saveExportFile">保存为文件</JcButton>
      </template>
    </JcModal>

    <!-- 保存/返回前校验提示 -->
    <JcModal :open="!!confirmBox" title="提示" width="460" @update:open="(v: boolean) => { if (!v) confirmBox = null }">
      <ul class="ae-confirm-items">
        <li v-for="(it, i) in confirmBox?.items ?? []" :key="i">{{ it }}</li>
      </ul>
      <template #footer>
        <JcButton @click="confirmBox = null">取消</JcButton>
        <JcButton v-if="confirmBox?.withSave" @click="confirmSaveAndBack">保存并返回</JcButton>
        <JcButton type="primary" @click="confirmBox?.onConfirm()">{{ confirmBox?.confirmText ?? '确定' }}</JcButton>
      </template>
    </JcModal>
  </div>
</template>

<style scoped lang="scss">
.automation-editor {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ae-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
}
.ae-name {
  flex: 1;
  min-width: 0;
}
.ae-acts {
  display: flex;
  gap: 6px;
}
.ae-dirty {
  flex-shrink: 0;
  color: var(--jc-color-warning, #ff9c6e);
  font-size: 12px;
  line-height: 1;
}
.ae-confirm-items {
  margin: 0;
  padding-left: 18px;
  color: var(--jc-text-primary, #ddd);
  font-size: 13px;
  line-height: 1.9;
}
.ae-confirm-items li::marker {
  color: var(--jc-color-warning, #ff9c6e);
}
.ae-canvas-wrap {
  flex: 1;
  min-width: 0;
  position: relative;
}
.ae-canvas {
  width: 100%;
  height: 100%;
  display: block;
  cursor: grab;
  touch-action: none;
  outline: none;
}
.ae-minimap {
  position: absolute;
  right: 12px;
  bottom: 12px;
  border-radius: 8px;
  border: 1px solid rgba(128,128,128,0.35);
  box-shadow: 0 2px 8px rgba(0,0,0,0.4);
  cursor: pointer;
  z-index: 10;
  background: var(--jc-bg-elevated, rgba(24,24,26,0.82));
  pointer-events: auto;
}
.ae-mm-close {
  position: absolute;
  right: 15px;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: none;
  background: rgba(255,255,255,0.12);
  color: rgba(255,255,255,0.85);
  font-size: 11px;
  line-height: 1;
  cursor: pointer;
  z-index: 11;
  pointer-events: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  &:hover { background: rgba(255,255,255,0.28); }
}
.ae-mm-zoom {
  position: absolute;
  right: 36px;
  height: 18px;
  min-width: 24px;
  padding: 0 4px;
  border-radius: 4px;
  border: none;
  background: rgba(128,128,128,0.25);
  color: var(--jc-text-primary, #e6e6e6);
  font-size: 10px;
  line-height: 1;
  cursor: pointer;
  z-index: 11;
  pointer-events: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  &:hover { background: rgba(128,128,128,0.45); }
}
.ae-mm-toggle {
  position: absolute;
  right: 10px;
  bottom: 10px;
  width: 44px;
  height: 44px;
  border-radius: 12px;
  border: none;
  background: transparent;
  color: var(--jc-text-primary, #e6e6e6);
  cursor: pointer;
  z-index: 10;
  pointer-events: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  &:hover { background: rgba(128,128,128,0.18); }
}
.ae-main {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ae-top {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}
.ae-splitbar {
  height: 5px;
  flex-shrink: 0;
  cursor: row-resize;
  background: var(--jc-bg-panel, #252526);
  border-top: 1px solid var(--jc-border-default, #3e3e42);
  transition: background 0.15s;
  &:hover { background: var(--jc-color-accent, #8a58ff); }
}
.ae-inspector {
  width: 264px;
  flex-shrink: 0;
  border-left: 1px solid var(--jc-border-default, #3e3e42);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.ae-ins-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary, #e6e6e6);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  background: var(--jc-bg-panel, #252526);
}
.ae-ins-close {
  background: none;
  border: none;
  color: var(--jc-text-secondary, #aaa);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
}
.ae-ins-close:hover {
  color: var(--jc-text-primary, #e6e6e6);
  background: var(--jc-bg-hover, #2a2d2e);
}
.ae-inspector > .inspector {
  flex: 1;
  min-height: 0;
}
.ae-runbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 4px 12px;
  font-size: 11px;
  color: var(--jc-text-secondary, #aaa);
  background: var(--jc-bg-panel, #252526);
  border-top: 1px solid var(--jc-border-default, #3e3e42);
  overflow: hidden;
  white-space: nowrap;
}
.ae-run-step {
  flex-shrink: 0;
  color: #52c41a;
  font-weight: 600;
}
.ae-run-tail,
.ae-run-vars {
  overflow: hidden;
  text-overflow: ellipsis;
}
.ae-run-vars {
  color: var(--jc-text-tertiary, #858585);
}
/* 运行日志（下方面板：实时 step_log，高度可拖拽分割） */
.ae-runlog {
  flex-shrink: 0;
  border-top: 1px solid var(--jc-border-default, #3e3e42);
  background: var(--jc-bg-panel, #252526);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ae-runlog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  flex-shrink: 0;
}
.ae-runlog-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary, #e6e6e6);
}
.ae-runlog-clear {
  background: none;
  border: none;
  color: var(--jc-text-secondary, #aaa);
  font-size: 11px;
  cursor: pointer;
  padding: 2px 6px;
  &:hover { color: var(--jc-text-primary, #e6e6e6); }
}
.ae-runlog-out {
  flex-shrink: 0;
  max-height: 140px;
  overflow: auto;
  scrollbar-gutter: stable;
  margin: 0;
  padding: 6px 10px;
  font-family: ui-monospace, Consolas, 'Courier New', monospace;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--jc-text-secondary, #aaa);
  background: var(--jc-bg-input, #2b2b2e);
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
  white-space: pre-wrap;
  word-break: break-all;
}
.ae-runlog-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 6px 10px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ae-rl-step {
  display: flex;
  align-items: flex-start;
  flex-wrap: wrap;
  gap: 4px 8px;
  padding: 5px 8px;
  border-radius: 6px;
  background: var(--jc-bg-elevated, #2d2d30);
  font-size: 12px;
  &.fail { outline: 1px solid rgba(255, 77, 79, .35); }
  .ae-rl-dot { width: 8px; height: 8px; border-radius: 50%; margin-top: 4px; flex-shrink: 0; }
  .ae-rl-name { color: var(--jc-text-primary, #e6e6e6); font-weight: 500; }
  .ae-rl-status {
    font-size: 11px;
    &.fail { color: var(--jc-color-error, #ff4d4f); }
  }
  .ae-rl-dur,
  .ae-rl-code,
  .ae-rl-iter { font-size: 11px; color: var(--jc-text-tertiary, #858585); }
  .ae-rl-auth { font-size: 11px; color: var(--jc-color-warning, #faad14); }
  .ae-rl-detail {
    flex-basis: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    .ae-rl-line {
      font-size: 11px;
      color: var(--jc-text-secondary, #aaa);
      display: flex;
      gap: 6px;
      .lbl { color: var(--jc-text-tertiary, #858585); flex-shrink: 0; }
      .mono { font-family: var(--jc-font-mono, ui-monospace, monospace); word-break: break-all; }
    }
  }
}
</style>
