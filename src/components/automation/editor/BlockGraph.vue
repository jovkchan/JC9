<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getBlockColor, getBlockLabel } from '@/components/automation/blocks/palette'
import { BLOCK_W, blockHeight, blockSummary, MAX_SUMMARY_LINES } from '@/components/automation/blocks/summary'
import type { BlockNode, Edge } from '@/types/automation'

const props = defineProps<{
  nodes: BlockNode[]
  edges: Edge[]
  activeId?: string | null
  failId?: string | null
  doneIds?: string[]
}>()

const canvasRef = ref<HTMLCanvasElement | null>(null)
let ro: ResizeObserver | null = null
let anim: number | null = null

/** 内容边界 */
const bounds = computed(() => {
  let minX = 0, minY = 0, maxX = BLOCK_W + 40, maxY = 100
  for (const n of props.nodes) {
    minX = Math.min(minX, n.x)
    minY = Math.min(minY, n.y)
    maxX = Math.max(maxX, n.x + BLOCK_W)
    maxY = Math.max(maxY, n.y + blockHeight(n.type, n.config))
  }
  return { minX, minY, maxX, maxY }
})

const view = ref({ scale: 0.9, ox: 40, oy: 40 })

function cssVar(name: string, fb: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fb
}

function roundRect(g: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rr = Math.max(0, Math.min(r, w / 2, h / 2))
  g.beginPath()
  g.moveTo(x + rr, y)
  g.arcTo(x + w, y, x + w, y + h, rr)
  g.arcTo(x + w, y + h, x, y + h, rr)
  g.arcTo(x, y + h, x, y, rr)
  g.arcTo(x, y, x + w, y, rr)
  g.closePath()
}

function ellipsis(s: string, maxW: number, g: CanvasRenderingContext2D): string {
  if (g.measureText(s).width <= maxW) return s
  while (s.length > 1 && g.measureText(s + '…').width > maxW) s = s.slice(0, -1)
  return s + '…'
}

function draw() {
  const cv = canvasRef.value
  if (!cv) return
  const g = cv.getContext('2d')
  if (!g) return
  const W = cv.clientWidth, H = cv.clientHeight
  if (cv.width !== W) cv.width = W
  if (cv.height !== H) cv.height = H
  g.clearRect(0, 0, W, H)

  const { scale, ox, oy } = view.value
  const tx = (x: number) => ox + x * scale
  const ty = (y: number) => oy + y * scale
  const s = (v: number) => v * scale

  // 网格
  g.strokeStyle = 'rgba(138,88,255,.07)'
  g.lineWidth = 1
  const gs = 20 * scale
  for (let x = ((ox % gs) + gs) % gs; x < W; x += gs) { g.beginPath(); g.moveTo(x, 0); g.lineTo(x, H); g.stroke() }
  for (let y = ((oy % gs) + gs) % gs; y < H; y += gs) { g.beginPath(); g.moveTo(0, y); g.lineTo(W, y); g.stroke() }

  // 连线（贝塞尔）
  const nb = new Map(props.nodes.map(n => [n.id, n]))
  g.strokeStyle = 'rgba(120,120,140,.5)'
  g.lineWidth = Math.max(1, 1.5 * scale)
  g.lineCap = 'round'
  for (const e of props.edges) {
    const a = nb.get(e.fromBlock), b = nb.get(e.toBlock)
    if (!a || !b) continue
    const x1 = tx(a.x + BLOCK_W), y1 = ty(a.y + blockHeight(a.type, a.config) / 2)
    const x2 = tx(b.x), y2 = ty(b.y + blockHeight(b.type, b.config) / 2)
    const mx = (x1 + x2) / 2
    g.beginPath()
    g.moveTo(x1, y1)
    g.bezierCurveTo(mx, y1, mx, y2, x2, y2)
    g.stroke()
  }

  // 积木块
  const doneSet = new Set(props.doneIds ?? [])
  const r = 6 * scale
  const fs = 12 * scale
  for (const n of props.nodes) {
    const hh = blockHeight(n.type, n.config)
    const summary = blockSummary(n.type, n.config).slice(0, MAX_SUMMARY_LINES)
    const x = tx(n.x), y = ty(n.y), w = s(BLOCK_W), h = s(hh)
    const active = n.id === props.activeId
    const fail = n.id === props.failId
    const done = doneSet.has(n.id)

    // 背景
    roundRect(g, x, y, w, h, r)
    g.fillStyle = cssVar('--jc-bg-elevated', '#2d2d30')
    g.fill()

    // 边框 / 高亮
    if (active) { g.strokeStyle = '#52c41a'; g.lineWidth = 2; g.shadowColor = 'rgba(82,196,26,.55)'; g.shadowBlur = 12 * scale }
    else if (fail) { g.strokeStyle = '#ff4d4f'; g.lineWidth = 1.5; g.shadowColor = 'rgba(255,77,79,.45)'; g.shadowBlur = 10 * scale }
    else if (done) { g.strokeStyle = 'rgba(82,196,26,.45)'; g.lineWidth = 1 }
    else { g.strokeStyle = 'rgba(120,120,140,.4)'; g.lineWidth = 1 }
    roundRect(g, x, y, w, h, r)
    g.stroke()
    g.shadowBlur = 0

    // 类型色条
    const barW = 4 * scale
    roundRect(g, x + barW / 2, y + h * 0.25, barW, h * 0.5, 2 * scale)
    g.fillStyle = getBlockColor(n.type)
    g.fill()

    // 标题（无摘要时垂直居中；有摘要时置顶，下方排摘要行）
    g.font = `500 ${fs}px system-ui, sans-serif`
    g.textBaseline = 'middle'
    const sub = cssVar('--jc-text-secondary', 'rgba(196,196,196,0.78)')
    g.fillStyle = cssVar('--jc-text-primary', '#e6e6e6')
    const label = getBlockLabel(n.type)
    const badgeW = active || fail || done ? 44 * scale : 0
    const textW = w - barW - 8 * scale - badgeW - 6 * scale
    if (summary.length) {
      g.fillText(ellipsis(label, Math.max(10, textW), g), x + barW + 8 * scale, y + 14 * scale)
      const fs2 = 11 * scale
      g.font = `400 ${fs2}px system-ui, sans-serif`
      g.fillStyle = sub
      const sw = w - barW - 24 * scale
      summary.forEach((line, i) => {
        g.fillText(ellipsis(line, Math.max(10, sw), g), x + barW + 8 * scale, y + 32 * scale + i * (14 * scale))
      })
    } else {
      g.fillText(ellipsis(label, Math.max(10, textW), g), x + barW + 8 * scale, y + h / 2)
    }

    // 状态角标
    if (active || fail || done) {
      const badge = active ? '运行中' : fail ? '失败' : '完成'
      const bw = fs * 2.1 + 10 * scale
      const bh = fs + 6 * scale
      const bx = x + w - bw - 6 * scale
      const by = y + 6 * scale
      roundRect(g, bx, by, bw, bh, Math.round(bh / 2))
      g.fillStyle = active ? 'rgba(82,196,26,.22)' : fail ? 'rgba(255,77,79,.22)' : 'rgba(82,196,26,.13)'
      g.fill()
      g.fillStyle = active ? '#52c41a' : fail ? '#ff4d4f' : 'rgba(82,196,26,.85)'
      g.font = `${fs}px system-ui, sans-serif`
      g.fillText(badge, bx + (bw - g.measureText(badge).width) / 2, by + bh / 2)
    }
  }
}

