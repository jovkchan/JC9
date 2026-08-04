<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useAutomationStore } from '@/stores/automation'
import { getBlockDef, getBlockColor } from '@/components/automation/blocks/palette'
import type { BlockNode, Port } from '@/types/automation'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'

const store = useAutomationStore()

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

function snap(v: number) { return Math.round(v / 8) * 8 }

// ── 端口几何 ──
function getPorts(block: { type: string }) {
  const def = getBlockDef(block.type)
  return { inputs: def?.inputs ?? [], outputs: def?.outputs ?? [] }
}
function portPos(block: { type: string; x: number; y: number }, p: Port) {
  const { inputs, outputs } = getPorts(block)
  if (p.direction === 'in') {
    const i = inputs.findIndex(x => x.id === p.id)
    return { x: block.x, y: block.y + (BLOCK_H / (inputs.length + 1)) * (i + 1) }
  }
  const j = outputs.findIndex(x => x.id === p.id)
  return { x: block.x + BLOCK_W, y: block.y + (BLOCK_H / (outputs.length + 1)) * (j + 1) }
}

// ── 命中检测 ──
function hitBlock(wx: number, wy: number): BlockNode | null {
  const nodes = store.current?.nodes ?? []
  for (let k = nodes.length - 1; k >= 0; k--) {
    const n = nodes[k]
    if (wx >= n.x && wx <= n.x + BLOCK_W && wy >= n.y && wy <= n.y + BLOCK_H) return n
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
    c.strokeStyle = portColor(fPort, ec.get(portKey(edge.fromBlock, edge.fromPort)) ?? 0)
    c.globalAlpha = 0.85
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

const BLOCK_W = 200
const BLOCK_H = 60

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
  // 主题感知：块背景/文字随明暗模式切换
  const bg = cssVar('--jc-bg-elevated') || '#2d2d30'
  const text = cssVar('--jc-text-primary') || '#e6e6e6'
  for (const n of a.nodes) {
    const def = getBlockDef(n.type)
    const color = getBlockColor(n.type)
    const isSel = n.id === selectedId.value
    c.fillStyle = bg
    c.strokeStyle = color   // 边框始终用块主题色；选中仅加粗 + 同色阴影
    c.lineWidth = isSel ? 2 / s : 1 / s
    if (isSel) { c.shadowColor = color; c.shadowBlur = 16 / s }
    roundRect(n.x, n.y, BLOCK_W, BLOCK_H, 8)
    c.fill()
    c.stroke()
    c.shadowColor = 'transparent'
    c.shadowBlur = 0
    c.fillStyle = text
    c.font = `500 ${13 / s}px system-ui`
    c.textBaseline = 'middle'
    c.textAlign = 'left'
    c.fillText(def?.label ?? n.type, n.x + 14 / s, n.y + 14 / s)
    const { inputs, outputs } = getPorts(n)
    inputs.forEach((p, i) => {
      drawPort(n.x, n.y + (BLOCK_H / (inputs.length + 1)) * (i + 1), portColor(p, ec.get(portKey(n.id, p.id)) ?? 0), 'in')
    })
    outputs.forEach((p, i) => {
      drawPort(n.x + BLOCK_W, n.y + (BLOCK_H / (outputs.length + 1)) * (i + 1), portColor(p, ec.get(portKey(n.id, p.id)) ?? 0), 'out')
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
  // 2) 块 → 选中 + 拖拽
  const blk = hitBlock(w.x, w.y)
  if (blk) {
    selectedId.value = blk.id
    if (e.button === 0) {
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
  // 3) 空白 → 平移 + 取消选中
  selectedId.value = null
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
    if (selectedId.value) {
      store.removeNode(selectedId.value)
      selectedId.value = null
      e.preventDefault()
    }
  }
}

// 积木/连线变化 → 重绘
watch(() => store.current?.nodes, () => schedule(), { deep: true })
watch(() => store.current?.edges, () => schedule(), { deep: true })

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
  resizeObs = new ResizeObserver(() => schedule())
  resizeObs.observe(cv)
  // 主题（明暗）切换时重绘，块背景/文字跟随 --jc-* 变量
  themeObs = new MutationObserver(() => schedule())
  themeObs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })
  schedule()
})

onBeforeUnmount(() => {
  if (raf) cancelAnimationFrame(raf)
  resizeObs?.disconnect()
  themeObs?.disconnect()
  const cv = canvasRef.value
  if (cv) {
    cv.removeEventListener('wheel', onWheel)
    cv.removeEventListener('pointerdown', onPointerDown)
    cv.removeEventListener('pointermove', onPointerMove)
    cv.removeEventListener('pointerup', onPointerUp)
    cv.removeEventListener('keydown', onKeydown)
  }
})
</script>

<template>
  <div class="automation-editor">
    <div class="ae-bar">
      <JcButton size="small" @click="store.closeEditor()">← 返回</JcButton>
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
        <JcButton size="small" type="primary" @click="store.save()">保存</JcButton>
        <JcButton size="small" @click="store.current && store.run(store.current.id)">运行</JcButton>
        <JcButton size="small" disabled>停止</JcButton>
      </div>
    </div>
    <div class="ae-canvas-wrap">
      <canvas ref="canvasRef" class="ae-canvas" tabindex="0"></canvas>
    </div>
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
.ae-canvas-wrap {
  flex: 1;
  min-height: 0;
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
</style>
