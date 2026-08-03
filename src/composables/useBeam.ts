import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { JcBorderBeamColor } from '../components/ui/JcBorderBeam.vue'

/** 流光渐变尾部占位（percent 0~100 映射到此，为尾部透明过渡预留空间） */
export const BEAM_COLOR_STOP_PERCENT = 70

/** 构建流光渐变（对齐 antd util.ts）：percent 0~100 → 0~70，尾部补透明渐变 */
export function buildBeamGradient(
  color: JcBorderBeamColor | undefined,
  angle: string,
): string | undefined {
  if (!color) return undefined
  const items = Array.isArray(color) ? color : [{ color, percent: 0 }]
  if (items.length === 0) return undefined

  const mapped = items.map((it) => ({
    color: it.color,
    percent: Number(
      ((Math.min(Math.max(it.percent, 0), 100) / 100) * BEAM_COLOR_STOP_PERCENT).toFixed(2),
    ),
  }))
  const last = mapped[mapped.length - 1]
  const stops =
    last.percent >= BEAM_COLOR_STOP_PERCENT
      ? mapped
      : [...mapped, { color: last.color, percent: BEAM_COLOR_STOP_PERCENT }]

  return `linear-gradient(${angle}, ${stops.map((s) => `${s.color} ${s.percent}%`).join(', ')}, transparent)`
}

export interface UseBeamOptions {
  /** 是否开启流光 */
  enabled: () => boolean
  /** 流光颜色（单色或渐变停靠点数组） */
  color: () => JcBorderBeamColor | undefined
  /** 渐变方向，如 'to left' 或 '-225deg' */
  angle: () => string
  /** 拐角变速：true=拐角轻微加速 / false=匀速 */
  accelerate: () => boolean
  /** 宿主元素（用于测量宽度计算光束长度） */
  root: () => HTMLElement | undefined
  /** 光束长度 = 宿主宽度 × 比例（如 0.4）；细长条高度固定 < 高度，避免上下双轨 */
  sizeRatio: () => number
}

/**
 * 表单控件共用流光逻辑（JcInput / JcTextarea / JcSelect）：
 * 渐变构建 + 光束长度随宽度实时变化（ResizeObserver）+ beamStyle 变量。
 * 输出与全局 `.jc-beam` 共享样式约定的 CSS 变量对齐：
 * `--jc-beam-size` / `--jc-beam-gradient` / `--jc-beam-anim`
 */
export function useBeam(opts: UseBeamOptions) {
  const beamGradient = computed(() => buildBeamGradient(opts.color(), opts.angle()))

  const beamSizeRef = ref('100px')
  let ro: ResizeObserver | undefined

  function syncBeamSize() {
    const el = opts.root()
    if (!opts.enabled() || !el) return
    beamSizeRef.value = `${Math.round(el.clientWidth * opts.sizeRatio())}px`
  }

  onMounted(() => {
    if (!opts.enabled()) return
    syncBeamSize()
    const el = opts.root()
    if (el) {
      ro = new ResizeObserver(syncBeamSize)
      ro.observe(el)
    }
  })
  onBeforeUnmount(() => ro?.disconnect())

  const beamStyle = computed(() => ({
    '--jc-beam-size': beamSizeRef.value,
    '--jc-beam-gradient': beamGradient.value,
    '--jc-beam-anim': opts.accelerate() ? 'jc-beam-move-acc' : 'jc-beam-move',
  }) as Record<string, string>)

  return { beamGradient, beamStyle }
}