/** 视图定位：有 active → 该块居中（平滑过渡）；否则 fit 全部 */
function layout() {
  const cv = canvasRef.value
  if (!cv) return
  const W = cv.clientWidth, H = cv.clientHeight
  const active = props.activeId ? props.nodes.find(n => n.id === props.activeId) : undefined
  if (active) {
    const toX = W / 2 - (active.x + BLOCK_W / 2) * view.value.scale
    const toY = H / 2 - (active.y + blockHeight(active.type, active.config) / 2) * view.value.scale
    if (anim) { cancelAnimationFrame(anim); anim = null }
    const fromX = view.value.ox, fromY = view.value.oy
    const t0 = performance.now(), dur = 240
    const step = (t: number) => {
      const p = Math.min(1, (t - t0) / dur)
      const e = 1 - Math.pow(1 - p, 3)
      view.value.ox = fromX + (toX - fromX) * e
      view.value.oy = fromY + (toY - fromY) * e
      draw()
      if (p < 1) anim = requestAnimationFrame(step)
      else anim = null
    }
    anim = requestAnimationFrame(step)
  } else {
    const { minX, minY, maxX, maxY } = bounds.value
    const bw = maxX - minX, bh = maxY - minY
    view.value.scale = Math.max(0.25, Math.min(0.95, Math.min((W - 40) / bw, (H - 40) / bh)))
    view.value.ox = (W - bw * view.value.scale) / 2 - minX * view.value.scale
    view.value.oy = (H - bh * view.value.scale) / 2 - minY * view.value.scale
    draw()
  }
}

watch(() => props.activeId, () => layout())
watch(() => props.nodes, () => layout(), { deep: true })
watch(() => props.failId, () => layout())

onMounted(() => {
  ro = new ResizeObserver(() => layout())
  if (canvasRef.value) ro.observe(canvasRef.value)
  layout()
})
onBeforeUnmount(() => {
  ro?.disconnect()
  if (anim) cancelAnimationFrame(anim)
})

// ── 拖动平移（整体拖动图查看，active 自动居中仍保留：运行到哪跟随到哪）──
let panning = false
let panStartX = 0, panStartY = 0, panOX = 0, panOY = 0
function onPointerDown(e: PointerEvent) {
  if (e.button !== 0) return
  panning = true
  panStartX = e.clientX
  panStartY = e.clientY
  panOX = view.value.ox
  panOY = view.value.oy
  canvasRef.value?.setPointerCapture(e.pointerId)
  if (anim) { cancelAnimationFrame(anim); anim = null }
}
function onPointerMove(e: PointerEvent) {
  if (!panning) return
  view.value.ox = panOX + (e.clientX - panStartX)
  view.value.oy = panOY + (e.clientY - panStartY)
  draw()
}
function onPointerUp() {
  panning = false
}
</script>

<template>
  <canvas
    ref="canvasRef"
    class="bg-canvas"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="panning = false"
  ></canvas>
</template>

<style scoped lang="scss">
.bg-canvas {
  flex: 1;
  min-height: 0;
  width: 100%;
  height: 100%;
  display: block;
  cursor: grab;
  &:active { cursor: grabbing; }
}
</style>
